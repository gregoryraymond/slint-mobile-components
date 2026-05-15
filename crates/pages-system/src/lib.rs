//! Page templates in the `system` category. The crate ships `.slint`
//! source files only — consumers wire them in via `slint-build` library
//! paths under the `mobile-pages-system` alias.

/// Filesystem path to this crate's `ui/` directory.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
