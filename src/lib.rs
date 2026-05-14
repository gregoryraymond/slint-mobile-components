//! Slint UI components and design tokens for mobile (Android) apps built
//! with Slint. Sister project to the `slint-mobile` cargo-generate template.
//!
//! The primary surface of this crate is **the `ui/` directory of `.slint`
//! files** — consumed via Slint's `library_paths`. The Rust side is thin
//! and exists mainly to (a) validate the `.slint` sources compile cleanly
//! in CI, and (b) hand consumers a stable path to the library entry point.
//!
//! # Consumption from a `slint-mobile`-generated app
//!
//! 1. Add this crate as a dependency in the app's `Cargo.toml`:
//!
//!    ```toml
//!    [dependencies]
//!    slint-mobile-components = { path = "../slint-mobile-components" }
//!
//!    [build-dependencies]
//!    slint-mobile-components = { path = "../slint-mobile-components" }
//!    ```
//!
//! 2. In the app's `build.rs`, point Slint at the components library:
//!
//!    ```ignore
//!    use std::collections::HashMap;
//!    use std::path::PathBuf;
//!
//!    fn main() {
//!        let config = slint_build::CompilerConfiguration::new()
//!            .with_library_paths(HashMap::from([(
//!                "mobile-components".into(),
//!                PathBuf::from(slint_mobile_components::UI_LIBRARY_DIR),
//!            )]));
//!        slint_build::compile_with_config("ui/main.slint", config)
//!            .expect("Slint build failed");
//!    }
//!    ```
//!
//! 3. Import each component by path through the `@mobile-components` alias:
//!
//!    ```ignore
//!    import { Theme } from "@mobile-components/theme.slint";
//!    import { MobileButton } from "@mobile-components/button.slint";
//!    import { Card } from "@mobile-components/card.slint";
//!    import { AppBar } from "@mobile-components/app-bar.slint";
//!    import { HomePage } from "@mobile-components/pages/home.slint";
//!    ```

// `slint::include_modules!()` only includes ONE file (the most recent
// call to slint_build::compile sets `SLINT_INCLUDE_GENERATED`), so when
// `build.rs` compiles multiple .slint inputs we have to include each
// generated module explicitly. We wrap each in its own private mod so
// the per-file `pub use` chains can't collide with one another (e.g.
// `BottomNavDistribution` is re-exported from every file that imports
// `bottom-nav.slint`).
mod _generated_gallery {
    include!(concat!(env!("OUT_DIR"), "/gallery.rs"));
}
mod _generated_snapshot_scenes {
    include!(concat!(env!("OUT_DIR"), "/snapshot_scenes.rs"));
}
mod _generated_behavior_scenes {
    include!(concat!(env!("OUT_DIR"), "/behavior_scenes.rs"));
}
// Only built under the `showcase` feature (see build.rs) — the review
// grid instantiates every page template at once and is slow to compile.
#[cfg(feature = "showcase")]
mod _generated_showcase {
    include!(concat!(env!("OUT_DIR"), "/showcase.rs"));
}

// Production surface — Gallery exposes the design-system globals
// (Theme, BottomNavDistribution) that consumers expect at the crate root.
pub use _generated_gallery::*;

// Test scenes — re-exported by exact name only, so they don't shadow
// the Theme / BottomNavDistribution from the gallery export above.
pub use _generated_behavior_scenes::{
    BehaviorBottomNav, BehaviorButtonClick, BehaviorCheckbox, BehaviorChip, BehaviorListItem,
    BehaviorRadio, BehaviorSegmented, BehaviorSlider, BehaviorStepper, BehaviorSwitchToggle,
    BehaviorTabBar, BehaviorTextField,
};
pub use _generated_snapshot_scenes::{
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
    SnapHabitTrackerPage, SnapHelpCenterPage, SnapHotelBookingPage, SnapIconButtonActive,
    SnapInboxPage, SnapInsuranceClaimPage, SnapInvestmentDetailPage, SnapInvoicePage,
    SnapJobListingPage, SnapJournalEntryPage, SnapLabResultsPage, SnapLeaderboardPage,
    SnapLiveSportsScorePage, SnapLiveStreamPage, SnapLoyaltyCardPage, SnapMapPage, SnapMealLogPage,
    SnapMediaPlayerLockscreenPage, SnapMedicationPage, SnapMeditationPage, SnapMessageComposerPage,
    SnapMobileButtonPrimary, SnapMobileButtonSecondary, SnapMortgageCalculatorPage,
    SnapMultiSelectListPage, SnapMusicLibraryPage, SnapNetWorthPage, SnapNewsArticleFeedPage,
    SnapNotificationCenterPage, SnapNutritionLabelPage, SnapOnboardingHintPage, SnapOnboardingPage,
    SnapOrderHistoryPage, SnapOrderTrackingPage, SnapParkingSessionPage, SnapPaymentMethodPage,
    SnapPaymentSplitPage, SnapPaywallPage, SnapPetAdoptionPage, SnapPhotoGridPage,
    SnapPhotoViewerPage, SnapPlaylistDetailPage, SnapPodcastPage, SnapPollResultsPage,
    SnapPostCreatorPage, SnapPostDetailPage, SnapProductDetailPage, SnapProfileEditPage,
    SnapProfilePage, SnapProgressDeterminate, SnapQRScannerPage, SnapQuizPage, SnapRadioGroup,
    SnapReadingListPage, SnapRecipePage, SnapReferralPage, SnapRestaurantMenuPage,
    SnapReviewSummaryPage, SnapRideShareBookingPage, SnapRoomThermostatPage, SnapSavingsGoalPage,
    SnapSearchResultsPage, SnapSeatSelectionPage, SnapSecurityCheckupPage, SnapSegmentedThree,
    SnapSendMoneyPage, SnapSignupPage, SnapSkeletonRow, SnapSleepTrackingPage, SnapSliderAt35,
    SnapSmartHomePage, SnapSmartTVRemotePage, SnapSnackbarTones, SnapSpinnerStatic, SnapStepperAt3,
    SnapStockWatchlistPage, SnapStorageManagerPage, SnapStoreLocatorPage, SnapSubscriptionsPage,
    SnapTVShowDetailPage, SnapTabBar, SnapTaskListPage, SnapTimerPage, SnapTimezoneConverterPage,
    SnapTipJarPage, SnapTransitDeparturesPage, SnapTrendingTopicsPage, SnapTripItineraryPage,
    SnapTurnByTurnNavPage, SnapTwoFactorAuthPage, SnapVideoCallPage, SnapVideoFeedPage,
    SnapVideoPlayerPage, SnapVoiceRecorderPage, SnapVoicemailPage, SnapVotingBallotPage,
    SnapWalletPage, SnapWeatherPage, SnapWeeklyMealPlanPage, SnapWelcomeSplashPage,
    SnapWiFiSettingsPage, SnapWordlePuzzlePage, SnapWorkoutSessionPage, SnapWorldClockPage,
    SnapWriteReviewPage,
};

// Desktop review grid — consumed by `examples/showcase.rs`.
#[cfg(feature = "showcase")]
pub use _generated_showcase::Showcase;

/// Filesystem path to this crate's `ui/` directory — the entry point Slint
/// resolves `@mobile-components/...` imports against. Pass this (wrapped in
/// a `PathBuf`) to `slint_build::CompilerConfiguration::with_library_paths`
/// from a consuming crate's `build.rs`.
pub const UI_LIBRARY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui");
