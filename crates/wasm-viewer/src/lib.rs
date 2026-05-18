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
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use slint_interpreter::{Compiler, Value};
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
/// return its parsed metadata. `_*` aggregators are skipped; map
/// pages (anything that imports from `@mapping/...`) are skipped too
/// because we don't bundle the mapping crate for v1.
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
            // Skip map pages — they import from @mapping/... which
            // isn't bundled. Trying to compile them would error.
            if text.contains("from \"@mapping/") {
                continue;
            }
            let Some(class) = scan_page_class(text) else {
                continue;
            };
            let stem = name.trim_end_matches(".slint").to_string();
            let virt = virtual_root().join(dir.path()).join(name);
            out.push(PageMeta {
                path: virt,
                class,
                display: stem,
            });
        }
    }
    out.sort_by(|a, b| a.display.cmp(&b.display));
    out
}

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
    Some(ComponentFactory::new(move |ctx| {
        def.create_embedded(ctx).ok()
    }))
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
