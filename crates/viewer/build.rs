use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // `ComponentContainer` and the `component-factory` type are gated
    // behind Slint's experimental flag. Without this env var, both
    // compile to "unknown element / type". The flag is read by the
    // slint compiler each call; we also set it in main.rs so the
    // runtime slint-interpreter compilations honour it identically.
    std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");

    // Compile only the viewer chrome (Window + pagination + 20
    // ComponentContainer slots). Page templates themselves are NEVER
    // codegen'd into this binary — they're parsed on demand by
    // slint-interpreter at runtime, so the rustc compile cost stays
    // tiny no matter how many pages exist.
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
    slint_build::compile_with_config("ui/viewer.slint", config).expect("Slint build failed");
}
