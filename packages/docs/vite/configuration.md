---
description: Reference for all vite-svg-2-webfont options, including file discovery, output paths, codepoints, preload behavior, and font generation controls.
---

# Vite Plugin Configuration

Options for the plugin, grouped by what they control. [`context`](#context) is required; everything else is optional.

## Input

### `context`

A path that resolves to a directory in which the glob pattern for matching SVG files runs. Matching SVG files are used to generate the icon font.

- **Required**
- Type: `string`

### `files`

Glob or array of globs for SVG files inside [`context`](#context).

- Type: `string | string[]`
- Default: `['*.svg']`
- Mutually exclusive with [`variants`](#variants).

### `variants`

<!--@include: @/_shared/options/variants.md-->

- Type: `IconPluginVariant[]`
- Default: omitted (ordinary single-design mode)

Each variant accepts:

| Field     | Type                 | Default                | Description                                                                                                                                                                                         |
| --------- | -------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`    | `string`             | Required               | Unique design name used by CSS modifiers.                                                                                                                                                           |
| `context` | `string`             | Top-level `context`    | Relative to the top-level context, or an absolute directory.                                                                                                                                        |
| `files`   | `string \| string[]` | `['*.svg']`            | Glob or globs within the variant context; parent traversal and absolute globs are rejected. Use an absolute context for external input directories. Matches are deduplicated and sorted per design. |
| `weight`  | `number`             | Automatically assigned | Explicit increasing weight from 1–1000; follows the generator's weight rules.                                                                                                                       |
| `default` | `boolean`            | `false`                | Exactly one design must be the default.                                                                                                                                                             |

Every design must resolve at least one file. Variant directories are watched recursively, including nested matching directories created during development. Multiple designs can reference the same physical SVG.

See [multi-weight usage](/vite/usage#multi-weight-icon-families) and [variants](/guide/features/variants).

### `variantClassPrefix`

<!--@include: @/_shared/options/variant-class-prefix.md-->

- Type: `string`
- Availability: variant mode only
- Default: Derived by the generator from `classPrefix` (normally `icon--`).

### `missingGlyphs`

<!--@include: @/_shared/options/missing-glyphs.md-->

- Type: `{ behavior: 'error' | 'blank' | 'fallback'; variant?: string }`
- Availability: variant mode only

## Output

Fonts reach the page through the virtual CSS module and Vite's asset pipeline. The destination options and `cssFontsUrl` only apply to files written directly with [`generateFiles`](#generatefiles).

### `fontName`

<!--@include: @/_shared/options/font-name.md-->

- Type: `string`
- Default: `'iconfont'`

### `types`

<!--@include: @/_shared/options/types.md-->

- Type: `string | string[]`
- Default: `['eot', 'woff', 'woff2', 'ttf', 'svg']`, or `['woff', 'woff2']` for families

### `inline`

Inline font assets inside CSS using base64 encoding.

- Type: `boolean`
- Default: `false`

### `generateFiles`

Controls which generated files are written directly to disk during development, and during builds when [`allowWriteFilesInBuild`](#allowwritefilesinbuild) is enabled.

- Type: `boolean | string | string[]`
- Valid values:
    - `true` for CSS, HTML and fonts, plus TS only when `tsDest` is explicitly supplied
    - `false` for no generated files
    - `'html'`
    - `'css'`
    - `'fonts'`
    - `'ts'`
- Default: `false`

These files use [`dest`](#dest), [`cssDest`](#cssdest), [`htmlDest`](#htmldest), and [`tsDest`](#tsdest). This option does not control Vite's bundled assets or virtual metadata: both virtual modules remain available when `generateFiles` is `false`.

### `allowWriteFilesInBuild`

Allow the generator to write the files selected by [`generateFiles`](#generatefiles) directly during a build.

- Type: `boolean`
- Default: `false`
- Reference: [issue discussion](https://github.com/atlowChemi/vite-svg-2-webfont/issues/32#issuecomment-2203187501)

With `generateFiles: true`, font files are written to [`dest`](#dest), and CSS/HTML use their configured destinations. These files are additional to any assets bundled by Vite through the virtual CSS import; enabling this option does not relocate Vite's copies. See [writing additional generator files during builds](/vite/usage#write-additional-generator-files-during-builds).

### `dest`

Directory for font files written directly by the generator when [`generateFiles`](#generatefiles) includes fonts.

- Type: `string`
- Default: `path.resolve(context, '..', 'artifacts')`
- Build behavior: Direct file output also requires [`allowWriteFilesInBuild`](#allowwritefilesinbuild). Fonts bundled through the virtual CSS import use Vite's output settings instead of `dest`; see [production font output](/vite/usage#configure-production-font-output).
- Reference: [`@atlowchemi/webfont-generator#dest`](/node/configuration#dest)

### `cssDest`

Output path for generated CSS.

- Type: `string`
- Default: `path.join(dest, fontName + '.css')`
- Scope: Applies to directly generated CSS files. Vite controls the output path of bundled CSS.
- Reference: [`@atlowchemi/webfont-generator#cssDest`](/node/configuration#cssdest)

### `htmlDest`

Output path for generated HTML preview.

- Type: `string`
- Default: `path.join(dest, fontName + '.html')`
- Reference: [`@atlowchemi/webfont-generator#htmlDest`](/node/configuration#htmldest)

### `tsDest`

Destination for fixed TypeScript metadata source. Relative paths resolve against [`dest`](#dest); directory paths have `<fontName>.ts` appended, like [`cssDest`](#cssdest).

- Type: `string`
- Default: `<dest>/<fontName>.ts`

Enable writing with [`generateFiles`](#generatefiles). With `generateFiles: true`, TS is written only if `tsDest` is explicit. Builds additionally require [`allowWriteFilesInBuild: true`](#allowwritefilesinbuild).

### `cssFontsUrl`

<!--@include: @/_shared/options/css-fonts-url.md-->

- Type: `string`
- Default: value derived from `cssDest`
- Build behavior: Does not override font URLs in the bundled virtual CSS. Vite resolves those URLs from its emitted assets and public `base` path.

## Virtual modules

### `moduleId`

Virtual module id used when importing generated plugin artifacts.

- Type: `string`
- Default: `'vite-svg-2-webfont.css'`

With the default value, import the module like this:

```ts [main.ts]
import 'virtual:vite-svg-2-webfont.css';
```

### `tsModuleId`

Unprefixed virtual metadata module ID, independent of `generateFiles` and `tsDest`.

- Type: `string`
- Default: `'vite-svg-2-webfont.ts'`

```ts
import { icons, iconBaseSelector, iconClassPrefix, type IconName } from 'virtual:vite-svg-2-webfont.ts';
```

The module exports a readonly literal tuple and exact icon-name union. Add `/// <reference types="vite-svg-2-webfont/virtuals" />` to your existing `vite-env.d.ts`, or include `vite-svg-2-webfont/virtuals` in `compilerOptions.types` in your application's `tsconfig.json`. Run Vite before the first standalone type-check after installing dependencies. No application-source declaration file is generated.

Declarations share the installed package instance: distinct IDs avoid overwrites but do not isolate apps. See the [component example](/vite/examples#type-safe-icon-component) for initialization, shared-installation, stale-entry, and write-failure details.

## Templates and class names

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `cssTemplate`

<!--@include: @/_shared/options/css-template.md-->

- Type: `string`
- Template context also includes:
    - `fontName`
    - `src`
    - `codepoints`

### `htmlTemplate`

<!--@include: @/_shared/options/html-template.md-->

- Type: `string`
- Template context also includes:
    - `fontName`
    - `styles`
    - `names`

### `cssContext`

Hook for mutating the rendering context passed to the CSS template before the CSS file is generated. The `context` argument carries the named fields documented on [`CssContext`](/node/configuration#csscontext) (`fontName`, `src`, `codepoints`) plus the [`baseSelector`](#baseselector) and [`classPrefix`](#classprefix) keys the plugin forwards to the underlying generator.

- Type: `(context: CssContext) => void`
- Variant configurations infer `CssContext<true>`, including named designs, default weight, and family rendering metadata. Context callbacks use full generation on watch updates rather than incremental regeneration.
- Reference: [`@atlowchemi/webfont-generator#cssContext`](/node/configuration#csscontext)

### `classPrefix`

Class prefix for generated icon classes.

- Type: `string`
- Default: `'icon-'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/node/configuration#templateoptions)

### `baseSelector`

Base selector to which the font styles are applied.

- Type: `string`
- Default: `'.icon'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/node/configuration#templateoptions)

## Glyphs

### `codepoints`

<!--@include: @/_shared/options/codepoints.md-->

- Type: `{ [key: string]: number }`

### `ligature`

<!--@include: @/_shared/options/ligature.md-->

- Type: `boolean`
- Default: `true`

### `fontHeight`

<!--@include: @/_shared/options/font-height.md-->

- Type: `number`
- Default: `1000` (the plugin's default; the generator alone defaults to the tallest icon's height)

### `descent`

<!--@include: @/_shared/options/descent.md-->

- Type: `number`
- Default: `0`

### `normalize`

<!--@include: @/_shared/options/normalize.md-->

- Type: `boolean`
- Default: `true` (the generator default; the plugin forwards the option only when you set it)

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
- Other properties inside `formatOptions.svg` are preserved.

### `round`

<!--@include: @/_shared/options/round.md-->

- Type: `number`
- Default: `10e12`

### `optimizeOutput`

<!--@include: @/_shared/options/optimize-output.md-->

- Type: `boolean`
- Default: `false`
- Available since v7.

## Color

### `colorGlyphs`

Keep the SVG colors of every icon (`true`) or of the listed icon names. Color output requires TTF, WOFF, or WOFF2; the plugin's default `types` include EOT and SVG fonts, so set `types` explicitly.

- Type: `true | string[]`
- Default: omitted (single-color)

```ts
viteSvgToWebfont({
    context: './src/icons',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
});
```

Paint-only edits update the font and virtual CSS during development, including inline fonts. Production builds emit color fonts through the existing asset pipeline. See [color glyphs](/guide/features/color-glyphs) for the selection rules, supported SVG paint, and browser support.

### `colorSlots`

<!--@include: @/_shared/options/color-slots.md-->

- Type: `Record<string, string>`
- Default: omitted

## Font metadata

### `formatOptions`

<!--@include: @/_shared/options/format-options.md-->

- Type: `FormatOptions`
- Note: `formatOptions.ttf.ts` is ignored. Like upstream `@vusion/webfonts-generator`, the generator always writes a fixed TTF timestamp, so builds are reproducible.

### `woff2CompressionQuality`

Brotli compression quality for WOFF2 output. Higher is smaller but slower to encode; every level decodes to an identical font.

- Type: `number` (`0`–`11`)
- Default: `undefined`
- Notes:
    - Convenience alias for `formatOptions.woff2.compressionQuality`; the format-level option takes precedence when both are set
    - When left unset, the plugin uses a faster quality (`10`) during development (`serve`) so font rebuilds on icon changes are quicker, while production builds use the engine default (`11`) for the smallest output. The dev-served font renders identically to production — only its bytes differ.
    - Set this to pin a single quality across both dev and build (e.g. `11` to make dev byte-for-byte match production)
- Reference: [`@atlowchemi/webfont-generator#formatOptions`](/node/configuration#formatoptions)

## Preloading

### `preloadFormats`

Font formats to preload in production HTML output.

- Type: `string | string[]`
- Notes:
    - Only affects build HTML output
    - Values outside `types` are ignored
    - No preload tags are injected when `inline` is `true`

```ts [vite.config.ts]
viteSvgToWebfont({
    context: './src/icons',
    types: ['woff2', 'ttf'],
    preloadFormats: ['woff2'], // [!code focus]
});
```

### `shouldProcessHtml`

Conditionally skip preload injection for selected HTML entrypoints (See [`preloadFormats`](/vite/configuration#preloadformats) for preload injection controls).

- Type: `(context: IndexHtmlTransformContext) => boolean`
- Notes:
    - Only affects preload injection
    - Returning `false` skips preload tag injection for the current HTML file

```ts [vite.config.ts]
import { resolve as pathResolve } from 'node:path';

viteSvgToWebfont({
    context: './src/icons',
    preloadFormats: ['woff2'], // [!code focus:2]
    shouldProcessHtml: context => context.filename === pathResolve(import.meta.dirname, 'src', 'index.html'),
});
```

## Performance

### `cache`

Reuse work across Vite runs. The plugin keeps its output cache, and the engine keeps a persistent cache of per-icon work and finished fonts, both inside Vite's [`cacheDir`](https://vite.dev/config/shared-options.html#cachedir). Output is the same with or without it. Set `false` to generate from scratch on every run; the plugin then writes nothing to `cacheDir`.

- Type: `boolean`
- Default: `true`

See [caching](/vite/how-it-works#caching) for what is stored and when each cache is used, and the [engine cache](/guide/features/caching) for what the engine reuses.
