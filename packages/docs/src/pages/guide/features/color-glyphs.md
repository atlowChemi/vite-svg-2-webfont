---
description: Keep the SVG colors of selected icons with COLR/CPAL color glyphs, theme them with CSS palettes, and know where they fall back to one color.
---

# Color Glyphs

By default, every icon becomes a single-color glyph that follows the text color. Color glyphs keep the solid fills from their SVGs instead. They're stored in the font's COLR v1 and CPAL tables, with a single-color fallback for platforms that can't render them.

## Select icons

Color is opt-in. Set `colorGlyphs` to `true` for every icon, or list the icons by name:

::: code-group

```ts [Vite]
viteSvgToWebfont({
    context: './src/icons',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
});
```

```ts [Node.js]
await generateWebfonts({
    files: ['icons/logo.svg', 'icons/add.svg'],
    dest: 'dist/fonts',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
});
```

```ts [Browser]
using result = await generateWebfonts({
    sources,
    types: ['woff2'],
    colorGlyphs: ['logo'],
});
```

```json [CLI]
{
    "dest": "dist/fonts",
    "files": ["icons"],
    "types": ["woff2", "woff"],
    "colorGlyphs": ["logo"]
}
```

```rust [Rust]
let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    files: vec!["icons/logo.svg".to_owned(), "icons/add.svg".to_owned()],
    types: Some(vec![FontType::Woff2, FontType::Woff]),
    color_glyphs: Some(ColorGlyphSelection::Named(vec!["logo".into()])),
    ..Default::default()
};
```

:::

Here `logo` keeps its colors and `add` stays single-color. The CLI has no color flag; use a JSON manifest with `--config`.

- **Names** are the final icon names, after any `rename`. Unknown names fail generation, all reported together. Duplicates are harmless.
- **Formats:** color needs TTF, WOFF, or WOFF2. SVG fonts and EOT are rejected, and the default formats for single-design fonts include EOT, so set `types` explicitly.
- **Off:** omit the option or pass an empty list. `false`, `null`, and plain strings are rejected.
- **[Variants](/guide/features/variants):** a selected name applies to every design, including designs that borrow it through a `fallback` [missing glyph policy](/guide/features/variants#icons-missing-from-a-design). Each design can use its own colors.
- **Font URLs:** the selection is part of the hash in generated font URLs, so changing it busts caches. The order of names, duplicates, and an empty list versus no option don't change the hash.

## How SVG paint maps to the glyph

| SVG source                                         | Color glyph paint                        |
| -------------------------------------------------- | ---------------------------------------- |
| No authored fill                                   | Text color                               |
| A solid fill, including one inherited from a group | That fixed color                         |
| `fill="currentColor"` with no authored `color`     | Text color                               |
| `fill="currentColor"` with an authored `color`     | That fixed color                         |
| A solid fill with `fill-opacity`                   | That color, with the layer's own opacity |

Paint order, curves, nonzero winding, and ordinary nested `evenodd` holes are supported. Color layers and the fallback share positioning and normalization. Inline styles and internal stylesheets follow the SVG cascade.

Everything else is **best effort**: gradients, strokes as color paint, text, images, clipping, masks, filters, paint-dependent selectors, CSS resets or invalid declarations, and intersecting `evenodd` contours can render incompletely or incorrectly, and the engine doesn't warn about them. If a supported case renders wrong, please [report it](https://github.com/atlowChemi/vite-svg-2-webfont/issues) with a minimal SVG.

## Theme colors

A color glyph's fixed colors can be swapped at runtime with CSS `@font-palette-values` and `font-palette`, so one glyph can serve light and dark themes. Paths with `currentColor` keep following the text color.

To theme a color, reserve a named palette slot for it with `colorSlots`, such as `{ secondary: '#ff9900' }`, and draw the paths that should change in that color. Generated CSS templates receive `colorSlots.secondary`, the palette index to override in `@font-palette-values`' `override-colors`:

```css
@font-palette-values --icons-dark {
    font-family: icons;
    override-colors: 0 #6ee7b7; /* 0 is colorSlots.secondary, the only slot */
}

.dark .icon {
    font-palette: --icons-dark;
}
```

- Slot names use ASCII letters, digits, hyphens, and underscores. Values are distinct `#RRGGBB` colors, matched against each path's resolved color.
- Slots need an active `colorGlyphs` selection, and only affect selected icons.
- Indices start at zero in ASCII order of slot names, followed by the other colors the engine finds. Every slot is reserved, even when no icon uses its color yet, so adding or changing icons never moves them. Adding, removing, or renaming slots can, so read indices from the template context instead of hard-coding them.
- Palettes take explicit colors, not CSS variables: define one `@font-palette-values` rule per theme.

The [theme colors example](/vite/examples#theme-colors-in-one-glyph) shows the complete template and a live light and dark glyph.

## Browser and platform support

Chromium and Firefox render fixed colors, the text color, and per-layer opacity. WebKit on Linux renders color too. In the tested macOS WebKit configuration, Safari uses the single-color fallback: the font still loads and keeps the text color, weight, ligatures, and widths, but loses fixed colors and layer opacity. No separate font or setting is needed, and this may change in future Safari versions; see [WebKit issue 233496](https://bugs.webkit.org/show_bug.cgi?id=233496). Platforms that use the fallback also lose theme palette overrides.

Color layers add glyphs, so color fonts are larger. A font must stay within 65,535 glyphs and 65,278 unique glyph names; exceeding either fails the build.

## Updates

Color works with [incremental updates](/guide/features/incremental) and needs no separate API. A paint-only edit changes the font bytes and their URL hash, and a fresh build and an incremental build of the same inputs produce the same output. The selection and slots stay fixed for a result's lifetime. Removing the last icon with a selected name fails rather than silently dropping it from the selection; failed updates keep the previous output.

## Authoring tips

Icons that aren't selected follow the [single-color rules](/guide/svg-authoring#colors-are-ignored-nested-shapes-become-holes): fills are ignored and nested shapes become holes. Before reaching for color, check whether the icon only needs a background removed.
