//! Regenerates `ui/showcase.slint` (and the parallel `examples/showcase.rs`
//! PAGE_NAMES hint at `.claude/page_names.rs.txt`) from the
//! [`SHOWCASE_ORDER`] list. The order is significant — `showcase-verdicts.json`
//! is parallel-indexed, so moving a stem in this slice shifts every
//! verdict downstream of it.
//!
//! Page class names (e.g. `MortgageCalculatorPage`, `NewsArticleFeedPage`)
//! are *not* hardcoded; we scan each crate's `.slint` file at runtime for
//! its top-level `export component`. That keeps the order list and the
//! file content the only sources of truth.

use crate::{categories::category_of, workspace_root};
use std::fs;

/// Display order — every cell, top-to-bottom and left-to-right. Adding a
/// new page: append its stem here (and create the .slint file under the
/// matching `crates/pages-<cat>/ui/`). Removing a page: delete it here.
const SHOWCASE_ORDER: &[&str] = &[
    "home", "settings", "login", "podcast", "inbox", "profile", "chat", "dashboard",
    "music-library", "onboarding", "photo-grid", "search-results", "checkout", "post-detail",
    "map", "video-player", "calendar", "notification-center", "wallet", "weather",
    "activity-rings", "order-tracking", "paywall", "comments", "task-list", "news-feed",
    "boarding-pass", "restaurant-menu", "leaderboard", "crypto-portfolio", "product-detail",
    "smart-home", "timer", "email-thread", "ride-share-booking", "meditation", "form-wizard",
    "account-settings", "help-center", "meal-log", "video-feed", "job-listing", "trip-itinerary",
    "currency-converter", "habit-tracker", "payment-split", "playlist-detail", "e-reader",
    "poll-results", "group-chat-list", "cart", "address-book", "weekly-meal-plan",
    "tv-show-detail", "post-creator", "code-review", "workout-session", "hotel-booking",
    "world-clock", "medication", "event-detail", "write-review", "journal-entry",
    "room-thermostat", "app-error", "app-lock", "subscriptions", "expense-report", "photo-viewer",
    "game-lobby", "document-scanner", "transit-departures", "voice-recorder", "payment-methods",
    "wifi-settings", "investment-detail", "app-permissions", "order-history", "reading-list",
    "review-summary", "flight-search", "gift-card", "wordle-puzzle", "onboarding-hint",
    "bug-report", "calculator", "message-composer", "tip-jar", "recipe", "multi-select-list",
    "signup", "app-store-listing", "album-detail", "countdown-event", "trending-topics",
    "welcome-splash", "insurance-claim", "country-selector", "smart-tv-remote", "pet-adoption",
    "carpool-search", "media-lockscreen", "voting-ballot", "driver-on-the-way",
    "timezone-converter", "invoice", "sleep-tracking", "grocery-list", "qr-scanner",
    "live-sports-score", "appearance-settings", "live-stream", "two-factor-auth",
    "storage-manager", "quiz", "profile-edit", "turn-by-turn-nav", "community-forum",
    "loyalty-card", "donation", "audiobook-player", "video-call", "file-browser", "savings-goal",
    "doctor-appointment", "stock-watchlist", "referral", "achievements", "parking-session",
    "budget-overview", "voicemail", "contact-detail", "seat-selection", "send-money",
    "net-worth", "dialer", "lab-results", "security-checkup", "store-locator", "equalizer",
    "camera-capture", "download-manager", "nutrition-label", "delivery-driver",
    "mortgage-calculator",
];

const COLS: usize = 3;
const CELL_W: u32 = 412;
const CELL_H: u32 = 944; // 52 header + 892 phone
const GAP: u32 = 16;
const PAD: u32 = 16;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Scrape (stem -> page class name) from each .slint file. The "page
    // class" is the last `export component XxxPage|XxxScreen inherits`
    // line in the file — by convention, internal helpers come first and
    // the page-level component is the export at the bottom.
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(SHOWCASE_ORDER.len());
    for stem in SHOWCASE_ORDER {
        let cat = category_of(stem);
        let path = root
            .join("crates")
            .join(format!("pages-{cat}"))
            .join("ui")
            .join(format!("{stem}.slint"));
        if !path.exists() {
            return Err(format!(
                "missing page source for stem '{stem}' — expected {}",
                path.display()
            )
            .into());
        }
        let text = fs::read_to_string(&path)?;
        let class = last_page_class(&text).ok_or_else(|| {
            format!(
                "no `export component XxxPage|XxxScreen` found in {}",
                path.display()
            )
        })?;
        pairs.push((stem.to_string(), class));
    }

    let rows = pairs.len().div_ceil(COLS);
    let viewport_w = COLS as u32 * CELL_W + (COLS as u32 - 1) * GAP + 2 * PAD;
    let viewport_h = rows as u32 * CELL_H + (rows as u32 - 1) * GAP + 2 * PAD;

    let showcase_path = root.join("ui").join("showcase.slint");
    fs::write(&showcase_path, emit_showcase(&pairs, rows, viewport_w, viewport_h))?;
    println!(
        "wrote {} ({} cells, {} rows, viewport {}x{})",
        showcase_path.display(),
        pairs.len(),
        rows,
        viewport_w,
        viewport_h
    );

    let names_path = root.join(".claude").join("page_names.rs.txt");
    fs::write(&names_path, emit_page_names_rs(&pairs))?;
    println!("wrote {}", names_path.display());
    Ok(())
}

