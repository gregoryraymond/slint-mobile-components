//! Infinite-scroll page-template browser.
//!
//! The viewer Window itself is compiled (a small chrome + a grid of
//! `ComponentContainer` slots). Every page template — there are ~145 —
//! is parsed at runtime by `slint-interpreter` and embedded into one of
//! those slots via a [`slint::ComponentFactory`].
//!
//! Loading strategy: parse the first ~20 pages synchronously on
//! startup so there's something to scroll. After the window shows,
//! a `slint::Timer` ticks roughly every 16 ms and appends one more
//! parsed page to the two parallel models (`titles` + `cells`). The
//! grid in `viewer.slint` is index-positioned, so adding a cell just
//! grows the scrollable area — there's no "page X of Y" — and a
//! "loading…" indicator under the bottom row vanishes once every
//! discovered page has been parsed.
//!
//! Library paths (`@mobile-theme`, `@mobile-components`) are wired so
//! interpreted page templates resolve the same imports that the
//! compiled tests do.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

// `ComponentFactory` is the public-but-deprecated bridge between an
// interpreted `ComponentDefinition::create_embedded` and the
// `ComponentContainer` slot in compiled slint. It's the documented
// integration path; the deprecation is incidental.
#[allow(deprecated)]
use slint::ComponentFactory;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_interpreter::{ComponentDefinition, ComponentInstance, Compiler, Struct, Value};
use slint_mapping::source::TileSource;
use slint_mapping::sources::FileTileSource;

slint::include_modules!();

/// Number of pages to parse before the window first appears. The rest
/// stream in during the event loop.
const INITIAL_BATCH: usize = 20;

#[derive(Debug, Clone)]
struct PageMeta {
    path: PathBuf,
    class: String,
    display: String,
    category: String,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Walk `crates/pages-*/ui/*.slint`, skipping `_*` aggregators.
fn discover_pages(root: &Path) -> Vec<PageMeta> {
    let mut out = Vec::new();
    let crates_dir = root.join("crates");
    let Ok(entries) = std::fs::read_dir(&crates_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let Some(name) = dir.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(cat) = name.strip_prefix("pages-") else {
            continue;
        };
        let ui = dir.join("ui");
        let Ok(files) = std::fs::read_dir(&ui) else {
            continue;
        };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|s| s.to_str()) != Some("slint") {
                continue;
            }
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.starts_with('_') {
                continue;
            }
            if let Some(class) = scan_page_class(&p) {
                out.push(PageMeta {
                    path: p.clone(),
                    class,
                    display: stem.to_string(),
                    category: cat.to_string(),
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.display.cmp(&b.display))
    });
    out
}

/// Last `export component XxxPage|XxxScreen inherits …` in the file.
fn scan_page_class(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let prefix = "export component ";
    let mut hit = None;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let mut it = rest.split_whitespace();
        let Some(name) = it.next() else {
            continue;
        };
        let ends_ok = name.ends_with("Page") || name.ends_with("Screen");
        let starts_ok = name.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        if !ends_ok || !starts_ok {
            continue;
        }
        if it.next() != Some("inherits") {
            continue;
        }
        hit = Some(name.to_string());
    }
    hit
}

struct PageCompiler {
    compiler: Compiler,
}

impl PageCompiler {
    fn new(root: &Path) -> Self {
        let mut compiler = Compiler::default();
        let mut paths = HashMap::new();
        paths.insert("mobile-theme".into(), root.join("crates/theme/ui"));
        paths.insert(
            "mobile-components".into(),
            root.join("crates/components/ui"),
        );
        // `@mapping` resolves to the sibling slint-mapping repo — the 6
        // map-using pages import `MapEmbed` from there. Without this
        // entry, interpreting any of those pages fails with
        // "Cannot find requested import @mapping/map.slint".
        paths.insert("mapping".into(), PathBuf::from(slint_mapping::UI_LIBRARY_DIR));
        compiler.set_library_paths(paths);
        Self { compiler }
    }

    fn definition_for(&self, page: &PageMeta) -> Option<ComponentDefinition> {
        let result = pollster::block_on(self.compiler.build_from_path(&page.path));
        for diag in result.diagnostics() {
            eprintln!("[{}] {}", page.display, diag);
        }
        let def = result.component(&page.class);
        if def.is_none() {
            eprintln!(
                "[{}] component `{}` not found in {}",
                page.display,
                page.class,
                page.path.display(),
            );
        }
        def
    }
}

