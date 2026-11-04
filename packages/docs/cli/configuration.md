---
description: Every option of the webfont-generator command-line tool, as a flag and in the JSON manifest, grouped by what it controls.
---

# CLI Configuration

The CLI takes options as flags for quick runs, or as a [JSON manifest](/cli/usage#json-manifest) with `--config` for everything else. Each option below lists its flag, if it has one; every option works in a manifest, under the name in its heading. `dest` and either `files` or `variants` are required.

## Input

### `files`

The SVG files to include, or directories of them, for a font with one design of each icon. A directory adds its `.svg` files, sorted by name, without subdirectories. Leave it out when you use `variants`.

- Flag: `<FILES>...`, the positional arguments
- Type: `string[]`
- Required unless `variants` is set

On the command line, inputs must be explicit paths such as `./icons/` or `../shared/plus.svg`, because bare names like `icons` are reserved for [subcommands](/cli/usage#subcommands). In a manifest, paths are relative to the manifest, and duplicate files are rejected.

### `variants`

<!--@include: @/_shared/options/variants.md-->

- Flag: none, manifest only
- Type: `{ name: string; files: string[]; weight?: number; default?: boolean }[]`
- Default: omitted (a single-design font)

Each design's `files` takes files and directories like the top-level `files`. The same file may appear in several designs.

### `variantClassPrefix`

<!--@include: @/_shared/options/variant-class-prefix.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `'icon--'`

### `missingGlyphs`

<!--@include: @/_shared/options/missing-glyphs.md-->

- Flag: none, manifest only
- Type: `{ behavior: 'blank' | 'error' | 'fallback'; variant?: string }`
- Default: `{ behavior: 'blank' }`

## Output

### `dest`

<!--@include: @/_shared/options/dest.md-->

- Flag: `-d, --dest <DEST>`
- Type: `string`
- Required

### `fontName`

<!--@include: @/_shared/options/font-name.md-->

- Flag: `-n, --font-name <NAME>`
- Type: `string`
- Default: `'iconfont'`

### `types`

<!--@include: @/_shared/options/types.md-->

- Flag: `-t, --types <TYPES>`, comma-separated, such as `woff2,woff`
- Type: `FontType[]`
- Default: `['eot', 'woff', 'woff2']`, or `['woff', 'woff2']` for families

### `order`

<!--@include: @/_shared/options/order.md-->

- Flag: none, manifest only
- Type: `FontType[]`
- Default: `['eot', 'woff2', 'woff', 'ttf', 'svg']`, filtered to `types`

### `writeFiles`

<!--@include: @/_shared/options/write-files.md-->

- Flag: `--write` / `--no-write`
- Type: `boolean`
- Default: `true`

With `--no-write`, the CLI generates everything and writes nothing: a dry run that still reports errors.

### `css`

<!--@include: @/_shared/options/css.md-->

- Flag: `--css` / `--no-css`
- Type: `boolean`
- Default: `true`

### `cssDest`

<!--@include: @/_shared/options/css-dest.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `<dest>/<fontName>.css`

### `cssFontsUrl`

<!--@include: @/_shared/options/css-fonts-url.md-->

- Flag: `--css-fonts-url <URL>`
- Type: `string`
- Default: the relative path from `cssDest` to `dest`

### `html`

<!--@include: @/_shared/options/html.md-->

- Flag: `--html` / `--no-html`
- Type: `boolean`
- Default: `false`

### `htmlDest`

<!--@include: @/_shared/options/html-dest.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `<dest>/<fontName>.html`

### `ts`

<!--@include: @/_shared/options/ts.md-->

- Flag: `--ts` / `--no-ts`
- Type: `boolean`
- Default: `false`

### `tsDest`

<!--@include: @/_shared/options/ts-dest.md-->

- Flag: `--ts-dest <PATH>`
- Type: `string`
- Default: `<dest>/<fontName>.ts`

Setting it alone doesn't enable the output; set `ts` too.

## Templates

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `cssTemplate`

<!--@include: @/_shared/options/css-template.md-->

- Flag: `--css-template <PATH>`
- Type: `string`
- Default: the built-in CSS template

### `htmlTemplate`

<!--@include: @/_shared/options/html-template.md-->

- Flag: `--html-template <PATH>`
- Type: `string`
- Default: the built-in HTML template

### `templateOptions`

<!--@include: @/_shared/options/template-options.md-->

- Flag: none, manifest only
- Type: `object`
- Default: `{}`

## Glyphs

### `codepoints`

<!--@include: @/_shared/options/codepoints.md-->

- Flag: none, manifest only
- Type: `Record<string, number>`
- Default: `{}`

JSON has no hexadecimal numbers, so write codepoints in decimal: `61697` is `0xF101`.

### `startCodepoint`

<!--@include: @/_shared/options/start-codepoint.md-->

- Flag: `--start-codepoint <HEX>`, such as `0xE000`
- Type: `number`
- Default: `0xF101`

The flag takes hexadecimal; the manifest takes a decimal number, such as `57344`.

### `ligature`

<!--@include: @/_shared/options/ligature.md-->

- Flag: `--ligature` / `--no-ligature`
- Type: `boolean`
- Default: `true`

### `fontHeight`

<!--@include: @/_shared/options/font-height.md-->

- Flag: `--font-height <N>`
- Type: `number`
- Default: the tallest icon's height

### `ascent`

<!--@include: @/_shared/options/ascent.md-->

- Flag: `--ascent <N>`
- Type: `number`

### `descent`

<!--@include: @/_shared/options/descent.md-->

- Flag: `--descent <N>`
- Type: `number`

### `normalize`

<!--@include: @/_shared/options/normalize.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `true`

### `fixedWidth`

<!--@include: @/_shared/options/fixed-width.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `centerHorizontally`

<!--@include: @/_shared/options/center-horizontally.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `centerVertically`

<!--@include: @/_shared/options/center-vertically.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `preserveAspectRatio`

<!--@include: @/_shared/options/preserve-aspect-ratio.md-->

- Flag: none, manifest only
- Type: `boolean`

### `round`

<!--@include: @/_shared/options/round.md-->

- Flag: none, manifest only
- Type: `number`

### `optimizeOutput`

<!--@include: @/_shared/options/optimize-output.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

## Color

### `colorGlyphs`

<!--@include: @/_shared/options/color-glyphs.md-->

- Flag: none, manifest only
- Type: `true | string[]`
- Default: omitted (single-color)

### `colorSlots`

<!--@include: @/_shared/options/color-slots.md-->

- Flag: none, manifest only
- Type: `Record<string, string>`
- Default: omitted

## Font metadata

### `fontStyle`

<!--@include: @/_shared/options/font-style.md-->

- Flag: none, manifest only
- Type: `string`

### `fontWeight`

<!--@include: @/_shared/options/font-weight.md-->

- Flag: none, manifest only
- Type: `string`

### `formatOptions`

<!--@include: @/_shared/options/format-options.md-->

- Flag: none, manifest only
- Type: `object`

The CLI doesn't fix `ttf.ts`, so each run records the current time in the font. Set it, for example to `0`, for output that is identical on every run, and for consistent timestamps with `cacheLocation`.

## Performance

### `cacheLocation`

<!--@include: @/_shared/options/cache-location.md-->

- Flag: `--cache-location <DIR>`
- Type: `string`
- Default: omitted (no persistent cache)

Remove the files the engine created with [`clear-cache`](/cli/usage#subcommands). Cached fonts are reused as-is, so only restore trusted caches; see [keep it between CI runs](/guide/features/caching#keep-it-between-ci-runs).

### `threads`

<!--@include: @/_shared/options/threads.md-->

- Flag: `--threads <N>`
- Type: `number` (a positive integer)
- Default: one per CPU core

The flag also works with `--config`, where it overrides the manifest's value, since the right number depends on the machine.
