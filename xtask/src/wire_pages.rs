//! Regenerates `Cargo.toml`, `build.rs`, and `src/lib.rs` in every
//! `crates/pages-<cat>/` directory. Each pages crate is a thin wrapper:
//! it ships `.slint` sources, compiles its own `_snapshot_scenes.slint`
//! aggregator, and exposes the resulting `Snap*Page` Rust types under
//! a `scenes` sub-module so the workspace root can `pub use
//! slint_mobile_pages_<cat>::scenes::*` without colliding on shared
//! names (`Theme`, widget types) across sibling pages crates.
//!
//! Idempotent. Re-run after editing this file, after adding pages, or
//! after introducing a new category.

use crate::{categories::CATEGORIES, workspace_root};
use std::fs;
use std::path::Path;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let crates_dir = root.join("crates");

    for (cat, _pages) in CATEGORIES {
        let crate_dir = crates_dir.join(format!("pages-{cat}"));
        if !crate_dir.is_dir() {
            return Err(format!(
                "missing crate directory: {} (did you create it before running wire-pages?)",
                crate_dir.display()
            )
            .into());
        }
        fs::create_dir_all(crate_dir.join("src"))?;
        fs::create_dir_all(crate_dir.join("ui"))?;

        write_cargo_toml(&crate_dir, cat)?;
        write_build_rs(&crate_dir, cat)?;
        let snap_count = write_lib_rs(&crate_dir, cat)?;
        println!("wired pages-{cat} ({snap_count} snap scenes)");
    }
    println!("done: {} crates", CATEGORIES.len());
    Ok(())
}

fn write_cargo_toml(crate_dir: &Path, cat: &str) -> std::io::Result<()> {
    let body = format!(
        "[package]\n\
         name = \"slint-mobile-pages-{cat}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         license = \"MIT OR Apache-2.0\"\n\
         description = \"Page templates ({cat} category) for slint-mobile-components.\"\n\
         publish = false\n\
         \n\
         [lib]\n\
         crate-type = [\"rlib\"]\n\
         \n\
         [dependencies]\n\
         slint = {{ version = \"1\", default-features = false, features = [\n    \
             \"compat-1-2\",\n    \
             \"std\",\n\
         ] }}\n\
         slint-mobile-theme = {{ path = \"../theme\" }}\n\
         slint-mobile-components-widgets = {{ path = \"../components\" }}\n\
         \n\
         [build-dependencies]\n\
         slint-build = \"1\"\n\
         slint-mobile-theme = {{ path = \"../theme\" }}\n\
         slint-mobile-components-widgets = {{ path = \"../components\" }}\n",
    );
    fs::write(crate_dir.join("Cargo.toml"), body)
}

fn write_build_rs(crate_dir: &Path, cat: &str) -> std::io::Result<()> {
    let body = format!(
        "use std::collections::HashMap;\n\
         use std::path::PathBuf;\n\
         \n\
         fn main() {{\n    \
             // The {cat} pages import @mobile-theme/, @mobile-components/, and\n    \
             // @mobile-components/icons/... — wire those library_paths before\n    \
             // compiling this crate's _snapshot_scenes aggregator.\n    \
             let config = slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([\n        \
                 (\n            \
                     \"mobile-theme\".into(),\n            \
                     PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),\n        \
                 ),\n        \
                 (\n            \
                     \"mobile-components\".into(),\n            \
                     PathBuf::from(slint_mobile_components_widgets::UI_LIBRARY_DIR),\n        \
                 ),\n    \
             ]));\n    \
             slint_build::compile_with_config(\"ui/_snapshot_scenes.slint\", config)\n        \
                 .expect(\"Slint build failed\");\n\
         }}\n",
    );
    fs::write(crate_dir.join("build.rs"), body)
}

/// Writes `src/lib.rs` and returns the number of Snap* names re-exported.
fn write_lib_rs(crate_dir: &Path, cat: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let scenes_path = crate_dir.join("ui").join("_snapshot_scenes.slint");
    let snaps = if scenes_path.exists() {
        extract_snap_names(&fs::read_to_string(&scenes_path)?)
    } else {
        Vec::new()
    };
    let snap_list = if snaps.is_empty() {
        String::new()
    } else {
        let mut s = String::new();
        for (i, name) in snaps.iter().enumerate() {
            s.push_str("        ");
            s.push_str(name);
            if i + 1 < snaps.len() {
                s.push(',');
            }
            s.push('\n');
        }
        s
    };
    let body = format!(
        "//! Page templates in the `{cat}` category, plus snapshot-test scene\n\
         //! wrappers (`Snap*Page` Windows) for every page. Consumers wire in\n\
         //! the `.slint` sources via the `mobile-pages-{cat}` library_paths alias.\n\
         \n\
         mod _generated_snapshot_scenes {{\n    \
             include!(concat!(env!(\"OUT_DIR\"), \"/_snapshot_scenes.rs\"));\n\
         }}\n\
         \n\
         /// Snapshot-test scene wrappers — kept inside a sub-module so the\n\
         /// workspace root can `pub use slint_mobile_pages_{cat}::scenes::*` to\n\
         /// surface just these names without dragging in `UI_LIBRARY_DIR` or the\n\
         /// slint-generated `Theme` / widget types (which would collide with the\n\
         /// identical names re-exported from sibling pages-* crates).\n\
         pub mod scenes {{\n    \
             pub use crate::_generated_snapshot_scenes::{{\n\
         {snap_list}    }};\n\
         }}\n\
         \n\
         /// Filesystem path to this crate's `ui/` directory.\n\
         pub const UI_LIBRARY_DIR: &str = concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/ui\");\n",
    );
    fs::write(crate_dir.join("src").join("lib.rs"), body)?;
    Ok(snaps.len())
}

/// Extract `Snap<X>` names from `^export component Snap<X> inherits Window`
/// lines. The format is regular enough that we don't need a regex engine.
fn extract_snap_names(slint: &str) -> Vec<String> {
    let prefix = "export component ";
    let mut names: Vec<String> = slint
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix(prefix)?;
            // `Snap…` token, then whitespace, then `inherits Window`.
            let mut it = rest.split_whitespace();
            let name = it.next()?;
            if !name.starts_with("Snap") {
                return None;
            }
            // Validate the rest matches `inherits Window`.
            if it.next()? != "inherits" {
                return None;
            }
            let after = it.next()?;
            if !after.starts_with("Window") {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}
