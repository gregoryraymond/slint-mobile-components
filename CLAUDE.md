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
