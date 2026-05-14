---
name: slint-mobile-components
description: Working guide for the slint-mobile-components repo — a Slint UI component library and design system for mobile (Android) apps in pure Rust. Apply when adding, modifying, testing, theming, or screenshot-snapshotting components in this repo, when wiring the library into a slint-mobile-generated app, or when running the on-device Maestro flows against the demo APK. Encodes the architecture, the 25-component catalogue, the three-layer test pyramid, the add-a-component recipe, and the Slint syntax gotchas caught during initial development.
user-invocable: true
allowed-tools: Read, Edit, Write, Grep, Glob, Bash
---

# slint-mobile-components

A Slint UI component library + opinionated mobile design system for
Rust apps targeting Android. Sister project to the `slint-mobile`
cargo-generate template — that one scaffolds the Android app shell;
this one supplies the visual language.

## What this repo is — at a glance

```
slint-mobile-components/                  ← Cargo workspace + library crate
├── Cargo.toml                            # [workspace] + [package]
├── build.rs                              # compiles 3 .slint files (gallery + 2 test files)
├── src/lib.rs                            # explicit include!()s + re-exports
├── ui/                                   # The actual components (consumed via library_paths)
│   ├── theme.slint                       # Theme global — colors, spacing, type, motion
│   ├── *.slint                           # 25 components, one per file
│   ├── gallery.slint                     # Window-rooted preview Window + CI validation entry
│   ├── icons/*.svg                       # Solid-fill SVGs, tinted via Image.colorize
│   └── pages/                            # Example pages (HomePage / SettingsPage / LoginPage)
├── android-demo/                         # cargo-apk crate that runs Gallery on Android
├── examples/gallery.rs                   # Desktop preview (--features gallery)
├── tests/
│   ├── snapshot_scenes.slint + snapshots.rs        # --features snapshots
│   ├── behavior_scenes.slint + behavior.rs         # --features behaviors
│   └── snapshot_baselines/*.png          # Committed visual baselines
└── maestro/flows/                        # On-device E2E (gallery-tabs.yaml + baselines/)
```

**Consumption pattern** (in a slint-mobile-generated app):

```rust
// build.rs:
slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([(
    "mobile-components".into(),
    PathBuf::from(slint_mobile_components::UI_LIBRARY_DIR),
)]))
```

```slint
// any .slint file:
import { Theme } from "@mobile-components/theme.slint";
import { MobileButton } from "@mobile-components/button.slint";
```

Per-file imports — there is NO `lib.slint` re-export hub. `lib.slint`
was removed early; `gallery.slint` is the build's compilation entry
purely so non-Window component exports don't trigger slint-build's
"no code generated" advisory.

## Component catalogue (25)

**Buttons / actions** — `MobileButton`, `TextButton`, `Fab`, `IconButton`
**Surfaces** — `Card`, `Banner`
**Navigation** — `AppBar`, `BottomNav` + `BottomNavDistribution` enum, `TabBar` + `Tab`
**Lists / rows** — `ListItem`, `Divider`
**Form controls** — `TextField`, `MobileSwitch`, `Checkbox`, `Radio`, `Slider`, `SegmentedControl`, `Stepper`
**Decorative** — `Chip`, `Avatar`, `Badge`, `ProgressBar`, `Spinner`, `Skeleton`
**Page patterns** — `EmptyState`, `HomePage`, `SettingsPage`, `LoginPage`

Every interactive component carries accessibility metadata
(`accessible-role`, `accessible-label`, `accessible-action-default`)
so screen readers AND the behavior-test harness can drive it.

## Theme (design tokens) — single source of truth

`ui/theme.slint` exposes one `global Theme` with `in-out` properties so
consuming apps can rebind at runtime. Token groups:

- **Colour**: `background`, `surface`, `surface-variant`, `surface-pressed`, `primary`, `primary-pressed`, `on-primary`, `on-background`, `on-surface`, `muted`, `outline`, `danger`, `success`
- **Spacing**: `spacing-xs` (4 px) → `spacing-xl` (32 px)
- **Typography**: `font-size-caption` (12 px) → `font-size-headline` (28 px)
- **Touch**: `touch-target` (48 px — Material minimum, never hardcode this)
- **Radii**: `radius-sm` (6 px), `radius-md` (12 px), `radius-lg` (20 px)
- **Motion**: `motion-fast` (120 ms), `motion-base` (200 ms)

