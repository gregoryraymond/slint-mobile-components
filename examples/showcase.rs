//! Desktop review grid for slint-mobile-components.
//!
//! ```sh
//! cargo run --example showcase --features showcase
//! ```
//!
//! Tiles every page template at native phone resolution (412 × 892) in a
//! scrollable grid. Each screen has a tick (keep) / cross (redo) toggle
//! in its header; clicking one rewrites `showcase-verdicts.json` in the
//! repo root *immediately*, so feedback is captured in realtime — no
//! "save" step, no end-of-session dump.
//!
//! Verdict values: `0` unrated, `1` keep, `2` redo. Unrated screens are
//! omitted from the JSON file, so it only ever lists screens you've
//! actually judged.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use slint_mobile_components::Showcase;

// Page file-stems in the exact cell order of `ui/showcase.slint`. This
// list and that file are generated together by `.claude/gen_showcase.py`
// — regenerate both if the page set changes.
const PAGE_NAMES: &[&str] = &[
    "home", "settings", "login", "podcast", "inbox", "profile", "chat", "dashboard",
    "music-library", "onboarding", "photo-grid", "search-results", "checkout", "post-detail",
    "map", "video-player", "calendar", "notification-center", "wallet", "weather",
    "activity-rings", "order-tracking", "paywall", "comments", "task-list", "news-feed",
    "boarding-pass", "restaurant-menu", "leaderboard", "crypto-portfolio", "product-detail",
    "smart-home", "timer", "email-thread", "ride-share-booking", "meditation", "form-wizard",
    "account-settings", "help-center", "meal-log", "video-feed", "job-listing",
    "trip-itinerary", "currency-converter", "habit-tracker", "payment-split",
    "playlist-detail", "e-reader", "poll-results", "group-chat-list", "cart", "address-book",
    "weekly-meal-plan", "tv-show-detail", "post-creator", "code-review", "workout-session",
    "hotel-booking", "world-clock", "medication", "event-detail", "write-review",
    "journal-entry", "room-thermostat", "app-error", "app-lock", "subscriptions",
    "expense-report", "photo-viewer", "game-lobby", "document-scanner", "transit-departures",
    "voice-recorder", "payment-methods", "wifi-settings", "investment-detail",
    "app-permissions", "order-history", "reading-list", "review-summary", "flight-search",
    "gift-card", "wordle-puzzle", "onboarding-hint", "bug-report", "calculator",
    "message-composer", "tip-jar", "recipe", "multi-select-list", "signup",
    "app-store-listing", "album-detail", "countdown-event", "trending-topics",
    "welcome-splash", "insurance-claim", "country-selector", "smart-tv-remote", "pet-adoption",
    "carpool-search", "media-lockscreen", "voting-ballot", "driver-on-the-way",
    "timezone-converter", "invoice", "sleep-tracking", "grocery-list", "qr-scanner",
    "live-sports-score", "appearance-settings", "live-stream", "two-factor-auth",
    "storage-manager", "quiz", "profile-edit", "turn-by-turn-nav", "community-forum",
    "loyalty-card", "donation", "audiobook-player", "video-call", "file-browser",
    "savings-goal", "doctor-appointment", "stock-watchlist", "referral", "achievements",
    "parking-session", "budget-overview", "voicemail", "contact-detail", "seat-selection",
    "send-money", "net-worth", "dialer", "lab-results", "security-checkup", "store-locator",
    "equalizer", "camera-capture", "download-manager", "nutrition-label", "delivery-driver",
    "mortgage-calculator",
];

fn verdicts_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("showcase-verdicts.json")
}

/// Read `showcase-verdicts.json` into a name → verdict map. A missing
/// file or unparseable content yields an empty map (every screen
/// unrated) rather than an error — this is a review tool, not a build
/// step.
fn load_verdicts() -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(verdicts_path()) else {
        return map;
    };
    let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(&text)
    else {
        return map;
    };
    for (key, value) in obj {
        if let Some(n) = value.as_i64() {
            map.insert(key, n as i32);
        }
    }
    map
}

/// Rewrite `showcase-verdicts.json` — pretty-printed, key-sorted, with
/// unrated (`0`) screens omitted. Called on every verdict toggle.
fn save_verdicts(verdicts: &[i32]) {
    let mut obj = serde_json::Map::new();
    for (name, &v) in PAGE_NAMES.iter().zip(verdicts) {
        if v != 0 {
            obj.insert((*name).to_string(), serde_json::Value::from(v as i64));
        }
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .unwrap_or_else(|_| "{}".to_string());
    if let Err(e) = std::fs::write(verdicts_path(), json + "\n") {
        eprintln!(
            "showcase: could not write {}: {e}",
            verdicts_path().display()
        );
    }
}

fn summary(verdicts: &[i32]) -> String {
    let keep = verdicts.iter().filter(|&&v| v == 1).count();
    let redo = verdicts.iter().filter(|&&v| v == 2).count();
    let rated = keep + redo;
    format!(
        "{} screens   ·   {keep} keep   ·   {redo} redo   ·   {} unrated   ·   {rated} reviewed",
        verdicts.len(),
        verdicts.len() - rated,
    )
}

fn main() -> Result<(), slint::PlatformError> {
    let app = Showcase::new()?;

    // Runtime typeface override — re-run with a different SHOWCASE_FONT to
    // try another face; no rebuild needed since the binary is unchanged:
    //
    //   SHOWCASE_FONT="Nimbus Sans" cargo run --example showcase --features showcase
    //
    // An empty / unset value keeps the library default (Theme.font-family).
    match std::env::var("SHOWCASE_FONT") {
        Ok(font) if !font.trim().is_empty() => {
            app.set_app_font(font.trim().into());
            println!("showcase: font override — \"{}\"", font.trim());
        }
        _ => println!("showcase: font — library default (set SHOWCASE_FONT to try others)"),
    }

    // Seed verdicts from disk, ordered to match the cells.
    let stored = load_verdicts();
    let initial: Vec<i32> = PAGE_NAMES
        .iter()
        .map(|name| stored.get(*name).copied().unwrap_or(0))
        .collect();

    let titles: Vec<SharedString> = PAGE_NAMES.iter().map(|n| SharedString::from(*n)).collect();
    app.set_titles(ModelRc::new(VecModel::from(titles)));

    let verdicts: Rc<VecModel<i32>> = Rc::new(VecModel::from(initial.clone()));
    app.set_verdicts(ModelRc::from(verdicts.clone()));
    app.set_summary(summary(&initial).into());

    // Each tick / cross updates the model, persists immediately, and
    // refreshes the summary line.
    let weak = app.as_weak();
    let model = verdicts.clone();
    app.on_verdict_changed(move |index, value| {
        let index = index as usize;
        if index >= PAGE_NAMES.len() {
            return;
        }
        model.set_row_data(index, value);
        let current: Vec<i32> = model.iter().collect();
        save_verdicts(&current);
        if let Some(app) = weak.upgrade() {
            app.set_summary(summary(&current).into());
        }
    });

    println!(
        "showcase: {} screens — verdicts persist live to {}",
        PAGE_NAMES.len(),
        verdicts_path().display()
    );
    app.run()
}
