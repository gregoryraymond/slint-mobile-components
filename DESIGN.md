# Design goals

> The opinionated visual language for `slint-mobile-components`. Locked-in
> via deliberate choice on four axes; the rest of this doc explains what
> each decision means in practice, and what it asks of the implementation.

## References

Two reference systems, deliberately combined; nothing else:

- **Linear** (linear.app) — dense layouts, single warm accent on
  near-monochrome neutral, high-contrast type, restrained motion. Sets
  the *function-first* baseline.
- **iOS Liquid Glass** (iOS 18+ / 2025 redesign direction) — layered
  surfaces, depth through translucency, motion as a structural language.
  Sets the *depth* baseline.

We are **not** targeting Material 3, Tailwind UI, Spotify, Arc, or
Notion — those reference points were considered and explicitly rejected
during direction-setting. When in doubt, ask "does Linear or iOS do this
on mobile?" — that's the test.

## Principles

1. **Function first, ornament second.** Density and clarity beat
   decoration. If a visual element doesn't carry meaning, remove it.
2. **Depth through translucency, not frames.** No borders. Stacked
   surfaces and shadow communicate elevation.
3. **One accent.** A single warm primary carries brand. Everything else
   is monochrome neutral. Status colours are functional, not decorative.
4. **Honest motion.** 120–200 ms tween for state changes, slightly
   slower (240 ms) for overlays appearing. No spring physics, no
   choreographed entrances.
5. **Mobile-safe defaults.** 48 dp touch targets, 16 px form inputs,
   safe-area-aware overlays, opaque content where readability matters.
6. **The theme is the API.** Every visual decision routes through
   `Theme`. No component hardcodes a colour, padding, radius, or
   duration.

## Visual language

### 1. Borders — none

We don't draw frames around things. Surfaces are separated by:

- **Subtle alpha** over the background (Card on background reads as
  "raised" because it's a slightly different luminance, not because of
  a stroke).
- **Shadow** for elevation cues (a Sheet over content has a drop shadow
  that grounds it).
- **Surface tier** (a Dialog on Card uses `surface-2`, a Snackbar uses
  `surface-3` — the luminance difference is the "edge").

The single exception: **focus rings** on form inputs. A focused
`TextField` gets a 2 px primary stroke. This is the only place a
border carries semantic weight; it earns its visibility.

The previous `Theme.outline` token is retired. Components that used it
for dividers switch to surface tiering or shadow.

### 2. Transparency

Two flavours, both applied:

- **Subtle alpha on surfaces.** `Card`, `Drawer`, `Sheet`, and
  scrolled-`AppBar` render at 90–95 % opacity over their parent. The
  faint background bleed-through gives a layered-but-readable feel,
  without sacrificing legibility.
- **Blurred overlays — approximated.** iOS Liquid Glass's defining
  trick is real-time backdrop blur. Slint's renderer does not currently
  support `backdrop-filter: blur`-equivalent live blur on Android (only
  a few platforms expose it at the *window* level via native APIs). We
  approximate with: **stacked translucent surface tier + scrim under
  the overlay**, producing a "the layer above is heavier than the
  content below" effect, without claiming to be glass. If Slint ever
  ships live backdrop blur, the overlays trade up to it without further
  design work.

Opaque surfaces are NOT a tier we use. `ListItem` rows are *almost*
opaque (95 %) so the parent surface still shows through faintly when
they're inside a Card.

### 3. Motion

| Token | Duration | Use |
|---|---|---|
| `motion-fast` | 120 ms | Press feedback, hover, colour transitions |
| `motion-base` | 200 ms | Layout reflows, value changes (Slider thumb, ProgressBar fill, MobileSwitch knob) |
| `motion-overlay` | 240 ms | Drawer / Sheet / Dialog enter & exit |

Easing: `ease-out` on enters, `ease-in` on exits, plain interpolation
for state colour changes. No springs.

### 4. Type & density

Compact-leaning, mobile-safe:

