// Root crate's slint build: snapshot scenes (widget-level only — page
// snaps live in each pages-* crate) and behavior scenes. Consumers
// never depend on this build's output; they call
// `slint_mobile_components::library_paths()` and import individual
// files via the `@mobile-*` aliases.
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // `ElementHandle` in i-slint-backend-testing (used by the behavior
    // tests) needs Slint's compile-time debug info to traverse the
    // element tree by accessible label / element id / type name. The
    // metadata is enormous (it's literally extra strings + element
    // tables baked into every generated component) and pushes rustc's
    // memory through the roof on `snapshot_scenes.slint`. Only set it
    // when the behaviors feature is on — that's the only consumer.
    if std::env::var_os("CARGO_FEATURE_BEHAVIORS").is_some() {
        std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");
    }

    let config = || {
        slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([
            (
                "mobile-theme".into(),
                PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),
            ),
            (
                "mobile-components".into(),
                PathBuf::from(slint_mobile_components_widgets::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-auth".into(),
                PathBuf::from(slint_mobile_pages_auth::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-commerce".into(),
                PathBuf::from(slint_mobile_pages_commerce::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-finance".into(),
                PathBuf::from(slint_mobile_pages_finance::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-health".into(),
                PathBuf::from(slint_mobile_pages_health::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-media".into(),
                PathBuf::from(slint_mobile_pages_media::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-misc".into(),
                PathBuf::from(slint_mobile_pages_misc::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-productivity".into(),
                PathBuf::from(slint_mobile_pages_productivity::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-social".into(),
                PathBuf::from(slint_mobile_pages_social::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-system".into(),
                PathBuf::from(slint_mobile_pages_system::UI_LIBRARY_DIR),
            ),
            (
                "mobile-pages-travel".into(),
                PathBuf::from(slint_mobile_pages_travel::UI_LIBRARY_DIR),
            ),
        ]))
    };

    slint_build::compile_with_config("tests/snapshot_scenes.slint", config())
        .expect("Snapshot scenes build failed");
    slint_build::compile_with_config("tests/behavior_scenes.slint", config())
        .expect("Behavior scenes build failed");
}
