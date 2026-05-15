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
// `ui/gallery.slint` is only compiled under `--features gallery` because
// it pulls in every widget + ~80 pages and is the largest single slint
// codegen in the workspace. Each component / page crate validates its
// own .slint sources independently, so non-gallery builds skip it.
#[cfg(feature = "gallery")]
mod _generated_gallery {
    include!(concat!(env!("OUT_DIR"), "/gallery.rs"));
}
mod _generated_snapshot_scenes {
    include!(concat!(env!("OUT_DIR"), "/snapshot_scenes.rs"));
}
mod _generated_behavior_scenes {
    include!(concat!(env!("OUT_DIR"), "/behavior_scenes.rs"));
}
// Only built under the `showcase` feature (see build.rs) — the review
// grid instantiates every page template at once and is slow to compile.
#[cfg(feature = "showcase")]
mod _generated_showcase {
    include!(concat!(env!("OUT_DIR"), "/showcase.rs"));
}

// Theme / Tone live in the dedicated `slint-mobile-theme` crate now —
// consumers import them from there. Gallery is only generated under
// `--features gallery` because of its weight.
#[cfg(feature = "gallery")]
pub use _generated_gallery::Gallery;

// Test scenes — re-exported by exact name only, so they don't shadow
// the Theme / BottomNavDistribution from the gallery export above.
pub use _generated_behavior_scenes::{
    BehaviorBottomNav, BehaviorButtonClick, BehaviorCheckbox, BehaviorChip, BehaviorListItem,
    BehaviorRadio, BehaviorSegmented, BehaviorSlider, BehaviorStepper, BehaviorSwitchToggle,
    BehaviorTabBar, BehaviorTextField,
};
// Widget-level snap scenes live in this crate's `tests/snapshot_scenes.slint`.
// Page-level snaps (`Snap*Page` Windows) moved into per-category pages-*
// crates so each one codegens in its own rustc invocation — split below.
pub use _generated_snapshot_scenes::{
    SnapAvatarSizes, SnapBadgeOnIcon, SnapBanner, SnapBannerTones, SnapBottomNavSpaced,
    SnapButtonTones, SnapCardWithSubtitle, SnapCheckboxPair, SnapChipRow, SnapDialog, SnapDrawer,
    SnapEmptyState, SnapIconButtonActive, SnapMobileButtonPrimary, SnapMobileButtonSecondary,
    SnapProgressDeterminate, SnapRadioGroup, SnapSegmentedThree, SnapSkeletonRow, SnapSliderAt35,
    SnapSnackbarTones, SnapSpinnerStatic, SnapStepperAt3, SnapTabBar,
};
pub use slint_mobile_pages_auth::scenes::*;
pub use slint_mobile_pages_commerce::scenes::*;
pub use slint_mobile_pages_finance::scenes::*;
pub use slint_mobile_pages_health::scenes::*;
pub use slint_mobile_pages_media::scenes::*;
pub use slint_mobile_pages_misc::scenes::*;
pub use slint_mobile_pages_productivity::scenes::*;
pub use slint_mobile_pages_social::scenes::*;
pub use slint_mobile_pages_system::scenes::*;
pub use slint_mobile_pages_travel::scenes::*;

// Desktop review grid — consumed by `examples/showcase.rs`.
#[cfg(feature = "showcase")]
pub use _generated_showcase::Showcase;

/// Filesystem path to this crate's `ui/` directory — holds the aggregator
/// `.slint` files (`gallery.slint`, `showcase.slint`). Most consumers want
/// the per-crate `UI_LIBRARY_DIR` constants on the sibling sub-crates
/// (e.g. `slint_mobile_theme::UI_LIBRARY_DIR`,
/// `slint_mobile_components_widgets::UI_LIBRARY_DIR`). Use
/// [`library_paths`] to get the full alias → path map in one call.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");

/// Full set of Slint `library_paths` aliases this workspace publishes —
/// `@mobile-theme`, `@mobile-components`, and every `@mobile-pages-*`
/// category. Pass the returned map (or merge it into your own) into
/// `slint_build::CompilerConfiguration::with_library_paths` from a
/// consuming crate's `build.rs` so every `@mobile-*/foo.slint` import
/// resolves.
pub fn library_paths() -> std::collections::HashMap<String, std::path::PathBuf> {
    use std::path::PathBuf;
    [
        ("mobile-aggregator", UI_LIBRARY_DIR),
        ("mobile-theme", slint_mobile_theme::UI_LIBRARY_DIR),
        ("mobile-components", slint_mobile_components_widgets::UI_LIBRARY_DIR),
        ("mobile-pages-auth", slint_mobile_pages_auth::UI_LIBRARY_DIR),
        ("mobile-pages-commerce", slint_mobile_pages_commerce::UI_LIBRARY_DIR),
        ("mobile-pages-finance", slint_mobile_pages_finance::UI_LIBRARY_DIR),
        ("mobile-pages-health", slint_mobile_pages_health::UI_LIBRARY_DIR),
        ("mobile-pages-media", slint_mobile_pages_media::UI_LIBRARY_DIR),
        ("mobile-pages-misc", slint_mobile_pages_misc::UI_LIBRARY_DIR),
        ("mobile-pages-productivity", slint_mobile_pages_productivity::UI_LIBRARY_DIR),
        ("mobile-pages-social", slint_mobile_pages_social::UI_LIBRARY_DIR),
        ("mobile-pages-system", slint_mobile_pages_system::UI_LIBRARY_DIR),
        ("mobile-pages-travel", slint_mobile_pages_travel::UI_LIBRARY_DIR),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), PathBuf::from(v)))
    .collect()
}