| Token | Size | Weight | Use |
|---|---|---|---|
| `font-size-caption` | 12 px | 500 | Auxiliary, badge labels, captions |
| `font-size-body` | 14 px | 400 | Secondary body, list subtitles |
| `font-size-body-large` | 16 px | 400 | Primary body, form inputs (iOS-zoom-safe floor) |
| `font-size-title` | 20 px | 600 | Card titles, AppBar titles, dialog titles |
| `font-size-headline` | 28 px | 600 | Empty-state / login headlines, hero |

Weights stay at 400 / 500 / 600 / 700. No 300 (anti-pattern on small
screens) and no 800/900 (overkill at mobile sizes).

Spacing scale unchanged (4 / 8 / 16 / 24 / 32 px). Touch target floor
unchanged (48 dp).

## Token system

A full overhaul of `Theme`. New structure:

```slint
export enum ColorScheme { dark, light }

export global Theme {
    in-out property <ColorScheme> color-scheme: ColorScheme.dark;

    // ---- Surface tiers ----
    // The luminance ladder. Use the lowest tier that reads as raised.
    out property <color> background;       // page chrome behind everything
    out property <color> surface-1;        // base elevation — Card, ListItem-on-page
    out property <color> surface-2;        // mid elevation — Drawer, Dialog
    out property <color> surface-3;        // top elevation — Snackbar, FAB-above-content
    out property <color> surface-pressed;  // touch-feedback overlay

    // ---- Foreground ----
    out property <color> on-background;
    out property <color> on-surface;
    out property <color> muted;            // secondary text, icons-not-active

    // ---- Brand ----
    in-out property <color> primary;
    out property <color> primary-pressed;
    out property <color> on-primary;

    // ---- Status accents ----
    in-out property <color> accent-success;
    in-out property <color> accent-warning;
    in-out property <color> accent-danger;
    in-out property <color> accent-info;   // defaults to `primary`

    // ---- Elevation (shadow recipe per tier) ----
    out property <length> elevation-1-blur;   // Card
    out property <length> elevation-1-y;
    out property <color>  elevation-1-color;
    out property <length> elevation-2-blur;   // Drawer, Dialog
    out property <length> elevation-2-y;
    out property <color>  elevation-2-color;
    out property <length> elevation-3-blur;   // Snackbar, FAB above scrolled content
    out property <length> elevation-3-y;
    out property <color>  elevation-3-color;

    // ---- Spacing / radii / motion / type ----  (unchanged)
    // …existing tokens kept …
}
```

### Default values (dark scheme)

```
background          #0f1115
surface-1           #1a1d24 @ 92 %       ← slight alpha for Card
surface-2           #23262f @ 90 %       ← Drawer / Dialog
surface-3           #2d313a @ 90 %       ← Snackbar
surface-pressed     #2a2f3a               (opaque — instant feedback)
on-background       #f5f5f7
on-surface          #e4e6eb
muted               #9aa0a6
primary             #4f8cff
primary-pressed     #3a72e0
on-primary          #ffffff
accent-success      #22c55e
accent-warning      #f59e0b
accent-danger       #ef4444
accent-info         (alias of primary)

elevation-1: blur  8px, y  2px, color #00000033  (20 % black)
elevation-2: blur 16px, y  4px, color #00000055  (33 % black)
elevation-3: blur 24px, y  8px, color #00000077  (47 % black)
```

### Default values (light scheme)

```
background          #fafafa
surface-1           #ffffff @ 92 %
surface-2           #f4f5f7 @ 90 %
surface-3           #ebecef @ 90 %
surface-pressed     #e6e7eb
on-background       #0f1115
on-surface          #1a1d24
muted               #6b7280
primary             #2a72e0
primary-pressed     #1d5fc4
on-primary          #ffffff
accent-success      #16a34a
accent-warning      #d97706
accent-danger       #dc2626
accent-info         (alias of primary)

elevation-1: blur  8px, y  2px, color #00000018  (~10 % black)
elevation-2: blur 16px, y  4px, color #00000022  (~13 % black)
elevation-3: blur 24px, y  8px, color #00000033  (~20 % black)
```

