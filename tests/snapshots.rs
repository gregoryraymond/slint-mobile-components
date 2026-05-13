//! Component snapshot harness.
//!
//! For each scene defined in `tests/snapshot_scenes.slint`, this binary
//! renders the scene to an RGB buffer via Slint's software renderer and
//! either writes the baseline PNG (when `SLINT_CREATE_SCREENSHOTS=1`) or
//! diffs against the existing baseline under
//! `tests/snapshot_baselines/`.
//!
//! Usage:
//!
//! ```sh
//! # Initial run / refresh baselines after an intended visual change:
//! SLINT_CREATE_SCREENSHOTS=1 cargo test --features snapshots --test snapshots
//!
//! # CI / verify nothing changed:
//! cargo test --features snapshots --test snapshots
//! ```
//!
//! On mismatch, the actual render is written next to the baseline as
//! `<name>.actual.png` so the diff can be inspected.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Once;

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel,
};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, PlatformError};

use slint_mobile_components::{
    SnapAvatarSizes, SnapBadgeOnIcon, SnapBanner, SnapBottomNavSpaced,
    SnapCardWithSubtitle, SnapCheckboxPair, SnapChipRow, SnapIconButtonActive,
    SnapMobileButtonPrimary, SnapMobileButtonSecondary, SnapProgressDeterminate,
    SnapSliderAt35, SnapSpinnerStatic, SnapTabBar,
};

// Allow at most this fraction of pixels to differ before we consider a
// snapshot test failed. Fonts and SVG rasterization are not perfectly
// deterministic across machines; 0.5 % absorbs that drift without
// hiding meaningful visual changes.
const FAIL_THRESHOLD_FRAC: f32 = 0.005;

thread_local! {
    static LAST_WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> =
        const { RefCell::new(None) };
}

struct SnapshotPlatform;

impl Platform for SnapshotPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        LAST_WINDOW.with(|cell| *cell.borrow_mut() = Some(window.clone()));
        Ok(window)
    }
}

fn ensure_platform() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        slint::platform::set_platform(Box::new(SnapshotPlatform))
            .expect("set_platform failed");
    });
}

fn baseline_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshot_baselines")
}

fn snapshot<T: ComponentHandle>(
    name: &str,
    width: u32,
    height: u32,
    factory: impl FnOnce() -> Result<T, PlatformError>,
) {
    ensure_platform();
    LAST_WINDOW.with(|c| c.borrow_mut().take());

    let _component = factory().expect("failed to construct component");
    let window = LAST_WINDOW
        .with(|c| c.borrow().clone())
        .expect("no window was created by the platform");

    window.set_size(PhysicalSize::new(width, height));
    window.request_redraw();

    let pixel_count = (width * height) as usize;
    let mut buffer = vec![Rgb565Pixel(0); pixel_count];
    let drew = window.draw_if_needed(|renderer| {
        renderer.render(&mut buffer, width as usize);
    });
    assert!(drew, "{name}: draw_if_needed returned false");

    // Rgb565 → Rgb888 expansion (replicate high bits into the low bits
    // so values like 0xff render as 0xff rather than 0xf8).
    let mut rgb8 = vec![0u8; pixel_count * 3];
    for (i, &Rgb565Pixel(p)) in buffer.iter().enumerate() {
        let r = ((p >> 11) & 0x1f) as u8;
        let g = ((p >> 5) & 0x3f) as u8;
        let b = (p & 0x1f) as u8;
        rgb8[i * 3] = (r << 3) | (r >> 2);
        rgb8[i * 3 + 1] = (g << 2) | (g >> 4);
        rgb8[i * 3 + 2] = (b << 3) | (b >> 2);
    }

    let actual = image::RgbImage::from_raw(width, height, rgb8)
        .expect("buffer length mismatch for actual image");
    let baseline_path = baseline_dir().join(format!("{name}.png"));

    let write_baseline =
        std::env::var("SLINT_CREATE_SCREENSHOTS").is_ok() || !baseline_path.exists();

    if write_baseline {
        std::fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();
        actual.save(&baseline_path).expect("save baseline");
        eprintln!("wrote baseline: {}", baseline_path.display());
        return;
    }

    let baseline = image::open(&baseline_path)
        .unwrap_or_else(|e| panic!("{name}: failed to open baseline: {e}"))
        .to_rgb8();
    assert_eq!(
        baseline.dimensions(),
        (width, height),
        "{name}: baseline dimensions {:?} != render {:?}",
        baseline.dimensions(),
        (width, height),
    );

    let mismatches = baseline
        .pixels()
        .zip(actual.pixels())
        .filter(|(a, b)| a != b)
        .count();
    let pct = mismatches as f32 / pixel_count as f32;
    if pct > FAIL_THRESHOLD_FRAC {
        let actual_path = baseline_path.with_extension("actual.png");
        actual.save(&actual_path).ok();
        panic!(
            "{name}: {:.3}% pixel mismatch (threshold {:.3}%). \
             Actual image written to {}",
            pct * 100.0,
            FAIL_THRESHOLD_FRAC * 100.0,
            actual_path.display(),
        );
    }
}

#[test]
fn render_snapshots() {
    snapshot("mobile-button-primary", 320, 80, SnapMobileButtonPrimary::new);
    snapshot("mobile-button-secondary", 320, 80, SnapMobileButtonSecondary::new);
    snapshot("card-with-subtitle", 320, 140, SnapCardWithSubtitle::new);
    snapshot("icon-button-active", 96, 96, SnapIconButtonActive::new);
    snapshot("bottom-nav-spaced", 412, 72, SnapBottomNavSpaced::new);
    snapshot("chip-row", 360, 56, SnapChipRow::new);
    snapshot("avatar-sizes", 200, 80, SnapAvatarSizes::new);
    snapshot("badge-on-icon", 72, 56, SnapBadgeOnIcon::new);
    snapshot("progress-determinate", 320, 32, SnapProgressDeterminate::new);
    snapshot("spinner-static", 96, 96, SnapSpinnerStatic::new);
    snapshot("checkbox-pair", 320, 112, SnapCheckboxPair::new);
    snapshot("slider-at-35", 320, 64, SnapSliderAt35::new);
    snapshot("tab-bar", 360, 48, SnapTabBar::new);
    snapshot("banner", 360, 96, SnapBanner::new);
}
