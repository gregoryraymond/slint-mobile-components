#!/usr/bin/env python3
"""One-off generator for ui/showcase.slint — the desktop review grid that
tiles every page template at native phone resolution with a pass/fail
verdict toggle. Kept in .claude/ (not shipped); rerun if the page set
changes. Also prints the Rust PAGE_NAMES block for examples/showcase.rs."""
import os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# (file-stem, exported-component) in display order. Order MUST match the
# PAGE_NAMES const in examples/showcase.rs.
PAIRS = [
    ("home", "HomePage"), ("settings", "SettingsPage"), ("login", "LoginPage"),
    ("podcast", "PodcastPage"), ("inbox", "InboxPage"), ("profile", "ProfilePage"),
    ("chat", "ChatPage"), ("dashboard", "DashboardPage"), ("music-library", "MusicLibraryPage"),
    ("onboarding", "OnboardingPage"), ("photo-grid", "PhotoGridPage"), ("search-results", "SearchResultsPage"),
    ("checkout", "CheckoutPage"), ("post-detail", "PostDetailPage"), ("map", "MapPage"),
    ("video-player", "VideoPlayerPage"), ("calendar", "CalendarPage"), ("notification-center", "NotificationCenterPage"),
    ("wallet", "WalletPage"), ("weather", "WeatherPage"), ("activity-rings", "ActivityRingsPage"),
    ("order-tracking", "OrderTrackingPage"), ("paywall", "PaywallPage"), ("comments", "CommentsPage"),
    ("task-list", "TaskListPage"), ("news-feed", "NewsArticleFeedPage"), ("boarding-pass", "BoardingPassPage"),
    ("restaurant-menu", "RestaurantMenuPage"), ("leaderboard", "LeaderboardPage"), ("crypto-portfolio", "CryptoPortfolioPage"),
    ("product-detail", "ProductDetailPage"), ("smart-home", "SmartHomePage"), ("timer", "TimerPage"),
    ("email-thread", "EmailThreadPage"), ("ride-share-booking", "RideShareBookingPage"), ("meditation", "MeditationPage"),
    ("form-wizard", "FormWizardPage"), ("account-settings", "AccountSettingsPage"), ("help-center", "HelpCenterPage"),
    ("meal-log", "MealLogPage"), ("video-feed", "VideoFeedPage"), ("job-listing", "JobListingPage"),
    ("trip-itinerary", "TripItineraryPage"), ("currency-converter", "CurrencyConverterPage"), ("habit-tracker", "HabitTrackerPage"),
    ("payment-split", "PaymentSplitPage"), ("playlist-detail", "PlaylistDetailPage"), ("e-reader", "EReaderPage"),
    ("poll-results", "PollResultsPage"), ("group-chat-list", "GroupChatListPage"), ("cart", "CartPage"),
    ("address-book", "AddressBookPage"), ("weekly-meal-plan", "WeeklyMealPlanPage"), ("tv-show-detail", "TVShowDetailPage"),
    ("post-creator", "PostCreatorPage"), ("code-review", "CodeReviewPage"), ("workout-session", "WorkoutSessionPage"),
    ("hotel-booking", "HotelBookingPage"), ("world-clock", "WorldClockPage"), ("medication", "MedicationPage"),
    ("event-detail", "EventDetailPage"), ("write-review", "WriteReviewPage"), ("journal-entry", "JournalEntryPage"),
    ("room-thermostat", "RoomThermostatPage"), ("app-error", "AppErrorPage"), ("app-lock", "AppLockScreen"),
    ("subscriptions", "SubscriptionsPage"), ("expense-report", "ExpenseReportPage"), ("photo-viewer", "PhotoViewerPage"),
    ("game-lobby", "GameLobbyPage"), ("document-scanner", "DocumentScannerPage"), ("transit-departures", "TransitDeparturesPage"),
    ("voice-recorder", "VoiceRecorderPage"), ("payment-methods", "PaymentMethodPage"), ("wifi-settings", "WiFiSettingsPage"),
    ("investment-detail", "InvestmentDetailPage"), ("app-permissions", "AppPermissionsPage"), ("order-history", "OrderHistoryPage"),
    ("reading-list", "ReadingListPage"), ("review-summary", "ReviewSummaryPage"), ("flight-search", "FlightSearchPage"),
    ("gift-card", "GiftCardPage"), ("wordle-puzzle", "WordlePuzzlePage"), ("onboarding-hint", "OnboardingHintPage"),
    ("bug-report", "BugReportPage"), ("calculator", "CalculatorPage"), ("message-composer", "MessageComposerPage"),
    ("tip-jar", "TipJarPage"), ("recipe", "RecipePage"), ("multi-select-list", "MultiSelectListPage"),
    ("signup", "SignupPage"), ("app-store-listing", "AppStoreListingPage"), ("album-detail", "AlbumDetailPage"),
    ("countdown-event", "CountdownEventPage"), ("trending-topics", "TrendingTopicsPage"), ("welcome-splash", "WelcomeSplashPage"),
    ("insurance-claim", "InsuranceClaimPage"), ("country-selector", "CountrySelectorPage"), ("smart-tv-remote", "SmartTVRemotePage"),
    ("pet-adoption", "PetAdoptionPage"), ("carpool-search", "CarpoolSearchPage"), ("media-lockscreen", "MediaPlayerLockscreenPage"),
    ("voting-ballot", "VotingBallotPage"), ("driver-on-the-way", "DriverOnTheWayPage"), ("timezone-converter", "TimezoneConverterPage"),
    ("invoice", "InvoicePage"), ("sleep-tracking", "SleepTrackingPage"), ("grocery-list", "GroceryListPage"),
    ("qr-scanner", "QRScannerPage"), ("live-sports-score", "LiveSportsScorePage"), ("appearance-settings", "AppearanceSettingsPage"),
    ("live-stream", "LiveStreamPage"), ("two-factor-auth", "TwoFactorAuthPage"), ("storage-manager", "StorageManagerPage"),
    ("quiz", "QuizPage"), ("profile-edit", "ProfileEditPage"), ("turn-by-turn-nav", "TurnByTurnNavPage"),
    ("community-forum", "CommunityForumPage"), ("loyalty-card", "LoyaltyCardPage"), ("donation", "DonationPage"),
    ("audiobook-player", "AudiobookPlayerPage"), ("video-call", "VideoCallPage"), ("file-browser", "FileBrowserPage"),
    ("savings-goal", "SavingsGoalPage"), ("doctor-appointment", "DoctorAppointmentPage"), ("stock-watchlist", "StockWatchlistPage"),
    ("referral", "ReferralPage"), ("achievements", "AchievementsPage"), ("parking-session", "ParkingSessionPage"),
    ("budget-overview", "BudgetOverviewPage"), ("voicemail", "VoicemailPage"), ("contact-detail", "ContactDetailPage"),
    ("seat-selection", "SeatSelectionPage"), ("send-money", "SendMoneyPage"), ("net-worth", "NetWorthPage"),
    ("dialer", "DialerPage"), ("lab-results", "LabResultsPage"), ("security-checkup", "SecurityCheckupPage"),
    ("store-locator", "StoreLocatorPage"), ("equalizer", "EqualizerPage"), ("camera-capture", "CameraCapturePage"),
    ("download-manager", "DownloadManagerPage"), ("nutrition-label", "NutritionLabelPage"), ("delivery-driver", "DeliveryDriverPage"),
    ("mortgage-calculator", "MortgageCalculatorPage"),
]

