//! Browser-side entry for the slint-mobile-components catalogue.
//!
//! The chrome (grid + status bar) is compiled at build time from
//! `ui/wasm-viewer.slint`. The 145 page templates are interpreted at
//! runtime by `slint-interpreter` against an embedded directory tree
//! produced by `build.rs` — that tree is a copy of every workspace
//! `.slint` source with:
//!
//!   - `import "..ttf";` lines stripped (the chrome already statically
//!     embedded the fonts at build time, so the runtime re-import
//!     would fail in the browser sandbox).
//!   - Every `@image-url("…")` literal inlined as a base64 `data:`
//!     URL so the interpreter never has to hit a filesystem.
//!
//! The runtime then:
//!   1. Walks the embedded tree to discover pages
//!      (`mobile-pages-<cat>/*.slint`, excluding `_*` aggregators).
//!   2. Skips map pages — they import from `@mapping/...` which is
//!      not bundled here; supporting them would require a wasm-
//!      friendly tile pipeline (slint-mapping has one, but it's
//!      kept out of the v1 catalogue to keep the bundle smaller).
//!   3. For each page, compiles via `Compiler::build_from_source`,
//!      wraps the resulting `ComponentDefinition` in a
//!      `ComponentFactory`, and pushes it into the chrome's `cells`
//!      model so the corresponding `ComponentContainer` slot
//!      renders it.

use include_dir::{include_dir, Dir};
use slint::{ComponentHandle, ModelRc, SharedPixelBuffer, SharedString, VecModel};
use slint_interpreter::{Compiler, ComponentInstance, Struct, Value};
use slint_mapping::source::{TileKey, TileSource};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[allow(deprecated)]
use slint::ComponentFactory;

slint::include_modules!();

/// Embedded copy of every workspace `.slint` source, pre-rewritten by
/// build.rs to inline image URLs and strip font imports. The directory
/// layout under here is:
///
///   mobile-theme/<file>.slint
///   mobile-components/<file>.slint
///   mobile-components/<subdir>/<file>.slint
///   mobile-pages-<cat>/<file>.slint
///
/// Each top-level segment matches the library-path alias the runtime
/// installs into `slint_interpreter::Compiler::set_library_paths`.
static EMBEDDED: Dir<'_> = include_dir!("$OUT_DIR/embedded");

/// Sample OSM tile bundle, JPEG-Q70 transcoded by build.rs from the
/// PNGs that ship inside the published `slint-mapping` crate
/// (`SAMPLE_TILES_DIR` const). Worldwide z0–3 + Greater London z4–12,
/// ~2.4 MB of JPEGs linked into the wasm binary at compile time. The
/// 6 map-using pages in the catalogue all centre on London at
/// z10–12, so this bundle covers every default-camera tile they
/// request without a single network call. Pan past Greater London
/// or zoom past 12 and tiles will miss; the EmbeddedTileSource
/// returns None and `map.slint` paints its loading placeholder.
static EMBEDDED_TILES: Dir<'_> = include_dir!("$OUT_DIR/jpeg-tiles");

/// PNG → SharedPixelBuffer cache keyed by TileKey. First read decodes
/// the PNG via the `image` crate; subsequent reads hit the cache.
/// Wrapped in Arc<Mutex<…>> to satisfy TileSource's Send + Sync
/// bound — wasm is single-threaded so the lock never actually
/// contends, but the trait requires it for the native-target case.
type DecodedTiles = Arc<Mutex<HashMap<TileKey, SharedPixelBuffer<slint::Rgba8Pixel>>>>;

/// Read-only TileSource backed by the EMBEDDED_TILES dir. No network,
/// no async, no LRU bookkeeping — bytes are already in the wasm
/// binary's data segments, and the bundle is small enough that we
/// just hang on to every tile that's been decoded for the lifetime
/// of the page.
struct EmbeddedTileSource {
    cache: DecodedTiles,
}

