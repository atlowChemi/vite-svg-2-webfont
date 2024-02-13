# vite-svg-2-webfont

[![npm](https://img.shields.io/npm/v/vite-svg-2-webfont.svg?style=flat-square)](https://www.npmjs.com/package/vite-svg-2-webfont)
[![npm](https://img.shields.io/npm/dm/vite-svg-2-webfont.svg?style=flat-square)](https://www.npmjs.com/package/vite-svg-2-webfont)
[![license](https://img.shields.io/github/license/ChemiAtlow/vite-svg-2-webfont.svg?style=flat-square)](https://github.com/ChemiAtlow/vite-svg-2-webfont/blob/master/LICENSE)
[![npm bundle size](https://img.shields.io/bundlephobia/minzip/vite-svg-2-webfont?style=flat-square)](https://img.shields.io/bundlephobia/minzip/vite-svg-2-webfont?style=flat-square)
[![node engine](https://img.shields.io/node/v/vite-svg-2-webfont?style=flat-square)](https://img.shields.io/node/v/vite-svg-2-webfont?style=flat-square)

A Vite Plugin that generates fonts from your SVG icons and allows you to use your icons in your HTML.

`vite-svg-2-webfont` uses the [`webfonts-generator`](https://github.com/vusion/webfonts-generator) package to create fonts in any format.
It also generates CSS files so that you can use your icons directly in your HTML, using CSS classes.

## Installation

#### NPM

```
npm i -D vite-svg-2-webfont
```

#### YARN

```
yarn add -D vite-svg-2-webfont
```

#### PNPM

```
pnpm add -D vite-svg-2-webfont
```

## Usage

Add the plugin to the `vite.config.ts` with the wanted setup, and import the virtual module, to inject the icons CSS font to the bundle.

### Vite config

Add the plugin to your vite configs plugin array.

```typescript
// vite.config.ts
import { resolve } from 'path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            context: resolve(__dirname, 'icons'),
        }),
    ],
});
```

### Import virtual module

Import the virtual module into the app

```typescript
// main.ts
import 'virtual:vite-svg-2-webfont.css';
```

### Add class-name to HTML element to use font

Use the font in templates with the icon font class and an icon class name.
The default font class name is `.icon` and can be overriden by passing the [`baseSelector`](#baseselector) configuration option.
Icon class names are derived from their `.svg` file name, and prefixed with the value of [`classPrefix`](#classprefix) which defaults to `icon-`.

In the below example, the `add` class would display the icon created from the file `{context}/add.svg`

```html
<i class="icon icon-add"></i>
```

## Configuration

The plugin has an API consisting of one required parameter and multiple optional parameters allowing to fully customize plugin setup.

### context

-   **required**
-   **type**: `string`
-   **description**: A path that resolves to a directory, in which a [glob pattern to find `svg` files will execute](#files). The SVG files will be used to generate the icon font.

### files

-   **type**: `string`
-   **description**: An array of globs, of the SVG files to add into the webfont, from within the [context](#context)
-   **default** `['*.svg']`

### fontName

-   **type**: `string`
-   **description**: Name of font and base name of font files.
-   **default** `'iconfont'`

### dest

-   **type**: `string`
-   **description**: Directory for generated font files.
-   **default** `path.resolve(context, '..', 'artifacts')`
-   See [webfonts-generator#dest](https://github.com/vusion/webfonts-generator#dest)

### cssDest

-   **type**: `string`
-   **description**: Path for generated CSS file.
-   **default** `path.join(dest, fontName + '.css')`
-   See [webfonts-generator#cssdest](https://github.com/vusion/webfonts-generator#cssdest)

### cssTemplate

-   **type**: `string`
-   **description**: Path of custom CSS template. Generator uses handlebars templates. Tht template receives options from `templateOptions` along with the following options:

    -   fontName
    -   src `string` – Value of the `src` property for `@font-face`.
    -   codepoints `object` - Codepoints of icons in hex format.

-   Paths of default templates are stored in the `webfontsGenerator.templates` object.
    -   `webfontsGenerator.templates.css` – Default CSS template path. It generates classes with names based on values from `options.templateOptions`.
    -   `webfontsGenerator.templates.scss` – Default SCSS template path. It generates mixin `webfont-icon` to add icon styles. It is safe to use multiple generated files with mixins together.
-   See [webfonts-generator#csstemplate](https://github.com/vusion/webfonts-generator#csstemplate)

### cssFontsUrl

-   **type**: `string`
-   **description**: Fonts path used in CSS file.
-   **default** [`cssDest`](#cssdest)

### htmlDest

-   **type**: `string`
-   **description**: Path for generated HTML file.
-   **default** `path.join(dest, fontName + '.html')`
-   See [webfonts-generator#htmldest](https://github.com/vusion/webfonts-generator#htmldest)

### htmlTemplate

-   **type**: `string`
-   **description**: HTML template path. Generator uses handlebars templates. Template receives options from `options.templateOptions` along with the following options:
    -   fontName
    -   styles `string` – Styles generated with default CSS template. (`cssFontsPath` is changed to relative path from `htmlDest` to `dest`)
    -   names `string[]` – Names of icons.
-   See [webfonts-generator#htmltemplate](https://github.com/vusion/webfonts-generator#htmltemplate)

### ligature

-   **type**: `boolean`
-   **description**: Enable or disable ligature function.
-   **default** `true`
-   See [webfonts-generator#ligature](https://github.com/vusion/webfonts-generator#ligature)

### normalize

-   **type**: `boolean`
-   **description**: Normalize icons by scaling them to the height of the highest icon.
-   **default** `false`
-   See [svgicons2svgfont#optionsnormalize](https://github.com/nfroidure/svgicons2svgfont#optionsnormalize)

### round

-   **type**: `number`
-   **description**: Setup SVG path rounding.
-   **default** `10e12`
-   See [svgicons2svgfont#optionsround](https://github.com/nfroidure/svgicons2svgfont#optionsround)

### descent

-   **type**: `number`
-   **description**: The font descent. It is useful to fix the font baseline yourself.
-   **default** `0`
-   See [svgicons2svgfont#optionsdescent](https://github.com/nfroidure/svgicons2svgfont#optionsdescent)

### fixedWidth

-   **type**: `boolean`
-   **description**: Creates a monospace font of the width of the largest input icon.
-   **default** `false`
-   See [svgicons2svgfont#optionsfixedwidth](https://github.com/nfroidure/svgicons2svgfont#optionsfixedwidth)

### fontHeight

-   **type**: `number`
-   **description**: The outputted font height (defaults to the height of the highest input icon).
-   See [svgicons2svgfont#optionsfontheight](https://github.com/nfroidure/svgicons2svgfont#optionsfontheight)

### centerHorizontally

-   **type**: `boolean`
-   **description**: Calculate the bounds of a glyph and center it horizontally.
-   **default** `false`
-   See [svgicons2svgfont#optionscenterhorizontally](https://github.com/nfroidure/svgicons2svgfont#optionscenterhorizontally)

### generateFiles

-   **type**: `boolean | string | string[]`
-   **description**: Sets the type of files to be saved to system during development.
-   **valid inputs**:
    -   `true` Generate all file types.
    -   `false` Generate no files.
    -   `'html'` - Generate a HTML file
    -   `'css'` - Generate CSS file
    -   `'fonts'` - Generate font files (based on the [types](#types) requested)
-   **default** `false`

### types

-   **type**: `string | string[]`
-   **description**: Font file types to generate. Possible values:
    -   `svg`
    -   `ttf`
    -   `woff`
    -   `woff2`
    -   `eot`
-   **default** `['eot', 'woff', 'woff2', 'ttf', 'svg']`
-   See [webfonts-generator#types](https://github.com/vusion/webfonts-generator#types)

### codepoints

-   **type**: `{ [key: string]: number }`
-   **description**: Specific code-points for certain icons. Icons without code-points will have code-points incremented from [`startCodepoint`](https://github.com/vusion/webfonts-generator#startcodepoint) skipping duplicates.
-   See [webfonts-generator#codepoints](https://github.com/vusion/webfonts-generator#codepoints)

### classPrefix

-   **type**: `string`
-   **description**: CSS class prefix for each of the generated icons.
-   **default** `'icon-'`
-   See [webfonts-generator#templateoptions](https://github.com/vusion/webfonts-generator#templateoptions)

### baseSelector

-   **type**: `string`
-   **description**: CSS base selector to which the font will be applied.
-   **default** `'.icon'`
-   See [webfonts-generator#templateoptions](https://github.com/vusion/webfonts-generator#templateoptions)

### formatOptions

-   **type**: `{ [format in 'svg' | 'ttf' | 'woff2' | 'woff' | 'eot']?: unknown };`
-   **description**: Specific per format arbitrary options to pass to the generator. Format and matching generator:
    -   svg - [svgicons2svgfont](https://github.com/nfroidure/svgicons2svgfont).
    -   ttf - [svg2ttf](https://github.com/fontello/svg2ttf).
    -   woff2 - [ttf2woff2](https://github.com/nfroidure/ttf2woff2).
    -   woff - [ttf2woff](https://github.com/fontello/ttf2woff).
    -   eot - [ttf2eot](https://github.com/fontello/ttf2eot).
-   See [webfonts-generator#formatoptions](https://github.com/vusion/webfonts-generator#formatoptions)

### moduleId

-   **type**: `string`
-   **description**: Virtual module id which is used by Vite to import the plugin artifacts. E.g. the default value is "vite-svg-2-webfont.css" so "virtual:vite-svg-2-webfont.css" should be imported.
-   **default** `'vite-svg-2-webfont.css'`
