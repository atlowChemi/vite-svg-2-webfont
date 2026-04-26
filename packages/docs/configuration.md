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

## `fontName`

- Type: `string`
- Description: Name of the generated font and base name of font files
- Default: `'iconfont'`

## `dest`

- Type: `string`
- Description: Directory for generated font files
- Default: `path.resolve(context, '..', 'artifacts')`
- Reference: [`@atlowchemi/webfont-generator#dest`](/webfont-generator/node#dest)

## `cssDest`

- Type: `string`
- Description: Output path for generated CSS
- Default: `path.join(dest, fontName + '.css')`
- Reference: [`@atlowchemi/webfont-generator#cssDest`](/webfont-generator/node#cssdest)

## `cssTemplate`

- Type: `string`
- Description: Path to a custom Handlebars CSS template
- Template context also includes:
    - `fontName`
    - `src`
    - `codepoints`
- Reference: [`@atlowchemi/webfont-generator#cssTemplate`](/webfont-generator/node#csstemplate) and [`templates`](/webfont-generator/node#templates)

## `cssContext`

- Type: `(context: CssContext) => void`
- Description: Hook for mutating the rendering context passed to the CSS template before the CSS file is generated. The `context` argument carries the named fields documented on [`CssContext`](/webfont-generator/node#csscontext) (`fontName`, `src`, `codepoints`) plus the [`baseSelector`](#baseselector) and [`classPrefix`](#classprefix) keys the plugin forwards to the underlying generator.
- Reference: [`@atlowchemi/webfont-generator#cssContext`](/webfont-generator/node#csscontext)

## `cssFontsUrl`

- Type: `string`
- Description: Fonts path used in the generated CSS file
- Default: value derived from `cssDest`

## `htmlDest`

- Type: `string`
- Description: Output path for generated HTML preview
- Default: `path.join(dest, fontName + '.html')`
- Reference: [`@atlowchemi/webfont-generator#htmlDest`](/webfont-generator/node#htmldest)

## `htmlTemplate`

- Type: `string`
- Description: Path to a custom Handlebars HTML template
- Template context also includes:
    - `fontName`
    - `styles`
    - `names`
- Reference: [`@atlowchemi/webfont-generator#htmlTemplate`](/webfont-generator/node#htmltemplate)

## `ligature`

- Type: `boolean`
- Description: Enable or disable ligatures
- Default: `true`
- Reference: [`@atlowchemi/webfont-generator#ligature`](/webfont-generator/node#ligature)

## `normalize`

- Type: `boolean`
- Description: Scale icons to the height of the tallest icon
- Default: `false`
- Reference: [`@atlowchemi/webfont-generator#normalize`](/webfont-generator/node#normalize)

## `round`

- Type: `number`
- Description: SVG path rounding precision
- Default: `10e12`
- Reference: [`@atlowchemi/webfont-generator#round`](/webfont-generator/node#round)

## `descent`

- Type: `number`
- Description: Font descent, useful when you want to tune baseline alignment manually
- Default: `0`
- Reference: [`@atlowchemi/webfont-generator#descent`](/webfont-generator/node#descent)

## `fixedWidth`

- Type: `boolean`
- Description: Create a monospace font based on the widest icon
- Default: `false`
- Reference: [`@atlowchemi/webfont-generator#fixedWidth`](/webfont-generator/node#fixedwidth)

## `fontHeight`

- Type: `number`
- Description: Explicit output font height
- Reference: [`@atlowchemi/webfont-generator#fontHeight`](/webfont-generator/node#fontheight)

## `centerHorizontally`

- Type: `boolean`
- Description: Center glyphs horizontally based on their bounds
- Default: `false`
- Reference: [`@atlowchemi/webfont-generator#centerHorizontally`](/webfont-generator/node#centerhorizontally)

## `centerVertically`

- Type: `boolean`
- Description: Center glyphs vertically based on their bounds
- Default: `false`
- Notes:
    - This option is a convenience alias for `formatOptions.svg.centerVertically`
    - If both `centerVertically` and `formatOptions.svg.centerVertically` are defined, `formatOptions.svg.centerVertically` takes precedence
    - Any other properties inside `formatOptions.svg` are preserved
- Reference: [`@atlowchemi/webfont-generator#centerVertically`](/webfont-generator/node#centervertically)

## `optimizeOutput`

- Type: `boolean`
- Description: Run an SVG path optimizer over each glyph before assembling the font, trading a small amount of build time for smaller output bytes
- Default: `false`
- Notes:
    - Available since v7 — exposes the underlying [`@atlowchemi/webfont-generator#optimizeOutput`](/webfont-generator/node#optimizeoutput) option through the plugin
    - Convenience alias for `formatOptions.svg.optimizeOutput`; the format-level option takes precedence when both are set

## `generateFiles`

- Type: `boolean | string | string[]`
- Description: Controls which generated files are written to disk during development
- Valid values:
    - `true` for all generated file types
    - `false` for no generated files
    - `'html'`
    - `'css'`
    - `'fonts'`
- Default: `false`

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
- Reference: [`@atlowchemi/webfont-generator#types`](/webfont-generator/node#types)

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
- Description: Conditionally skip preload injection for selected HTML entrypoints (See [`preloadFormats`](./configuration#preloadformats) for preload injection controls)
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
- Reference: [`@atlowchemi/webfont-generator#codepoints`](/webfont-generator/node#codepoints)

## `classPrefix`

- Type: `string`
- Description: Class prefix for generated icon classes
- Default: `'icon-'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/webfont-generator/node#templateoptions)

## `baseSelector`

- Type: `string`
- Description: Base selector to which the font styles are applied
- Default: `'.icon'`
- Reference: [`@atlowchemi/webfont-generator#templateOptions`](/webfont-generator/node#templateoptions)

## `formatOptions`

- Type: `FormatOptions`
- Description: Per-format options forwarded to the underlying [`@atlowchemi/webfont-generator`](/webfont-generator/). The shape is `{ svg?: SvgFormatOptions; ttf?: TtfFormatOptions; woff?: WoffFormatOptions }` — see the engine docs for the fields available on each.
- Reference: [`@atlowchemi/webfont-generator#formatOptions`](/webfont-generator/node#formatoptions)

## `moduleId`

- Type: `string`
- Description: Virtual module id used when importing generated plugin artifacts
- Default: `'vite-svg-2-webfont.css'`

With the default value, import the module like this:

```ts [main.ts]
import 'virtual:vite-svg-2-webfont.css';
```

## `inline`

- Type: `boolean`
- Description: Inline font assets inside CSS using base64 encoding
- Default: `false`

## `allowWriteFilesInBuild`

- Type: `boolean`
- Description: Allow HTML, CSS, and font files to be written during build
- Default: `false`
- Reference: [issue discussion](https://github.com/atlowChemi/vite-svg-2-webfont/issues/32#issuecomment-2203187501)
