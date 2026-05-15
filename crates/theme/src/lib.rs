//! Design tokens (colours, spacing, typography, elevation, radii) and
//! the `Theme` global for slint-mobile-components.
//!
//! The primary surface of this crate is `ui/theme.slint`. Consuming
//! crates configure Slint's `library_paths` with
//! `(@mobile-theme, slint_mobile_theme::UI_LIBRARY_DIR)` and import via:
//!
//! ```ignore
//! import { Theme, Tone, ColorScheme } from "@mobile-theme/theme.slint";
//! ```

mod _generated {
    include!(concat!(env!("OUT_DIR"), "/_validate.rs"));
}

// Re-export the design-system types from the generated module so
// consumers can use `slint_mobile_theme::Theme` (etc.) from Rust.
pub use _generated::*;

/// Filesystem path to this crate's `ui/` directory. Pass this to
/// `slint_build::CompilerConfiguration::with_library_paths` under the
/// `mobile-theme` alias from a consuming crate's `build.rs`.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
