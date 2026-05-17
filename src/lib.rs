//! Slint UI components and design tokens for mobile (Android) apps built
//! with Slint. Sister project to the `slint-mobile` cargo-generate template.
//!
//! The primary surface of this crate is the **`.slint` sources shipped by
//! the sibling crates** (`slint-mobile-theme`, `slint-mobile-components-widgets`,
//! `slint-mobile-pages-*`), consumed via Slint's `library_paths`. The Rust
//! side is thin — it re-exports the test scenes for the snapshot/behavior
//! runners and exposes a single helper that hands consumers the full
//! `library_paths` alias map.
//!
//! # Consumption from a `slint-mobile`-generated app
//!
//! ```toml
//! [dependencies]
//! slint-mobile-components = { path = "../slint-mobile-components" }
//!
//! [build-dependencies]
//! slint-mobile-components = { path = "../slint-mobile-components" }
//! ```
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     let config = slint_build::CompilerConfiguration::new()
//!         .with_library_paths(slint_mobile_components::library_paths());
//!     slint_build::compile_with_config("ui/main.slint", config)
//!         .expect("Slint build failed");
//! }
//! ```
//!
//! ```ignore
//! // ui/main.slint
//! import { Theme } from "@mobile-theme/theme.slint";
//! import { MobileButton } from "@mobile-components/button.slint";
//! import { HomePage } from "@mobile-pages-misc/home.slint";
//! ```

mod _generated_snapshot_scenes {
    include!(concat!(env!("OUT_DIR"), "/snapshot_scenes.rs"));
}
mod _generated_behavior_scenes {
    include!(concat!(env!("OUT_DIR"), "/behavior_scenes.rs"));
}

// Test scenes — re-exported by exact name only, so they don't shadow
// names exported by the page-scene re-exports below.
pub use _generated_behavior_scenes::{
    BehaviorBottomNav, BehaviorButtonClick, BehaviorCheckbox, BehaviorChip, BehaviorListItem,
    BehaviorRadio, BehaviorSegmented, BehaviorSlider, BehaviorStepper, BehaviorSwitchToggle,
    BehaviorTabBar, BehaviorTextField,
};
// Widget-level snap scenes (root `tests/snapshot_scenes.slint`).
pub use _generated_snapshot_scenes::{
    SnapAvatarSizes, SnapBadgeOnIcon, SnapBanner, SnapBannerTones, SnapBottomNavSpaced,
    SnapButtonTones, SnapCardWithSubtitle, SnapCheckboxPair, SnapChipRow, SnapDialog, SnapDrawer,
    SnapEmptyState, SnapIconButtonActive, SnapMobileButtonPrimary, SnapMobileButtonSecondary,
    SnapProgressDeterminate, SnapRadioGroup, SnapSegmentedThree, SnapSkeletonRow, SnapSliderAt35,
    SnapSnackbarTones, SnapSpinnerStatic, SnapStepperAt3, SnapTabBar,
};
// Page-level snap scenes — one per category, codegen'd in their own crates
// so the root crate's link unit stays small.
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

/// Full set of Slint `library_paths` aliases this workspace publishes —
/// `@mobile-theme`, `@mobile-components`, and every `@mobile-pages-*`
/// category. Pass the returned map (or merge it into your own) into
/// `slint_build::CompilerConfiguration::with_library_paths` from a
/// consuming crate's `build.rs` so every `@mobile-*/foo.slint` import
/// resolves.
pub fn library_paths() -> std::collections::HashMap<String, std::path::PathBuf> {
    use std::path::PathBuf;
    [
        ("mobile-theme", slint_mobile_theme::UI_LIBRARY_DIR),
        (
            "mobile-components",
            slint_mobile_components_widgets::UI_LIBRARY_DIR,
        ),
        ("mobile-pages-auth", slint_mobile_pages_auth::UI_LIBRARY_DIR),
        (
            "mobile-pages-commerce",
            slint_mobile_pages_commerce::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-finance",
            slint_mobile_pages_finance::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-health",
            slint_mobile_pages_health::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-media",
            slint_mobile_pages_media::UI_LIBRARY_DIR,
        ),
        ("mobile-pages-misc", slint_mobile_pages_misc::UI_LIBRARY_DIR),
        (
            "mobile-pages-productivity",
            slint_mobile_pages_productivity::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-social",
            slint_mobile_pages_social::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-system",
            slint_mobile_pages_system::UI_LIBRARY_DIR,
        ),
        (
            "mobile-pages-travel",
            slint_mobile_pages_travel::UI_LIBRARY_DIR,
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), PathBuf::from(v)))
    .collect()
}
