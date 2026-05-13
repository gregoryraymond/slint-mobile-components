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
fn main() {
    slint_build::compile("ui/gallery.slint").expect("Slint build failed");
}
