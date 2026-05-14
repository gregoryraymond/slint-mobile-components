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
    // `ElementHandle` in i-slint-backend-testing (used by the behavior
    // tests) needs Slint's compile-time debug info to traverse the
    // element tree by accessible label / element id / type name. The
    // flag is also harmless when the behavior tests aren't running, so
    // we set it unconditionally — consumers who query elements at
    // runtime via accessibility get it for free.
    std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");

    slint_build::compile("ui/gallery.slint").expect("Slint build failed");
    slint_build::compile("tests/snapshot_scenes.slint").expect("Snapshot scenes build failed");
    slint_build::compile("tests/behavior_scenes.slint").expect("Behavior scenes build failed");

    // `ui/showcase.slint` tiles all ~145 page templates into one Window —
    // it's expensive to compile, so only build it for the `showcase`
    // example. Everyday `cargo check` / `cargo test` stays fast.
    if std::env::var_os("CARGO_FEATURE_SHOWCASE").is_some() {
        slint_build::compile("ui/showcase.slint").expect("Showcase build failed");
    }
}