fn emit_showcase(pairs: &[(String, String)], rows: usize, vw: u32, vh: u32) -> String {
    let mut s = String::with_capacity(64 * 1024);
    s.push_str(HEADER);
    for (stem, comp) in pairs {
        let cat = category_of(stem);
        s.push_str(&format!(
            "import {{ {comp} }} from \"@mobile-pages-{cat}/{stem}.slint\";\n"
        ));
    }
    s.push('\n');
    s.push_str(CELL_COMPONENT);
    s.push_str(SHOWCASE_HEAD_AND_SUMMARY);
    s.push_str(&format!(
        "        ScrollView {{\n            \
             viewport-width: {vw}px;\n            \
             viewport-height: {vh}px;\n            \
             VerticalLayout {{\n                \
                 width: {vw}px;\n                \
                 padding: {PAD}px;\n                \
                 spacing: {GAP}px;\n                \
                 alignment: start;\n",
    ));
    for r in 0..rows {
        s.push_str("                HorizontalLayout {\n");
        s.push_str(&format!("                    spacing: {GAP}px;\n"));
        s.push_str("                    alignment: start;\n");
        for c in 0..COLS {
            let i = r * COLS + c;
            if i >= pairs.len() {
                break;
            }
            let (_, comp) = &pairs[i];
            s.push_str("                    ShowcaseCell {\n");
            s.push_str(&format!("                        title: root.titles[{i}];\n"));
            s.push_str(&format!("                        verdict: root.verdicts[{i}];\n"));
            s.push_str(&format!(
                "                        set-verdict(v) => {{ root.verdict-changed({i}, v); }}\n"
            ));
            s.push_str(&format!(
                "                        {comp} {{ width: 412px; height: 892px; }}\n"
            ));
            s.push_str("                    }\n");
        }
        s.push_str("                }\n");
    }
    s.push_str("            }\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("}\n");
    s
}

/// Find the *last* `export component XxxPage|XxxScreen inherits …`
/// component in a `.slint` source. Internal helpers come first by
/// convention; the page-level component is the export at the bottom.
fn last_page_class(slint: &str) -> Option<String> {
    let prefix = "export component ";
    let mut hit: Option<String> = None;
    for line in slint.lines() {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let mut it = rest.split_whitespace();
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        let first = name.chars().next();
        let suffix_ok = name.ends_with("Page") || name.ends_with("Screen");
        let starts_upper = matches!(first, Some(c) if c.is_ascii_uppercase());
        if !suffix_ok || !starts_upper {
            continue;
        }
        if it.next() != Some("inherits") {
            continue;
        }
        hit = Some(name.to_string());
    }
    hit
}

fn emit_page_names_rs(pairs: &[(String, String)]) -> String {
    let mut out = String::from("const PAGE_NAMES: &[&str] = &[\n");
    let mut line = String::from("    ");
    for (stem, _) in pairs {
        let tok = format!("\"{stem}\", ");
        if line.len() + tok.len() > 96 {
            out.push_str(line.trim_end());
            out.push('\n');
            line.clear();
            line.push_str("    ");
        }
        line.push_str(&tok);
    }
    if !line.trim().is_empty() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.push_str("];\n");
    out
}

const HEADER: &str = "// Showcase — desktop review grid.
// =====================================================================
//
// Tiles every page template at native phone resolution (412x892), each
// with a pass / fail verdict toggle in a header strip. Run with:
//
//   cargo run --example showcase --features showcase
//
// `examples/showcase.rs` loads verdicts from `showcase-verdicts.json`,
// and rewrites that file in realtime on every tick / cross. This file
// is generated by `cargo xtask gen-showcase` — edit that, not this.

import { Theme } from \"@mobile-theme/theme.slint\";
import { ScrollView } from \"std-widgets.slint\";
";

