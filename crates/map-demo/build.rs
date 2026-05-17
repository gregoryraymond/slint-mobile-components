use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([
        (
            "mobile-theme".into(),
            PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),
        ),
        (
            "mobile-components".into(),
            PathBuf::from(slint_mobile_components_widgets::UI_LIBRARY_DIR),
        ),
        (
            "mapping".into(),
            PathBuf::from(slint_mapping::UI_LIBRARY_DIR),
        ),
        (
            "mobile-pages-travel".into(),
            PathBuf::from(slint_mobile_pages_travel::UI_LIBRARY_DIR),
        ),
        (
            "mobile-pages-commerce".into(),
            PathBuf::from(slint_mobile_pages_commerce::UI_LIBRARY_DIR),
        ),
    ]));
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint build failed");
}
