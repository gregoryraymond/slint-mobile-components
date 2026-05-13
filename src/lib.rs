//! Slint UI components and design tokens for mobile (Android) apps built
//! with Slint. Sister project to the `slint-mobile` cargo-generate template.
//!
//! The primary surface of this crate is **the `ui/` directory of `.slint`
//! files** — consumed via Slint's `library_paths`. The Rust side is thin
//! and exists mainly to (a) validate the `.slint` sources compile cleanly
//! in CI, and (b) hand consumers a stable path to the library entry point.
//!
//! # Consumption from a `slint-mobile`-generated app
//!
//! 1. Add this crate as a dependency in the app's `Cargo.toml`:
//!
//!    ```toml
//!    [dependencies]
//!    slint-mobile-components = { path = "../slint-mobile-components" }
//!
//!    [build-dependencies]
//!    slint-mobile-components = { path = "../slint-mobile-components" }
//!    ```
//!
//! 2. In the app's `build.rs`, point Slint at the components library:
//!
//!    ```ignore
//!    use std::collections::HashMap;
//!    use std::path::PathBuf;
//!
//!    fn main() {
//!        let config = slint_build::CompilerConfiguration::new()
//!            .with_library_paths(HashMap::from([(
//!                "mobile-components".into(),
//!                PathBuf::from(slint_mobile_components::UI_LIBRARY_DIR),
//!            )]));
//!        slint_build::compile_with_config("ui/main.slint", config)
//!            .expect("Slint build failed");
//!    }
//!    ```
//!
//! 3. Import each component by path through the `@mobile-components` alias:
//!
//!    ```ignore
//!    import { Theme } from "@mobile-components/theme.slint";
//!    import { MobileButton } from "@mobile-components/button.slint";
//!    import { Card } from "@mobile-components/card.slint";
//!    import { AppBar } from "@mobile-components/app-bar.slint";
//!    import { HomePage } from "@mobile-components/pages/home.slint";
//!    ```

// `slint::include_modules!()` only includes ONE file (the most recent
// call to slint_build::compile sets `SLINT_INCLUDE_GENERATED`), so when
// `build.rs` compiles multiple .slint inputs we have to include each
// generated module explicitly. We wrap each in its own private mod so
// the per-file `pub use` chains can't collide with one another (e.g.
// `BottomNavDistribution` is re-exported from every file that imports
// `bottom-nav.slint`).
mod _generated_gallery {
    include!(concat!(env!("OUT_DIR"), "/gallery.rs"));
}
mod _generated_snapshot_scenes {
    include!(concat!(env!("OUT_DIR"), "/snapshot_scenes.rs"));
}
mod _generated_behavior_scenes {
    include!(concat!(env!("OUT_DIR"), "/behavior_scenes.rs"));
}

// Production surface — Gallery exposes the design-system globals
// (Theme, BottomNavDistribution) that consumers expect at the crate root.
pub use _generated_gallery::*;

// Test scenes — re-exported by exact name only, so they don't shadow
// the Theme / BottomNavDistribution from the gallery export above.
pub use _generated_snapshot_scenes::{
    SnapAvatarSizes, SnapBadgeOnIcon, SnapBanner, SnapBottomNavSpaced,
    SnapCardWithSubtitle, SnapCheckboxPair, SnapChipRow, SnapIconButtonActive,
    SnapMobileButtonPrimary, SnapMobileButtonSecondary, SnapProgressDeterminate,
    SnapSliderAt35, SnapSpinnerStatic, SnapTabBar,
};
pub use _generated_behavior_scenes::{
    BehaviorBottomNav, BehaviorButtonClick, BehaviorCheckbox, BehaviorChip,
    BehaviorListItem, BehaviorSlider, BehaviorSwitchToggle, BehaviorTabBar,
    BehaviorTextField,
};

/// Filesystem path to this crate's `ui/` directory — the entry point Slint
/// resolves `@mobile-components/...` imports against. Pass this (wrapped in
/// a `PathBuf`) to `slint_build::CompilerConfiguration::with_library_paths`
/// from a consuming crate's `build.rs`.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
