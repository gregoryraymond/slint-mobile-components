use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // Wire Slint's `library_paths` so `@mobile-components/...` resolves
    // to the sibling components crate's `ui/` directory. This is the
    // pattern documented in the components crate's README for consumers.
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        HashMap::from([(
            "mobile-components".into(),
            PathBuf::from(slint_mobile_components::UI_LIBRARY_DIR),
        )]),
    );
    slint_build::compile_with_config("ui/main.slint", config)
        .expect("Slint build failed");
}
