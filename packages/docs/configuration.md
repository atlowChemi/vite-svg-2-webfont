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
- Reference: [`webfonts-generator#dest`](https://github.com/vusion/webfonts-generator#dest)

## `cssDest`

- Type: `string`
- Description: Output path for generated CSS
- Default: `path.join(dest, fontName + '.css')`
- Reference: [`webfonts-generator#cssdest`](https://github.com/vusion/webfonts-generator#cssdest)

## `cssTemplate`

- Type: `string`
- Description: Path to a custom Handlebars CSS template
- Template context also includes:
    - `fontName`
    - `src`
    - `codepoints`
- References:
    - [`webfontsGenerator.templates.css`](https://github.com/vusion/webfonts-generator/blob/98cdca62a7266323d0c89d68b3787b531a46fe61/templates/css.hbs)
    - [`webfontsGenerator.templates.scss`](https://github.com/vusion/webfonts-generator/blob/98cdca62a7266323d0c89d68b3787b531a46fe61/templates/scss.hbs)
    - [`webfonts-generator#cssTemplate`](https://github.com/vusion/webfonts-generator#csstemplate)

## `cssContext`

- Reference: [`webfonts-generator#cssContext`](https://github.com/vusion/webfonts-generator#cssContext)

## `cssFontsUrl`

- Type: `string`
- Description: Fonts path used in the generated CSS file
- Default: value derived from `cssDest`

## `htmlDest`

- Type: `string`
- Description: Output path for generated HTML preview
- Default: `path.join(dest, fontName + '.html')`
- Reference: [`webfonts-generator#htmlDest`](https://github.com/vusion/webfonts-generator#htmldest)

## `htmlTemplate`

- Type: `string`
- Description: Path to a custom Handlebars HTML template
- Template context also includes:
    - `fontName`
    - `styles`
    - `names`
- Reference: [`webfonts-generator#htmlTemplate`](https://github.com/vusion/webfonts-generator#htmltemplate)

## `ligature`

- Type: `boolean`
- Description: Enable or disable ligatures
- Default: `true`
- Reference: [`webfonts-generator#ligature`](https://github.com/vusion/webfonts-generator#ligature)

## `normalize`

- Type: `boolean`
- Description: Scale icons to the height of the tallest icon
- Default: `false`
- Reference: [`svgicons2svgfont#normalize`](https://github.com/nfroidure/svgicons2svgfont#optionsnormalize)

## `round`

- Type: `number`
- Description: SVG path rounding precision
- Default: `10e12`
- Reference: [`svgicons2svgfont#round`](https://github.com/nfroidure/svgicons2svgfont#optionsround)

## `descent`

- Type: `number`
- Description: Font descent, useful when you want to tune baseline alignment manually
- Default: `0`
- Reference: [`svgicons2svgfont#descent`](https://github.com/nfroidure/svgicons2svgfont#optionsdescent)

## `fixedWidth`

- Type: `boolean`
- Description: Create a monospace font based on the widest icon
- Default: `false`
- Reference: [`svgicons2svgfont#fixedWidth`](https://github.com/nfroidure/svgicons2svgfont#optionsfixedwidth)

## `fontHeight`

- Type: `number`
- Description: Explicit output font height
- Reference: [`svgicons2svgfont#fontHeight`](https://github.com/nfroidure/svgicons2svgfont#optionsfontheight)

## `centerHorizontally`

- Type: `boolean`
- Description: Center glyphs horizontally based on their bounds
- Default: `false`
- Reference: [`svgicons2svgfont#centerHorizontally`](https://github.com/nfroidure/svgicons2svgfont#optionscenterhorizontally)

## `centerVertically`

- Type: `boolean`
- Description: Center glyphs vertically based on their bounds
- Default: `false`
- Notes:
    - This option is a convenience alias for `formatOptions.svg.centerVertically`
    - If both `centerVertically` and `formatOptions.svg.centerVertically` are defined, `formatOptions.svg.centerVertically` takes precedence
    - Any other properties inside `formatOptions.svg` are preserved
- Reference: [`svgicons2svgfont#centerVertically`](https://github.com/nfroidure/svgicons2svgfont#optionscentervertically)

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
- Reference: [`webfonts-generator#types`](https://github.com/vusion/webfonts-generator#types)

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
    shouldProcessHtml: context => context.filename === pathResolve(__dirname, 'src', 'index.html'),
});
```

## `codepoints`

- Type: `{ [key: string]: number }`
- Description: Explicit codepoints for selected icons. Icons without assigned codepoints continue from the generator start codepoint while avoiding duplicates.
- Reference: [`webfonts-generator#codepoints`](https://github.com/vusion/webfonts-generator#codepoints)

## `classPrefix`

- Type: `string`
- Description: Class prefix for generated icon classes
- Default: `'icon-'`
- Reference: [`webfonts-generator#templateOptions`](https://github.com/vusion/webfonts-generator#templateoptions)

## `baseSelector`

- Type: `string`
- Description: Base selector to which the font styles are applied
- Default: `'.icon'`
- Reference: [`webfonts-generator#templateOptions`](https://github.com/vusion/webfonts-generator#templateoptions)

## `formatOptions`

- Type: `{ [format in 'svg' | 'ttf' | 'woff2' | 'woff' | 'eot']?: unknown }`
- Description: Per-format options passed to the corresponding generator
- Format mapping:
    - `svg`: `svgicons2svgfont`
    - `ttf`: `svg2ttf`
    - `woff2`: `ttf2woff2`
    - `woff`: `ttf2woff`
    - `eot`: `ttf2eot`
- Reference: [`webfonts-generator#formatOptions`](https://github.com/vusion/webfonts-generator#formatoptions)

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