**Rule**: components MUST NOT hardcode colours, paddings, font sizes, or
radii — read from `Theme`. Adding a new visual constant means adding a
new `Theme` token first.

## Three-layer test pyramid

| Layer | What it covers | Where it runs | Run |
|---|---|---|---|
| **Snapshots** | Pixel-level component appearance | Desktop, software renderer, no display needed | `cargo test --features snapshots --test snapshots` — refresh with `SLINT_CREATE_SCREENSHOTS=1` |
| **Behaviors** | Click / toggle / value-change via the accessibility tree | Desktop, no rendering | `cargo test --features behaviors --test behavior` |
| **Maestro flows** | End-to-end UI on real Android | Emulator / device | `just maestro` (refresh baselines: `just maestro-refresh`) |

Use `just demo-build` / `just demo-run` to push the demo APK to the
emulator. The justfile sets `ANDROID_HOME`, `ANDROID_NDK_ROOT`,
`JAVA_HOME`, and prepends `~/.maestro/bin` to PATH.

## Recipe: add a new component

Walking example — adding a hypothetical `Foo`:

1. **Create `ui/foo.slint`** with a spec docstring at the top:

   ```slint
   // Foo
   // =====================================================================
   //
   // <One-paragraph description of when to use this component.>
   //
   // Specification
   // -------------
   // Geometry: ...
   // Content: <prop list>
   // State: <bool / int / enum properties>
   // Interaction: <callbacks fired, tap zones>
   // Accessibility: <accessible-role + label + default-action>
   // Composition: <where this fits in a page>

   import { Theme } from "theme.slint";

   export component Foo inherits Rectangle {
       in property <string> label;
       in property <bool> enabled: true;
       callback clicked();

       accessible-role: button;       // for interactive
       accessible-label: root.label;
       accessible-enabled: root.enabled;
       accessible-action-default => { root.clicked(); }

       // Read Theme.* for all dimensions, colours, radii, durations.
       min-height: Theme.touch-target;
       border-radius: Theme.radius-md;
       background: ...;
       animate background { duration: Theme.motion-fast; }
       // ...
   }
   ```

2. **Add a section to `ui/gallery.slint`** Toolbox — import at the top
   of the file, then add a `VerticalLayout { SectionLabel { text: "FOO"; } Foo { ... } }`
   alongside the other sections.

3. **Snapshot scene** in `tests/snapshot_scenes.slint`:

   ```slint
   import { Foo } from "../ui/foo.slint";

   export component SnapFoo inherits Window {
       preferred-width: 320px;
       preferred-height: 64px;
       background: Theme.background;
       Foo { x: Theme.spacing-md; y: Theme.spacing-md; ... label: "Demo"; }
   }
   ```

   Re-export in `src/lib.rs` under `_generated_snapshot_scenes` and
   register in `tests/snapshots.rs::render_snapshots`:
   ```rust
   snapshot("foo", 320, 64, SnapFoo::new);
   ```

   Generate the baseline: `SLINT_CREATE_SCREENSHOTS=1 cargo test --features snapshots --test snapshots`.
   Commit the new PNG under `tests/snapshot_baselines/`.

4. **Behavior test** (only if interactive) in
   `tests/behavior_scenes.slint`:

   ```slint
   export component BehaviorFoo inherits Window {
       out property <int> click-count: 0;
       Foo { label: "Tap me"; clicked => { root.click-count += 1; } }
   }
   ```

   Then in `tests/behavior.rs`:
   ```rust
   #[test]
   fn foo_default_action_fires_clicked() {
       ensure_backend();
       let scene = BehaviorFoo::new().unwrap();
       let foo = ElementHandle::find_by_accessible_label(&scene, "Tap me")
           .next().unwrap();
       foo.invoke_accessible_default_action();
       assert_eq!(scene.get_click_count(), 1);
   }
   ```

