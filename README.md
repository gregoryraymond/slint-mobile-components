# slint-mobile-components

A [Slint](https://slint.dev) UI component library and design system for
**mobile (Android) apps** built in Rust. Sister project to the
[`slint-mobile`](../slint-mobile) `cargo-generate` template — slint-mobile
generates the app skeleton; this crate supplies the visual language.

The primary surface is the `ui/` directory of `.slint` files, consumed
from a sibling project via Slint's `library_paths`. A thin Rust crate
wraps it so `cargo check` validates every component in CI.

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
└── ui/
    ├── _check.slint        # CI validation entry — not consumed externally
    ├── theme.slint         # Theme global (colors, spacing, type, motion)
    ├── button.slint        # MobileButton, TextButton, Fab
    ├── card.slint          # Card
    ├── app-bar.slint       # AppBar (top)
    ├── bottom-nav.slint    # BottomNav, BottomNavItem
    ├── list-item.slint     # ListItem
    ├── text-field.slint    # TextField
    ├── switch.slint        # MobileSwitch
    └── pages/
        ├── home.slint      # HomePage
        ├── settings.slint  # SettingsPage
        └── login.slint     # LoginPage
```

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
