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
use slint_interpreter::{ComponentDefinition, Compiler};

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
    page: &PageMeta,
) {
    let Some(def) = compiler.definition_for(page) else {
        return;
    };
    // `ComponentFactory::new` is generic over `T: ComponentHandle`. The
    // interpreter's `ComponentInstance` implements `ComponentHandle`, so
    // we return it directly — no `.into()`. (Forcing a conversion through
    // `VRc<ItemTreeVTable, _>` is ambiguous because every compiled
    // component also satisfies the bound.)
    let factory = ComponentFactory::new(move |ctx| def.create_embedded(ctx).ok());
    titles.push(SharedString::from(page.display.as_str()));
    cells.push(factory);
    viewer.set_loaded(titles.row_count() as i32);
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
        append_page(&viewer, &titles_model, &cells_model, &compiler, page);
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
                &pages[i],
            );
        });
    }

    viewer.run().expect("viewer event loop");
    drop(timer);
}