impl EmbeddedTileSource {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl TileSource for EmbeddedTileSource {
    fn tile(&self, key: TileKey) -> Option<slint::Image> {
        if let Some(buf) = self.cache.lock().unwrap().get(&key).cloned() {
            return Some(slint::Image::from_rgba8(buf));
        }
        // Tiles are stored as JPEG-Q70 (raw OSM PNGs averaged 24 KB,
        // re-encoded JPEGs ~10 KB — a 3.2 MB saving on the wasm
        // binary). The slint-mapping sample-tiles directory itself
        // ships PNGs; the JPEG conversion is local to this crate
        // and re-runs on demand (see `just rebake-tiles` if you ever
        // need to refresh).
        let rel = format!("{}/{}/{}.jpg", key.z, key.x, key.y);
        let file = EMBEDDED_TILES.get_file(&rel)?;
        let bytes = file.contents();
        let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
            .ok()?
            .to_rgba8();
        let (w, h) = decoded.dimensions();
        let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(decoded.as_raw(), w, h);
        self.cache.lock().unwrap().insert(key, buf.clone());
        Some(slint::Image::from_rgba8(buf))
    }

    fn tile_size(&self) -> u32 {
        256
    }
    fn min_zoom(&self) -> u8 {
        0
    }
    fn max_zoom(&self) -> u8 {
        12
    }
}

/// The library-path roots the interpreter sees. They're "virtual" —
/// the paths don't exist on disk in the browser. `set_file_loader`
/// intercepts every resolution and serves from EMBEDDED instead.
fn virtual_root() -> &'static Path {
    Path::new("/embedded")
}

/// Map a canonical resolved path back to an entry in EMBEDDED.
/// Returns the in-memory bytes if the path lives in our embedded
/// tree; `None` otherwise (lets the interpreter's normal fallback
/// kick in, which will then fail with a useful error message for
/// missing imports we genuinely don't have).
fn read_embedded(path: &Path) -> Option<&'static str> {
    let rel = path.strip_prefix(virtual_root()).ok()?;
    let file = EMBEDDED.get_file(rel)?;
    file.contents_utf8()
}

/// Parsed metadata for one discovered page — enough to compile it
/// and expose it under a human-readable title.
#[derive(Clone)]
struct PageMeta {
    /// Virtual path the interpreter compiles from (e.g.
    /// "/embedded/mobile-pages-misc/home.slint").
    path: PathBuf,
    /// The `export component XxxPage|XxxScreen inherits …` name.
    class: String,
    /// File stem, used as the cell title in the catalogue grid.
    display: String,
}

/// Walk EMBEDDED for every `mobile-pages-<cat>/<name>.slint` and
/// return its parsed metadata. `_*` aggregators are skipped. Map
/// pages (`from "@mapping/…"`) are kept — their imports resolve
/// against the embedded `mapping/` virtual dir, and each instance
/// is wired to `EmbeddedTileSource` at factory-creation time.
fn discover_pages() -> Vec<PageMeta> {
    let mut out = Vec::new();
    for dir in EMBEDDED.dirs() {
        let dir_name = dir.path().to_string_lossy().to_string();
        let Some(cat) = dir_name.strip_prefix("mobile-pages-") else {
            continue;
        };
        let _ = cat; // kept for future per-category grouping in titles
        for file in dir.files() {
            let Some(name) = file.path().file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if name.starts_with('_') || !name.ends_with(".slint") {
                continue;
            }
            let Some(text) = file.contents_utf8() else {
                continue;
            };
            let Some(class) = scan_page_class(text) else {
                continue;
            };
            let stem = name.trim_end_matches(".slint").to_string();
            let virt = virtual_root().join(dir.path()).join(name);
            // Three sort tiers, lowest number = front of catalogue:
            //   0 — hand-picked showcase pages (look polished, good
            //       first impression in a screenshot or live demo)
            //   1 — map-using pages (top-of-shelf so the offline tile
            //       pipeline is the second thing a visitor sees, and
            //       so screenshot-based verification is trivial)
            //   2 — everything else, alphabetical
            let tier = if SHOWCASE_STEMS.contains(&stem.as_str()) {
                0
            } else if text.contains("@mapping/") {
                1
            } else {
                2
            };
            out.push((
                tier,
                PageMeta {
                    path: virt,
                    class,
                    display: stem,
                },
            ));
        }
    }
    out.sort_by(|(at, a), (bt, b)| at.cmp(bt).then_with(|| a.display.cmp(&b.display)));
    out.into_iter().map(|(_, p)| p).collect()
}

