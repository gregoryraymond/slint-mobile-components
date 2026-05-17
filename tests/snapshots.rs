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

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, PlatformError};

use slint_mobile_components::{
    SnapAccountSettingsPage, SnapAchievementsPage, SnapActivityRingsPage, SnapAddressBookPage,
    SnapAlbumDetailPage, SnapAppErrorPage, SnapAppLockScreen, SnapAppPermissionsPage,
    SnapAppStoreListingPage, SnapAppearanceSettingsPage, SnapAudiobookPlayerPage, SnapAvatarSizes,
    SnapBadgeOnIcon, SnapBanner, SnapBannerTones, SnapBoardingPassPage, SnapBottomNavSpaced,
    SnapBudgetOverviewPage, SnapBugReportPage, SnapButtonTones, SnapCalculatorPage,
    SnapCalendarPage, SnapCameraCapturePage, SnapCardWithSubtitle, SnapCarpoolSearchPage,
    SnapCartPage, SnapChatPage, SnapCheckboxPair, SnapCheckoutPage, SnapChipRow,
    SnapCodeReviewPage, SnapCommentsPage, SnapCommunityForumPage, SnapContactDetailPage,
    SnapCountdownEventPage, SnapCountrySelectorPage, SnapCryptoPortfolioPage,
    SnapCurrencyConverterPage, SnapDashboardPage, SnapDeliveryDriverPage, SnapDialerPage,
    SnapDialog, SnapDoctorAppointmentPage, SnapDocumentScannerPage, SnapDonationPage,
    SnapDownloadManagerPage, SnapDrawer, SnapDriverOnTheWayPage, SnapEReaderPage,
    SnapEmailThreadPage, SnapEmptyState, SnapEqualizerPage, SnapEventDetailPage,
    SnapExpenseReportPage, SnapFileBrowserPage, SnapFlightSearchPage, SnapFormWizardPage,
    SnapGameLobbyPage, SnapGiftCardPage, SnapGroceryListPage, SnapGroupChatListPage,
    SnapHabitTrackerPage, SnapHelpCenterPage, SnapHomePage, SnapHotelBookingPage,
    SnapIconButtonActive, SnapInboxPage, SnapInsuranceClaimPage, SnapInvestmentDetailPage,
    SnapInvoicePage, SnapJobListingPage, SnapJournalEntryPage, SnapLabResultsPage,
    SnapLeaderboardPage, SnapLiveSportsScorePage, SnapLiveStreamPage, SnapLoginPage,
    SnapLoyaltyCardPage, SnapMapPage, SnapMealLogPage, SnapMediaPlayerLockscreenPage,
    SnapMedicationPage, SnapMeditationPage, SnapMessageComposerPage, SnapMobileButtonPrimary,
    SnapMobileButtonSecondary, SnapMortgageCalculatorPage, SnapMultiSelectListPage,
    SnapMusicLibraryPage, SnapNetWorthPage, SnapNewsArticleFeedPage, SnapNotificationCenterPage,
    SnapNutritionLabelPage, SnapOnboardingHintPage, SnapOnboardingPage, SnapOrderHistoryPage,
    SnapOrderTrackingPage, SnapParkingSessionPage, SnapPaymentMethodPage, SnapPaymentSplitPage,
    SnapPaywallPage, SnapPetAdoptionPage, SnapPhotoGridPage, SnapPhotoViewerPage,
    SnapPlaylistDetailPage, SnapPodcastPage, SnapPollResultsPage, SnapPostCreatorPage,
    SnapPostDetailPage, SnapProductDetailPage, SnapProfileEditPage, SnapProfilePage,
    SnapProgressDeterminate, SnapQRScannerPage, SnapQuizPage, SnapRadioGroup, SnapReadingListPage,
    SnapRecipePage, SnapReferralPage, SnapRestaurantMenuPage, SnapReviewSummaryPage,
    SnapRideShareBookingPage, SnapRoomThermostatPage, SnapSavingsGoalPage, SnapSearchResultsPage,
    SnapSeatSelectionPage, SnapSecurityCheckupPage, SnapSegmentedThree, SnapSendMoneyPage,
    SnapSignupPage, SnapSkeletonRow, SnapSleepTrackingPage, SnapSliderAt35, SnapSmartHomePage,
    SnapSmartTVRemotePage, SnapSnackbarTones, SnapSpinnerStatic, SnapStepperAt3,
    SnapStockWatchlistPage, SnapStorageManagerPage, SnapStoreLocatorPage, SnapSubscriptionsPage,
    SnapTVShowDetailPage, SnapTabBar, SnapTaskListPage, SnapTimerPage, SnapTimezoneConverterPage,
    SnapTipJarPage, SnapTransitDeparturesPage, SnapTrendingTopicsPage, SnapTripItineraryPage,
    SnapTurnByTurnNavPage, SnapTwoFactorAuthPage, SnapVideoCallPage, SnapVideoFeedPage,
    SnapVideoPlayerPage, SnapVoiceRecorderPage, SnapVoicemailPage, SnapVotingBallotPage,
    SnapWalletPage, SnapWeatherPage, SnapWeeklyMealPlanPage, SnapWelcomeSplashPage,
    SnapWiFiSettingsPage, SnapWordlePuzzlePage, SnapWorkoutSessionPage, SnapWorldClockPage,
    SnapWriteReviewPage,
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
        slint::platform::set_platform(Box::new(SnapshotPlatform)).expect("set_platform failed");
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
    snapshot(
        "mobile-button-primary",
        320,
        80,
        SnapMobileButtonPrimary::new,
    );
    snapshot(
        "mobile-button-secondary",
        320,
        80,
        SnapMobileButtonSecondary::new,
    );
    snapshot("card-with-subtitle", 320, 140, SnapCardWithSubtitle::new);
    snapshot("icon-button-active", 96, 96, SnapIconButtonActive::new);
    snapshot("bottom-nav-spaced", 412, 72, SnapBottomNavSpaced::new);
    snapshot("chip-row", 360, 56, SnapChipRow::new);
    snapshot("avatar-sizes", 200, 80, SnapAvatarSizes::new);
    snapshot("badge-on-icon", 72, 56, SnapBadgeOnIcon::new);
    snapshot(
        "progress-determinate",
        320,
        32,
        SnapProgressDeterminate::new,
    );
    snapshot("spinner-static", 96, 96, SnapSpinnerStatic::new);
    snapshot("checkbox-pair", 320, 112, SnapCheckboxPair::new);
    snapshot("slider-at-35", 320, 64, SnapSliderAt35::new);
    snapshot("tab-bar", 360, 48, SnapTabBar::new);
    snapshot("banner", 360, 96, SnapBanner::new);
    snapshot("radio-group", 320, 168, SnapRadioGroup::new);
    snapshot("segmented-three", 320, 56, SnapSegmentedThree::new);
    snapshot("stepper-at-3", 200, 56, SnapStepperAt3::new);
    snapshot("skeleton-row", 320, 64, SnapSkeletonRow::new);
    snapshot("empty-state", 360, 320, SnapEmptyState::new);
    snapshot("podcast-page", 412, 892, SnapPodcastPage::new);
    snapshot("inbox-page", 412, 892, SnapInboxPage::new);
    snapshot("profile-page", 412, 892, SnapProfilePage::new);
    snapshot("chat-page", 412, 892, SnapChatPage::new);
    snapshot("dashboard-page", 412, 892, SnapDashboardPage::new);
    snapshot("home-page", 412, 892, SnapHomePage::new);
    snapshot("login-page", 412, 892, SnapLoginPage::new);
    snapshot("music-library-page", 412, 892, SnapMusicLibraryPage::new);
    snapshot("onboarding-page", 412, 892, SnapOnboardingPage::new);
    snapshot("photo-grid-page", 412, 892, SnapPhotoGridPage::new);
    snapshot("search-results-page", 412, 892, SnapSearchResultsPage::new);
    snapshot("checkout-page", 412, 892, SnapCheckoutPage::new);
    snapshot("post-detail-page", 412, 892, SnapPostDetailPage::new);
    snapshot("map-page", 412, 892, SnapMapPage::new);
    snapshot("dialog", 412, 360, SnapDialog::new);
    snapshot("snackbar-tones", 360, 280, SnapSnackbarTones::new);
    snapshot("button-tones", 320, 360, SnapButtonTones::new);
    snapshot("banner-tones", 360, 320, SnapBannerTones::new);
    snapshot("drawer", 412, 480, SnapDrawer::new);
    snapshot("video-player-page", 412, 892, SnapVideoPlayerPage::new);
    snapshot("calendar-page", 412, 892, SnapCalendarPage::new);
    snapshot(
        "notification-center-page",
        412,
        892,
        SnapNotificationCenterPage::new,
    );
    snapshot("wallet-page", 412, 892, SnapWalletPage::new);
    snapshot("weather-page", 412, 892, SnapWeatherPage::new);
    snapshot("activity-rings-page", 412, 892, SnapActivityRingsPage::new);
    snapshot("order-tracking-page", 412, 892, SnapOrderTrackingPage::new);
    snapshot("paywall-page", 412, 892, SnapPaywallPage::new);
    snapshot("comments-page", 412, 892, SnapCommentsPage::new);
    snapshot("task-list-page", 412, 892, SnapTaskListPage::new);
    snapshot("news-feed-page", 412, 892, SnapNewsArticleFeedPage::new);
    snapshot("boarding-pass-page", 412, 892, SnapBoardingPassPage::new);
    snapshot(
        "restaurant-menu-page",
        412,
        892,
        SnapRestaurantMenuPage::new,
    );
    snapshot("leaderboard-page", 412, 892, SnapLeaderboardPage::new);
    snapshot(
        "crypto-portfolio-page",
        412,
        892,
        SnapCryptoPortfolioPage::new,
    );
    snapshot("product-detail-page", 412, 892, SnapProductDetailPage::new);
    snapshot("smart-home-page", 412, 892, SnapSmartHomePage::new);
    snapshot("timer-page", 412, 892, SnapTimerPage::new);
    snapshot("email-thread-page", 412, 892, SnapEmailThreadPage::new);
    snapshot(
        "ride-share-booking-page",
        412,
        892,
        SnapRideShareBookingPage::new,
    );
    snapshot("meditation-page", 412, 892, SnapMeditationPage::new);
    snapshot("form-wizard-page", 412, 892, SnapFormWizardPage::new);
    snapshot(
        "account-settings-page",
        412,
        892,
        SnapAccountSettingsPage::new,
    );
    snapshot("help-center-page", 412, 892, SnapHelpCenterPage::new);
    snapshot("meal-log-page", 412, 892, SnapMealLogPage::new);
    snapshot("video-feed-page", 412, 892, SnapVideoFeedPage::new);
    snapshot("job-listing-page", 412, 892, SnapJobListingPage::new);
    snapshot("trip-itinerary-page", 412, 892, SnapTripItineraryPage::new);
    snapshot(
        "currency-converter-page",
        412,
        892,
        SnapCurrencyConverterPage::new,
    );
    snapshot("habit-tracker-page", 412, 892, SnapHabitTrackerPage::new);
    snapshot("payment-split-page", 412, 892, SnapPaymentSplitPage::new);
    snapshot(
        "playlist-detail-page",
        412,
        892,
        SnapPlaylistDetailPage::new,
    );
    snapshot("e-reader-page", 412, 892, SnapEReaderPage::new);
    snapshot("poll-results-page", 412, 892, SnapPollResultsPage::new);
    snapshot("group-chat-list-page", 412, 892, SnapGroupChatListPage::new);
    snapshot("cart-page", 412, 892, SnapCartPage::new);
    snapshot("address-book-page", 412, 892, SnapAddressBookPage::new);
    snapshot(
        "weekly-meal-plan-page",
        412,
        892,
        SnapWeeklyMealPlanPage::new,
    );
    snapshot("tv-show-detail-page", 412, 892, SnapTVShowDetailPage::new);
    snapshot("post-creator-page", 412, 892, SnapPostCreatorPage::new);
    snapshot("code-review-page", 412, 892, SnapCodeReviewPage::new);
    snapshot(
        "workout-session-page",
        412,
        892,
        SnapWorkoutSessionPage::new,
    );
    snapshot("hotel-booking-page", 412, 892, SnapHotelBookingPage::new);
    snapshot("world-clock-page", 412, 892, SnapWorldClockPage::new);
    snapshot("medication-page", 412, 892, SnapMedicationPage::new);
    snapshot("event-detail-page", 412, 892, SnapEventDetailPage::new);
    snapshot("write-review-page", 412, 892, SnapWriteReviewPage::new);
    snapshot("journal-entry-page", 412, 892, SnapJournalEntryPage::new);
    snapshot(
        "room-thermostat-page",
        412,
        892,
        SnapRoomThermostatPage::new,
    );
    snapshot("app-error-page", 412, 892, SnapAppErrorPage::new);
    snapshot("app-lock-screen", 412, 892, SnapAppLockScreen::new);
    snapshot("subscriptions-page", 412, 892, SnapSubscriptionsPage::new);
    snapshot("expense-report-page", 412, 892, SnapExpenseReportPage::new);
    snapshot("photo-viewer-page", 412, 892, SnapPhotoViewerPage::new);
    snapshot("game-lobby-page", 412, 892, SnapGameLobbyPage::new);
    snapshot(
        "document-scanner-page",
        412,
        892,
        SnapDocumentScannerPage::new,
    );
    snapshot(
        "transit-departures-page",
        412,
        892,
        SnapTransitDeparturesPage::new,
    );
    snapshot("voice-recorder-page", 412, 892, SnapVoiceRecorderPage::new);
    snapshot("payment-methods-page", 412, 892, SnapPaymentMethodPage::new);
    snapshot("wifi-settings-page", 412, 892, SnapWiFiSettingsPage::new);
    snapshot(
        "investment-detail-page",
        412,
        892,
        SnapInvestmentDetailPage::new,
    );
    snapshot(
        "app-permissions-page",
        412,
        892,
        SnapAppPermissionsPage::new,
    );
    snapshot("order-history-page", 412, 892, SnapOrderHistoryPage::new);
    snapshot("reading-list-page", 412, 892, SnapReadingListPage::new);
    snapshot("review-summary-page", 412, 892, SnapReviewSummaryPage::new);
    snapshot("flight-search-page", 412, 892, SnapFlightSearchPage::new);
    snapshot("gift-card-page", 412, 892, SnapGiftCardPage::new);
    snapshot("wordle-puzzle-page", 412, 892, SnapWordlePuzzlePage::new);
    snapshot(
        "onboarding-hint-page",
        412,
        892,
        SnapOnboardingHintPage::new,
    );
    snapshot("bug-report-page", 412, 892, SnapBugReportPage::new);
    snapshot("calculator-page", 412, 892, SnapCalculatorPage::new);
    snapshot(
        "message-composer-page",
        412,
        892,
        SnapMessageComposerPage::new,
    );
    snapshot("tip-jar-page", 412, 892, SnapTipJarPage::new);
    snapshot("recipe-page", 412, 892, SnapRecipePage::new);
    snapshot(
        "multi-select-list-page",
        412,
        892,
        SnapMultiSelectListPage::new,
    );
    snapshot("signup-page", 412, 892, SnapSignupPage::new);
    snapshot(
        "app-store-listing-page",
        412,
        892,
        SnapAppStoreListingPage::new,
    );
    snapshot("album-detail-page", 412, 892, SnapAlbumDetailPage::new);
    snapshot(
        "countdown-event-page",
        412,
        892,
        SnapCountdownEventPage::new,
    );
    snapshot(
        "trending-topics-page",
        412,
        892,
        SnapTrendingTopicsPage::new,
    );
    snapshot("welcome-splash-page", 412, 892, SnapWelcomeSplashPage::new);
    snapshot(
        "insurance-claim-page",
        412,
        892,
        SnapInsuranceClaimPage::new,
    );
    snapshot(
        "country-selector-page",
        412,
        892,
        SnapCountrySelectorPage::new,
    );
    snapshot("smart-tv-remote-page", 412, 892, SnapSmartTVRemotePage::new);
    snapshot("pet-adoption-page", 412, 892, SnapPetAdoptionPage::new);
    snapshot("carpool-search-page", 412, 892, SnapCarpoolSearchPage::new);
    snapshot(
        "media-lockscreen-page",
        412,
        892,
        SnapMediaPlayerLockscreenPage::new,
    );
    snapshot("voting-ballot-page", 412, 892, SnapVotingBallotPage::new);
    snapshot(
        "driver-on-the-way-page",
        412,
        892,
        SnapDriverOnTheWayPage::new,
    );
    snapshot(
        "timezone-converter-page",
        412,
        892,
        SnapTimezoneConverterPage::new,
    );
    snapshot("invoice-page", 412, 892, SnapInvoicePage::new);
    snapshot("sleep-tracking-page", 412, 892, SnapSleepTrackingPage::new);
    snapshot("grocery-list-page", 412, 892, SnapGroceryListPage::new);
    snapshot("qr-scanner-page", 412, 892, SnapQRScannerPage::new);
    snapshot(
        "live-sports-score-page",
        412,
        892,
        SnapLiveSportsScorePage::new,
    );
    snapshot(
        "appearance-settings-page",
        412,
        892,
        SnapAppearanceSettingsPage::new,
    );
    snapshot("live-stream-page", 412, 892, SnapLiveStreamPage::new);
    snapshot("two-factor-auth-page", 412, 892, SnapTwoFactorAuthPage::new);
    snapshot(
        "storage-manager-page",
        412,
        892,
        SnapStorageManagerPage::new,
    );
    snapshot("quiz-page", 412, 892, SnapQuizPage::new);
    snapshot("profile-edit-page", 412, 892, SnapProfileEditPage::new);
    snapshot(
        "turn-by-turn-nav-page",
        412,
        892,
        SnapTurnByTurnNavPage::new,
    );
    snapshot(
        "community-forum-page",
        412,
        892,
        SnapCommunityForumPage::new,
    );
    snapshot("loyalty-card-page", 412, 892, SnapLoyaltyCardPage::new);
    snapshot("donation-page", 412, 892, SnapDonationPage::new);
    snapshot(
        "audiobook-player-page",
        412,
        892,
        SnapAudiobookPlayerPage::new,
    );
    snapshot("video-call-page", 412, 892, SnapVideoCallPage::new);
    snapshot("file-browser-page", 412, 892, SnapFileBrowserPage::new);
    snapshot("savings-goal-page", 412, 892, SnapSavingsGoalPage::new);
    snapshot(
        "doctor-appointment-page",
        412,
        892,
        SnapDoctorAppointmentPage::new,
    );
    snapshot(
        "stock-watchlist-page",
        412,
        892,
        SnapStockWatchlistPage::new,
    );
    snapshot("referral-page", 412, 892, SnapReferralPage::new);
    snapshot("achievements-page", 412, 892, SnapAchievementsPage::new);
    snapshot(
        "parking-session-page",
        412,
        892,
        SnapParkingSessionPage::new,
    );
    snapshot(
        "budget-overview-page",
        412,
        892,
        SnapBudgetOverviewPage::new,
    );
    snapshot("voicemail-page", 412, 892, SnapVoicemailPage::new);
    snapshot("contact-detail-page", 412, 892, SnapContactDetailPage::new);
    snapshot("seat-selection-page", 412, 892, SnapSeatSelectionPage::new);
    snapshot("send-money-page", 412, 892, SnapSendMoneyPage::new);
    snapshot("net-worth-page", 412, 892, SnapNetWorthPage::new);
    snapshot("dialer-page", 412, 892, SnapDialerPage::new);
    snapshot("lab-results-page", 412, 892, SnapLabResultsPage::new);
    snapshot(
        "security-checkup-page",
        412,
        892,
        SnapSecurityCheckupPage::new,
    );
    snapshot("store-locator-page", 412, 892, SnapStoreLocatorPage::new);
    snapshot("equalizer-page", 412, 892, SnapEqualizerPage::new);
    snapshot("camera-capture-page", 412, 892, SnapCameraCapturePage::new);
    snapshot(
        "download-manager-page",
        412,
        892,
        SnapDownloadManagerPage::new,
    );
    snapshot(
        "nutrition-label-page",
        412,
        892,
        SnapNutritionLabelPage::new,
    );
    snapshot(
        "delivery-driver-page",
        412,
        892,
        SnapDeliveryDriverPage::new,
    );
    snapshot(
        "mortgage-calculator-page",
        412,
        892,
        SnapMortgageCalculatorPage::new,
    );
}
