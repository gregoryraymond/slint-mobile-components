use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // The social pages import @mobile-theme/, @mobile-components/, and
    // @mobile-components/icons/... — wire those library_paths before
    // compiling this crate's _snapshot_scenes aggregator.
    let config = slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([
        (
            "mobile-theme".into(),
            PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),
        ),
        (
            "mobile-components".into(),
            PathBuf::from(slint_mobile_components_widgets::UI_LIBRARY_DIR),
        ),
    ]));
    slint_build::compile_with_config("ui/_snapshot_scenes.slint", config)
        .expect("Slint build failed");
}