/// Parse a single page, append its factory + title to the live models,
/// and bump the loaded counter on the Window.
fn append_page(
    viewer: &Viewer,
    titles: &Rc<VecModel<SharedString>>,
    cells: &Rc<VecModel<ComponentFactory>>,
    compiler: &PageCompiler,
    map_source: &Rc<dyn TileSource>,
    page: &PageMeta,
) {
    let Some(def) = compiler.definition_for(page) else {
        return;
    };
    // Detect map-using pages by looking for the `map-tiles` property in
    // the component definition. If present, the factory attaches a
    // dynamic map handler when the cell instantiates the component;
    // other pages instantiate as-is.
    let is_map_page = def
        .properties()
        .any(|(name, _)| name == "map-tiles");
    let source_for_factory = if is_map_page {
        Some(Rc::clone(map_source))
    } else {
        None
    };
    let factory = ComponentFactory::new(move |ctx| {
        let instance = def.create_embedded(ctx).ok()?;
        if let Some(src) = &source_for_factory {
            attach_map_handler(&instance, Rc::clone(src));
        }
        Some(instance)
    });
    titles.push(SharedString::from(page.display.as_str()));
    cells.push(factory);
    viewer.set_loaded(titles.row_count() as i32);
}

/// Viewport size that the viewer paints each interpreted page into —
/// matches the `PageCell.ComponentContainer` size in `ui/viewer.slint`.
const MAP_VIEWPORT_W: f64 = 412.0;
const MAP_VIEWPORT_H: f64 = 892.0;

/// Wire a `slint_mapping::TileSource` into a freshly-loaded interpreted
/// page that exposes the canonical map-* property + callback surface
/// (`map-latitude`, `map-longitude`, `map-zoom`, `map-tiles`, `map-pan`,
/// `map-zoom-by`). Goes through slint-interpreter's dynamic property /
/// callback API so we don't need a Rust handle to the page type.
fn attach_map_handler(instance: &ComponentInstance, source: Rc<dyn TileSource>) {
    // Start centred on London at zoom 10 — the bundled sample bundle
    // includes Greater London at zoom 4-12, so this opens with real
    // street-level detail (scroll-wheel zooms to 12; below z=4 falls
    // back to the world tiles).
    let _ = instance.set_property("map-latitude", Value::Number(51.5074));
    let _ = instance.set_property("map-longitude", Value::Number(-0.1276));
    let _ = instance.set_property("map-zoom", Value::Number(10.0));

    refresh_map(instance, &source);

    // map-pan(dx, dy) — projection-correct camera shift, then refresh.
    let inst_pan = instance.clone_strong();
    let source_pan = Rc::clone(&source);
    let _ = instance.set_callback("map-pan", move |args| {
        let dx = number_arg(args, 0);
        let dy = number_arg(args, 1);
        let (lon, lat, zoom) = read_camera(&inst_pan);
        let (new_lon, new_lat) =
            slint_mapping::camera::pan(lon, lat, zoom, dx, dy, source_pan.tile_size());
        let _ = inst_pan.set_property("map-longitude", Value::Number(new_lon));
        let _ = inst_pan.set_property("map-latitude", Value::Number(new_lat));
        refresh_map(&inst_pan, &source_pan);
        Value::Void
    });

    // map-zoom-by(delta, anchor-x, anchor-y) — cursor-anchored zoom.
    let inst_zoom = instance.clone_strong();
    let source_zoom = Rc::clone(&source);
    let _ = instance.set_callback("map-zoom-by", move |args| {
        let delta = number_arg(args, 0);
        let ax = number_arg(args, 1);
        let ay = number_arg(args, 2);
        let (lon, lat, zoom) = read_camera(&inst_zoom);
        let (new_lon, new_lat, new_zoom) = slint_mapping::camera::zoom_anchored(
            lon,
            lat,
            zoom,
            delta,
            ax,
            ay,
            MAP_VIEWPORT_W,
            MAP_VIEWPORT_H,
            source_zoom.tile_size(),
            source_zoom.min_zoom(),
            source_zoom.max_zoom(),
        );
        let _ = inst_zoom.set_property("map-longitude", Value::Number(new_lon));
        let _ = inst_zoom.set_property("map-latitude", Value::Number(new_lat));
        let _ = inst_zoom.set_property("map-zoom", Value::Number(new_zoom));
        refresh_map(&inst_zoom, &source_zoom);
        Value::Void
    });
}