# Sanity checks ----------------------------------------------------------
on_disk = sorted(f[:-6] for f in os.listdir(os.path.join(ROOT, "ui/pages")) if f.endswith(".slint"))
listed = sorted(s for s, _ in PAIRS)
missing = [s for s in on_disk if s not in listed]
extra = [s for s in listed if s not in on_disk]
if missing or extra:
    print(f"MISMATCH  missing-from-script={missing}  not-on-disk={extra}", file=sys.stderr)
    sys.exit(1)
assert len(PAIRS) == len(on_disk), f"{len(PAIRS)} != {len(on_disk)}"

COLS = 3
CELL_W = 412
CELL_H = 944          # 52px header + 892px phone
GAP = 16
PAD = 16
rows = (len(PAIRS) + COLS - 1) // COLS
viewport_w = COLS * CELL_W + (COLS - 1) * GAP + 2 * PAD
viewport_h = rows * CELL_H + (rows - 1) * GAP + 2 * PAD

# Emit ui/showcase.slint -------------------------------------------------
out = []
out.append("// Showcase — desktop review grid.")
out.append("// =====================================================================")
out.append("//")
out.append("// Tiles every page template at native phone resolution (412x892), each")
out.append("// with a pass / fail verdict toggle in a header strip. Run with:")
out.append("//")
out.append("//   cargo run --example showcase --features showcase")
out.append("//")
out.append("// `examples/showcase.rs` loads verdicts from `showcase-verdicts.json`,")
out.append("// and rewrites that file in realtime on every tick / cross. This file")
out.append("// is generated by `.claude/gen_showcase.py` — edit that, not this.")
out.append("")
out.append('import { Theme } from "theme.slint";')
out.append('import { ScrollView } from "std-widgets.slint";')
for stem, comp in PAIRS:
    out.append(f'import {{ {comp} }} from "pages/{stem}.slint";')