/// Hand-picked pages shown first in the catalogue grid. Kept small and
/// updated by eye — these are the ones that look most finished when a
/// first-time visitor lands on the live demo. Stems are filenames
/// without the `.slint` extension; case-sensitive match against
/// `PageMeta::display`.
const SHOWCASE_STEMS: &[&str] = &["album-detail", "app-lock"];

/// Last `export component XxxPage|XxxScreen inherits …` in a source.
/// Mirrors the heuristic used by the desktop viewer.
fn scan_page_class(text: &str) -> Option<String> {
    let prefix = "export component ";
    let mut hit = None;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let mut it = rest.split_whitespace();
        let Some(name) = it.next() else { continue };
        if !(name.ends_with("Page") || name.ends_with("Screen")) {
            continue;
        }
        if !name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            continue;
        }
        if it.next() != Some("inherits") {
            continue;
        }
        hit = Some(name.to_string());
    }
    hit
}

/// Build a fresh `slint_interpreter::Compiler` wired with our virtual
/// library paths and an embedded-source `set_file_loader`. Each page
/// uses its own compiler so a per-page parse error can't poison the
/// shared diagnostics state of sibling pages.
fn make_compiler() -> Compiler {
    let mut compiler = Compiler::default();

    let mut paths = std::collections::HashMap::new();
    for top in EMBEDDED.dirs() {
        let alias = top.path().to_string_lossy().to_string();
        paths.insert(alias.clone(), virtual_root().join(top.path()));
    }
    compiler.set_library_paths(paths);

    // Synchronous fast-path: every embedded source is already in
    // memory as a &'static str, so the future the slint API expects
    // resolves immediately.
    compiler.set_file_loader(|path| {
        let owned = path.to_path_buf();
        Box::pin(
            async move { read_embedded(&owned).map(|s| Ok::<String, io::Error>(s.to_string())) },
        )
    });

    compiler
}

// JS bridge for canvas → slint Window size. Slint's femtovg backend
// reads the canvas's CSS box once on start; after that, the only way
// to push a new size through is `slint::Window::set_size`. JS calls
// `set_canvas_size` after each ResizeObserver tick on the shell-frame
// so the Window tracks the actual visible area and the grid reflows.
// The Weak<WasmViewer> is held in a thread-local set by `run()` —
// wasm is single-threaded so a TLS slot is fine.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static VIEWER_HANDLE: RefCell<Option<slint::Weak<WasmViewer>>> = const { RefCell::new(None) };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn set_canvas_size(_w: f32, _h: f32) {
    // Defer the actual `Window::set_size` to a fresh event-loop tick.
    // Calling it directly from a ResizeObserver callback re-enters
    // winit-web's runner while it's still holding its RefCell, which
    // panics with "RefCell already borrowed" — the same reentrancy
    // trap that bit slint-mapping's WASM tile pipeline.
    // `invoke_from_event_loop` posts the work between frames, which
    // is the documented escape hatch.
    #[cfg(target_arch = "wasm32")]
    {
        let w = _w.max(320.0) as u32;
        let h = _h.max(240.0) as u32;
        let _ = slint::invoke_from_event_loop(move || {
            VIEWER_HANDLE.with(|holder| {
                if let Some(weak) = holder.borrow().as_ref() {
                    if let Some(viewer) = weak.upgrade() {
                        viewer.window().set_size(slint::PhysicalSize::new(w, h));
                    }
                }
            });
        });
    }
}

