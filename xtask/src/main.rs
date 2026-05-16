//! Project tooling for slint-mobile-components.
//!
//! Subcommands:
//!
//! - `wire-pages`       — regenerate every `crates/pages-*/{Cargo.toml,build.rs,src/lib.rs}`.
//!                        Idempotent.
//! - `split-snapshots`  — pull page `Snap*Page` Windows out of the root
//!                        `tests/snapshot_scenes.slint` and emit
//!                        `crates/pages-<cat>/ui/_snapshot_scenes.slint`
//!                        per category. One-off; safe to re-run.
//!
//! Invoked as `cargo xtask <subcommand>` thanks to the alias in
//! `.cargo/config.toml`.

use std::path::PathBuf;
use std::process::ExitCode;

mod categories;
mod split_snapshots;
mod wire_pages;

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (which is the
/// `xtask/` directory).
pub fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is set when cargo invokes us");
    PathBuf::from(manifest)
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

fn usage() {
    eprintln!(
        "usage: cargo xtask <subcommand>\n\
         \n\
         subcommands:\n  \
           wire-pages        regenerate per-pages-crate scaffolding\n  \
           split-snapshots   split root snapshot_scenes.slint into per-category files"
    );
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = match args.next() {
        Some(c) => c,
        None => {
            usage();
            return ExitCode::FAILURE;
        }
    };
    let result = match cmd.as_str() {
        "wire-pages" => wire_pages::run(),
        "split-snapshots" => split_snapshots::run(),
        "-h" | "--help" | "help" => {
            usage();
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown subcommand: {other}");
            usage();
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
