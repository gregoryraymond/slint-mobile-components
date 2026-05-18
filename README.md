<a name="top"></a>

# slint-mobile-components

[![Live demo](https://img.shields.io/badge/demo-live-1d76db?style=flat-square)](https://gregoryraymond.github.io/slint-mobile-components/)
[![License](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue?style=flat-square)](#-license)
[![Slint](https://img.shields.io/badge/Slint-1.x-2379f4?style=flat-square)](https://slint.dev)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange?style=flat-square)](https://www.rust-lang.org)
[![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20Android%20%7C%20Wasm-555?style=flat-square)](#)
[![Pages](https://github.com/gregoryraymond/slint-mobile-components/actions/workflows/pages.yml/badge.svg)](https://github.com/gregoryraymond/slint-mobile-components/actions/workflows/pages.yml)
[![CI](https://github.com/gregoryraymond/slint-mobile-components/actions/workflows/ci.yml/badge.svg)](https://github.com/gregoryraymond/slint-mobile-components/actions/workflows/ci.yml)

A UI screen library and design system for **mobile (Android) apps**
built in pure Rust with [Slint](https://slint.dev). Sister project to
the [`slint-mobile`](https://github.com/gregoryraymond/slint-mobile)
`cargo-generate` template — `slint-mobile` scaffolds the app, this
crate supplies the visual language.

> 📱 **[Browse the live catalogue →](https://gregoryraymond.github.io/slint-mobile-components/)**
> 145 page templates rendered side-by-side in the browser — every
> screen the library ships, scrollable on a grid that reflows with
> the viewport width. Wasm-compiled, runs in any modern browser.

The primary surface is **~145 page templates** and **~31 reusable
widgets**, all shipped as `.slint` source. Consumers import individual
screens into their own apps via Slint's `library_paths`; the Rust side
is thin — it exists to hand consumers the full alias map in one call
and to validate every screen compiles cleanly in CI.

## Table of contents

- [📱 About](#-about)
- [✨ What's inside](#-whats-inside)
- [🎨 Design system](#-design-system)
- [🔤 Bundled typefaces](#-bundled-typefaces)
- [🚀 Quick start](#-quick-start)
- [🧱 How it's built](#-how-its-built)
- [📂 Layout](#-layout)
- [🧪 Testing](#-testing)
- [🤖 Android demo APK](#-android-demo-apk)
- [🛠️ Local development](#%EF%B8%8F-local-development)
- [➕ Adding a new component or page](#-adding-a-new-component-or-page)
- [🤝 Contributing](#-contributing)
- [☕ Support the project](#-support-the-project)
- [📃 License](#-license)

## 📱 About

This crate is a complete mobile design system, not a widget toolkit.
The 31 components in `crates/components/` cover everything you need to
assemble a mobile screen (buttons, cards, app bars, lists, forms,
chips, sheets), the 145 page templates in `crates/pages-*/` show those
components composed into realistic screens (login flows, dashboards,
media players, checkouts, maps, calendars), and the `Theme` global in
`crates/theme/` is the single rebinding point for every colour,
spacing, radius, font, and motion token.

Each page is a `.slint` file you can copy into your own app and adapt.
The viewer at `crates/viewer/` lets you scroll through every screen at
native phone resolution (412 × 892) so you can audition the catalogue
before picking what to use. The same viewer compiles to wasm and runs
the entire catalogue in the browser — that's what the live demo above
serves.

## ✨ What's inside

**~145 composite screen templates** across 10 themed categories —
auth, commerce, finance, health, media, misc, productivity, social,
system, travel. Each one is a standalone `Window` sized for a 412 × 892
phone viewport, assembled from the component library, locked down by a
committed PNG snapshot baseline.

Broad groupings:

- **Lists & feeds** — inbox, news feed, comments, group chat list,
  notification centre, order history, reading list, transit
  departures, trending topics, multi-select list, community forum,
  address book.
- **Media & playback** — podcast, music library, video player, video
  feed, playlist detail, album detail, tv-show detail, live stream,
  media lockscreen, e-reader, equalizer, voicemail, voice recorder.
- **Commerce & payments** — checkout, cart, product detail, paywall,
  wallet, payment methods, payment split, gift card, loyalty card,
  invoice, subscriptions, tip jar, donation, restaurant menu.
- **Maps & travel** — map, turn-by-turn nav, ride-share booking,
  driver-on-the-way, carpool search, flight search, hotel booking,
  trip itinerary, boarding pass, world clock, timezone converter,
  store locator, parking session, transit departures.
- **Health & tracking** — activity rings, workout session, meal log,
  sleep tracking, medication, meditation, habit tracker, timer,
  nutrition label, lab results, doctor appointment, insurance claim,
  pet adoption, weekly meal plan.
- **Forms & input** — login, signup, form wizard, profile edit,
  write review, bug report, insurance claim, message composer,
  post creator, journal entry, two-factor auth, voting ballot.
- **Settings & system** — settings, account settings, appearance
  settings, wifi settings, app permissions, storage manager, app
  lock, app error, onboarding, onboarding hint, welcome splash,
  help centre, security checkup, notification centre.
- **Detail & dashboards** — dashboard, profile, post detail, event
  detail, job listing, investment detail, crypto portfolio, expense
  report, review summary, poll results, leaderboard, weather, pet
  adoption, app store listing, code review.
- **Smart home & devices** — smart home, room thermostat, smart-tv
  remote, document scanner, qr scanner, voice recorder.
- **Utilities & games** — calculator, currency converter, country
  selector, calendar, weekly meal plan, grocery list, recipe, photo
  grid, photo viewer, search results, game lobby, wordle puzzle,
  quiz, countdown event, live sports score, achievements.

**~31 reusable widgets** — buttons (`MobileButton`, `TextButton`,
`Fab`, `IconButton`), surfaces (`Card`, `Banner`), navigation
(`AppBar`, `BottomNav`, `TabBar` / `Tab`), lists (`ListItem`,
`Divider`), form controls (`TextField`, `MobileSwitch`, `Checkbox`,
`Radio`, `Slider`, `SegmentedControl`, `Stepper`), decorative
(`Chip`, `Avatar`, `Badge`, `ProgressBar`, `Spinner`, `Skeleton`),
and the `Heading` component plus its `HeadingStyle` preset enum.

## 🎨 Design system

All visual decisions go through globals in
[`crates/theme/ui/`](crates/theme/ui/). Two layers:

**`Theme`** (`theme.slint`) — the base dark / light token set. Every
widget and page reads colours, spacings, font sizes, radii, and
motion durations from it. Consumers override at runtime:

```rust
let ui = MainWindow::new()?;
ui.global::<Theme>().set_primary(slint::Color::from_rgb_u8(0xff, 0x6b, 0x35));
ui.global::<Theme>().set_color_scheme(ColorScheme::Light);
```

| Token group | Examples                                                       |
|-------------|----------------------------------------------------------------|
| Colour      | `background`, `surface-1/2/3`, `primary`, `on-surface`, `muted`, `accent-success/warning/danger/info` |
| Spacing     | `spacing-xs` (4px) … `spacing-xl` (32px)                       |
| Typography  | `font-family`, `font-size-caption` … `font-size-headline`      |
| Touch       | `touch-target` (48px — Material minimum)                       |
| Radii       | `radius-sm`, `radius-md`, `radius-lg`                          |
| Motion      | `motion-fast` (120ms), `motion-base` (200ms)                   |
| Elevation   | `elevation-1/2/3-blur`, `-y`, `-color`                         |

**Skins** (`skins.slint`) — opinionated whole-page look bundles, each
one composing a typeface + a non-flat background + a translucent
surface palette. Pages opt into a skin by importing the global they
want; the existing skinned pages serve as starting points to copy:

| Skin               | Page using it    | Look                              | Background                                |
|--------------------|------------------|-----------------------------------|-------------------------------------------|
| `LiquidGlassSkin`  | `login`          | warm bokeh + serif                | `backgrounds/warm-bokeh.svg`              |
| `AuroraSkin`       | `meditation`     | deep night sky + Plex serif       | `backgrounds/aurora.svg`                  |
| `DawnSkin`         | `weather`        | dawn sky gradient + Space Grotesk | `backgrounds/dawn-sky.svg`                |
| `NebulaSkin`       | `music-library`  | deep-space radial + JetBrains Mono| `backgrounds/nebula.svg`                  |
| `LatticeSkin`      | `home`           | dot-grid + Inter                  | `backgrounds/dot-grid.svg`                |

Each skin exposes the same property surface — `font-family`,
`page-background`, `page-background-image`, `has-background-image`,
`text-primary/secondary/muted`, `bar-background`, `glass-fill`,
`hairline` — so dropping a different skin into a page is a one-import
change.

**`HeadingStyle`** (`components/ui/heading.slint`) — typography
presets for heading-shaped text. Every one of the 145 pages picks a
bespoke recipe from this enum (or sets typography on `Text` directly)
so the catalogue feels like a typeface portfolio when scrolled:

```slint
import { Heading, HeadingStyle } from "@mobile-components/heading.slint";

Heading {
    text: "Sign in";
    style: HeadingStyle.serif-display;       // Fraunces 28px / 700
}

AppBar {
    title: "STORAGE";
    title-style: HeadingStyle.uppercase-spaced;  // Inter 13px / 700 / 3px tracking
}
```

Available presets: `plain` (AppBar default), `serif-display`,
`serif-light`, `uppercase-spaced`, `mono`, `bold-impact`,
`plex-medium`, `grotesk-bold`, `lowercase-soft`, `caps-tight`.

## 🔤 Bundled typefaces

The library ships five free, widely-deployed mobile-UI typefaces in
[`crates/components/ui/fonts/`](crates/components/ui/fonts/). Each is
imported into the build via `import "..ttf"` in `skins.slint`, so any
consumer that depends on the library gets them embedded automatically
— no system-font fallback, identical rendering on Android, desktop
preview, and the wasm catalogue.

| Family            | Role                       | Licence  |
|-------------------|----------------------------|----------|
| **Inter**         | Neutral modern UI sans     | OFL-1.1  |
| **IBM Plex Sans** | Humanist sans, slabby ends | OFL-1.1  |
| **Space Grotesk** | Geometric grotesque        | OFL-1.1  |
| **Fraunces 9pt**  | Display serif (ball terms) | OFL-1.1  |
| **JetBrains Mono**| Monospace                  | OFL-1.1  |

Per-family licence text lives next to the TTFs at
`crates/components/ui/fonts/<family>/LICENSE.txt`.

## 🚀 Quick start

### Browse the catalogue locally

```sh
cargo run     # or `cargo view` for the release build
```

Opens an infinite-scroll viewer that tiles every page template at
native phone resolution (412 × 892), in a grid that reflows to fit
the window. The viewer is interpreter-backed — page templates are
parsed at runtime by `slint-interpreter`, so the binary stays cheap
to build and adding new pages costs nothing.

> Cold builds pull `slint-interpreter` for the first time (~1 minute,
> high memory). Subsequent builds are incremental. On Linux, the
> renderer also needs `pkg-config`, `fontconfig`, `freetype`, `clang`,
> `cmake`, `ninja` — see `just install-host-deps`.

### Consume from a slint-mobile-generated app

Layout assumed:

```
my-workspace/
├── slint-mobile-components/   ← this repo
└── my-app/                    ← cargo-generated from slint-mobile
```

In `my-app/Cargo.toml`:

```toml
[dependencies]
slint-mobile-components = { path = "../slint-mobile-components" }

[build-dependencies]
slint-mobile-components = { path = "../slint-mobile-components" }
```

In `my-app/app/build.rs`:

```rust
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(slint_mobile_components::library_paths());
    slint_build::compile_with_config("ui/main.slint", config)
        .expect("Slint build failed");
}
```

In `my-app/app/ui/main.slint`:

```slint
import { Theme } from "@mobile-theme/theme.slint";
import { MobileButton } from "@mobile-components/button.slint";
import { HomePage } from "@mobile-pages-misc/home.slint";

export component MainWindow inherits HomePage {}
```

Or — if you'd rather lazy-load pages at runtime — copy the pattern
from `crates/viewer/`: it parses `.slint` sources via
`slint-interpreter` and embeds them via `ComponentContainer`, so your
APK doesn't statically link every screen it might one day show.

## 🧱 How it's built

The workspace is a Cargo workspace of small crates, each with a tight
scope:

- **`crates/theme/`** — design tokens (`Theme`, `ColorScheme`,
  `Skin*` globals). Pure declarative — no Rust logic.
- **`crates/components/`** — 31 widgets + bundled fonts + shared
  icons + background SVGs. Each widget is one `.slint` file with a
  spec docstring at the top.
- **`crates/pages-{category}/`** — 145 page templates split across 10
  themed crates so the workspace compiles in parallel. Per-crate
  snapshot scene aggregators sit in `_snapshot_scenes.slint`.
- **`crates/viewer/`** — interpreter-backed infinite-scroll browser
  for every page, with verdict-toggle buttons that write to
  `showcase-verdicts.json` for offline review passes.
- **`xtask/`** — small Rust binary for repo housekeeping
  (`cargo xtask split-snapshots`, `cargo xtask wire-pages`).
- **`android-demo/`** — minimal `cargo-apk` crate that proves the
  workspace compiles end-to-end for the device.
- **`wasm-viewer/`** — browser build of the catalogue, deployed to
  GitHub Pages on every push to `main`.

Per-page library paths (`@mobile-theme`, `@mobile-components`,
`@mobile-pages-<cat>`) are wired centrally via the
`slint_mobile_components::library_paths()` helper, so consumers don't
have to know about the workspace's internal crate split.

## 📂 Layout

```
slint-mobile-components/
├── Cargo.toml                  # workspace root
├── rust-toolchain.toml
├── justfile                    # fmt / clippy / check / test / ci / demo / maestro
├── .github/workflows/
│   ├── ci.yml                  # fmt + clippy + check + test on every PR
│   └── pages.yml               # build wasm viewer + deploy to gh-pages on main
├── .cargo/config.toml          # `cargo view`, `cargo xtask` aliases
├── src/lib.rs                  # library_paths() + test-scene re-exports
├── crates/
│   ├── theme/
│   │   └── ui/{theme,skins}.slint
│   ├── components/
│   │   └── ui/
│   │       ├── *.slint         # 31 widgets, one per file
│   │       ├── heading.slint   # HeadingStyle enum + Heading component
│   │       ├── icons/*.svg     # solid-fill SVGs, tinted via Image.colorize
│   │       ├── backgrounds/    # SVG page backgrounds for the 5 skins
│   │       └── fonts/          # bundled TTFs + LICENSE per family
│   ├── pages-{auth,commerce,finance,health,media,misc,
│   │          productivity,social,system,travel}/
│   │   └── ui/
│   │       ├── *.slint         # ~145 page templates total
│   │       └── _snapshot_scenes.slint   # category snap aggregator
│   ├── viewer/                 # `cargo run` — desktop catalogue browser
│   └── wasm-viewer/            # browser build of the same catalogue
├── xtask/                      # cargo xtask housekeeping
├── android-demo/               # minimal APK proving Android target builds
└── tests/
    ├── snapshot_scenes.slint   # widget-level snap scenes
    ├── snapshots.rs            # visual regression (--features snapshots)
    ├── snapshot_baselines/     # committed golden PNGs (one per snap)
    ├── behavior_scenes.slint
    └── behavior.rs             # accessibility-driven behaviour tests
```

## 🧪 Testing

Three layers, each catching a different class of breakage:

| Layer                   | What it catches                                  | Run                                                                |
|-------------------------|--------------------------------------------------|--------------------------------------------------------------------|
| **Snapshots**           | Pixel-level component + page appearance          | `cargo test --features snapshots --test snapshots`                 |
| **Behaviour tests**     | Click / toggle / value-change via a11y tree      | `cargo test --features behaviors --test behavior`                  |
| **Maestro flows**       | End-to-end UI on a real Android emulator/device  | `just maestro` (refresh baselines: `just maestro-refresh`)         |

Refresh snapshot baselines after an intended visual change:

```sh
SLINT_CREATE_SCREENSHOTS=1 cargo test --features snapshots --test snapshots
```

The harness uses Slint's pure-Rust software renderer (no display, no
emulator), diffs at 0.5 % pixel-drift tolerance, and writes
`*.actual.png` next to the baseline on mismatch so the diff is
inspectable.

Behaviour tests find interactive elements through the same
accessibility tree that drives screen readers (`accessible-role` /
`accessible-label`) and invoke their default actions without
rendering anything — so adding a behaviour test means adding a small
scene to `tests/behavior_scenes.slint` and a `#[test]` to
`tests/behavior.rs`.

## 🤖 Android demo APK

`android-demo/` is a minimal `cargo-apk` crate that proves the
workspace compiles end-to-end for the device. It instantiates
`HomePage` as its `MainWindow` and exists so a `cargo apk build` from
that directory verifies the screen library is buildable for Android.

```sh
just demo-build       # cargo apk build (debug, multi-arch)
just demo-run         # build + install + launch on the connected device
```

Prerequisites: Android SDK + NDK + JDK 17 (the `justfile` assumes the
paths the `slint-mobile` devcontainer ships); `cargo install
cargo-apk`; a running emulator or device on `adb devices`.

## 🛠️ Local development

```sh
just         # list recipes
just fmt     # cargo fmt --all
just clippy  # cargo clippy --all-targets -- -D warnings
just check   # cargo check (also compiles every .slint via build.rs)
just test    # cargo test
just ci      # fmt-check + clippy + check + test (mirrors GH Actions)
```

This crate enables **no Slint backend feature** — it's pure UI
definitions plus build-time validation. The consuming app picks the
backend (`backend-android-activity-06` + `renderer-skia` on Android,
`backend-default` on desktop, `backend-winit + renderer-femtovg` on
wasm).

## ➕ Adding a new component or page

**A new widget** — drop a file under
`crates/components/ui/<name>.slint`, import `Theme` for tokens, add
a section to `crates/viewer/ui/viewer.slint`'s Toolbox, and add a
snapshot scene to `tests/snapshot_scenes.slint`. The
`slint-mobile-components` skill manual (in
`.claude/skills/slint-mobile-components/SKILL.md`) walks through every
step.

**A new page** — drop the file under
`crates/pages-<category>/ui/<name>.slint`, then run
`cargo xtask split-snapshots` to wire it into the matching
`_snapshot_scenes.slint` aggregator. No new workspace crate needed
unless the page genuinely doesn't fit an existing category (use
`pages-misc/` as the catch-all).

Conventions: ≥48 dp touch targets, generous spacing, dark-theme
contrast by default, no hardcoded colours / paddings / radii / motion
durations — read everything from `Theme`.

## 🤝 Contributing

Issues and PRs welcome. The next thing built is usually the next thing
someone actually needs — so if there's a screen pattern missing from
the catalogue, a widget you wish existed, or a token the theme should
expose, open an issue and say so.

If you're poking at the internals, `tests/` is the honest
documentation: every page has a snapshot, every interactive widget has
a behaviour test, and the snapshot baselines under
`tests/snapshot_baselines/` form a visual table of contents for the
whole catalogue.

## ☕ Support the project

If slint-mobile-components has saved you a weekend of building a
mobile design system from scratch, or made an app feel polished
without you having to draw every component yourself — a coffee keeps
weekend hacking time available for it.

[![Buy me a coffee](https://img.shields.io/badge/buy_me_a_coffee-FFDD00?logo=buy-me-a-coffee&logoColor=000&style=for-the-badge)](https://buymeacoffee.com/gregoryraymond)
[![GitHub Sponsors](https://img.shields.io/badge/GitHub-sponsor-EA4AAA?logo=github-sponsors&logoColor=white&style=for-the-badge)](https://github.com/sponsors/gregoryraymond)

One-offs are great. If you're a company shipping this in a product, a
small recurring sponsorship via GitHub Sponsors is more useful — it
gives a rough sense of how many real users depend on it, which affects
how much I'm willing to break in a refactor.

## 📃 License

Dual-licensed under either [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.

Bundled fonts each retain their original licence (all OFL-1.1) —
see `crates/components/ui/fonts/<family>/LICENSE.txt`.

---

<sub>[↑ Back to top](#top)</sub>
