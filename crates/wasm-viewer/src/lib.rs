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
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_interpreter::{Compiler, ComponentInstance};
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// `Struct` / `Value` (interpreter dynamic API) and the `TileSource`
// trait are only referenced by the wasm-only map handler + tile
// refresh code; importing them unconditionally trips unused-import
// warnings on a native `cargo check`.
#[cfg(target_arch = "wasm32")]
use slint_interpreter::{Struct, Value};
#[cfg(target_arch = "wasm32")]
use slint_mapping::source::{TileKey, TileSource};

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

// Map tiles are NOT embedded in the wasm binary. build.rs transcodes
// the slint-mapping sample PNGs to JPEG-Q70 under `crates/wasm-viewer/
// tiles/`, trunk's `copy-dir` ships that to `dist/tiles/`, and the
// runtime (see `attach_map_handler`) points a `WasmOsmTileSource` at
// the same-origin `tiles/{z}/{x}/{y}.jpg` URL. Tiles are fetched
// lazily — only the visible tiles of the 6 map pages, only when those
// cells render — keeping ~2.4 MB out of the initial download.

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
const SHOWCASE_STEMS: &[&str] = &[
    "album-detail",
    "app-lock",
    "control-panel",
    "clay-profile",
    "terminal-dashboard",
    "editorial-article",
    "vaporwave-player",
];

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

// JS bridge for canvas → slint Window size. The chrome's WasmViewer
// binds its `width`/`height` to the `canvas-w`/`canvas-h` in-props;
// JS calls `set_canvas_size` after each ResizeObserver tick on the
// shell-frame and we push the values into those props. Driving the
// size through a bound property (rather than `Window::set_size`) is
// what makes it survive the incremental loader's relayouts — a plain
// `set_size` gets stomped by `preferred-*` on every layout pass.
// The Weak<WasmViewer> is held in a thread-local set by `run()` —
// wasm is single-threaded so a TLS slot is fine.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static VIEWER_HANDLE: RefCell<Option<slint::Weak<WasmViewer>>> = const { RefCell::new(None) };
}