out.append("")
out.append("// One review cell: a verdict header strip above the phone screen.")
out.append("// `verdict`: 0 unrated / 1 keep / 2 redo. The phone screen is injected")
out.append("// as `@children` into a fixed 412x892 clipped frame.")
out.append("component ShowcaseCell inherits Rectangle {")
out.append("    in property <string> title;")
out.append("    in property <int> verdict;")
out.append("    callback set-verdict(int);")
out.append("")
out.append(f"    width: {CELL_W}px;")
out.append(f"    height: {CELL_H}px;")
out.append("    background: Theme.surface-1;")
out.append("    border-radius: Theme.radius-md;")
out.append("    border-width: 2px;")
out.append("    border-color: root.verdict == 1 ? Theme.accent-success")
out.append("        : root.verdict == 2 ? Theme.accent-danger")
out.append("        : Theme.surface-variant;")
out.append("    clip: true;")
out.append("")
out.append("    VerticalLayout {")
out.append("        spacing: 0;")
out.append("        // Verdict header.")
out.append("        Rectangle {")
out.append("            height: 52px;")
out.append("            background: Theme.surface-2;")
out.append("            HorizontalLayout {")
out.append("                padding-left: 12px;")
out.append("                padding-right: 8px;")
out.append("                spacing: 6px;")
out.append("                alignment: stretch;")
out.append("                VerticalLayout {")
out.append("                    horizontal-stretch: 1;")
out.append("                    alignment: center;")
out.append("                    Text {")
out.append("                        text: root.title;")
out.append("                        color: Theme.on-surface;")
out.append("                        font-size: 15px;")
out.append("                        font-weight: 800;")
out.append("                        overflow: elide;")
out.append("                    }")
out.append("                }")
out.append("                // Keep (tick).")
out.append("                VerticalLayout {")
out.append("                    alignment: center;")
out.append("                    Rectangle {")
out.append("                        width: 40px;")
out.append("                        height: 40px;")
out.append("                        border-radius: 20px;")
out.append("                        background: root.verdict == 1 ? Theme.accent-success")
out.append("                            : keep-touch.pressed ? Theme.surface-pressed : Theme.surface-1;")
out.append("                        animate background { duration: Theme.motion-fast; }")
out.append("                        Image {")
out.append("                            width: 20px;")
out.append("                            height: 20px;")
out.append("                            x: (parent.width - self.width) / 2;")
out.append("                            y: (parent.height - self.height) / 2;")
out.append('                            source: @image-url("icons/check.svg");')
out.append("                            colorize: root.verdict == 1 ? Theme.on-primary : Theme.muted;")
out.append("                            image-fit: contain;")
out.append("                        }")
out.append("                        keep-touch := TouchArea {")
out.append("                            clicked => { root.set-verdict(root.verdict == 1 ? 0 : 1); }")
out.append("                        }")
out.append("                    }")
out.append("                }")
out.append("                // Redo (cross).")
out.append("                VerticalLayout {")
out.append("                    alignment: center;")
out.append("                    Rectangle {")
out.append("                        width: 40px;")
out.append("                        height: 40px;")
out.append("                        border-radius: 20px;")
out.append("                        background: root.verdict == 2 ? Theme.accent-danger")
out.append("                            : redo-touch.pressed ? Theme.surface-pressed : Theme.surface-1;")
out.append("                        animate background { duration: Theme.motion-fast; }")
out.append("                        Image {")
out.append("                            width: 18px;")
out.append("                            height: 18px;")
out.append("                            x: (parent.width - self.width) / 2;")
out.append("                            y: (parent.height - self.height) / 2;")
out.append('                            source: @image-url("icons/close.svg");')
out.append("                            colorize: root.verdict == 2 ? Theme.on-primary : Theme.muted;")
out.append("                            image-fit: contain;")
out.append("                        }")
out.append("                        redo-touch := TouchArea {")
out.append("                            clicked => { root.set-verdict(root.verdict == 2 ? 0 : 2); }")
out.append("                        }")
out.append("                    }")
out.append("                }")
out.append("            }")
out.append("        }")
out.append("        // Phone screen (412x892), injected by the caller.")
out.append("        Rectangle {")
out.append("            width: 412px;")
out.append("            height: 892px;")
out.append("            clip: true;")
out.append("            @children")
out.append("        }")
out.append("    }")
out.append("}")
out.append("")
out.append("export component Showcase inherits Window {")
out.append('    title: "slint-mobile-components — showcase";')
out.append("    preferred-width: 1340px;")
out.append("    preferred-height: 940px;")
out.append("    background: Theme.background;")
out.append("    // The typeface every tiled page inherits (any Text without its own")
out.append("    // font-family picks this up). Defaults to the library token; "
           "examples/showcase.rs")