/// Embedded source-count probe — wasm-bindgen exports this so a
/// JS caller can read it (`init().then(() => embedded_file_count())`)
/// to confirm the build embedded what build.rs produced. Also
/// guarantees the `EMBEDDED` static can't be dead-stripped by the
/// linker: this function has to walk the directory tree.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn embedded_file_count() -> u32 {
    fn walk(dir: &include_dir::Dir<'_>, total: &mut u32) {
        *total += dir.files().count() as u32;
        for sub in dir.dirs() {
            walk(sub, total);
        }
    }
    let mut total = 0;
    walk(&EMBEDDED, &mut total);
    total
}

/// `#[wasm_bindgen(start)]` makes this run automatically when the
/// `init()` JS shim resolves.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    // Surface Rust panics in the browser console with a real stack
    // trace rather than the default opaque "unreachable executed"
    // wasm trap. Cheap; always wanted in dev + prod.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    // `SLINT_ENABLE_EXPERIMENTAL_FEATURES` gates `ComponentContainer`
    // + `component-factory`. The chrome already had it set at
    // build time. On native we re-set it for the runtime interpreter
    // path; on wasm `std::env::set_var` panics ("cannot set env vars
    // on this platform"), so the call is gated out — the interpreter
    // accepts the experimental syntax in compiled IR regardless of
    // the runtime env var.
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");

    let viewer = WasmViewer::new().expect("WasmViewer::new");

    // Stash a Weak<WasmViewer> for `set_canvas_size` to drive.
    #[cfg(target_arch = "wasm32")]
    VIEWER_HANDLE.with(|h| *h.borrow_mut() = Some(viewer.as_weak()));

    let titles: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(Vec::new()));
    let cells: Rc<VecModel<ComponentFactory>> = Rc::new(VecModel::from(Vec::new()));
    viewer.set_titles(ModelRc::from(titles.clone()));
    viewer.set_cells(ModelRc::from(cells.clone()));

    let pages = discover_pages();
    viewer.set_total(pages.len() as i32);
    viewer.set_loaded(0);
    viewer.set_summary("compiling pages…".into());

    // Parse + register every page synchronously. Wasm is single-
    // threaded; a streaming-parse loop (like the desktop viewer)
    // would just yield to the browser between iterations without
    // gaining real parallelism, and would make the initial paint
    // feel jankier. With ~140 pages this loop is bounded by
    // interpreter speed and resolves in a fraction of a second.
    let cursor = Rc::new(RefCell::new(0_usize));
    let pages_rc = Rc::new(pages);
    let viewer_weak = viewer.as_weak();
    {
        let cursor = Rc::clone(&cursor);
        let pages_rc = Rc::clone(&pages_rc);
        let titles = titles.clone();
        let cells = cells.clone();
        // Run a single batch right now. If this becomes too long,
        // it's safe to split across requestAnimationFrame ticks via
        // slint::Timer::single_shot — but with the v1 page count it
        // hasn't been a problem.
        for page in pages_rc.iter() {
            let Some(factory) = compile_to_factory(page) else {
                continue;
            };
            titles.push(SharedString::from(page.display.as_str()));
            cells.push(factory);
            *cursor.borrow_mut() += 1;
        }
        if let Some(v) = viewer_weak.upgrade() {
            let loaded = *cursor.borrow() as i32;
            v.set_loaded(loaded);
            v.set_summary(format!("{loaded} pages ready").into());
        }
    }

    // On wasm, `.run()` hands off to winit's web backend which drives
    // the Slint event loop via requestAnimationFrame. Returns
    // immediately on wasm32 (browser owns the event loop from here);
    // on a native dev build it blocks like any other Slint app.
    viewer.run().expect("run Slint event loop");
}

