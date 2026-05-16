# slint-mobile-components

A [Slint](https://slint.dev) UI screen library and design system for
**mobile (Android) apps** built in Rust. Sister project to the
[`slint-mobile`](../slint-mobile) `cargo-generate` template — slint-mobile
generates the app skeleton; this crate supplies the visual language.

The primary surface is **~145 page templates** plus ~31 reusable widgets,
all shipped as `.slint` source. Consumers import individual screens into
their own apps via Slint's `library_paths`; this crate's Rust side is
thin and exists to (a) hand consumers the full alias map in one call and
(b) validate every screen compiles cleanly in CI.

## Browse the screens

```sh
cargo run     # or `cargo view` for the release build
```

Opens an infinite-scroll viewer that tiles every page template at native
phone resolution (412 × 892). The viewer is interpreter-backed — page
templates are parsed at runtime by `slint-interpreter`, so the binary
itself stays cheap to build (~10 s clean, sub-second incremental) and
adding new pages costs nothing.

> Cold builds pull `slint-interpreter` for the first time (~1 minute,
> high memory). Subsequent builds are incremental. On Linux, the
> renderer also needs `pkg-config`, `fontconfig`, `freetype`, `clang`,
> `cmake`, `ninja` — see `just install-host-deps`.

## What's inside

```
slint-mobile-components/
├── Cargo.toml                # workspace root + thin re-export crate
├── build.rs                  # tests/snapshot_scenes + behavior_scenes
├── rust-toolchain.toml
├── justfile                  # fmt / clippy / check / test / ci
├── .github/workflows/ci.yml
├── .cargo/config.toml        # `cargo view`, `cargo xtask` aliases
├── src/lib.rs                # library_paths() helper + test-scene re-exports
├── crates/
│   ├── theme/                # design tokens (`@mobile-theme/theme.slint`)
│   ├── components/           # 31 widgets + icons (`@mobile-components/…`)
│   ├── pages-{auth,commerce,finance,health,media,misc,
│   │          productivity,social,system,travel}/
│   │                         # ~145 page templates (`@mobile-pages-<cat>/…`)
│   │                         # each crate compiles its own snapshot scenes
│   └── viewer/               # `cargo run` — interpreter-backed UI browser
├── xtask/                    # `cargo xtask wire-pages|split-snapshots`
├── android-demo/             # minimal APK proving Android target builds
├── tests/
│   ├── snapshot_scenes.slint # widget-level snap scenes only
│   ├── snapshots.rs          # visual regression (--features snapshots)
│   ├── snapshot_baselines/   # committed golden PNGs
│   ├── behavior_scenes.slint
│   └── behavior.rs           # behavior tests (--features behaviors)
```

See the per-crate `ui/` directories for the actual `.slint` sources —
e.g. `crates/components/ui/button.slint`,
`crates/pages-media/ui/podcast.slint`,
`crates/theme/ui/theme.slint`.

## Screen catalogue

The 10 `crates/pages-<cat>/` crates hold **~145 composite screen
templates** — full mobile screens assembled from the component library,
each one a starting point you can copy and adapt. Most pages are locked
down by a committed snapshot baseline. Broad groupings:

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
[`crates/theme/ui/theme.slint`](crates/theme/ui/theme.slint). It
exposes `in-out` properties so a consuming app can override the
palette at runtime:

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

**2. In `my-app/app/build.rs`**, point Slint at every alias this
workspace publishes (`@mobile-theme`, `@mobile-components`, every
`@mobile-pages-*`):

```rust
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(slint_mobile_components::library_paths());
    slint_build::compile_with_config("ui/main.slint", config)
        .expect("Slint build failed");
}
```

**3. In `my-app/app/ui/main.slint`**, import each screen / widget by
path through the matching alias:

```slint
import { Theme } from "@mobile-theme/theme.slint";
import { MobileButton } from "@mobile-components/button.slint";
import { HomePage } from "@mobile-pages-misc/home.slint";

export component MainWindow inherits HomePage {}
```

Or — if you'd rather lazy-load pages at runtime — copy the pattern
from `crates/viewer/`: it parses `.slint` sources via
`slint-interpreter` and embeds them via `ComponentContainer`, so
your APK doesn't statically link every screen it might one day show.

## Android demo APK

`android-demo/` is a minimal `cargo-apk` crate that proves the workspace
compiles end-to-end for an Android target. It instantiates `HomePage` as
its `MainWindow` — nothing more — and exists so a `cargo apk build` from
that directory verifies the screen library is buildable for the device.

```sh
just demo-build       # cargo-apk build (debug, multi-arch)
just demo-run         # build + install + launch on the connected device
```

Prerequisites: Android SDK + NDK + JDK 17 (the `justfile` assumes the
paths the `slint-mobile` devcontainer ships); `cargo install cargo-apk`;
a running emulator or device on `adb devices`.

### The two test layers at a glance

| Layer | What it covers | When to add a test |
|---|---|---|
| Snapshots (`--features snapshots`) | Pixel-level component + page appearance | A new visual state of a component, or a new page template |
| Behavior tests (`--features behaviors`) | Click / toggle / value-change logic via the accessibility tree | A new interaction or callback |

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
