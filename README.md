# slint-mobile-components

A [Slint](https://slint.dev) UI component library and design system for
**mobile (Android) apps** built in Rust. Sister project to the
[`slint-mobile`](../slint-mobile) `cargo-generate` template — slint-mobile
generates the app skeleton; this crate supplies the visual language.

The primary surface is the `ui/` directory of `.slint` files, consumed
from a sibling project via Slint's `library_paths`. A thin Rust crate
wraps it so `cargo check` validates every component in CI.

## See it: the Gallery preview

```sh
cargo run --example gallery --features gallery
```

A desktop window opens at the mobile aspect ratio (412 × 892). A tab
strip at the top switches between:

- **Home / Settings / Login** — the three example pages
- **Toolbox** — every component in the library, captioned and scrollable

Use this during development to iterate on components without spinning up
an Android emulator. The `gallery` Cargo feature pulls in Slint's winit
backend + femtovg renderer; it's off by default so Android consumers
aren't forced to drag those crates in.

> On Linux, the renderer needs a few system packages — see
> `just install-host-deps` (pkg-config, fontconfig, freetype, clang, cmake,
> ninja). The slint-mobile devcontainer already has them.

## What's inside

```
slint-mobile-components/
├── Cargo.toml
├── build.rs                # slint-build → ui/lib.slint
├── rust-toolchain.toml
├── justfile                # fmt / clippy / check / test / ci
├── .github/workflows/ci.yml
├── src/
│   └── lib.rs              # slint::include_modules!() + UI_LIBRARY_DIR
├── examples/
│   └── gallery.rs          # Desktop preview (--features gallery)
├── android-demo/           # cargo-apk demo APK that runs the Gallery
├── maestro/                # On-device E2E (`just maestro`)
│   ├── config.yaml
│   └── flows/
│       ├── gallery-tabs.yaml
│       └── baselines/      # Committed E2E baseline PNGs
├── tests/
│   ├── snapshot_scenes.slint
│   ├── snapshots.rs        # Visual regression (--features snapshots)
│   ├── snapshot_baselines/ # Committed golden PNGs
│   ├── behavior_scenes.slint
│   └── behavior.rs         # Behavior tests (--features behaviors)
└── ui/
    ├── gallery.slint       # Desktop preview Window + CI validation entry
    ├── theme.slint         # Theme global (colors, spacing, type, motion)
    ├── button.slint        # MobileButton, TextButton, Fab
    ├── card.slint          # Card
    ├── app-bar.slint       # AppBar (top)
    ├── bottom-nav.slint    # BottomNav, BottomNavItem
    ├── list-item.slint     # ListItem
    ├── text-field.slint    # TextField
    ├── switch.slint        # MobileSwitch
    ├── chip.slint          # Chip (toggleable, dismissible)
    ├── avatar.slint        # Avatar (image + initials fallback)
    ├── badge.slint         # Badge (dot / count overlay)
    ├── progress-bar.slint  # ProgressBar (determinate + indeterminate)
    ├── spinner.slint       # Spinner (continuous-rotate loader)
    ├── checkbox.slint      # Checkbox (square multi-select toggle)
    ├── slider.slint        # Slider (0..1 value picker, drag + tap)
    ├── tab-bar.slint       # TabBar + Tab (top tab strip, sibling to BottomNav)
    ├── divider.slint       # Divider (hairline separator)
    ├── banner.slint        # Banner (inline info strip with optional action)
    ├── radio.slint         # Radio (single-choice button; parent manages group)
    ├── segmented-control.slint  # SegmentedControl (iOS-style 2–4 choice strip)
    ├── stepper.slint       # Stepper (numeric +/- with bounds)
    ├── skeleton.slint      # Skeleton (animated loading placeholder)
    ├── empty-state.slint   # EmptyState (illustration + title + description + CTA)
    ├── bottom-bar.slint    # BottomBar (pinned bottom chrome with upward shadow)
    ├── bottom-sheet.slint  # BottomSheet (partial-height surface with drag handle, surface-2)
    ├── dialog.slint        # Dialog (centered modal: title + body + actions; danger flag)
    ├── snackbar.slint      # Snackbar (transient toast: tone dot + message + optional action)
    ├── contact-row.slint   # ContactRow (Avatar + 2-line text + trailing meta; recurring list row)
    ├── dot-indicator.slint # DotIndicator (paginated content active-dot strip)
    ├── section-header.slint # SectionHeader (caption-muted-bold list / section label)
    └── pages/              # 122 composite screen templates (see below)
        ├── home.slint           # HomePage (feed + cards + bottom nav + FAB)
        ├── settings.slint       # SettingsPage (sections of toggles and nav rows)
        ├── login.slint          # LoginPage (form + primary CTA)
        ├── podcast.slint        # PodcastPage (now-playing media)
        ├── inbox.slint          # InboxPage (list view + unread badges + compose FAB)
        ├── profile.slint        # ProfilePage (hero avatar + stats + sub-tabs)
        ├── chat.slint           # ChatPage (alternating bubbles + pinned input)
        ├── dashboard.slint      # DashboardPage (stat tiles + activity feed)
        └── ...                  # + 114 more — see "Screen catalogue" below
```

## Screen catalogue

The `ui/pages/` directory holds **122 composite screen templates** — full
mobile screens assembled from the component library, each one a starting
point you can copy and adapt. Every page is locked down by a committed
snapshot baseline. Broad groupings:

- **Lists & feeds** — inbox, news feed, comments, group chat list,
  notification centre, order history, reading list, transit departures,
  trending topics, multi-select list, community forum, address book.
- **Media & playback** — podcast, music library, video player, video
  feed, playlist detail, album detail, tv-show detail, live stream,
  media lockscreen, e-reader.
- **Commerce & payments** — checkout, cart, product detail, paywall,
  wallet, payment methods, payment split, gift card, loyalty card,
  invoice, subscriptions, tip jar, donation, restaurant menu.
- **Maps & travel** — map, turn-by-turn nav, ride-share booking,
  driver-on-the-way, carpool search, flight search, hotel booking,
  trip itinerary, boarding pass, world clock, timezone converter.
- **Health & tracking** — activity rings, workout session, meal log,
  sleep tracking, medication, meditation, habit tracker, timer.
- **Forms & input** — login, signup, form wizard, profile edit,
  write review, bug report, insurance claim, message composer,
  post creator, journal entry, two-factor auth, voting ballot.
- **Settings & system** — settings, account settings, appearance
  settings, wifi settings, app permissions, storage manager, app lock,
  app error, onboarding, onboarding hint, welcome splash, help centre.
- **Detail & dashboards** — dashboard, profile, post detail, event
  detail, job listing, investment detail, crypto portfolio, expense
  report, review summary, poll results, leaderboard, weather,
  pet adoption, app store listing, code review.
- **Smart home & devices** — smart home, room thermostat,
  smart-tv remote, document scanner, qr scanner, voice recorder.
- **Utilities & games** — calculator, currency converter,
  country selector, calendar, weekly meal plan, grocery list,
  recipe, photo grid, photo viewer, search results, game lobby,
  wordle puzzle, quiz, countdown event, live sports score.

## Design tokens

All visual decisions go through the `Theme` global in
[`ui/theme.slint`](ui/theme.slint). It exposes `in-out` properties so a
consuming app can override the palette at runtime:

```rust
let ui = MainWindow::new()?;
ui.global::<Theme>().set_primary(slint::Color::from_rgb_u8(0xff, 0x6b, 0x35));
```

| Token group | Examples                                                       |
|-------------|----------------------------------------------------------------|
| Color       | `background`, `surface`, `primary`, `on-surface`, `muted`      |
| Spacing     | `spacing-xs` (4px) … `spacing-xl` (32px)                       |
| Typography  | `font-size-caption` … `font-size-headline`                     |
| Touch       | `touch-target` (48px — Material minimum)                       |
| Radii       | `radius-sm`, `radius-md`, `radius-lg`                          |
| Motion      | `motion-fast` (120ms), `motion-base` (200ms)                   |

The defaults form a dark theme aligned with the `slint-mobile` starter's
`#0f1115` background.

## Consuming from a slint-mobile-generated app

Layout assumed:

```
my-workspace/
├── slint-mobile-components/   ← this repo
└── my-app/                    ← cargo-generated from slint-mobile
```

**1. In `my-app/Cargo.toml`**, add the components crate as both a runtime
and build dependency:

```toml
[dependencies]
slint-mobile-components = { path = "../slint-mobile-components" }

[build-dependencies]
slint-mobile-components = { path = "../slint-mobile-components" }
```

**2. In `my-app/app/build.rs`**, point Slint at the components library:

```rust
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(HashMap::from([(
            "mobile-components".into(),
            PathBuf::from(slint_mobile_components::UI_LIBRARY_DIR),
        )]));
    slint_build::compile_with_config("ui/main.slint", config)
        .expect("Slint build failed");
}
```

**3. In `my-app/app/ui/main.slint`**, import each component by path
through the `@mobile-components` alias:

```slint
import { Theme } from "@mobile-components/theme.slint";
import { HomePage } from "@mobile-components/pages/home.slint";

export component MainWindow inherits Window {
    background: Theme.background;
    preferred-width: 412px;
    preferred-height: 892px;

    HomePage {
        width: 100%;
        height: 100%;
    }
}
```

The Slint compiler resolves `@mobile-components/...` against the directory
returned by `slint_mobile_components::UI_LIBRARY_DIR`, so consumers don't
hard-code relative paths in their `.slint` sources.

## On-device E2E — Maestro against the Android demo APK

A third layer covers the actual device: a tiny Android app crate
(`android-demo/`) consumes this library and runs the `Gallery` as a
real APK; [Maestro](https://docs.maestro.dev) YAML flows drive it on
the connected emulator and assert against baseline screenshots.

### Prerequisites

- Android SDK + NDK + JDK 17 (the `justfile` defaults assume the same
  paths the `slint-mobile` devcontainer ships)
- `cargo-apk` (`cargo install cargo-apk`)
- A running emulator or device on `adb devices`
- Maestro: `curl -fsSL "https://get.maestro.mobile.dev" | bash`

### Workflow

```sh
just demo-build       # Build the demo APK (debug, multi-arch)
just demo-run         # Build, install, launch on the connected device
just maestro          # Run all flows under maestro/flows
just maestro-refresh  # Update baselines after an intended UI change
```

### How it works

```
slint-mobile-components/
├── android-demo/                 # cargo-apk crate
│   ├── Cargo.toml                # cdylib + [package.metadata.android]
│   ├── build.rs                  # library_paths → ../ui
│   ├── ui/main.slint             # imports `@mobile-components/gallery.slint`
│   └── src/lib.rs                # android_main → MainWindow::new().run()
└── maestro/
    ├── config.yaml
    └── flows/
        ├── gallery-tabs.yaml     # The flow
        └── baselines/            # Committed baseline PNGs
            ├── home.png
            ├── settings.png
            ├── login.png
            └── toolbox.png
```

Because Slint renders the entire UI to a single SurfaceView and exposes
nothing to Android's `AccessibilityService`, Maestro can't address
individual widgets by id or label. Every interaction is a percentage
coordinate tap, and every visual assertion is `assertScreenshot:
baselines/<name>.png` against a committed full-screen PNG with the
default 95 % match threshold (loose enough to absorb the status-bar
clock changing). Recapture baselines via `just maestro-refresh` after
an intended visual change.

### The three test layers at a glance

| Layer | What it covers | Where it runs | When to add a test |
|---|---|---|---|
| Snapshots (`--features snapshots`) | Pixel-level component appearance | Desktop / CI (software renderer) | A new visual state of a component |
| Behavior tests (`--features behaviors`) | Click / toggle / value-change logic via the accessibility tree | Desktop / CI (no rendering) | A new interaction or callback |
| Maestro flows | End-to-end UI on real Android | Emulator / device | A user-visible workflow that spans multiple screens |

## Behavior tests — accessibility-driven

A second harness uses `i-slint-backend-testing` to find interactive
elements via the same accessibility metadata that drives screen readers
(`accessible-role`, `accessible-label`, `accessible-action-default`)
and to invoke their default actions, without rendering anything.

```sh
cargo test --features behaviors --test behavior
```

Each test composes one or more components into a small Window scene
defined in `tests/behavior_scenes.slint`, exposing the result state via
`out` properties. The Rust test then:

1. Finds the element by accessibility metadata
   (`ElementHandle::find_by_accessible_label(&scene, "Submit")`)
2. Triggers the default action (`invoke_accessible_default_action()`)
3. Asserts on the scene's exposed state (`scene.get_click_count()`)

Layout:

```
tests/
├── behavior_scenes.slint   # Scenes that expose state via out properties
└── behavior.rs             # Tests find elements by a11y, invoke, assert
```

**Why this works.** Every interactive component in `ui/` declares
`accessible-role` (button / switch / list-item / text-input), an
`accessible-label` bound to its visible text, and an
`accessible-action-default` callback that fires the same Rust-facing
callback (`clicked()`, `toggled(v)`, …) the inner `TouchArea` does on
tap. The accessibility tree is the test surface — and the same one
screen readers see.

Adding a behavior test: add a scene to `tests/behavior_scenes.slint`,
add a `#[test]` to `tests/behavior.rs` that calls
`ElementHandle::find_by_accessible_label(&scene, "…")`, invoke the
action, and assert.

Each test thread calls `i_slint_backend_testing::init_no_event_loop()`
exactly once (the backend is per-thread); the harness handles that.

## Visual regression — component snapshots

A small harness renders each component scene to a PNG via Slint's
software renderer (no display, no emulator) and diffs against a
committed baseline. Run as part of `cargo test`:

```sh
# Verify nothing has changed (CI mode):
cargo test --features snapshots --test snapshots

# Refresh baselines after an intended visual change:
SLINT_CREATE_SCREENSHOTS=1 cargo test --features snapshots --test snapshots
```

Layout:

```
tests/
├── snapshot_scenes.slint   # Window-rooted scenes, one per component+state
├── snapshots.rs            # Runner (renders, diffs, writes actuals on fail)
└── snapshot_baselines/
    ├── mobile-button-primary.png
    ├── card-with-subtitle.png
    └── ...                 # committed; this is the golden set
```

**Adding a scene.** Define a Window component in
`tests/snapshot_scenes.slint` with explicit `preferred-width` and
`preferred-height`, then add a line to `render_snapshots` in
`tests/snapshots.rs`:

```rust
snapshot("my-new-scene", 320, 120, SnapMyNewScene::new);
```

Run with `SLINT_CREATE_SCREENSHOTS=1` once to write the baseline, commit
the PNG, and the diff job will keep it locked in from then on.

**Failure output.** On mismatch the actual render is saved next to the
baseline as `<name>.actual.png` so the visual diff can be inspected. The
threshold is 0.5 % of pixels — tight enough to catch real changes,
loose enough to absorb minor rasterizer drift between machines.

**Why software-rendered.** The on-device Android build uses Skia; this
test harness uses Slint's pure-Rust software renderer. The harness's
job is to detect changes to **your** code, not to mirror device
output — the baseline and the actual are both produced by the same
renderer, so any deterministic visual change shows up. For on-device
visual checks see the layered testing notes in `CLAUDE.md`.

## Local development

```sh
just         # list recipes
just fmt     # cargo fmt --all
just clippy  # cargo clippy --all-targets -- -D warnings
just check   # cargo check (also compiles every .slint via build.rs)
just test    # cargo test
just ci      # fmt-check + clippy + check + test (mirrors GH Actions)
```

This crate enables **no Slint backend feature** — it's pure UI definitions
plus build-time validation. The consuming app picks the backend
(`backend-android-activity-06` + `renderer-skia` on Android,
`backend-default` on desktop).

## Adding a new component

1. Drop a new file under `ui/`, e.g. `ui/chip.slint`.
2. Import `Theme` from `theme.slint` for any tokens you need.
3. Re-export from `ui/lib.slint` so consumers see it on the
   `@mobile-components/lib.slint` import.
4. `just check` to validate it parses and the Rust bindings build.

Aim for ≥48dp touch targets, generous spacing, and dark-theme contrast.
