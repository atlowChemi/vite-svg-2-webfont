---
description: Every option of the @atlowchemi/webfont-generator npm package, grouped by what it controls, with types and defaults.
---

# Node.js Configuration

Options for [`generateWebfonts()`](/node/api#generatewebfonts-options). `dest` and either `files` or `variants` are required; everything else is optional.

## Input

### `files`

<!--@include: @/_shared/options/files.md-->

- Type: `string[]`
- Required unless `variants` is set

### `variants`

<!--@include: @/_shared/options/variants.md-->

- Type: `FontVariant[]`, each `{ name: string; files: string[]; weight?: number; default?: boolean }`
- Default: omitted (a single-design font)

Design names are escaped in the generated selectors, and don't change output file names.

### `variantClassPrefix`

<!--@include: @/_shared/options/variant-class-prefix.md-->

- Type: `string`
- Default: `'icon--'`

Whitespace, NUL, and the old `templateOptions.variantClassPrefix` location are rejected.

### `missingGlyphs`

<!--@include: @/_shared/options/missing-glyphs.md-->

- Type: `{ behavior: 'blank' | 'error' | 'fallback'; variant?: string }`
- Default: `{ behavior: 'blank' }`

The `MissingGlyphBehavior` enum holds the behavior values:

```ts
import { MissingGlyphBehavior } from '@atlowchemi/webfont-generator';

const missingGlyphs = { behavior: MissingGlyphBehavior.Fallback, variant: 'regular' };
```

### `rename`

Derive each icon's name from its file path. Names become [class names](/guide/svg-authoring#file-names-become-class-names), so return letters, digits, hyphens, and underscores.

- Type: `(path: string) => string`
- Default: the file name without `.svg`

## Output

### `dest`

<!--@include: @/_shared/options/dest.md-->

- Type: `string`
- Required

### `fontName`

<!--@include: @/_shared/options/font-name.md-->

- Type: `string`
- Default: `'iconfont'`

### `types`

<!--@include: @/_shared/options/types.md-->

- Type: `FontType[]`
- Default: `['eot', 'woff', 'woff2']`, or `['woff', 'woff2']` for families

A literal list also narrows which result getters TypeScript treats as present; see [requested formats](/node/api#requested-formats).

### `order`

<!--@include: @/_shared/options/order.md-->

- Type: `FontType[]`
- Default: `['eot', 'woff2', 'woff', 'ttf', 'svg']`, filtered to `types`

### `writeFiles`

<!--@include: @/_shared/options/write-files.md-->

- Type: `boolean`
- Default: `true`

The result holds every output either way.

### `css`

<!--@include: @/_shared/options/css.md-->

- Type: `boolean`
- Default: `true`

### `cssDest`

<!--@include: @/_shared/options/css-dest.md-->

- Type: `string`
- Default: `<dest>/<fontName>.css`

### `cssFontsUrl`

<!--@include: @/_shared/options/css-fonts-url.md-->

- Type: `string`
- Default: the relative path from `cssDest` to `dest`

### `html`

<!--@include: @/_shared/options/html.md-->

- Type: `boolean`
- Default: `false`

### `htmlDest`

<!--@include: @/_shared/options/html-dest.md-->

- Type: `string`
- Default: `<dest>/<fontName>.html`

### `ts`

<!--@include: @/_shared/options/ts.md-->

- Type: `boolean`
- Default: `false`

`result.generateTs()` returns the same source even when this is off; see [TypeScript metadata](/node/api#generatets).

### `tsDest`

<!--@include: @/_shared/options/ts-dest.md-->

- Type: `string`
- Default: `<dest>/<fontName>.ts`

Setting it alone doesn't enable the output; set `ts` too.

## Templates

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `cssTemplate`

<!--@include: @/_shared/options/css-template.md-->

- Type: `string`
- Default: the built-in CSS template

The built-in templates' paths are exported for use as a starting point; see [template paths](/node/api#template-paths).

### `htmlTemplate`

<!--@include: @/_shared/options/html-template.md-->

- Type: `string`
- Default: the built-in HTML template

### `templateOptions`

<!--@include: @/_shared/options/template-options.md-->

- Type: `Record<string, any>`
- Default: `{}`

### `cssContext`

Change the template context before the CSS renders. See [context fields and callback typing](/guide/templates#node-callbacks).

- Type: `(context: CssContext) => void`, or `(context: CssContext<true>) => void` for families

Results created with this callback can't be [updated incrementally](/guide/features/incremental#restrictions).

### `htmlContext`

Change the template context before the HTML preview renders. See [context fields and callback typing](/guide/templates#node-callbacks).

- Type: `(context: HtmlContext) => void`, or `(context: HtmlContext<true>) => void` for families

Results created with this callback can't be [updated incrementally](/guide/features/incremental#restrictions).

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

See [color selection](/node/usage#color-selection) for examples.

### `colorSlots`

<!--@include: @/_shared/options/color-slots.md-->

- Type: `Record<string, string>`
- Default: omitted

The `cssContext` callback receives `colorSlots` too. It overrides a `templateOptions` value with the same name.

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

This package always writes a fixed TTF timestamp, like `@vusion/webfonts-generator`, so builds are reproducible and `ttf.ts` is ignored.

::: details FormatOptions type definition

```ts
interface FormatOptions {
    svg?: { centerVertically?: boolean; fontId?: string; metadata?: string; optimizeOutput?: boolean; preserveAspectRatio?: boolean };
    ttf?: { copyright?: string; description?: string; ts?: number; url?: string; version?: string };
    woff?: { metadata?: string };
    woff2?: { compressionQuality?: number };
}
```

:::

## Performance

### `incremental`

<!--@include: @/_shared/options/incremental.md-->

- Type: `boolean`
- Default: `false`

Update the result with [`regenerate()`, `regenerateAsync()`, or the single-file methods](/node/api#updating-a-result).

### `cacheLocation`

<!--@include: @/_shared/options/cache-location.md-->

- Type: `string`
- Default: omitted (no persistent cache)

- This package always sets `formatOptions.ttf.ts`, so cached fonts never carry another run's timestamp.
- With [`incremental`](#incremental), every committed update queues a background cache write. Await [`result.flushCache()`](/node/api#flushcache) before shutdown.
- Remove the files the engine created with [`clearCache()`](/node/api#clearcache-cachelocation). Rebuilding the native engine locally without a version change keeps old entries, so clear the cache after such a build.
- A directory where the cache file belongs is never removed; generation prints a warning and stays uncached until you remove it.
- Cached fonts are reused as-is, so only restore trusted caches. See [keep it between CI runs](/guide/features/caching#keep-it-between-ci-runs).

### `threads`

<!--@include: @/_shared/options/threads.md-->

- Type: `number` (a positive integer)
- Default: one per CPU core