out.append("    // overrides it at runtime from the SHOWCASE_FONT env var — no rebuild.")
out.append("    in property <string> app-font: Theme.font-family;")
out.append("    default-font-family: root.app-font;")
out.append("")
out.append("    // Filled in by examples/showcase.rs. `verdicts` and `titles` are")
out.append("    // parallel to the cell order below; `summary` is recomputed on")
out.append("    // every change.")
out.append("    in property <[string]> titles;")
out.append("    in-out property <[int]> verdicts;")
out.append('    in property <string> summary: "Loading…";')
out.append("    // (cell-index, new-verdict) — Rust persists + refreshes summary.")
out.append("    callback verdict-changed(int, int);")
out.append("")
out.append("    VerticalLayout {")
out.append("        spacing: 0;")
out.append("        // Summary bar.")
out.append("        Rectangle {")
out.append("            height: 56px;")
out.append("            background: Theme.surface-1;")
out.append("            drop-shadow-blur: Theme.elevation-1-blur;")
out.append("            drop-shadow-color: Theme.elevation-1-color;")
out.append("            drop-shadow-offset-y: Theme.elevation-1-y;")
out.append("            HorizontalLayout {")
out.append("                padding-left: 20px;")
out.append("                padding-right: 20px;")
out.append("                spacing: 16px;")
out.append("                Text {")
out.append("                    text: root.summary;")
out.append("                    color: Theme.on-surface;")
out.append("                    font-size: 16px;")
out.append("                    font-weight: 800;")
out.append("                    vertical-alignment: center;")
out.append("                    horizontal-stretch: 1;")
out.append("                }")
out.append("                Text {")
out.append('                    text: "tick = keep   ·   cross = redo   ·   saved live to showcase-verdicts.json";')
out.append("                    color: Theme.muted;")
out.append("                    font-size: 13px;")
out.append("                    font-weight: 600;")
out.append("                    vertical-alignment: center;")
out.append("                }")
out.append("            }")
out.append("        }")
out.append("        ScrollView {")
out.append(f"            viewport-width: {viewport_w}px;")
out.append(f"            viewport-height: {viewport_h}px;")
out.append("            VerticalLayout {")
out.append(f"                width: {viewport_w}px;")
out.append(f"                padding: {PAD}px;")
out.append(f"                spacing: {GAP}px;")
out.append("                alignment: start;")

for r in range(rows):
    out.append("                HorizontalLayout {")
    out.append(f"                    spacing: {GAP}px;")
    out.append("                    alignment: start;")
    for c in range(COLS):
        i = r * COLS + c
        if i >= len(PAIRS):
            break
        stem, comp = PAIRS[i]
        out.append("                    ShowcaseCell {")
        out.append(f"                        title: root.titles[{i}];")
        out.append(f"                        verdict: root.verdicts[{i}];")
        out.append(f"                        set-verdict(v) => {{ root.verdict-changed({i}, v); }}")
        out.append(f"                        {comp} {{ width: 412px; height: 892px; }}")
        out.append("                    }")
    out.append("                }")

out.append("            }")
out.append("        }")
out.append("    }")
out.append("}")
out.append("")

with open(os.path.join(ROOT, "ui/showcase.slint"), "w") as f:
    f.write("\n".join(out))

# Emit the Rust PAGE_NAMES block ----------------------------------------
names = ", ".join(f'"{s}"' for s, _ in PAIRS)
rust = "const PAGE_NAMES: &[&str] = &[\n"
line = "    "
for s, _ in PAIRS:
    tok = f'"{s}", '
    if len(line) + len(tok) > 96:
        rust += line.rstrip() + "\n"
        line = "    "
    line += tok
rust += line.rstrip() + "\n];"
with open(os.path.join(ROOT, ".claude/page_names.rs.txt"), "w") as f:
    f.write(rust + "\n")

print(f"wrote ui/showcase.slint  ({len(PAIRS)} cells, {rows} rows, "
      f"viewport {viewport_w}x{viewport_h})")
print("wrote .claude/page_names.rs.txt")
