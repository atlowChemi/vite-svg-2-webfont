---
description: Selective COLR v1 glyphs, SVG paint semantics, output formats, and browser compatibility.
---

# Color glyphs

Color generation is opt-in. It preserves solid SVG fills in COLR v1/CPAL tables and retains a monochrome `glyf` fallback in the same font. It supports ordinary fonts and discrete multi-weight families. This option is available in the generator APIs and CLI manifest; the Vite plugin does not yet expose it.

## Selection and formats

| Interface                  | Option         | Values                                                                        |
| -------------------------- | -------------- | ----------------------------------------------------------------------------- |
| Node and CLI JSON manifest | `colorGlyphs`  | `true` or an array of strings                                                 |
| Rust                       | `color_glyphs` | `Some(ColorGlyphSelection::All)` or `Some(ColorGlyphSelection::Named(names))` |

Omit the option or use an empty list for existing monochrome behavior. Explicit `null`, `false`, a string, and mixed-type arrays are invalid. Named selection uses final glyph names **after rename**, not source paths. Duplicates are harmless. Unknown names are reported together in first-occurrence input order. In a variant family, a selected logical name applies to every resolved design, including fallback states; blank states have no color record.

Active color supports **TTF, WOFF, and WOFF2**. SVG-font and EOT output are rejected during option resolution. Ordinary defaults include EOT, so ordinary color callers must specify compatible `types`. Variant defaults are already WOFF and WOFF2.

```ts
const result = await generateWebfonts({
    files: ['icons/logo.svg', 'icons/add.svg'],
    dest: 'dist/fonts',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
    incremental: true,
});

// After changing only logo.svg's fill, update the existing result.
result.regenerate({ files: ['icons/logo.svg', 'icons/add.svg'] }, [{ path: 'icons/logo.svg', changeType: 'changed' }]);
```

The unselected `add` glyph remains monochrome. Use `true` to select both.

```ts
const family = await generateWebfonts({
    dest: 'dist/fonts',
    variants: [
        { name: 'light', files: ['light/logo.svg'], weight: 300, default: true },
        { name: 'bold', files: ['bold/logo.svg'], weight: 700 },
    ],
    colorGlyphs: ['logo'],
    incremental: true,
});
// Each SVG can use different fixed colors. Weight selection chooses its paint and outline.
family.regenerate(
    {
        variants: [
            { variant: 'light', files: ['light/logo.svg'] },
            { variant: 'bold', files: ['bold/logo.svg'] },
        ],
    },
    [{ path: 'bold/logo.svg', changeType: 'changed' }],
);
```

## SVG paint

| SVG source                                               | Color glyph paint                         |
| -------------------------------------------------------- | ----------------------------------------- |
| No authored effective fill                               | Host text color                           |
| Authored solid fill, including inherited root/group fill | Fixed SVG color                           |
| `fill="currentColor"` without effective authored `color` | Host text color                           |
| `fill="currentColor"` with effective authored `color`    | Fixed resolved SVG color                  |
| Solid fill with `fill-opacity`                           | That paint with independent layer opacity |

Source paint order, curves, nonzero winding, and ordinary nested evenodd holes are supported. Color layers and monochrome fallback share positioning and normalization. Authored inline styles and internal stylesheets use the SVG parser's cascade.

Advanced input is **best-effort**: gradients, strokes as color paint, text, images, clipping, masks, filters, paint-dependent selectors, CSS resets/invalid declarations, and intersecting evenodd contours can produce incomplete or incorrect results. There is no dedicated unsupported-input detection, warning, or rejection. Please report a minimal SVG reproduction when a supported case renders incorrectly.

## Incremental results and failures

The existing result and regeneration methods are used; there are no color-specific getters. Paint-only edits change font bytes and their hashes, including generated CSS font URLs. Changing color selection between builds also changes the URL hash; array order and duplicate names do not. Omission and an empty array retain the same monochrome hash. A fresh build and an incremental build of the same final inputs produce matching output. Variant fallback consumers receive their source's paint changes, while blank/fallback membership transitions update the COLR records.

Named selection is checked again after regeneration renames and membership changes. Removing the last source of a selected logical name fails rather than silently disabling that selection. Selection, parsing, or font-build failures retain the previous output and permit a corrected retry.

The existing write-failure contracts still apply. Ordinary regeneration restores the previous in-memory result; retry with the changes or a full re-diff. Rust and synchronous variant regeneration retain newly committed in-memory output and pending disk writes; a no-op retry completes those writes. Node asynchronous regeneration preserves the receiver and returns a new result only on success; failed writes can be reconciled on retry. See [Node regeneration](./node#incremental) and [Rust usage](./rust).

## Browser and platform compatibility

| Tested renderer                 | Result                                           |
| ------------------------------- | ------------------------------------------------ |
| Chromium 153.0.8010.12          | Fixed color, foreground color, and layer opacity |
| Firefox 155.0                   | Fixed color, foreground color, and layer opacity |
| Playwright WebKit on Linux (CI) | Fixed color, foreground color, and layer opacity |
| WebKit 26.6 on macOS 26.4.1     | Monochrome fallback                              |

WebKit behavior depends on the operating system and platform text-rendering libraries. In the tested macOS configuration, the font still loads: the fallback retains host text color, selected weight, ligatures, and advances, but loses fixed colors and independent per-layer opacity. It uses the same font URL; no separate API or alternate resource is needed. These observations do not imply a permanent limitation in future Safari, WebKit, or macOS versions. See [WebKit issue 233496](https://bugs.webkit.org/show_bug.cgi?id=233496).

Color layers add physical glyphs and increase font size. Fonts must fit the 65,535-glyph limit and the `post` table's 65,278 unique custom-name limit; exceeding either returns a build error.