`color-scheme` is a single switch. All `out` tokens recompute when it
flips (Slint's reactive binding handles this for free). Per-token
overrides remain possible on the `in-out` properties (`primary` and the
status accents) for branded themes.

## Accessibility commitments (unchanged from current state)

- 48 dp touch target floor — every interactive component honours
  `Theme.touch-target`.
- 16 px form input floor — `TextField` uses `font-size-body-large`.
- `accessible-role` + `accessible-label` + `accessible-action-default`
  on every interactive component.
- Colour contrast ≥ 4.5 : 1 for body text, ≥ 3 : 1 for UI components.
  This is a design constraint on token values, not an afterthought —
  light-scheme `primary` is darker than dark-scheme `primary` for this
  reason.

## What this asks of the implementation

Roughly in priority order (smallest blast radius first):

1. **`Theme` overhaul** — add `ColorScheme` enum, parallel value tables,
   surface tiers, status accents, elevation tokens. Existing token
   names that change (`surface` → `surface-1`, `outline` retired)
   require a sweep across every component file.
2. **Drop borders.** Every `border-width` / `border-color` outside of
   `TextField`'s focus ring goes away. Most components were already
   borderless; offenders are `MobileButton` (secondary variant uses a
   border), `TabBar` (top hairline), `AppBar` (bottom hairline),
   `BottomNav` (top hairline), `Stepper` (outer pill border),
   `Checkbox`/`Radio` (unchecked outer ring — *exception kept*: this
   ring IS the unchecked-state visual). Replace with surface-tier shifts
   or shadow.
3. **Surface alpha.** `Card`, `Banner`, `AppBar`-when-scrolled use 8 %
   alpha over their parent. Slint colour literals like `#1a1d24eb` or
   `Theme.surface-1.with-alpha(0.92)`.
4. **Shadow recipe per elevation.** Replace ad-hoc `drop-shadow-*`
   values in `Card`, `Fab`, `MobileSwitch`, etc. with the
   `Theme.elevation-{1,2,3}-{blur,y,color}` tokens.
5. **New components enabled by the new tokens.** `Dialog`, `Snackbar`,
   `Drawer`, `Sheet` — all overlay components — become buildable once
   `surface-2` / `surface-3` / `elevation-2` / `elevation-3` exist.
   They use the "stacked translucent surface + scrim" approximation
   noted above, not real backdrop blur.
6. **Status-tinted variants.** `MobileButton`, `Banner`, `Chip`,
   `Snackbar` (future) get a `tone: ToneEnum {default, success, warning,
   danger, info}` property mapping to the accent slots. One typed
   surface for all status communication.
7. **Visual regression refresh.** Every snapshot baseline gets
   regenerated against the new tokens — this is one
   `SLINT_CREATE_SCREENSHOTS=1` run, the diffs reviewed by a human,
   then committed.

Behavior tests don't change — accessibility metadata is orthogonal to
visual design.

## Where this lives in the project

- This file (`DESIGN.md`) — the design goals.
- `CLAUDE.md` — the iconography rule (still applies).
- `README.md` — user-facing consumption guide (will reference this doc
  in passing once the overhaul ships).
- `ui/theme.slint` — the implementation of the token system.
- `.claude/skills/slint-mobile-components/SKILL.md` — agent-facing
  working guide; the "design philosophy" section there will be tightened
  to reference this doc.

## Constraint we're tracking upstream

Slint does not currently expose live backdrop blur on Android (or
Linux). Our overlay approximation is the right call today; if Slint
gains a `backdrop-filter` primitive, the overlay components trade up
without further design work, because the implementation is already
abstracted behind `Theme.surface-2` + `elevation-2` and a scrim.
[Tracking discussion](https://github.com/slint-ui/slint/discussions/5710).
