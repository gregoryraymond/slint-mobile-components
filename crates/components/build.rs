use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");

    let config = slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([(
        "mobile-theme".into(),
        PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),
    )]));

    slint_build::compile_with_config("ui/_validate.slint", config)
        .expect("Slint build failed");
}