5. **Run the full check**: `cargo check`, `cargo clippy --all-targets --features "snapshots behaviors gallery" -- -D warnings`, `cargo fmt --check`.

6. **Update `README.md`** file-tree under `ui/` to mention `foo.slint`.

## Slint syntax gotchas caught during initial development

These are the non-obvious sharp edges. Check this list when something
doesn't compile.

### Geometry / layout
- `Rectangle` has **no** `padding`, **no** `vertical-alignment`, **no** `rotation-angle`. Wrap in a `HorizontalLayout`/`VerticalLayout` for padding; wrap in `VerticalLayout { alignment: center; }` to centre a fixed-size box inside a taller row.
- Rotation properties were **renamed**: `rotation-angle` → `transform-rotation`, `rotation-origin-x/y` → `transform-origin: {x, y}`. `transform-origin` defaults to the element centre — don't set it unless you mean something else.
- Slint `LayoutAlignment` values: `stretch`, `center`, `start`, `end`, `space-between`, `space-around`. **No `space-evenly`.**
- `min-width: 0` on layout children is the trick to make `horizontal-stretch: 1` distribute *equally* regardless of children's natural content widths (e.g. BottomNav with varying label lengths).

### Properties + types
- `image.width` and `image.height` return `int`, not `length`. Use `image.width > 0`, NOT `image.width > 0px`.
- Enum-typed properties need qualified form when used as a default value: `in property <InputType> kind: InputType.text;` (bare `text` fails to parse for property defaults; ternary expressions on enum-typed properties tolerate either form).
- `accessible-label` and `accessible-description` can only be set when `accessible-role` is also set on the SAME element. If you don't want the element to be a known role, those props will fail to compile.
- Slint's `AccessibleRole` enum doesn't currently include `radio-button`. Use `accessible-role: button` + `accessible-checkable: true` + `accessible-checked: root.selected` for radios.

### Timers + animations
- `Timer` **cannot** be wrapped in `if cond:` blocks. Always render the Timer, use `running: cond` to control it.
- `animate <property> { duration: ... }` interpolates when the property *changes*. For continuous animation (e.g. spinner rotation) use a Timer to step the property.

### Multi-file builds + bindings
- `slint::include_modules!()` includes **only ONE file** — the last one passed to `slint_build::compile()`. With multiple `compile()` calls (gallery + snapshot_scenes + behavior_scenes), the env var `SLINT_INCLUDE_GENERATED` is overwritten and only the last file's types are exposed. Workaround in `src/lib.rs`:

  ```rust
  mod _generated_gallery { include!(concat!(env!("OUT_DIR"), "/gallery.rs")); }
  mod _generated_snapshot_scenes { include!(concat!(env!("OUT_DIR"), "/snapshot_scenes.rs")); }
  mod _generated_behavior_scenes { include!(concat!(env!("OUT_DIR"), "/behavior_scenes.rs")); }
  pub use _generated_gallery::*;
  pub use _generated_snapshot_scenes::{Snap...};  // explicit names to dodge type-name collisions
  pub use _generated_behavior_scenes::{Behavior...};
  ```

  Each generated module re-exports `BottomNavDistribution` etc., so `pub use *` from more than one of them would collide.

- `slint_build::CompilerConfiguration` has **no `with_debug_info`** method. Set the env var **before** the first `compile()` call:
  ```rust
  std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");
  ```
  Required for `ElementHandle::find_by_*` to traverse the element tree at all.

### Testing
- `i_slint_backend_testing::init_no_event_loop()` is **per-thread** (the backend instance is thread-local) and **panics if called twice on the same thread**. Cargo runs tests in parallel threads. Guard with a `thread_local!` `Cell<bool>`, NOT a process-wide `Once`.
- `ElementHandle::find_by_accessible_label` is **exact-match, case-sensitive**, and silently skips invisible items. Build-time debug info (see above) is required.
- For sync behavior tests, prefer `invoke_accessible_default_action()` (sync). `single_click()` / `double_click()` are async and need an event loop.
- Snapshot rendering uses `MinimalSoftwareWindow` + `SoftwareRenderer::render(&mut [Rgb565Pixel], stride)`. Slint doesn't ship an `Rgb8Pixel` target — manually expand Rgb565 → RGB8 with `(r << 3) | (r >> 2)` for the high-bit replication.