/// Compile one page to a `ComponentFactory` that the chrome's
/// `ComponentContainer` can host. Returns `None` and logs to the
/// browser console on failure so a single broken page doesn't kill
/// the whole catalogue.
fn compile_to_factory(page: &PageMeta) -> Option<ComponentFactory> {
    let compiler = make_compiler();
    // `build_from_path` would call `std::fs::read` on the top-level
    // path before consulting `set_file_loader`, which panics in wasm
    // ("operation not supported on this platform"). Resolve the
    // source from EMBEDDED ourselves and hand it to
    // `build_from_source` — the file_loader still handles every
    // import the page reaches into.
    let source = read_embedded(&page.path)?.to_string();
    let result = pollster::block_on(compiler.build_from_source(source, page.path.clone()));
    for diag in result.diagnostics() {
        web_log(&format!("[{}] {}", page.display, diag));
    }
    let def = result.component(&page.class)?;
    // Detect map pages by looking for the canonical `map-tiles`
    // property on the compiled definition. If present, the factory
    // attaches the EmbeddedTileSource handler when the cell
    // instantiates; otherwise it's a plain embed.
    let is_map_page = def.properties().any(|(name, _)| name == "map-tiles");
    Some(ComponentFactory::new(move |ctx| {
        let instance = def.create_embedded(ctx).ok()?;
        if is_map_page {
            attach_map_handler(&instance);
        }
        Some(instance)
    }))
}

/// Wire an interpreted map-page instance to an EmbeddedTileSource.
/// Each map page exposes the canonical map-* property + callback
/// surface (map-latitude, map-longitude, map-zoom, map-tiles,
/// map-pan, map-zoom-by); we read/write those via slint-interpreter's
/// dynamic property + callback API so we don't need a Rust handle
/// to the page type.
///
/// Compared to the desktop viewer's `attach_map_handler` this is a
/// stripped-down demo version: no per-page demo markers, no
/// burst-locked cursor-anchored zoom, no on_tile_ready callbacks
/// (the embedded source returns synchronously, so the first
/// `refresh_map` populates everything visible immediately). Camera
/// + pan + simple zoom all work; tiles outside the embedded London
/// bundle render as the loading placeholder.
fn attach_map_handler(instance: &ComponentInstance) {
    let source: Rc<dyn TileSource> = Rc::new(EmbeddedTileSource::new());

    // Open on London at z10 — the embedded bundle has full coverage
    // of Greater London from z4–12, so this is always tile-complete.
    let _ = instance.set_property("map-latitude", Value::Number(51.5074));
    let _ = instance.set_property("map-longitude", Value::Number(-0.1276));
    // z=11 gives a tighter London view (~20 km across) than the
    // wider z=10 ~40 km, while still staying within the embedded
    // bundle's z=4–12 range so the whole viewport is tile-complete
    // at default camera. Pages that pan or zoom further may hit
    // bundle edges and show the MapEmbed's #1a1a1a placeholder.
    let _ = instance.set_property("map-zoom", Value::Number(11.0));

    refresh_map(instance, source.as_ref());

    // map-pan(dx, dy) — projection-correct camera shift, then refresh.
    {
        let inst = instance.clone_strong();
        let src = Rc::clone(&source);
        let _ = instance.set_callback("map-pan", move |args| {
            let dx = number_arg(args, 0);
            let dy = number_arg(args, 1);
            let (lon, lat, zoom) = read_camera(&inst);
            let (new_lon, new_lat) =
                slint_mapping::camera::pan(lon, lat, zoom, dx, dy, src.tile_size());
            let _ = inst.set_property("map-longitude", Value::Number(new_lon));
            let _ = inst.set_property("map-latitude", Value::Number(new_lat));
            refresh_map(&inst, src.as_ref());
            Value::Void
        });
    }

    // map-zoom-by(delta, anchor-x, anchor-y) — simple unanchored zoom.
    // The desktop viewer's anchored-zoom-burst implementation gives
    // better UX (cursor stays put while zooming) but adds ~80 lines
    // of burst-state bookkeeping; stripped here to keep the demo
    // surface small.
    {
        let inst = instance.clone_strong();
        let src = Rc::clone(&source);
        let _ = instance.set_callback("map-zoom-by", move |args| {
            let delta = number_arg(args, 0);
            let (lon, lat, zoom) = read_camera(&inst);
            let new_zoom = (zoom + delta).clamp(src.min_zoom() as f64, src.max_zoom() as f64);
            let _ = inst.set_property("map-zoom", Value::Number(new_zoom));
            let _ = inst.set_property("map-longitude", Value::Number(lon));
            let _ = inst.set_property("map-latitude", Value::Number(lat));
            refresh_map(&inst, src.as_ref());
            Value::Void
        });
    }
}