fn read_camera(instance: &ComponentInstance) -> (f64, f64, f64) {
    let lon = match instance.get_property("map-longitude") {
        Ok(Value::Number(n)) => n,
        _ => 0.0,
    };
    let lat = match instance.get_property("map-latitude") {
        Ok(Value::Number(n)) => n,
        _ => 0.0,
    };
    let zoom = match instance.get_property("map-zoom") {
        Ok(Value::Number(n)) => n,
        _ => 2.0,
    };
    (lon, lat, zoom)
}

fn number_arg(args: &[Value], idx: usize) -> f64 {
    match args.get(idx) {
        Some(Value::Number(n)) => *n,
        _ => 0.0,
    }
}

/// Recompute visible tiles and push them as a `Value::Model` to the
/// page's `map-tiles` property. Each tile is built as an interpreter
/// `Struct` matching the shape of `Tile { x, y, size, image }` in
/// `slint-mapping/ui/map.slint`.
fn refresh_map(instance: &ComponentInstance, source: &Rc<dyn TileSource>) {
    let (lon, lat, zoom) = read_camera(instance);
    let placed = slint_mapping::viewport::visible_tiles(
        lon,
        lat,
        zoom,
        MAP_VIEWPORT_W,
        MAP_VIEWPORT_H,
        source.tile_size(),
    );
    let mut rows: Vec<Value> = Vec::with_capacity(placed.len());
    for p in placed {
        let Some(image) = source.tile(p.key) else { continue };
        let mut tile = Struct::default();
        tile.set_field("x".into(), Value::Number(p.x as f64));
        tile.set_field("y".into(), Value::Number(p.y as f64));
        tile.set_field("size".into(), Value::Number(p.size as f64));
        tile.set_field("image".into(), Value::Image(image));
        rows.push(Value::Struct(tile));
    }
    let model = Rc::new(VecModel::from(rows));
    let _ = instance.set_property("map-tiles", Value::Model(ModelRc::from(model)));
}

fn main() {
    // `ComponentContainer` + `component-factory` are gated behind this
    // flag — same one set in build.rs for the chrome compile.
    std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");

    let root = workspace_root();
    let pages = discover_pages(&root);
    if pages.is_empty() {
        eprintln!("no pages discovered under {}", root.display());
        std::process::exit(1);
    }
    eprintln!("discovered {} pages", pages.len());

    let compiler = PageCompiler::new(&root);
    // Tile source for the 6 map-using pages. The viewer detects them
    // by property name (`map-tiles`) when each page is instantiated;
    // non-map pages ignore the source. Bundled OSM sample tiles ship
    // with slint-mapping, no network needed.
    let map_source: Rc<dyn TileSource> =
        Rc::new(FileTileSource::new(slint_mapping::SAMPLE_TILES_DIR));

    let viewer = Viewer::new().expect("Viewer::new");
    viewer.set_total(pages.len() as i32);
    viewer.set_loaded(0);

    let titles_model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(Vec::new()));
    let cells_model: Rc<VecModel<ComponentFactory>> = Rc::new(VecModel::from(Vec::new()));
    viewer.set_titles(ModelRc::from(titles_model.clone()));
    viewer.set_cells(ModelRc::from(cells_model.clone()));

    // Synchronous initial batch — guarantees the window shows with
    // content rather than an empty grid.
    let initial = pages.len().min(INITIAL_BATCH);
    for page in &pages[..initial] {
        append_page(&viewer, &titles_model, &cells_model, &compiler, &map_source, page);
    }

    // Background loader: one page per ~16 ms tick (≈ frame rate). The
    // closure no-ops once every discovered page has been parsed. The
    // `Timer` itself is held by main() so it stays alive for the whole
    // event-loop run; when main exits, the Timer drops and stops.
    let cursor = Rc::new(RefCell::new(initial));
    let pages = Rc::new(pages);
    let compiler = Rc::new(compiler);
    let timer = Timer::default();
    {
        let viewer_weak = viewer.as_weak();
        let pages = Rc::clone(&pages);
        let compiler = Rc::clone(&compiler);
        let titles_model = titles_model.clone();
        let cells_model = cells_model.clone();
        let cursor = Rc::clone(&cursor);
        let map_source = Rc::clone(&map_source);
        timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let i = *cursor.borrow();
            if i >= pages.len() {
                return;
            }
            *cursor.borrow_mut() = i + 1;
            let Some(viewer) = viewer_weak.upgrade() else {
                return;
            };
            append_page(
                &viewer,
                &titles_model,
                &cells_model,
                &compiler,
                &map_source,
                &pages[i],
            );
        });
    }

    viewer.run().expect("viewer event loop");
    drop(timer);
}
