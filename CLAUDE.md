# slint-mobile-components — Claude notes

## Iconography

**Icons used within a single element type must render at visually identical
size.** This applies to every `BottomNavItem`'s `icon-text`, every
`ListItem`'s leading/trailing icons, every `AppBar`'s leading/trailing,
and `Fab`'s `icon-text`.

The current scaffold uses bare Unicode glyphs (e.g. `⌂` `⌕` `✉` `☺` in
the BottomNav of `ui/pages/home.slint`) — those four characters come from
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
