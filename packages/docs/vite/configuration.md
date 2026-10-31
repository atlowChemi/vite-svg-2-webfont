---
description: Reference for all vite-svg-2-webfont options, including file discovery, output paths, codepoints, preload behavior, and font generation controls.
---

# Configuration

The plugin API consists of one required option and multiple optional options for controlling file discovery, font generation, CSS generation, output paths, and build-time behavior.

## `context`

- **Required**
- Type: `string`
- Description: A path that resolves to a directory in which the glob pattern for matching SVG files runs. Matching SVG files are used to generate the icon font.

## `files`

- Type: `string | string[]`
- Description: Glob or array of globs for SVG files inside [`context`](#context).
- Default: `['*.svg']`
- Mutually exclusive with [`variants`](#variants).

## `variants`

- Type: `IconPluginVariant[]`
- Default: omitted (ordinary single-design mode)
- Description: Ordered SVG designs forming one shared font per format. Match SVG filenames across designs to identify the same logical icon. Cannot be combined with top-level `files`.

Each variant accepts:

| Field     | Type                 | Default                | Description                                                                                                                                                                                         |
| --------- | -------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`    | `string`             | Required               | Unique design name used by CSS modifiers.                                                                                                                                                           |
| `context` | `string`             | Top-level `context`    | Relative to the top-level context, or an absolute directory.                                                                                                                                        |
| `files`   | `string \| string[]` | `['*.svg']`            | Glob or globs within the variant context; parent traversal and absolute globs are rejected. Use an absolute context for external input directories. Matches are deduplicated and sorted per design. |
| `weight`  | `number`             | Automatically assigned | Explicit increasing weight from 1–1000; follows the generator's weight rules.                                                                                                                       |
| `default` | `boolean`            | `false`                | Exactly one design must be the default.                                                                                                                                                             |

Every design must resolve at least one file. Variant directories are watched recursively, including nested matching directories created during development. Multiple designs can reference the same physical SVG.

See [multi-weight usage](/vite/usage#multi-weight-icon-families) and the [generator's variant reference](/node/#multi-variant-fonts).

## `missingGlyphs`

- Type: `{ behavior: 'error' | 'blank' | 'fallback'; variant?: string }`
- Availability: variant mode only
- Description: Generator policy for icons missing from a design. See the [generator reference](/node/#missingglyphs) for defaults and fallback rules.

## `variantClassPrefix`

- Type: `string`
- Availability: variant mode only
- Default: Derived by the generator from `classPrefix` (normally `icon--`).
- Description: Prefix for design modifier classes, such as `icon--bold`. A modifier accompanies the base and icon classes; it does not emit an icon by itself.

## `colorSlots`

- Type: `Record<string, string>`
- Default: omitted
- Description: Reserve named palette entries for CSS theme overrides, for example `{ secondary: '#ff9900' }`. Nonempty slots require active `colorGlyphs`. The CSS context exposes a name-to-index `colorSlots` map. See [validation and ordering rules](/node/#colorslots) and the [live light/dark example](/vite/examples#theme-colors-in-one-glyph).

## `colorGlyphs`

- Type: `true | string[]`
- Default: omitted (monochrome)

By default, each SVG icon is flattened into a monochrome glyph. Set `colorGlyphs: true` to preserve solid SVG colors for all icons, or use logical glyph names such as `['logo']` to select icons. An empty array keeps monochrome output. `false`, `null`, and unknown names are rejected.

Color output requires TTF, WOFF, or WOFF2. Set `types` explicitly for ordinary fonts because the plugin defaults include EOT and SVG-font output. Variant defaults are already compatible. Selection applies across all variants, including fallback designs.

```ts
viteSvgToWebfont({
    context: './src/icons',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
});
```

Paint-only edits update the font and virtual CSS during development, including inline fonts. Production builds emit color fonts through the existing asset pipeline. Removing the last source of a selected name fails validation; the dev server retains the previous font until a valid update succeeds.

See the [generator's color guidance](/guide/engine#color-glyphs) for supported SVG paint, monochrome fallback, and browser limitations.

## `fontName`

- Type: `string`
- Description: Name of the generated font and base name of font files
- Default: `'iconfont'`

## `dest`

- Type: `string`
- Description: Directory for font files written directly by the generator when [`generateFiles`](#generatefiles) includes fonts
- Default: `path.resolve(context, '..', 'artifacts')`
- Build behavior: Direct file output also requires [`allowWriteFilesInBuild`](#allowwritefilesinbuild). Fonts bundled through the virtual CSS import use Vite's output settings instead of `dest`; see [production font output](/vite/usage#configure-production-font-output).
- Reference: [`@atlowchemi/webfont-generator#dest`](/node/#dest)

## `cssDest`

- Type: `string`
- Description: Output path for generated CSS
- Default: `path.join(dest, fontName + '.css')`
- Scope: Applies to directly generated CSS files. Vite controls the output path of bundled CSS.
- Reference: [`@atlowchemi/webfont-generator#cssDest`](/node/#cssdest)

## `cssTemplate`

- Type: `string`
- Description: Path to a custom Handlebars CSS template
- Template context also includes:
    - `fontName`
    - `src`
    - `codepoints`
- Reference: [`@atlowchemi/webfont-generator#cssTemplate`](/node/#csstemplate) and [`templates`](/node/#templates)

## `cssContext`

- Type: `(context: CssContext) => void`
- Variant configurations infer `CssContext<true>`, including named designs, default weight, and family rendering metadata. Context callbacks use full generation on watch updates rather than incremental regeneration.
- Description: Hook for mutating the rendering context passed to the CSS template before the CSS file is generated. The `context` argument carries the named fields documented on [`CssContext`](/node/#csscontext) (`fontName`, `src`, `codepoints`) plus the [`baseSelector`](#baseselector) and [`classPrefix`](#classprefix) keys the plugin forwards to the underlying generator.
- Reference: [`@atlowchemi/webfont-generator#cssContext`](/node/#csscontext)

## `cssFontsUrl`

- Type: `string`
- Description: Fonts path used in the generated CSS file
- Default: value derived from `cssDest`
- Build behavior: Does not override font URLs in the bundled virtual CSS. Vite resolves those URLs from its emitted assets and public `base` path.

## `htmlDest`

- Type: `string`
- Description: Output path for generated HTML preview
- Default: `path.join(dest, fontName + '.html')`
- Reference: [`@atlowchemi/webfont-generator#htmlDest`](/node/#htmldest)

## `htmlTemplate`

- Type: `string`
- Description: Path to a custom Handlebars HTML template
- Template context also includes:
    - `fontName`
    - `styles`
    - `names`
- Reference: [`@atlowchemi/webfont-generator#htmlTemplate`](/node/#htmltemplate)

## `ligature`

- Type: `boolean`
- Description: Enable or disable ligatures
- Default: `true`
- Reference: [`@atlowchemi/webfont-generator#ligature`](/node/#ligature)

## `normalize`

- Type: `boolean`
- Description: Scale icons to the height of the tallest icon
- Default: `true` (the generator default; the plugin forwards the option only when you set it)
- Reference: [`@atlowchemi/webfont-generator#normalize`](/node/#normalize)

## `round`

- Type: `number`
- Description: SVG path rounding precision
- Default: `10e12`
- Reference: [`@atlowchemi/webfont-generator#round`](/node/#round)

## `descent`

- Type: `number`
- Description: Font descent, useful when you want to tune baseline alignment manually
- Default: `0`
- Reference: [`@atlowchemi/webfont-generator#descent`](/node/#descent)

## `fixedWidth`

- Type: `boolean`
- Description: Create a monospace font based on the widest icon
- Default: `false`
- Reference: [`@atlowchemi/webfont-generator#fixedWidth`](/node/#fixedwidth)

## `fontHeight`

- Type: `number`
- Description: Explicit output font height
- Default: `1000` (the plugin's default; the generator alone defaults to the tallest icon's height)
- Reference: [`@atlowchemi/webfont-generator#fontHeight`](/node/#fontheight)

## `centerHorizontally`

- Type: `boolean`
- Description: Center glyphs horizontally based on their bounds
- Default: `false`
- Reference: [`@atlowchemi/webfont-generator#centerHorizontally`](/node/#centerhorizontally)

## `centerVertically`

- Type: `boolean`
- Description: Center glyphs vertically based on their bounds
- Default: `false`
- Notes:
    - This option is a convenience alias for `formatOptions.svg.centerVertically`
    - If both `centerVertically` and `formatOptions.svg.centerVertically` are defined, `formatOptions.svg.centerVertically` takes precedence
    - Any other properties inside `formatOptions.svg` are preserved
- Reference: [`@atlowchemi/webfont-generator#centerVertically`](/node/#centervertically)

## `optimizeOutput`

- Type: `boolean`
- Description: Run an SVG path optimizer over each glyph before assembling the font
- Default: `false`
- Notes:
    - Available since v7 — exposes the underlying [`@atlowchemi/webfont-generator#optimizeOutput`](/node/#optimizeoutput) option through the plugin
    - Convenience alias for `formatOptions.svg.optimizeOutput`; the format-level option takes precedence when both are set
    - Optimization may reduce SVG path data, but does not guarantee smaller binary font output; compare the generated sizes for your input set

## `woff2CompressionQuality`

- Type: `number` (`0`–`11`)
- Description: Brotli compression quality for WOFF2 output. Higher is smaller but slower to encode; every level decodes to an identical font.
- Default: `undefined`
- Notes:
    - Convenience alias for `formatOptions.woff2.compressionQuality`; the format-level option takes precedence when both are set
    - When left unset, the plugin uses a faster quality (`10`) during development (`serve`) so font rebuilds on icon changes are quicker, while production builds use the engine default (`11`) for the smallest output. The dev-served font renders identically to production — only its bytes differ.
    - Set this to pin a single quality across both dev and build (e.g. `11` to make dev byte-for-byte match production)
- Reference: [`@atlowchemi/webfont-generator#formatOptions`](/node/#formatoptions)

## `generateFiles`

- Type: `boolean | string | string[]`
- Description: Controls which generated files are written directly to disk during development, and during builds when [`allowWriteFilesInBuild`](#allowwritefilesinbuild) is enabled
- Valid values:
    - `true` for CSS, HTML and fonts, plus TS only when `tsDest` is explicitly supplied
    - `false` for no generated files
    - `'html'`
    - `'css'`
    - `'fonts'`
    - `'ts'`
- Default: `false`

These files use [`dest`](#dest), [`cssDest`](#cssdest), [`htmlDest`](#htmldest), and [`tsDest`](#tsdest). This option does not control Vite's bundled assets or virtual metadata: both virtual modules remain available when `generateFiles` is `false`.

## `tsDest`

- Type: `string`
- Default: `<dest>/<fontName>.ts`
- Description: Destination for fixed TypeScript metadata source. Relative paths resolve against [`dest`](#dest); directory paths have `<fontName>.ts` appended, like [`cssDest`](#cssdest).

Enable writing with [`generateFiles`](#generatefiles). With `generateFiles: true`, TS is written only if `tsDest` is explicit. Builds additionally require [`allowWriteFilesInBuild: true`](#allowwritefilesinbuild).

## `types`

- Type: `string | string[]`
- Description: Font file types to generate
- Supported values:
    - `svg`
    - `ttf`
    - `woff`
    - `woff2`
    - `eot`
- Default: `['eot', 'woff', 'woff2', 'ttf', 'svg']`
- Variant mode: only `ttf`, `woff`, and `woff2` are supported; default is `['woff', 'woff2']`.
- Reference: [`@atlowchemi/webfont-generator#types`](/node/#types)

## `preloadFormats`

- Type: `string | string[]`
- Description: Font formats to preload in production HTML output
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

## `shouldProcessHtml`

- Type: `(context: IndexHtmlTransformContext) => boolean`
- Description: Conditionally skip preload injection for selected HTML entrypoints (See [`preloadFormats`](/vite/configuration#preloadformats) for preload injection controls)
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

## `codepoints`

- Type: `{ [key: string]: number }`
- Description: Explicit codepoints for selected icons. Icons without assigned codepoints continue from the generator start codepoint while avoiding duplicates.
- Reference: [`@atlowchemi/webfont-generator#codepoints`](/node/#codepoints)

## `classPrefix`

- Type: `string`
- Description: Class prefix for generated icon classes
- Default: `'icon-'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/node/#templateoptions)

## `baseSelector`

- Type: `string`
- Description: Base selector to which the font styles are applied
- Default: `'.icon'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/node/#templateoptions)

## `formatOptions`

- Type: `FormatOptions`
- Description: Per-format options forwarded to the underlying [`@atlowchemi/webfont-generator`](/guide/engine). The shape is `{ svg?: SvgFormatOptions; ttf?: TtfFormatOptions; woff?: WoffFormatOptions; woff2?: Woff2FormatOptions }` — see the engine docs for the fields available on each.
- Reference: [`@atlowchemi/webfont-generator#formatOptions`](/node/#formatoptions)
- Note: `formatOptions.ttf.ts` is ignored. Like upstream `@vusion/webfonts-generator`, the generator always writes a fixed TTF timestamp, so builds are reproducible.

## `moduleId`

- Type: `string`
- Description: Virtual module id used when importing generated plugin artifacts
- Default: `'vite-svg-2-webfont.css'`

With the default value, import the module like this:

```ts [main.ts]
import 'virtual:vite-svg-2-webfont.css';
```

## `tsModuleId`

- Type: `string`
- Default: `'vite-svg-2-webfont.ts'`
- Description: Unprefixed virtual metadata module ID, independent of `generateFiles` and `tsDest`.

```ts
import { icons, iconBaseSelector, iconClassPrefix, type IconName } from 'virtual:vite-svg-2-webfont.ts';
```

The module exports a readonly literal tuple and exact icon-name union. Add `/// <reference types="vite-svg-2-webfont/virtuals" />` to your existing `vite-env.d.ts`, or include `vite-svg-2-webfont/virtuals` in `compilerOptions.types` in your application's `tsconfig.json`. Run Vite before the first standalone type-check after installing dependencies. No application-source declaration file is generated.

Declarations share the installed package instance: distinct IDs avoid overwrites but do not isolate apps. See the [component example](/vite/examples#type-safe-icon-component) for initialization, shared-installation, stale-entry, and write-failure details.

## `inline`

- Type: `boolean`
- Description: Inline font assets inside CSS using base64 encoding
- Default: `false`

## `allowWriteFilesInBuild`

- Type: `boolean`
- Description: Allow the generator to write the files selected by [`generateFiles`](#generatefiles) directly during a build
- Default: `false`
- Reference: [issue discussion](https://github.com/atlowChemi/vite-svg-2-webfont/issues/32#issuecomment-2203187501)

With `generateFiles: true`, font files are written to [`dest`](#dest), and CSS/HTML use their configured destinations. These files are additional to any assets bundled by Vite through the virtual CSS import; enabling this option does not relocate Vite's copies. See [writing additional generator files during builds](/vite/usage#write-additional-generator-files-during-builds).

## `cache`

- Type: `boolean`
- Description: Reuse work across Vite runs. The plugin keeps its output cache, and the engine keeps a persistent cache of per-icon work and finished fonts, both inside Vite's [`cacheDir`](https://vite.dev/config/shared-options.html#cachedir). Output is the same with or without it. Set `false` to generate from scratch on every run; the plugin then writes nothing to `cacheDir`.
- Default: `true`

See [caching](/vite/how-it-works#caching) for what is stored and when each cache is used.