const CELL_COMPONENT: &str = "// One review cell: a verdict header strip above the phone screen.
// `verdict`: 0 unrated / 1 keep / 2 redo. The phone screen is injected
// as `@children` into a fixed 412x892 clipped frame.
component ShowcaseCell inherits Rectangle {
    in property <string> title;
    in property <int> verdict;
    callback set-verdict(int);

    width: 412px;
    height: 944px;
    background: Theme.surface-1;
    border-radius: Theme.radius-md;
    border-width: 2px;
    border-color: root.verdict == 1 ? Theme.accent-success
        : root.verdict == 2 ? Theme.accent-danger
        : Theme.surface-variant;
    clip: true;

    VerticalLayout {
        spacing: 0;
        // Verdict header.
        Rectangle {
            height: 52px;
            background: Theme.surface-2;
            HorizontalLayout {
                padding-left: 12px;
                padding-right: 8px;
                spacing: 6px;
                alignment: stretch;
                VerticalLayout {
                    horizontal-stretch: 1;
                    alignment: center;
                    Text {
                        text: root.title;
                        color: Theme.on-surface;
                        font-size: 15px;
                        font-weight: 800;
                        overflow: elide;
                    }
                }
                // Keep (tick).
                VerticalLayout {
                    alignment: center;
                    Rectangle {
                        width: 40px;
                        height: 40px;
                        border-radius: 20px;
                        background: root.verdict == 1 ? Theme.accent-success
                            : keep-touch.pressed ? Theme.surface-pressed : Theme.surface-1;
                        animate background { duration: Theme.motion-fast; }
                        Image {
                            width: 20px;
                            height: 20px;
                            x: (parent.width - self.width) / 2;
                            y: (parent.height - self.height) / 2;
                            source: @image-url(\"@mobile-components/icons/check.svg\");
                            colorize: root.verdict == 1 ? Theme.on-primary : Theme.muted;
                            image-fit: contain;
                        }
                        keep-touch := TouchArea {
                            clicked => { root.set-verdict(root.verdict == 1 ? 0 : 1); }
                        }
                    }
                }
                // Redo (cross).
                VerticalLayout {
                    alignment: center;
                    Rectangle {
                        width: 40px;
                        height: 40px;
                        border-radius: 20px;
                        background: root.verdict == 2 ? Theme.accent-danger
                            : redo-touch.pressed ? Theme.surface-pressed : Theme.surface-1;
                        animate background { duration: Theme.motion-fast; }
                        Image {
                            width: 18px;
                            height: 18px;
                            x: (parent.width - self.width) / 2;
                            y: (parent.height - self.height) / 2;
                            source: @image-url(\"@mobile-components/icons/close.svg\");
                            colorize: root.verdict == 2 ? Theme.on-primary : Theme.muted;
                            image-fit: contain;
                        }
                        redo-touch := TouchArea {
                            clicked => { root.set-verdict(root.verdict == 2 ? 0 : 2); }
                        }
                    }
                }
            }
        }
        // Phone screen (412x892), injected by the caller.
        Rectangle {
            width: 412px;
            height: 892px;
            clip: true;
            @children
        }
    }
}

";

const SHOWCASE_HEAD_AND_SUMMARY: &str = "export component Showcase inherits Window {
    title: \"slint-mobile-components — showcase\";
    preferred-width: 1340px;
    preferred-height: 940px;
    background: Theme.background;
    // The typeface every tiled page inherits (any Text without its own
    // font-family picks this up). Defaults to the library token; examples/showcase.rs
    // overrides it at runtime from the SHOWCASE_FONT env var — no rebuild.
    in property <string> app-font: Theme.font-family;
    default-font-family: root.app-font;

    // Filled in by examples/showcase.rs. `verdicts` and `titles` are
    // parallel to the cell order below; `summary` is recomputed on
    // every change.
    in property <[string]> titles;
    in-out property <[int]> verdicts;
    in property <string> summary: \"Loading…\";
    // (cell-index, new-verdict) — Rust persists + refreshes summary.
    callback verdict-changed(int, int);

    VerticalLayout {
        spacing: 0;
        // Summary bar.
        Rectangle {
            height: 56px;
            background: Theme.surface-1;
            drop-shadow-blur: Theme.elevation-1-blur;
            drop-shadow-color: Theme.elevation-1-color;
            drop-shadow-offset-y: Theme.elevation-1-y;
            HorizontalLayout {
                padding-left: 20px;
                padding-right: 20px;
                spacing: 16px;
                Text {
                    text: root.summary;
                    color: Theme.on-surface;
                    font-size: 16px;
                    font-weight: 800;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                Text {
                    text: \"tick = keep   ·   cross = redo   ·   saved live to showcase-verdicts.json\";
                    color: Theme.muted;
                    font-size: 13px;
                    font-weight: 600;
                    vertical-alignment: center;
                }
            }
        }
";
