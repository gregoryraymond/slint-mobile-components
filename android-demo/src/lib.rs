//! Demo APK that runs the slint-mobile-components Gallery on Android.
//!
//! The Gallery is defined in the components crate's `ui/gallery.slint`;
//! this app's own `ui/main.slint` just re-exports it as `MainWindow`
//! and the build script wires `library_paths` so the
//! `@mobile-components/...` alias resolves correctly.

slint::include_modules!();

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("Slint Android init failed");
    let ui = MainWindow::new().expect("failed to construct MainWindow");
    ui.run().expect("event loop failed");
}
