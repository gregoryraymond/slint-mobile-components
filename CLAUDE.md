# slint-mobile-components — Claude notes

## Workspace layout

The repo is a Cargo workspace, split for compile-time scope:

- `crates/theme/` — design tokens (`Theme`, `Tone`, `ColorScheme`).
  `.slint` files imported as `@mobile-theme/theme.slint`.
- `crates/components/` — the 31 reusable widgets + shared `icons/`.
  Imported as `@mobile-components/<file>.slint`.
- `crates/pages-{auth,commerce,finance,health,media,misc,productivity,social,system,travel}/`
  — full-screen page templates (~145 total), one crate per category.
  Imported as `@mobile-pages-<cat>/<file>.slint`.
- Root crate (`./tests/`) — `snapshot_scenes.slint` and
  `behavior_scenes.slint` are widget-level test aggregators only;
  page-level snap scenes live inside each `crates/pages-<cat>/ui/_snapshot_scenes.slint`.
- `crates/viewer/` — bin crate (`cargo run` / `cargo view`) — the
  interpreter-backed paginated browser for the whole screen library.
- `android-demo/` — `cargo apk` crate, exists so the workspace
  compiles for an Android target. `ui/main.slint` shows `HomePage` —
  there's no Gallery anymore.

Downstream consumers (external apps) wire the `library_paths` by
calling `slint_mobile_components::library_paths()` from their
`build.rs` — see `android-demo/build.rs` as the reference.

When adding a new page: drop it in the relevant `crates/pages-<cat>/ui/`,
then run `cargo xtask split-snapshots` to add the matching `Snap*Page`
re-export to that crate's `_snapshot_scenes.slint`. No new workspace
crate is needed unless the page genuinely doesn't fit any existing
category — `pages-misc/` is the catch-all.

## Iconography

**Icons used within a single element type must render at visually identical
size.** This applies to every `BottomNavItem`'s `icon-text`, every
`ListItem`'s leading/trailing icons, every `AppBar`'s leading/trailing,
and `Fab`'s `icon-text`.

The current scaffold uses bare Unicode glyphs (e.g. `⌂` `⌕` `✉` `☺` in
the BottomNav of `crates/pages-misc/ui/home.slint`) — those four characters come from
different Unicode blocks and have wildly different intrinsic heights /
widths, so the bottom-nav row looks ragged. Same risk in any other place
arbitrary Unicode is dropped into an `icon-text` string.

When adding or changing icons, pick one of these in order of preference:

1. **Icon font** (Material Symbols, Phosphor, Lucide). Every glyph is
   metric-matched. Set `font-family` on the icon `Text` and use the
   font's codepoints.
2. **`Image` source with explicit `width` / `height`.** Replace the
   `string` icon properties with `image` properties; layout controls the
   rendered size, glyph metrics don't matter.
3. **Stopgap:** if you must keep `icon-text: string`, pick characters
   from a single Unicode block (e.g. Geometric Shapes, or Material's
   private-use range) so their intrinsic sizes are at least comparable,
   and wrap the glyph in a fixed-size square container.

Don't ship mixed-block Unicode as the long-term answer — convert to an
icon font or `Image` as soon as the icon set stabilises.


This file defines rules for writing Slint (`.slint`) code in this project. Follow them strictly. If a rule conflicts with what the user asks, surface the conflict before writing code rather than silently breaking the rule.

## Core principle

UI consistency comes from **never inventing values**. Every size, spacing, color, font, and radius must come from a theme singleton. If the value you need doesn't exist in the theme, stop and ask — do not invent one inline.

---

## 1. Theme tokens are the only source of truth

The project exposes design tokens through global singletons (e.g. `Theme`, `Spacing`, `Typography`, `Palette`). Use them for everything.

