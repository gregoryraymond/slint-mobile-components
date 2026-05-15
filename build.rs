// Compile `ui/gallery.slint`, which is Window-rooted and uses every
// component in the library — this serves two purposes at once:
//
//   1. CI validation: every component's `.slint` syntax is type-checked
//      on each `cargo check` (gallery.slint imports them transitively).
//   2. Desktop preview: with `--features gallery`, the `gallery` example
//      runs the resulting `Gallery` Window so you can see the library.
//
// Consumers never depend on this build's output — they configure Slint's
// `library_paths` to point at `ui/` and import individual files via
// `@mobile-components/<file>.slint`.
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // `ElementHandle` in i-slint-backend-testing (used by the behavior
    // tests) needs Slint's compile-time debug info to traverse the
    // element tree by accessible label / element id / type name. The
    // flag is also harmless when the behavior tests aren't running, so
    // we set it unconditionally — consumers who query elements at
    // runtime via accessibility get it for free.
    std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");

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

    slint_build::compile_with_config("ui/gallery.slint", config())
        .expect("Slint build failed");
    slint_build::compile_with_config("tests/snapshot_scenes.slint", config())
        .expect("Snapshot scenes build failed");
    slint_build::compile_with_config("tests/behavior_scenes.slint", config())
        .expect("Behavior scenes build failed");

    // `ui/showcase.slint` tiles all ~145 page templates into one Window —
    // it's expensive to compile, so only build it for the `showcase`
    // example. Everyday `cargo check` / `cargo test` stays fast.
    if std::env::var_os("CARGO_FEATURE_SHOWCASE").is_some() {
        slint_build::compile_with_config("ui/showcase.slint", config())
            .expect("Showcase build failed");
    }
}
