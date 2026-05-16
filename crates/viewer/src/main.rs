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
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

// `ComponentFactory` is the public-but-deprecated bridge between an
// interpreted `ComponentDefinition::create_embedded` and the
// `ComponentContainer` slot in compiled slint. It's the documented
// integration path; the deprecation is incidental.
#[allow(deprecated)]
use slint::ComponentFactory;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel, Weak};
use slint_interpreter::{ComponentDefinition, ComponentInstance, Compiler, Struct, Value};
use slint_mapping::cache::{FileTileCache, LayeredTileCache, TileCache};
use slint_mapping::source::TileSource;
use slint_mapping::sources::OsmTileSource;
use std::sync::Arc;

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

/// Parse a single page, append its factory + title + seeded verdict to
/// the live models, and bump the loaded counter on the Window.
fn append_page(
    viewer: &Viewer,
    titles: &Rc<VecModel<SharedString>>,
    cells: &Rc<VecModel<ComponentFactory>>,
    verdicts: &Rc<VecModel<i32>>,
    stored_verdicts: &BTreeMap<String, i32>,
    compiler: &PageCompiler,
    map_source: &Rc<dyn TileSource>,
    icons: &DemoIcons,
    page: &PageMeta,
) {
    let Some(def) = compiler.definition_for(page) else {
        return;
    };
    // Detect map-using pages by looking for the `map-tiles` property in
    // the component definition. If present, the factory attaches a
    // dynamic map handler when the cell instantiates the component;
    // other pages instantiate as-is. Marker specs are picked per-page
    // by display name — pure tile-only pages get an empty vec.
    let is_map_page = def
        .properties()
        .any(|(name, _)| name == "map-tiles");
    let map_state = if is_map_page {
        let specs = Rc::new(demo_markers_for_page(&page.display, icons));
        Some((Rc::clone(map_source), specs))
    } else {
        None
    };
    let factory = ComponentFactory::new(move |ctx| {
        let instance = def.create_embedded(ctx).ok()?;
        if let Some((src, specs)) = &map_state {
            attach_map_handler(&instance, Rc::clone(src), Rc::clone(specs));
        }
        Some(instance)
    });
    titles.push(SharedString::from(page.display.as_str()));
    cells.push(factory);
    verdicts.push(stored_verdicts.get(&page.display).copied().unwrap_or(0));
    viewer.set_loaded(titles.row_count() as i32);
}

// ---------- verdicts persistence ----------

fn verdicts_path() -> PathBuf {
    workspace_root().join("showcase-verdicts.json")
}

/// Read `showcase-verdicts.json` into a name → verdict map. A missing
/// or unparseable file yields an empty map — this is a review tool,
/// not a build step. Format: `{"name": 1, ...}` with 1 = keep, 2 = redo;
/// unrated screens are simply absent.
fn load_verdicts() -> BTreeMap<String, i32> {
    let Ok(text) = std::fs::read_to_string(verdicts_path()) else {
        return BTreeMap::new();
    };
    parse_verdicts_json(&text)
}

