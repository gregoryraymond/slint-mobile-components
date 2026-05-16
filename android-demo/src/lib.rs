//! Minimal Android demo APK. Exists so the workspace compiles end-to-end
//! for an Android target — the real "browse the screen library"
//! experience is the desktop viewer (`cargo run` at workspace root).
//!
//! `ui/main.slint` re-exports `HomePage` from
//! `crates/pages-misc/ui/home.slint` as `MainWindow`. Replace it with
//! your own scene if you want to build an actual app on top of this
//! library — see `android-demo/build.rs` for how to wire the
//! `library_paths` so `@mobile-*/` imports resolve.

slint::include_modules!();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Slint Android init failed");
    let ui = MainWindow::new().expect("failed to construct MainWindow");
    ui.run().expect("event loop failed");
}
