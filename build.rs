// Compile a Window-rooted entry file purely to validate every component's
// `.slint` syntax on every `cargo check`. The file (and its transitive
// imports) exercises each component; nothing here ships to consumers.
//
// Consumers don't depend on the output of this build — they configure
// Slint's `library_paths` to point at `ui/` and import components by
// file path (`@mobile-components/button.slint`).
fn main() {
    slint_build::compile("ui/_check.slint").expect("Slint build failed");
}