**Required:**
- Spacing, padding, gaps → `Spacing.xs`, `Spacing.sm`, `Spacing.md`, `Spacing.lg`, `Spacing.xl` (or the project's equivalent names).
- Colors → `Palette.*` (never hex literals, never `#rrggbb` inline).
- Font sizes and weights → `Typography.*`.
- Border radii, border widths, elevation → `Theme.*`.
- Icon sizes → `Theme.icon-size-sm` / `md` / `lg`.

**Forbidden:**
- Numeric literals for `padding`, `spacing`, `width`, `height`, `font-size`, `border-radius`, `border-width` anywhere in component code. The only exception is `0` (e.g. `padding: 0;` to explicitly disable padding).
- Hex or named colors inline (`#ffffff`, `white`, `red`). Always go through the palette.
- Re-declaring spacing or color values at the component level. If something is missing, add it to the theme singleton, not the component.

**If a token is missing:** stop writing code, list which tokens you need, and ask whether to add them to the theme or use the closest existing token. Do not pick a number.

---

## 2. Every visual child lives inside a layout

`Rectangle` does not lay out its children — it stacks them at `0,0`. This is the single biggest source of "mystery gap" and "elements overlapping" bugs.

**Rules:**
- Any element with more than one visual child must have a layout (`VerticalLayout`, `HorizontalLayout`, `GridLayout`, `VerticalBox`, `HorizontalBox`) as the container, or contain a layout as its only direct child.
- A `Rectangle` used as a styled background (for color, border, radius) must contain exactly one child: a layout. Never put multiple siblings directly inside a styling `Rectangle`.
- A single child inside a parent does not require a layout, but if there's any chance a second child will be added later, use one anyway.

**Pattern for a styled container:**

```slint
Rectangle {
    background: Palette.surface;
    border-radius: Theme.radius-md;

    VerticalLayout {
        padding: Spacing.md;
        spacing: Spacing.sm;

        // children here
    }
}
```

Never put the children directly inside the outer `Rectangle`.

---

## 3. Choose the right layout primitive

Slint has two families and they behave differently. Mixing them produces inconsistent spacing.

- **`VerticalBox` / `HorizontalBox` / `GridBox`** — have default `spacing` and `padding` from the style. Use these for **screen-level and section-level** layout where the style's defaults are appropriate.
- **`VerticalLayout` / `HorizontalLayout` / `GridLayout`** — have **no** default spacing or padding. Use these for **component-internal** layout where you control spacing explicitly via theme tokens.

**Rule:** Inside a reusable component, prefer `VerticalLayout` / `HorizontalLayout` / `GridLayout` and set `spacing` and `padding` explicitly from `Spacing.*`. This makes the component self-contained and predictable regardless of where it's used.

**For column alignment across rows:** use `GridLayout` with explicit `Row { }` blocks. Do not stack `HorizontalLayout` rows and hope columns line up — they won't unless every cell has identical `min-width`, and that's fragile.

---

## 4. Sizing rules

Hard-pinned `width` and `height` fight the layout system and are the main cause of overflows and wrong-sized buttons.

**Default to:**
- `min-width`, `max-width`, `preferred-width` (and the height equivalents) — let the layout negotiate.
- `horizontal-stretch` / `vertical-stretch` to control how excess space is distributed. Default is `1`; set `0` for elements that should stay at their preferred size.

**Only pin `width` / `height` for:**
- Icons and avatars (use `Theme.icon-size-*`).
- Elements with a genuine fixed-pixel requirement (rare; document why with a comment).

**To make an element fill its parent inside a layout:** set `horizontal-stretch: 1` (and/or `vertical-stretch: 1`). Do not write `width: parent.width` inside a layout — it breaks size negotiation.

**Buttons specifically:**
- Never set `width` on a button directly. Set `min-width` from the theme if needed, and let the layout size it.
- If a button should fill a row, wrap it in a layout cell with `horizontal-stretch: 1` or use `GridLayout` with `colspan`.

---

## 5. Clipping and overflow

Overflow is almost always one of: a child with a fixed size larger than its parent, a missing `clip`, or a missing `ScrollView`.

**Rules:**
- Any container with a `border-radius` that holds children which could visually exceed the rounded corners must set `clip: true`.
- Any region that holds content of unbounded or unknown size (lists, text that can grow, user-generated content) must be wrapped in `ScrollView`. Do not assume content will fit.
- Modal/popup/overlay containers must set `clip: true` on the backdrop to prevent shadow or content bleed.
- When a parent has a fixed size and a child does not, the child's `max-width` / `max-height` must be constrained — either explicitly or by being inside a layout that does so.

---

## 6. Custom components

When creating a new reusable component (anything that will be instantiated more than once, or anything that represents a named UI concept like `Card`, `ListRow`, `Toolbar`):

**Required:**
- The component's root element must be a layout, or a `Rectangle` whose only child is a layout (per Rule 2).
- All spacing, padding, and sizing inside the component must come from the theme.
- The component must expose `in` properties for any content that varies (text, icons, callbacks) — do not hardcode strings or icons.
- The component must declare `preferred-width` and `preferred-height` (or rely on its layout to compute them). Do not leave size implicit.
- Document any non-obvious layout decisions with a comment, especially stretch values and `min-width` choices.

**Layout choice inside a custom component:**
- Use `VerticalLayout` / `HorizontalLayout` / `GridLayout` (the non-Box variants) so spacing is explicit and not dependent on the parent style.
- Set `spacing` and `padding` from `Spacing.*` tokens at the top of the layout.

**Forbidden in custom components:**
- Reaching into `parent.*` for sizing (e.g. `width: parent.width - 20px`). Use layout stretch instead.
- Defining colors, sizes, or spacing as component-local properties when an equivalent theme token exists.
- Multiple direct children inside a styling `Rectangle` (per Rule 2).

---

## 7. GridLayout discipline

When using `GridLayout`:
- Always use explicit `Row { }` blocks rather than `row:` / `col:` properties on individual children. Rows make the structure visually obvious and prevent off-by-one alignment bugs.
- Set `spacing` (or `spacing-horizontal` / `spacing-vertical`) from `Spacing.*` at the top of the grid.
- If you need a cell to span columns, use `colspan` explicitly.
- Do not mix `Row { }` blocks with bare children in the same `GridLayout`.

---

## 8. Before writing non-trivial UI code

For any component with more than ~3 nested layouts, or any new screen:

1. **Sketch the layout tree first** as a nested outline showing which layout primitive contains what, with the theme tokens for spacing and sizing noted.
2. **Confirm the tokens you'll use** exist in the theme. If any are missing, stop and ask.
3. **Then write the code.**

This catches structural mistakes before they become rendered bugs.

---

## 9. When fixing UI bugs

- If the user reports a visual issue, ask for a screenshot before guessing. Visual problems are much easier to fix from an image than from a description.
- When fixing alignment or spacing, check whether the root cause is a missing layout, a wrong layout primitive (Box vs non-Box), or a hard-pinned size — in that order. Do not paper over the issue with a magic-number `padding` or `spacing`.
- After fixing, verify no new theme-bypass values were introduced.

---

## 10. Quick checklist before considering a Slint file done

- [ ] No numeric literals for spacing, sizing, font, color, or radius (except `0`).
- [ ] No hex or named color literals.
- [ ] Every multi-child container has a layout, not a bare `Rectangle`.
- [ ] `clip: true` on any rounded or bounded container that could overflow.
- [ ] `ScrollView` around any unbounded content region.
- [ ] No `width:` / `height:` on buttons or flexible elements — `min-width` / stretch instead.
- [ ] Custom components expose `in` properties for variable content and use non-Box layouts internally.
- [ ] `GridLayout` uses explicit `Row { }` blocks.

