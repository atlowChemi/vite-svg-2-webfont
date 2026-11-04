---
description: Every option of the @atlowchemi/webfont-browser-generator package, grouped by what it controls, with types and defaults.
---

# Browser Configuration

Options for [`generateWebfonts()`](/browser/api#generatewebfonts-options-runtime). Either `sources` or `variants` is required; everything else is optional. Unknown options are rejected, including the file options of the other tools, such as `dest`.

## Input

### `sources`

The SVG icons to include, for a font with one design of each icon. `svg` is the SVG document's text, `name` is the icon's name in class names and ligatures, and `id` identifies the source in [incremental updates](/browser/api#updating-a-result). Leave it out when you use `variants`.

- Type: `SvgSource[]`, each `{ id: string; name: string; svg: string }`
- Required unless `variants` is set

Names use Unicode letters, digits, hyphens, and underscores; other names are rejected.

### `variants`

<!--@include: @/_shared/options/variants.md-->

- Type: `SourceFontVariant[]`, each `{ name: string; sources: SvgSource[]; weight?: number; default?: boolean }`
- Default: omitted (a single-design font)

Sources with the same `name` are the same icon in each design; each keeps its own `id`.

### `variantClassPrefix`

<!--@include: @/_shared/options/variant-class-prefix.md-->

- Type: `string`
- Default: `'icon--'`

### `missingGlyphs`

<!--@include: @/_shared/options/missing-glyphs.md-->

- Type: `{ behavior: 'blank' | 'error' | 'fallback'; variant?: string }`
- Default: `{ behavior: 'blank' }`

## Output

Nothing is written to disk: every output stays on the [result](/browser/api#webfontresult).

### `fontName`

<!--@include: @/_shared/options/font-name.md-->

- Type: `string`
- Default: `'iconfont'`

### `types`

<!--@include: @/_shared/options/types.md-->

- Type: `FontType[]`
- Default: `['eot', 'woff', 'woff2']`, or `['woff', 'woff2']` for families

### `order`

<!--@include: @/_shared/options/order.md-->

- Type: `FontType[]`
- Default: `['eot', 'woff2', 'woff', 'ttf', 'svg']`, filtered to `types`

### `css`

Whether [`collectOutputs()`](/browser/api#collectoutputs) includes the CSS. `generateCss()` renders it either way.

- Type: `boolean`
- Default: `true`

### `html`

Whether [`collectOutputs()`](/browser/api#collectoutputs) includes the [HTML preview](/guide/templates#html-previews). `generateHtml()` renders it either way.

- Type: `boolean`
- Default: `false`

### `ts`

Whether [`collectOutputs()`](/browser/api#collectoutputs) includes the [TypeScript metadata](/guide/templates#typescript). `generateTs()` renders it either way.

- Type: `boolean`
- Default: `false`

### `cssFontsUrl`

<!--@include: @/_shared/options/css-fonts-url.md-->

- Type: `string`
- Default: none, so the CSS refers to the bare file names, such as `iconfont.woff2?…`

Set it to where you will serve the fonts. To use the fonts on the current page without serving them, see [use the fonts on the page](/browser/examples#live-preview).

## Templates

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `cssTemplate`

Text of a custom Handlebars template for the CSS, not a file path. It receives the [template context](/guide/templates#template-context) plus any `templateOptions`.

- Type: `string`
- Default: the built-in CSS template

### `htmlTemplate`

Text of a custom Handlebars template for the HTML preview, not a file path.

- Type: `string`
- Default: the built-in HTML template

### `templateOptions`

<!--@include: @/_shared/options/template-options.md-->

- Type: `Record<string, JsonValue>`
- Default: `{}`

Values must be JSON: there are no JavaScript callbacks in the browser.

## Glyphs

### `codepoints`

<!--@include: @/_shared/options/codepoints.md-->

- Type: `Record<string, number>`
- Default: `{}`

### `startCodepoint`

<!--@include: @/_shared/options/start-codepoint.md-->

- Type: `number`
- Default: `0xF101`

### `ligature`

<!--@include: @/_shared/options/ligature.md-->

- Type: `boolean`
- Default: `true`

### `fontHeight`

<!--@include: @/_shared/options/font-height.md-->

- Type: `number`
- Default: the tallest icon's height

### `ascent`

<!--@include: @/_shared/options/ascent.md-->

- Type: `number`

### `descent`

<!--@include: @/_shared/options/descent.md-->

- Type: `number`

### `normalize`

<!--@include: @/_shared/options/normalize.md-->

- Type: `boolean`
- Default: `true`

### `fixedWidth`

<!--@include: @/_shared/options/fixed-width.md-->

- Type: `boolean`
- Default: `false`

### `centerHorizontally`

<!--@include: @/_shared/options/center-horizontally.md-->

- Type: `boolean`
- Default: `false`

### `centerVertically`

<!--@include: @/_shared/options/center-vertically.md-->

- Type: `boolean`
- Default: `false`

### `preserveAspectRatio`

<!--@include: @/_shared/options/preserve-aspect-ratio.md-->

- Type: `boolean`

### `round`

<!--@include: @/_shared/options/round.md-->

- Type: `number`

### `optimizeOutput`

<!--@include: @/_shared/options/optimize-output.md-->

- Type: `boolean`
- Default: `false`

## Color

### `colorGlyphs`

<!--@include: @/_shared/options/color-glyphs.md-->

- Type: `true | string[]`
- Default: omitted (single-color)

### `colorSlots`

<!--@include: @/_shared/options/color-slots.md-->

- Type: `Record<string, string>`
- Default: omitted

## Font metadata

### `fontStyle`

<!--@include: @/_shared/options/font-style.md-->

- Type: `string`

### `fontWeight`

<!--@include: @/_shared/options/font-weight.md-->

- Type: `string`

### `formatOptions`

<!--@include: @/_shared/options/format-options.md-->

- Type: `FormatOptions`

Without `ttf.ts`, generation reads the browser clock once and keeps that timestamp through updates. Set it, for example to `0`, for output that is identical on every run.

## Performance

Generation runs in a worker, so it never blocks the page.

### `incremental`

<!--@include: @/_shared/options/incremental.md-->

- Type: `boolean`
- Default: `false`

Update the result with [`add()`, `update()`, `remove()`, or `regenerate()`](/browser/api#updating-a-result).

### `cache`

Bytes of the [engine cache](/guide/features/caching) from an earlier session, or an empty array to start cold. Unchanged icons and formats are reused, and the output is the same as without it. Store what [`exportCache()`](/browser/api#exportcache) returns and pass it back next time; see [persistent cache](/browser/usage#persistent-cache).

- Type: `Uint8Array`
- Default: omitted (no persistent cache)

Cached fonts are reused as-is, so only pass bytes your own app stored, never bytes from users or other origins.