/// Read the page's `map-viewport-width` / `map-viewport-height`
/// properties (bound to the MapEmbed's measured size on the slint
/// side). Falls back to the cell's default 412 × 892 if the page
/// hasn't declared them — projection is approximately right for
/// full-bleed maps even then.
fn read_viewport(instance: &ComponentInstance) -> (f64, f64) {
    let w = match instance.get_property("map-viewport-width") {
        Ok(Value::Number(n)) if n > 0.0 => n,
        _ => 412.0,
    };
    let h = match instance.get_property("map-viewport-height") {
        Ok(Value::Number(n)) if n > 0.0 => n,
        _ => 892.0,
    };
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

/// Recompute visible tiles for the current camera + viewport and push
/// them as a `Value::Model` of `Tile` structs to `map-tiles`. Map
/// pages also expect a `map-layers` model for marker / polyline
/// overlays — we set it to a single empty layer so the slint side
/// doesn't choke on a missing model.
fn refresh_map(instance: &ComponentInstance, source: &dyn TileSource) {
    let (lon, lat, zoom) = read_camera(instance);
    let (vp_w, vp_h) = read_viewport(instance);
    let placed =
        slint_mapping::viewport::visible_tiles(lon, lat, zoom, vp_w, vp_h, source.tile_size());

    let mut rows: Vec<Value> = Vec::with_capacity(placed.len());
    for p in placed {
        let image = source.tile(p.key).unwrap_or_default();
        let mut tile = Struct::default();
        tile.set_field("x".into(), Value::Number(p.x as f64));
        tile.set_field("y".into(), Value::Number(p.y as f64));
        tile.set_field("size".into(), Value::Number(p.size as f64));
        tile.set_field("image".into(), Value::Image(image));
        rows.push(Value::Struct(tile));
    }
    let tiles_model: Rc<VecModel<Value>> = Rc::new(VecModel::from(rows));
    let _ = instance.set_property("map-tiles", Value::Model(ModelRc::from(tiles_model)));

    // Empty layer so map.slint's `for layer in root.layers` iteration
    // sees a valid model. Pages that don't declare a `map-layers`
    // property just ignore the set.
    let markers: Rc<VecModel<Value>> = Rc::new(VecModel::from(Vec::<Value>::new()));
    let polylines: Rc<VecModel<Value>> = Rc::new(VecModel::from(Vec::<Value>::new()));
    let mut layer = Struct::default();
    layer.set_field("markers".into(), Value::Model(ModelRc::from(markers)));
    layer.set_field("polylines".into(), Value::Model(ModelRc::from(polylines)));
    let layers_model = Rc::new(VecModel::from(vec![Value::Struct(layer)]));
    let _ = instance.set_property("map-layers", Value::Model(ModelRc::from(layers_model)));
}

/// Tiny diagnostic logger that prints to the browser console on wasm
/// and to stderr elsewhere. Avoids depending on a full `log` /
/// `tracing` setup for a single error path.
fn web_log(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys_console_log(msg);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        eprintln!("{msg}");
    }
    let _ = Value::default; // touch slint_interpreter::Value to keep it in scope for future use
}

#[cfg(target_arch = "wasm32")]
fn web_sys_console_log(msg: &str) {
    // We don't depend on web-sys directly to keep the dep graph
    // small; instead reach into wasm-bindgen's `js_sys` minimal
    // surface. Falling back to a noop if the bind fails keeps a
    // logging hiccup from cratering the page.
    use wasm_bindgen::JsValue;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }
    log(msg);
    let _ = JsValue::NULL;
}
