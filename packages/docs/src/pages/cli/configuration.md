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

<!--@include: @/includes/options/variants.md-->

- Flag: none, manifest only
- Type: `{ name: string; files: string[]; weight?: number; default?: boolean }[]`
- Default: omitted (a single-design font)

Each design's `files` takes files and directories like the top-level `files`. The same file may appear in several designs.

### `variantClassPrefix`

<!--@include: @/includes/options/variant-class-prefix.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `'icon--'`

### `missingGlyphs`

<!--@include: @/includes/options/missing-glyphs.md-->

- Flag: none, manifest only
- Type: `{ behavior: 'blank' | 'error' | 'fallback'; variant?: string }`
- Default: `{ behavior: 'blank' }`

## Output

### `dest`

<!--@include: @/includes/options/dest.md-->

- Flag: `-d, --dest <DEST>`
- Type: `string`
- Required

### `fontName`

<!--@include: @/includes/options/font-name.md-->

- Flag: `-n, --font-name <NAME>`
- Type: `string`
- Default: `'iconfont'`

### `types`

<!--@include: @/includes/options/types.md-->

- Flag: `-t, --types <TYPES>`, comma-separated, such as `woff2,woff`
- Type: `FontType[]`
- Default: `['eot', 'woff', 'woff2']`, or `['woff', 'woff2']` for families

### `order`

<!--@include: @/includes/options/order.md-->

- Flag: none, manifest only
- Type: `FontType[]`
- Default: `['eot', 'woff2', 'woff', 'ttf', 'svg']`, filtered to `types`

### `writeFiles`

<!--@include: @/includes/options/write-files.md-->

- Flag: `--write` / `--no-write`
- Type: `boolean`
- Default: `true`

With `--no-write`, the CLI generates everything and writes nothing: a dry run that still reports errors.

### `css`

<!--@include: @/includes/options/css.md-->

- Flag: `--css` / `--no-css`
- Type: `boolean`
- Default: `true`

### `cssDest`

<!--@include: @/includes/options/css-dest.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `<dest>/<fontName>.css`

### `cssFontsUrl`

<!--@include: @/includes/options/css-fonts-url.md-->

- Flag: `--css-fonts-url <URL>`
- Type: `string`
- Default: the relative path from `cssDest` to `dest`

### `html`

<!--@include: @/includes/options/html.md-->

- Flag: `--html` / `--no-html`
- Type: `boolean`
- Default: `false`

### `htmlDest`

<!--@include: @/includes/options/html-dest.md-->

- Flag: none, manifest only
- Type: `string`
- Default: `<dest>/<fontName>.html`

### `ts`

<!--@include: @/includes/options/ts.md-->

- Flag: `--ts` / `--no-ts`
- Type: `boolean`
- Default: `false`

### `tsDest`

<!--@include: @/includes/options/ts-dest.md-->

- Flag: `--ts-dest <PATH>`
- Type: `string`
- Default: `<dest>/<fontName>.ts`

Setting it alone doesn't enable the output; set `ts` too.

## Templates

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `cssTemplate`

<!--@include: @/includes/options/css-template.md-->

- Flag: `--css-template <PATH>`
- Type: `string`
- Default: the built-in CSS template

### `htmlTemplate`

<!--@include: @/includes/options/html-template.md-->

- Flag: `--html-template <PATH>`
- Type: `string`
- Default: the built-in HTML template

### `templateOptions`

<!--@include: @/includes/options/template-options.md-->

- Flag: none, manifest only
- Type: `object`
- Default: `{}`

## Glyphs

### `codepoints`

<!--@include: @/includes/options/codepoints.md-->

- Flag: none, manifest only
- Type: `Record<string, number>`
- Default: `{}`

JSON has no hexadecimal numbers, so write codepoints in decimal: `61697` is `0xF101`.

### `startCodepoint`

<!--@include: @/includes/options/start-codepoint.md-->

- Flag: `--start-codepoint <HEX>`, such as `0xE000`
- Type: `number`
- Default: `0xF101`

The flag takes hexadecimal; the manifest takes a decimal number, such as `57344`.

### `ligature`

<!--@include: @/includes/options/ligature.md-->

- Flag: `--ligature` / `--no-ligature`
- Type: `boolean`
- Default: `true`

### `fontHeight`

<!--@include: @/includes/options/font-height.md-->

- Flag: `--font-height <N>`
- Type: `number`
- Default: the tallest icon's height

### `ascent`

<!--@include: @/includes/options/ascent.md-->

- Flag: `--ascent <N>`
- Type: `number`

### `descent`

<!--@include: @/includes/options/descent.md-->

- Flag: `--descent <N>`
- Type: `number`

### `normalize`

<!--@include: @/includes/options/normalize.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `true`

### `fixedWidth`

<!--@include: @/includes/options/fixed-width.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `centerHorizontally`

<!--@include: @/includes/options/center-horizontally.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `centerVertically`

<!--@include: @/includes/options/center-vertically.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

### `preserveAspectRatio`

<!--@include: @/includes/options/preserve-aspect-ratio.md-->

- Flag: none, manifest only
- Type: `boolean`

### `round`

<!--@include: @/includes/options/round.md-->

- Flag: none, manifest only
- Type: `number`

### `optimizeOutput`

<!--@include: @/includes/options/optimize-output.md-->

- Flag: none, manifest only
- Type: `boolean`
- Default: `false`

## Color

### `colorGlyphs`

<!--@include: @/includes/options/color-glyphs.md-->

- Flag: none, manifest only
- Type: `true | string[]`
- Default: omitted (single-color)

### `colorSlots`

<!--@include: @/includes/options/color-slots.md-->

- Flag: none, manifest only
- Type: `Record<string, string>`
- Default: omitted

## Font metadata

### `fontStyle`

<!--@include: @/includes/options/font-style.md-->

- Flag: none, manifest only
- Type: `string`

### `fontWeight`

<!--@include: @/includes/options/font-weight.md-->

- Flag: none, manifest only
- Type: `string`

### `formatOptions`

<!--@include: @/includes/options/format-options.md-->

- Flag: none, manifest only
- Type: `object`

The CLI doesn't fix `ttf.ts`, so each run records the current time in the font. Set it, for example to `0`, for output that is identical on every run, and for consistent timestamps with `cacheLocation`.

## Performance

### `cacheLocation`

<!--@include: @/includes/options/cache-location.md-->

- Flag: `--cache-location <DIR>`
- Type: `string`
- Default: omitted (no persistent cache)

Remove the files the engine created with [`clear-cache`](/cli/usage#subcommands). Cached fonts are reused as-is, so only restore trusted caches; see [keep it between CI runs](/guide/features/caching#keep-it-between-ci-runs).

### `threads`

<!--@include: @/includes/options/threads.md-->

- Flag: `--threads <N>`
- Type: `number` (a positive integer)
- Default: one per CPU core

The flag also works with `--config`, where it overrides the manifest's value, since the right number depends on the machine.
