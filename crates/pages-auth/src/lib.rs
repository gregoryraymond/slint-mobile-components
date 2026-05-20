//! Page templates in the `auth` category, plus snapshot-test scene
//! wrappers (`Snap*Page` Windows) for every page. Consumers wire in
//! the `.slint` sources via the `mobile-pages-auth` library_paths alias.

mod _generated_snapshot_scenes {
    include!(concat!(env!("OUT_DIR"), "/_snapshot_scenes.rs"));
}

/// Snapshot-test scene wrappers — kept inside a sub-module so the
/// workspace root can `pub use slint_mobile_pages_auth::scenes::*` to
/// surface just these names without dragging in `UI_LIBRARY_DIR` or the
/// slint-generated `Theme` / widget types (which would collide with the
/// identical names re-exported from sibling pages-* crates).
pub mod scenes {
    pub use crate::_generated_snapshot_scenes::{
        SnapAppLockScreen, SnapAppPermissionsPage, SnapLoginPage, SnapOnboardingHintPage,
        SnapOnboardingPage, SnapSignupPage, SnapTwoFactorAuthPage, SnapWelcomeSplashPage,
    };
}

/// Filesystem path to this crate's `ui/` directory.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
