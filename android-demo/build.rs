fn main() {
    // Reach the components/theme/pages-* `.slint` sources through the
    // aliases the root crate publishes (`@mobile-theme`, `@mobile-components`,
    // every `@mobile-pages-*`, plus `@mobile-aggregator` for `gallery.slint`).
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(slint_mobile_components::library_paths());
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint build failed");
}