fn parse_verdicts_json(text: &str) -> BTreeMap<String, i32> {
    let mut out = BTreeMap::new();
    let mut chars = text.char_indices().peekable();
    while let Some(&(_, c)) = chars.peek() {
        if c == '"' {
            chars.next();
            let mut name = String::new();
            for (_, ch) in chars.by_ref() {
                if ch == '"' {
                    break;
                }
                name.push(ch);
            }
            // Skip whitespace + colon.
            while let Some(&(_, ch)) = chars.peek() {
                if ch == ':' || ch.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            // Read integer (possibly signed).
            let mut num = String::new();
            while let Some(&(_, ch)) = chars.peek() {
                if ch == '-' || ch.is_ascii_digit() {
                    num.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(v) = num.parse::<i32>() {
                if !name.is_empty() && v != 0 {
                    out.insert(name, v);
                }
            }
        } else {
            chars.next();
        }
    }
    out
}

/// Rewrite `showcase-verdicts.json` — pretty-printed, key-sorted,
/// unrated (0) screens omitted. Called on every verdict toggle.
fn save_verdicts(names: &[String], verdicts: &[i32]) {
    let mut sorted: BTreeMap<&str, i32> = BTreeMap::new();
    for (name, &v) in names.iter().zip(verdicts) {
        if v != 0 {
            sorted.insert(name.as_str(), v);
        }
    }
    let mut json = String::from("{\n");
    let last = sorted.len().saturating_sub(1);
    for (i, (name, v)) in sorted.iter().enumerate() {
        let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str("  \"");
        json.push_str(&escaped);
        json.push_str("\": ");
        json.push_str(&v.to_string());
        if i != last {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("}\n");
    if let Err(e) = std::fs::write(verdicts_path(), json) {
        eprintln!("viewer: could not write {}: {e}", verdicts_path().display());
    }
}

fn summary(total: i32, verdicts: &[i32]) -> String {
    let keep = verdicts.iter().filter(|&&v| v == 1).count();
    let redo = verdicts.iter().filter(|&&v| v == 2).count();
    let rated = keep + redo;
    let unrated = total as usize - rated.min(total as usize);
    format!("{keep} keep · {redo} redo · {unrated} unrated · {rated} reviewed")
}

/// Fallback viewport size used only if the page doesn't expose
/// `map-viewport-width` / `map-viewport-height` — matches the
/// `PageCell.ComponentContainer` size in `ui/viewer.slint`. With the
/// uplifted pages this is rarely hit.
const FALLBACK_VIEWPORT_W: f64 = 412.0;
const FALLBACK_VIEWPORT_H: f64 = 892.0;

/// Inter-event gap before a zoom burst is considered over. A typical
/// mouse wheel emits events ~30-60 ms apart during a continuous spin;
/// 200 ms tolerates a brief pause without breaking the lock, but ends
/// promptly enough that a deliberately new zoom (cursor moved, fresh
/// spin) re-captures the anchor.
const ZOOM_BURST_QUIET: Duration = Duration::from_millis(200);

/// Anchor state held for the duration of a single continuous zoom
/// burst — the (lon, lat) of the cursor at burst-start and the
/// viewport pixel it sat on. Every event in the burst zooms while
/// keeping that geographic point pinned to the same pixel.
struct ZoomBurst {
    anchor_lon: f64,
    anchor_lat: f64,
    anchor_x: f64,
    anchor_y: f64,
    last_event_at: Instant,
}

/// Geographic marker description: stays in lon/lat through refreshes,
/// projected to viewport pixels each repaint via
/// `slint_mapping::viewport::lonlat_to_viewport_px`. Built per-cell by
/// the viewer's per-page demo dispatch (see `demo_markers_for_page`);
/// in a real app this would come from your domain data.
struct MarkerSpec {
    lon: f64,
    lat: f64,
    size: f64,
    colour: u32,
    icon: Option<slint::Image>,
}

/// Demo icons loaded once at startup. `pin`, `home`, `heart` cover
/// the three demo gestures used by `demo_markers_for_page` — origin,
/// destination, generic point of interest. Loaded via
/// `slint::Image::load_from_path` against the bundled icons in
/// `crates/components/ui/icons/`.
#[derive(Clone)]
struct DemoIcons {
    pin: slint::Image,
    home: slint::Image,
    heart: slint::Image,
}

impl DemoIcons {
    fn load(root: &Path) -> Self {
        let icons_dir = root.join("crates/components/ui/icons");
        let load = |name: &str| {
            slint::Image::load_from_path(&icons_dir.join(name)).unwrap_or_default()
        };
        Self {
            pin: load("pin.svg"),
            home: load("home.svg"),
            heart: load("heart.svg"),
        }
    }
}

/// Per-page demo marker dispatch. Page display names match the file
/// stems under `crates/pages-*/ui/`. Pages not listed render with no
/// markers — pure tile-only display, which is the right look for
/// non-marker map pages like `turn-by-turn-nav` (route polylines come
/// later as a `polygons`/`lines` Layer field).
fn demo_markers_for_page(page_display: &str, icons: &DemoIcons) -> Vec<MarkerSpec> {
    // All markers are around the default London camera (51.5074, -0.1276)
    // at zoom 13, so they land on-screen for every map page in the viewer.
    const RED: u32 = 0xff_ef_44_44;
    const BLUE: u32 = 0xff_25_63_eb;
    const AMBER: u32 = 0xff_f5_9e_0b;
    const PINK: u32 = 0xff_db_27_77;
    match page_display {
        "map" => vec![
            // Just the camera centre, as a heart POI.
            MarkerSpec { lon: -0.1276, lat: 51.5074, size: 28.0, colour: PINK, icon: Some(icons.heart.clone()) },
        ],
        "driver-on-the-way" => vec![
            MarkerSpec { lon: -0.1380, lat: 51.5030, size: 28.0, colour: BLUE,  icon: Some(icons.home.clone()) },
            MarkerSpec { lon: -0.1100, lat: 51.5140, size: 30.0, colour: RED,   icon: Some(icons.pin.clone()) },
            MarkerSpec { lon: -0.1220, lat: 51.5085, size: 26.0, colour: AMBER, icon: None /* car */ },
        ],
        "ride-share-booking" => vec![
            MarkerSpec { lon: -0.1400, lat: 51.5120, size: 28.0, colour: BLUE, icon: Some(icons.home.clone()) },
            MarkerSpec { lon: -0.1100, lat: 51.5020, size: 32.0, colour: RED,  icon: Some(icons.pin.clone()) },
        ],
        "store-locator" => vec![
            MarkerSpec { lon: -0.1350, lat: 51.5100, size: 26.0, colour: RED, icon: Some(icons.pin.clone()) },
            MarkerSpec { lon: -0.1180, lat: 51.5050, size: 26.0, colour: RED, icon: Some(icons.pin.clone()) },
            MarkerSpec { lon: -0.1290, lat: 51.5020, size: 26.0, colour: RED, icon: Some(icons.pin.clone()) },
            MarkerSpec { lon: -0.1220, lat: 51.5140, size: 26.0, colour: RED, icon: Some(icons.pin.clone()) },
        ],
        "parking-session" => vec![
            MarkerSpec { lon: -0.1276, lat: 51.5074, size: 28.0, colour: BLUE, icon: Some(icons.pin.clone()) },
        ],
        _ => Vec::new(),
    }
}

// UI-thread-only registry of live map page cells. Each cell's
// `refresher` returns the set of tile keys it currently depends on,
// or None if the cell's slint instance has been dropped (pruned).
// `source` is the shared OsmTileSource (same Rc across all cells —
// stored per cell so `refresh_all_map_pages` doesn't need to plumb
// it down some other path).
struct MapCell {
    refresher: Box<dyn Fn() -> Option<std::collections::HashSet<slint_mapping::TileKey>>>,
    source: Rc<dyn TileSource>,
}

thread_local! {
    static MAP_CELLS: RefCell<Vec<MapCell>> = const { RefCell::new(Vec::new()) };
}

/// Refresh every live map cell, then tell the shared source to cancel
/// queued fetches for tiles no cell needs anymore. Called from
/// `OsmTileSource::on_tile_ready` (debounced inside the source — one
/// call per quiescent burst) via `slint::invoke_from_event_loop`.
fn refresh_all_map_pages() {
    let mut union: std::collections::HashSet<slint_mapping::TileKey> =
        std::collections::HashSet::new();
    let mut source_handle: Option<Rc<dyn TileSource>> = None;
    MAP_CELLS.with(|cells| {
        let mut cells = cells.borrow_mut();
        cells.retain(|cell| match (cell.refresher)() {
            Some(keys) => {
                union.extend(keys);
                source_handle.get_or_insert_with(|| Rc::clone(&cell.source));
                true
            }
            None => false,
        });
    });
    if let Some(source) = source_handle {
        source.cancel_all_except(&union);
    }
}

/// Wire a `slint_mapping::TileSource` into a freshly-loaded interpreted
/// page that exposes the canonical map-* property + callback surface
/// (`map-latitude`, `map-longitude`, `map-zoom`, `map-tiles`, `map-pan`,
/// `map-zoom-by`). Goes through slint-interpreter's dynamic property /
/// callback API so we don't need a Rust handle to the page type.
fn attach_map_handler(
    instance: &ComponentInstance,
    source: Rc<dyn TileSource>,
    marker_specs: Rc<Vec<MarkerSpec>>,
) {
    // Start centred on London at zoom 10 — the bundled sample bundle
    // includes Greater London at zoom 4-12, so this opens with real
    // street-level detail (scroll-wheel zooms to 12; pan/zoom past
    // the cache triggers OsmTileSource to fetch in the background).
    let _ = instance.set_property("map-latitude", Value::Number(51.5074));
    let _ = instance.set_property("map-longitude", Value::Number(-0.1276));
    let _ = instance.set_property("map-zoom", Value::Number(10.0));

    refresh_map(instance, &source, &marker_specs);

    // Register this cell so `OsmTileSource::on_tile_ready` (debounced
    // by the source, invoked on the UI thread via
    // `slint::invoke_from_event_loop`) can repaint it and union its
    // visible-tile set into the global keep-list before cancelling.
    let inst_weak: Weak<ComponentInstance> = instance.as_weak();
    let source_for_refresh = Rc::clone(&source);
    let cell_source = Rc::clone(&source);
    let markers_for_refresh = Rc::clone(&marker_specs);
    MAP_CELLS.with(|cells| {
        cells.borrow_mut().push(MapCell {
            refresher: Box::new(move || {
                let inst = inst_weak.upgrade()?;
                Some(refresh_map(&inst, &source_for_refresh, &markers_for_refresh))
            }),
            source: cell_source,
        });
    });

    // map-pan(dx, dy) — projection-correct camera shift, then refresh.
    let inst_pan = instance.clone_strong();
    let source_pan = Rc::clone(&source);
    let markers_pan = Rc::clone(&marker_specs);
    let _ = instance.set_callback("map-pan", move |args| {
        let dx = number_arg(args, 0);
        let dy = number_arg(args, 1);
        let (lon, lat, zoom) = read_camera(&inst_pan);
        let (new_lon, new_lat) =
            slint_mapping::camera::pan(lon, lat, zoom, dx, dy, source_pan.tile_size());
        let _ = inst_pan.set_property("map-longitude", Value::Number(new_lon));
        let _ = inst_pan.set_property("map-latitude", Value::Number(new_lat));
        refresh_map(&inst_pan, &source_pan, &markers_pan);
        Value::Void
    });

    // map-zoom-by(delta, anchor-x, anchor-y) — burst-locked
    // cursor-anchored zoom. The first scroll-event in a burst captures
    // (anchor_lon, anchor_lat) at the current camera + cursor; every
    // subsequent event in the same burst zooms while keeping that
    // geographic anchor pinned to the original viewport pixel.
    // Without this, each event re-derives the anchor from the *current*
    // camera (which has shifted since the previous event) and the
    // camera drifts visibly across a continuous scroll.
    let zoom_burst: Rc<RefCell<Option<ZoomBurst>>> = Rc::new(RefCell::new(None));
    let inst_zoom = instance.clone_strong();
    let source_zoom = Rc::clone(&source);
    let markers_zoom = Rc::clone(&marker_specs);
    let burst_for_cb = Rc::clone(&zoom_burst);
    let _ = instance.set_callback("map-zoom-by", move |args| {
        let delta = number_arg(args, 0);
        let ax = number_arg(args, 1);
        let ay = number_arg(args, 2);
        let (lon, lat, zoom) = read_camera(&inst_zoom);
        let tile_size = source_zoom.tile_size();
        let now = Instant::now();

        let mut burst = burst_for_cb.borrow_mut();
        let still_in_burst = burst
            .as_ref()
            .is_some_and(|b| now.duration_since(b.last_event_at) < ZOOM_BURST_QUIET);
        let (vp_w, vp_h) = read_viewport(&inst_zoom);
        let (anchor_lon, anchor_lat, anchor_x, anchor_y, burst_kind) = if still_in_burst {
            let b = burst.as_ref().unwrap();
            (b.anchor_lon, b.anchor_lat, b.anchor_x, b.anchor_y, "cont")
        } else {
            let (alon, alat) = slint_mapping::viewport::viewport_px_to_lonlat(
                ax, ay, lon, lat, zoom, vp_w, vp_h, tile_size,
            );
            (alon, alat, ax, ay, "new ")
        };
        // Express the cursor as a viewport-relative offset so it's
        // obvious how far off-centre it is (e.g. "(-32 px left, -179 px
        // up)" → cursor is well in the upper-left of the viewport, so
        // the geo anchor should be NW of the camera). Verify against
        // the printed lon/lat by hovering the camera point on a real
        // map: anchor should be wherever your cursor was on screen.
        let (cx, cy) = (vp_w / 2.0, vp_h / 2.0);
        let (dx_px, dy_px) = (ax - cx, ay - cy);
        eprintln!(
            "[zoom {burst_kind}] cam_centre=lon{lon:.5}/lat{lat:.5} cursor_px=({ax:.1},{ay:.1}) vp={vp_w:.0}x{vp_h:.0} → offset_from_centre=({dx_px:+.1},{dy_px:+.1}) px → anchor=lon{anchor_lon:.5}/lat{anchor_lat:.5}  Δ_from_camera=({:+.5}°lon,{:+.5}°lat) z={zoom:.2}{delta:+.3}",
            anchor_lon - lon,
            anchor_lat - lat,
        );

        let new_zoom = (zoom + delta)
            .clamp(source_zoom.min_zoom() as f64, source_zoom.max_zoom() as f64);
        let (new_lon, new_lat) = slint_mapping::viewport::center_for_anchor_at_viewport_px(
            anchor_lon,
            anchor_lat,
            anchor_x,
            anchor_y,
            new_zoom,
            vp_w,
            vp_h,
            tile_size,
        );

        *burst = Some(ZoomBurst {
            anchor_lon,
            anchor_lat,
            anchor_x,
            anchor_y,
            last_event_at: now,
        });

        let _ = inst_zoom.set_property("map-longitude", Value::Number(new_lon));
        let _ = inst_zoom.set_property("map-latitude", Value::Number(new_lat));
        let _ = inst_zoom.set_property("map-zoom", Value::Number(new_zoom));
        refresh_map(&inst_zoom, &source_zoom, &markers_zoom);
        Value::Void
    });
}

/// Read the page's actual MapEmbed viewport size in logical pixels.
/// Pages expose `map-viewport-width` / `map-viewport-height` bound to
/// the MapEmbed's measured size. Falls back to the cell defaults
/// (412×892) if a page didn't declare them — cursor anchoring + marker
/// projection will still be approximately right for full-bleed maps,
/// but pages with a non-full-bleed map (mini-map strip etc.) need the
/// real size or they'd drift visibly.
fn read_viewport(instance: &ComponentInstance) -> (f64, f64) {
    let raw_w = instance.get_property("map-viewport-width");
    let raw_h = instance.get_property("map-viewport-height");
    let w = match &raw_w {
        Ok(Value::Number(n)) if *n > 0.0 => *n,
        _ => FALLBACK_VIEWPORT_W,
    };
    let h = match &raw_h {
        Ok(Value::Number(n)) if *n > 0.0 => *n,
        _ => FALLBACK_VIEWPORT_H,
    };
    eprintln!(
        "[viewport] raw_w={raw_w:?} raw_h={raw_h:?} → ({w:.1}, {h:.1})"
    );
    (w, h)
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
/// Refresh one cell's model + return the set of tile keys it depends
/// on (so `refresh_all_map_pages` can compute the union across cells
/// and call `cancel_all_except(union)` once per pass — cancelling per
/// cell would discard work in flight for sibling cells since all 6
/// share the same OsmTileSource).
fn refresh_map(
    instance: &ComponentInstance,
    source: &Rc<dyn TileSource>,
    markers: &[MarkerSpec],
) -> std::collections::HashSet<slint_mapping::TileKey> {
    let (lon, lat, zoom) = read_camera(instance);
    let (vp_w, vp_h) = read_viewport(instance);
    let mut placed = slint_mapping::viewport::visible_tiles(
        lon,
        lat,
        zoom,
        vp_w,
        vp_h,
        source.tile_size(),
    );
    let keep: std::collections::HashSet<slint_mapping::TileKey> =
        placed.iter().map(|p| p.key).collect();

    // Order requests centre-out so the tile under the cursor (and its
    // immediate neighbours) gets enqueued first. With the OSM source's
    // FIFO queue this translates directly into centre-first dequeue —
    // the area the user is actually looking at fills in before the
    // edges. Cheap: <50 tiles, integer-arithmetic sort key.
    let (centre_tx, centre_ty) =
        slint_mapping::projection::lonlat_to_tile(lon, lat, zoom.floor());
    let centre_tile_x = centre_tx.floor() as i64;
    let centre_tile_y = centre_ty.floor() as i64;
    placed.sort_by_key(|p| {
        let dx = (p.key.x as i64 - centre_tile_x).abs();
        let dy = (p.key.y as i64 - centre_tile_y).abs();
        dx + dy
    });

    let mut rows: Vec<Value> = Vec::with_capacity(placed.len());
    for p in placed {
        // Always emit a tile — even when the bytes aren't on disk yet —
        // so the slint side can paint a "loading" square in its slot.
        // A default `slint::Image` has zero width, which is the check
        // map.slint uses to switch to the placeholder.
        let image = source.tile(p.key).unwrap_or_default();
        let mut tile = Struct::default();
        tile.set_field("x".into(), Value::Number(p.x as f64));
        tile.set_field("y".into(), Value::Number(p.y as f64));
        tile.set_field("size".into(), Value::Number(p.size as f64));
        tile.set_field("image".into(), Value::Image(image));
        rows.push(Value::Struct(tile));
    }
    let model = Rc::new(VecModel::from(rows));
    let _ = instance.set_property("map-tiles", Value::Model(ModelRc::from(model)));

    // ---- Layer overlays ----
    // One layer holding the per-page demo markers (set by
    // `demo_markers_for_page`). Real apps would build any number of
    // layers from their domain data — search results, saved places,
    // route waypoints — and toggle them as a group.
    let mut marker_rows: Vec<Value> = Vec::with_capacity(markers.len());
    for spec in markers {
        let (px, py) = slint_mapping::viewport::lonlat_to_viewport_px(
            spec.lon, spec.lat, lon, lat, zoom,
            vp_w, vp_h, source.tile_size(),
        );
        let mut m = Struct::default();
        m.set_field("x".into(), Value::Number(px));
        m.set_field("y".into(), Value::Number(py));
        m.set_field("size".into(), Value::Number(spec.size));
        m.set_field("colour".into(), Value::Brush(rgba_brush(spec.colour)));
        m.set_field("icon".into(), Value::Image(spec.icon.clone().unwrap_or_default()));
        marker_rows.push(Value::Struct(m));
    }
    let markers_model: Rc<VecModel<Value>> = Rc::new(VecModel::from(marker_rows));
    // Empty polylines for now — the Layer struct gained a `polylines`
    // field for routing overlays; pages that don't display a route
    // still need to set it explicitly so the interpreter sees the full
    // struct shape.
    let polylines_model: Rc<VecModel<Value>> = Rc::new(VecModel::from(Vec::<Value>::new()));
    let mut layer = Struct::default();
    layer.set_field("markers".into(), Value::Model(ModelRc::from(markers_model)));
    layer.set_field("polylines".into(), Value::Model(ModelRc::from(polylines_model)));
    let layers_model = Rc::new(VecModel::from(vec![Value::Struct(layer)]));
    let _ = instance.set_property("map-layers", Value::Model(ModelRc::from(layers_model)));

    keep
}

/// Pack an `0xAARRGGBB` literal into a Slint `Brush`. Used to feed
/// interpreter callbacks that expect colour values.
fn rgba_brush(argb: u32) -> slint::Brush {
    let a = ((argb >> 24) & 0xff) as u8;
    let r = ((argb >> 16) & 0xff) as u8;
    let g = ((argb >> 8) & 0xff) as u8;
    let b = (argb & 0xff) as u8;
    slint::Brush::SolidColor(slint::Color::from_argb_u8(a, r, g, b))
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
    // Tile source for the 6 map-using pages. An OsmTileSource backed
    // by a FileTileCache rooted under target/tile-cache/: tiles
    // already on disk serve instantly, misses kick off background
    // HTTP fetches against the OSM standard tile server. When a fetch
    // lands, `on_tile_ready` schedules a UI-thread refresh of every
    // map-using page cell, so panning into uncached areas fills in as
    // the tiles stream in.
    let cache_dir = root.join("target/tile-cache");
    std::fs::create_dir_all(&cache_dir).ok();
    let cache: Arc<dyn TileCache> = Arc::new(LayeredTileCache::new(
        Box::new(FileTileCache::new(&cache_dir)),
        vec![Box::new(FileTileCache::new(slint_mapping::SAMPLE_TILES_DIR))],
    ));
    let osm = Arc::new(OsmTileSource::new(cache));
    // OsmTileSource already coalesces tile-ready notifications via
    // its internal notifier thread (one callback per burst, settled
    // ~25ms). Each notification → one UI refresh.
    osm.on_tile_ready(|| {
        let _ = slint::invoke_from_event_loop(refresh_all_map_pages);
    });
    let map_source: Rc<dyn TileSource> = {
        // OsmTileSource is Send+Sync; we wrap in Rc<dyn TileSource>
        // only because attach_map_handler takes that. The interior is
        // an Arc that handles cross-thread state correctly.
        struct OsmRc(Arc<OsmTileSource>);
        impl TileSource for OsmRc {
            fn tile(&self, k: slint_mapping::TileKey) -> Option<slint::Image> {
                self.0.tile(k)
            }
            fn tile_size(&self) -> u32 {
                self.0.tile_size()
            }
            fn min_zoom(&self) -> u8 {
                self.0.min_zoom()
            }
            fn max_zoom(&self) -> u8 {
                self.0.max_zoom()
            }
        }
        Rc::new(OsmRc(osm))
    };
    eprintln!("tile cache: {}", cache_dir.display());

    // Demo marker icons (pin / home / heart) loaded once. Shared via
    // Clone (cheap — slint::Image internally refcounts the decoded
    // pixmap) so per-cell MarkerSpecs can hold their own handles.
    let icons = DemoIcons::load(&root);

    let viewer = Viewer::new().expect("Viewer::new");
    viewer.set_total(pages.len() as i32);
    viewer.set_loaded(0);

    let stored_verdicts = load_verdicts();
    eprintln!(
        "verdicts: loaded {} rated entries from {}",
        stored_verdicts.len(),
        verdicts_path().display(),
    );

    let titles_model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(Vec::new()));
    let cells_model: Rc<VecModel<ComponentFactory>> = Rc::new(VecModel::from(Vec::new()));
    let verdicts_model: Rc<VecModel<i32>> = Rc::new(VecModel::from(Vec::new()));
    viewer.set_titles(ModelRc::from(titles_model.clone()));
    viewer.set_cells(ModelRc::from(cells_model.clone()));
    viewer.set_verdicts(ModelRc::from(verdicts_model.clone()));
    viewer.set_summary(summary(pages.len() as i32, &[]).into());

    // Wire ✓/✗ clicks → in-memory model + immediate persist + summary refresh.
    {
        let weak = viewer.as_weak();
        let titles_for_save = titles_model.clone();
        let verdicts_for_save = verdicts_model.clone();
        let total = pages.len() as i32;
        viewer.on_verdict_changed(move |index, value| {
            let i = index as usize;
            if i >= verdicts_for_save.row_count() {
                return;
            }
            verdicts_for_save.set_row_data(i, value);
            let names: Vec<String> = titles_for_save.iter().map(|s| s.to_string()).collect();
            let current: Vec<i32> = verdicts_for_save.iter().collect();
            save_verdicts(&names, &current);
            if let Some(v) = weak.upgrade() {
                v.set_summary(summary(total, &current).into());
            }
        });
    }

    // Synchronous initial batch — guarantees the window shows with
    // content rather than an empty grid.
    let initial = pages.len().min(INITIAL_BATCH);
    for page in &pages[..initial] {
        append_page(
            &viewer,
            &titles_model,
            &cells_model,
            &verdicts_model,
            &stored_verdicts,
            &compiler,
            &map_source,
            &icons,
            page,
        );
    }
    viewer.set_summary(
        summary(pages.len() as i32, &verdicts_model.iter().collect::<Vec<_>>()).into(),
    );

    // Background loader: one page per ~16 ms tick (≈ frame rate). The
    // closure no-ops once every discovered page has been parsed. The
    // `Timer` itself is held by main() so it stays alive for the whole
    // event-loop run; when main exits, the Timer drops and stops.
    let cursor = Rc::new(RefCell::new(initial));
    let pages = Rc::new(pages);
    let compiler = Rc::new(compiler);
    let stored_verdicts = Rc::new(stored_verdicts);
    let timer = Timer::default();
    {
        let viewer_weak = viewer.as_weak();
        let pages = Rc::clone(&pages);
        let compiler = Rc::clone(&compiler);
        let titles_model = titles_model.clone();
        let cells_model = cells_model.clone();
        let verdicts_model = verdicts_model.clone();
        let stored_verdicts = Rc::clone(&stored_verdicts);
        let cursor = Rc::clone(&cursor);
        let map_source = Rc::clone(&map_source);
        let icons = icons.clone();
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
                &verdicts_model,
                &stored_verdicts,
                &compiler,
                &map_source,
                &icons,
                &pages[i],
            );
            if i + 1 == pages.len() {
                let current: Vec<i32> = verdicts_model.iter().collect();
                viewer.set_summary(summary(pages.len() as i32, &current).into());
            }
        });
    }

    viewer.run().expect("viewer event loop");
    drop(timer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts_json_round_trips() {
        let names = vec!["home".to_string(), "settings".to_string(), "login".to_string()];
        let verdicts = vec![1, 0, 2];
        let mut sorted: BTreeMap<&str, i32> = BTreeMap::new();
        for (n, &v) in names.iter().zip(&verdicts) {
            if v != 0 {
                sorted.insert(n.as_str(), v);
            }
        }
        let mut json = String::from("{\n");
        let last = sorted.len().saturating_sub(1);
        for (i, (n, v)) in sorted.iter().enumerate() {
            json.push_str(&format!("  \"{n}\": {v}"));
            if i != last {
                json.push(',');
            }
            json.push('\n');
        }
        json.push_str("}\n");

        let parsed = parse_verdicts_json(&json);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.get("home"), Some(&1));
        assert_eq!(parsed.get("login"), Some(&2));
        assert!(parsed.get("settings").is_none(), "unrated keys must be absent");
    }

    #[test]
    fn missing_or_garbage_yields_empty_map() {
        assert!(parse_verdicts_json("").is_empty());
        assert!(parse_verdicts_json("not json at all").is_empty());
        assert!(parse_verdicts_json("{}").is_empty());
    }

    #[test]
    fn summary_counts_correctly() {
        let s = summary(5, &[1, 1, 2, 0, 0]);
        assert!(s.contains("2 keep"), "{s}");
        assert!(s.contains("1 redo"), "{s}");
        assert!(s.contains("2 unrated"), "{s}");
        assert!(s.contains("3 reviewed"), "{s}");
    }
}