// Slint Timer dropping cancels its scheduled work, so the incremental
// loader's Timer needs to outlive `run()`. Park it in a thread-local
// slot — wasm is single-threaded and the timer is set up exactly
// once.
thread_local! {
    static LOADER_TIMER: RefCell<Option<Timer>> = const { RefCell::new(None) };
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn set_canvas_size(_w: f32, _h: f32) {
    // Defer the property write to a fresh event-loop tick. Setting it
    // directly from a ResizeObserver callback re-enters winit-web's
    // runner while it's still holding its RefCell, which panics with
    // "RefCell already borrowed" — the same reentrancy trap that bit
    // slint-mapping's WASM tile pipeline. `invoke_from_event_loop`
    // posts the work between frames, which is the documented escape
    // hatch.
    #[cfg(target_arch = "wasm32")]
    {
        let w = _w.max(320.0);
        let h = _h.max(240.0);
        let _ = slint::invoke_from_event_loop(move || {
            VIEWER_HANDLE.with(|holder| {
                if let Some(weak) = holder.borrow().as_ref() {
                    if let Some(viewer) = weak.upgrade() {
                        // `length`-typed slint props map to `f32`
                        // (logical px) in the generated Rust API.
                        viewer.set_canvas_w(w);
                        viewer.set_canvas_h(h);
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

    // Incremental load. Compile the first batch synchronously so the
    // visitor sees content immediately (the showcase tier + the map
    // pages — the most polished cells), then drive the rest via a
    // Slint Timer that ticks between frames. Each tick compiles one
    // page and appends it to the model, so the catalogue grows
    // visibly under the user's scroll. ~25 ms per tick lands ~40
    // pages/sec, which means the full 145-page catalogue finishes in
    // ~3.5 s while keeping the canvas responsive.
    // Visitor-facing pacing. 6 pages up-front fills the first row at
    // most window widths. After that ~10 pages/sec is a deliberate
    // "watch the catalogue grow" pace — fast enough to land the full
    // 145-page catalogue in ~15 s but slow enough that a visitor sees
    // it as a load-in animation rather than a flash of bare cells.
    const INITIAL_BATCH: usize = 6;
    const TICK_MS: u64 = 100;

    let pages_rc = Rc::new(pages);
    let cursor = Rc::new(RefCell::new(0_usize));

    // ---- Synchronous initial batch ----
    let initial_end = INITIAL_BATCH.min(pages_rc.len());
    for page in &pages_rc[..initial_end] {
        let Some(factory) = compile_to_factory(page) else {
            *cursor.borrow_mut() += 1;
            continue;
        };
        titles.push(SharedString::from(page.display.as_str()));
        cells.push(factory);
        *cursor.borrow_mut() += 1;
    }
    viewer.set_loaded(*cursor.borrow() as i32);
    viewer.set_summary(if pages_rc.len() <= INITIAL_BATCH {
        format!("{} pages ready", *cursor.borrow()).into()
    } else {
        format!(
            "{} of {} pages — loading…",
            *cursor.borrow(),
            pages_rc.len()
        )
        .into()
    });

    // ---- Timer-driven trickle for the remaining pages ----
    // Store the timer somewhere it won't be dropped: a thread-local
    // slot owned by the wasm module (single-threaded, so a TLS Cell is
    // safe). Dropping the timer would cancel the trickle mid-load.
    let viewer_weak = viewer.as_weak();
    let timer = Timer::default();
    {
        let pages_rc = Rc::clone(&pages_rc);
        let cursor = Rc::clone(&cursor);
        let titles = titles.clone();
        let cells = cells.clone();
        timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(TICK_MS),
            move || {
                let i = *cursor.borrow();
                if i >= pages_rc.len() {
                    return;
                }
                *cursor.borrow_mut() = i + 1;
                let page = &pages_rc[i];
                let Some(factory) = compile_to_factory(page) else {
                    return;
                };
                titles.push(SharedString::from(page.display.as_str()));
                cells.push(factory);
                if let Some(v) = viewer_weak.upgrade() {
                    let loaded = titles.row_count() as i32;
                    v.set_loaded(loaded);
                    if i + 1 >= pages_rc.len() {
                        v.set_summary(format!("{loaded} pages ready").into());
                    } else {
                        v.set_summary(
                            format!("{loaded} of {} pages — loading…", pages_rc.len()).into(),
                        );
                    }
                }
            },
        );
    }
    LOADER_TIMER.with(|slot| *slot.borrow_mut() = Some(timer));

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

/// Coverage manifest of the tiles actually shipped under `dist/tiles/`
/// — one `z/x/y` key per line, generated by build.rs at transcode
/// time. The map source consults this before issuing an XHR.
#[cfg(target_arch = "wasm32")]
static TILE_MANIFEST: &str = include_str!(concat!(env!("OUT_DIR"), "/tile-manifest.txt"));

/// `WasmOsmTileSource` wrapped with the build-time coverage set. Tiles
/// in the manifest delegate to the inner XHR source; tiles outside it
/// (prefetch overscan past the London bundle edges) short-circuit to
/// `None` — the MapEmbed renders its placeholder — and log a single
/// `console.debug` line instead of letting the browser fire a 404 the
/// devtools console flags as an error.
#[cfg(target_arch = "wasm32")]
struct CoverageTileSource {
    inner: slint_mapping::sources::WasmOsmTileSource,
    available: std::collections::HashSet<(u8, u32, u32)>,
}

#[cfg(target_arch = "wasm32")]
impl CoverageTileSource {
    fn new() -> Self {
        let inner = slint_mapping::sources::WasmOsmTileSource::with_url("tiles/{z}/{x}/{y}.jpg")
            .with_zoom_range(0, 12);
        let available = TILE_MANIFEST
            .lines()
            .filter_map(|line| {
                let mut it = line.trim().split('/');
                let z = it.next()?.parse().ok()?;
                let x = it.next()?.parse().ok()?;
                let y = it.next()?.parse().ok()?;
                Some((z, x, y))
            })
            .collect();
        Self { inner, available }
    }

    /// Forward the inner source's tile-ready notification (used by the
    /// handler to refresh the model as async fetches land).
    fn on_tile_ready(&self, cb: impl Fn() + 'static) {
        self.inner.on_tile_ready(cb);
    }
}

#[cfg(target_arch = "wasm32")]
impl TileSource for CoverageTileSource {
    fn tile(&self, key: TileKey) -> Option<slint::Image> {
        if self.available.contains(&(key.z, key.x, key.y)) {
            self.inner.tile(key)
        } else {
            web_debug(&format!(
                "[map] overscan: tile {}/{}/{} outside shipped bundle — placeholder",
                key.z, key.x, key.y
            ));
            None
        }
    }
    fn tile_size(&self) -> u32 {
        self.inner.tile_size()
    }
    fn min_zoom(&self) -> u8 {
        self.inner.min_zoom()
    }
    fn max_zoom(&self) -> u8 {
        self.inner.max_zoom()
    }
}

/// Wire an interpreted map-page instance to a lazily-fetching tile
/// source. Each map page exposes the canonical map-* property +
/// callback surface (map-latitude, map-longitude, map-zoom,
/// map-tiles, map-pan, map-zoom-by); we read/write those via
/// slint-interpreter's dynamic property + callback API so we don't
/// need a Rust handle to the page type.
///
/// Tiles come from a `CoverageTileSource` (XHR-backed, same-origin
/// `tiles/{z}/{x}/{y}.jpg`, never embedded). `tile()` returns `None`
/// on a not-yet-fetched in-bundle tile and kicks off a background
/// fetch; `on_tile_ready` re-runs `refresh_map` as each lands, so the
/// loading placeholders fill in progressively. Tiles outside the
/// bundle are absorbed as overscan (debug log, no 404).
#[cfg(target_arch = "wasm32")]
fn attach_map_handler(instance: &ComponentInstance) {
    let source = Rc::new(CoverageTileSource::new());

    // London at z11 (~20 km across) — fully covered by the shipped
    // tile set so the default camera is tile-complete.
    let _ = instance.set_property("map-latitude", Value::Number(51.5074));
    let _ = instance.set_property("map-longitude", Value::Number(-0.1276));
    let _ = instance.set_property("map-zoom", Value::Number(11.0));

    // Re-pull the visible tiles each time an async fetch resolves, so
    // placeholders fill in as bytes arrive. Both captures are weak:
    // the source is kept alive by the pan/zoom callbacks (owned by
    // the instance), and the instance by the ComponentContainer — so
    // this closure never extends either's lifetime and there's no
    // Rc cycle.
    {
        let weak = instance.as_weak();
        let src_weak = Rc::downgrade(&source);
        source.on_tile_ready(move || {
            if let (Some(inst), Some(src)) = (weak.upgrade(), src_weak.upgrade()) {
                refresh_map(&inst, src.as_ref());
            }
        });
    }

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

/// Native (non-wasm) stub. The catalogue only runs in the browser and
/// `WasmOsmTileSource` is wasm-only, so the host `cargo check` (which
/// builds this crate as an rlib) compiles a no-op. Map pages on native
/// just render with empty tile models.
#[cfg(not(target_arch = "wasm32"))]
fn attach_map_handler(_instance: &ComponentInstance) {}

/// Read the page's `map-viewport-width` / `map-viewport-height`
/// properties (bound to the MapEmbed's measured size on the slint
/// side). Falls back to the cell's default 412 × 892 if the page
/// hasn't declared them — projection is approximately right for
/// full-bleed maps even then.
#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
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

/// `console.debug` — used for high-frequency, non-actionable notes
/// (e.g. map overscan) that should stay out of the default console
/// view but be available under the devtools "Verbose" filter. Only
/// referenced from the wasm-only tile source, so it's wasm-gated to
/// avoid a dead-code warning on the native `cargo check`.
#[cfg(target_arch = "wasm32")]
fn web_debug(msg: &str) {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn debug(s: &str);
    }
    debug(msg);
}