### Resources + assets
- `@image-url(path)` resolves the path RELATIVE TO THE .slint FILE containing the macro, not the compiling file. From `ui/pages/home.slint`, `@image-url("../icons/home.svg")` points at `ui/icons/home.svg`.
- All shipped icons are solid-fill SVGs with `fill="#000"`. Slint's `Image.colorize` tints them at render time — DON'T set colour in the SVG, set it in `colorize`. See `CLAUDE.md` for the rule that icons within a single element type must render at visually identical size (achieved by fixed 24×24 `Image { width: 24px; height: 24px; }`).

## Useful Slint syntax patterns (cheat-sheet)

```slint
// Property with default
in property <bool> enabled: true;
in-out property <int> value: 0;
out property <int> click-count: 0;     // read-only outside; writable inside

// Callback
callback clicked();
callback toggled(bool);

// Conditional rendering
if root.icon.width > 0: Image { source: root.icon; ... }

// Loop over a model
for option[i] in root.options: Segment {
    label: option;
    selected: root.selected-index == i;
}

// Two-way binding
checked <=> root.checked;

// Accessibility default action
accessible-role: button;
accessible-action-default => { root.clicked(); }

// Animation on a property change
animate background { duration: Theme.motion-fast; easing: ease-out; }
```

## Where things live (one-line index)

- `README.md` — user-facing guide (consumption recipe, test workflow)
- `CLAUDE.md` — iconography rule (icons within same element type must be visually identical size)
- `ui/theme.slint` — design tokens
- `ui/<component>.slint` — components, each with a spec docstring at top
- `ui/gallery.slint` — desktop preview + CI compile entry
- `examples/gallery.rs` — desktop preview runner (`--features gallery`)
- `tests/snapshot_scenes.slint` + `tests/snapshots.rs` — visual regression
- `tests/behavior_scenes.slint` + `tests/behavior.rs` — accessibility-driven behavior tests
- `tests/snapshot_baselines/` — committed PNGs
- `maestro/flows/gallery-tabs.yaml` — on-device E2E flow
- `maestro/flows/baselines/` — committed Maestro full-screen baselines
- `android-demo/` — cargo-apk crate that runs Gallery on Android
- `justfile` — `demo-build`, `demo-run`, `maestro`, `maestro-refresh`, `fmt`, `clippy`, `test`, `check`, `ci`

## Quick commands

```sh
# Compile everything (validates all .slint files via build.rs)
cargo check --all-targets

# Visual regression
cargo test --features snapshots --test snapshots
SLINT_CREATE_SCREENSHOTS=1 cargo test --features snapshots --test snapshots  # refresh

# Behavior
cargo test --features behaviors --test behavior

# Desktop preview
DISPLAY=:0 cargo run --example gallery --features gallery

# Android demo
just demo-build && just demo-run

# Maestro E2E (emulator must be running and demo APK installed)
just maestro
just maestro-refresh   # to recapture baselines after intended UI change

# Full local CI
cargo clippy --all-targets --features "snapshots behaviors gallery" -- -D warnings
cargo fmt --all -- --check
```

## Design philosophy

- **Mobile-shaped, not Material-spec-shaped.** This library doesn't try to be Material 3 — Slint's official `ui-libraries/material/` is that. This library is an opinionated, smaller, dark-themed mobile design system aligned with the slint-mobile starter's `#0f1115` background.
- **Touch-target floor**: every interactive component honours `Theme.touch-target` (48 dp). FAB is 56 dp (Material convention).
- **Accessibility everywhere**: every interactive component has `accessible-role` + `accessible-label` + `accessible-action-default`. This is what makes behavior tests possible AND it's the real-device a11y story for free.
- **No `lib.slint` re-export hub**: consumers import per-file (`@mobile-components/<file>.slint`). Avoids slint-build's "no code generated" warnings on non-Window exports and matches the more idiomatic Slint convention.
- **Theme is the API surface for theming**: rebind `Theme.*` from a consuming app's startup code to retheme the whole library. Don't fork.
