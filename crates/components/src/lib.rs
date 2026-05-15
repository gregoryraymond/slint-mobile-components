//! Reusable widget components for slint-mobile apps (Button, Card, AppBar,
//! BottomNav, ListItem, etc.). Depends on `slint-mobile-theme`.
//!
//! Consuming crates configure Slint's `library_paths` with
//! `(@mobile-components, slint_mobile_components_widgets::UI_LIBRARY_DIR)`
//! (and `@mobile-theme` from the theme crate) and import via:
//!
//! ```ignore
//! import { MobileButton } from "@mobile-components/button.slint";
//! import { AppBar } from "@mobile-components/app-bar.slint";
//! ```

mod _generated {
    include!(concat!(env!("OUT_DIR"), "/_validate.rs"));
}

pub use _generated::*;

/// Filesystem path to this crate's `ui/` directory. Pass this to
/// `slint_build::CompilerConfiguration::with_library_paths` under the
/// `mobile-components` alias from a consuming crate's `build.rs`.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
