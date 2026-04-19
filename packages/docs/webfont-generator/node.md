---
description: API reference for the @atlowchemi/webfont-generator npm package, including installation, options, result type, and templates.
---

# Node.js Usage

## Installation

::: code-group

```sh [npm]
npm install @atlowchemi/webfont-generator
```

```sh [pnpm]
pnpm add @atlowchemi/webfont-generator
```

```sh [yarn]
yarn add @atlowchemi/webfont-generator
```

```sh [bun]
bun add @atlowchemi/webfont-generator
```

:::

## Platform support

The package ships prebuilt native binaries for the following platforms:

| OS             | Architecture          | Target triple                   |
| -------------- | --------------------- | ------------------------------- |
| macOS          | x64                   | `x86_64-apple-darwin`           |
| macOS          | arm64 (Apple Silicon) | `aarch64-apple-darwin`          |
| Linux (glibc)  | x64                   | `x86_64-unknown-linux-gnu`      |
| Linux (glibc)  | arm64                 | `aarch64-unknown-linux-gnu`     |
| Linux (glibc)  | armv7                 | `armv7-unknown-linux-gnueabihf` |
| Linux (musl)   | x64                   | `x86_64-unknown-linux-musl`     |
| Linux (musl)   | arm64                 | `aarch64-unknown-linux-musl`    |
| Windows (MSVC) | x64                   | `x86_64-pc-windows-msvc`        |
| Windows (MSVC) | arm64                 | `aarch64-pc-windows-msvc`       |

## Basic usage

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    files: ['icons/add.svg', 'icons/remove.svg', 'icons/settings.svg'],
    dest: './dist/fonts',
    fontName: 'my-icons',
    types: ['woff2', 'woff'],
});

// Font data is available directly on the result
console.log(result.woff2); // Uint8Array
console.log(result.woff); // Uint8Array

// Generate CSS with default font URLs
const css = result.generateCss();

// Generate CSS with custom URLs
const cssCustom = result.generateCss({ woff2: '/fonts/icons.woff2' });
```

## Options reference

### `files`

- **Required**
- Type: `string[]`
- Description: Array of paths to SVG files to include in the font.

### `dest`

- **Required**
- Type: `string`
- Description: Output directory for generated font files.

### `fontName`

- Type: `string`
- Default: `'iconfont'`
- Description: Name of the generated font family. Also used as the base name for output files.

### `types`

- Type: `FontType[]`
- Default: `['eot', 'woff', 'woff2']`
- Description: Font formats to generate. Valid values: `'svg'`, `'ttf'`, `'eot'`, `'woff'`, `'woff2'`.

### `order`

- Type: `FontType[]`
- Default: `['eot', 'woff2', 'woff', 'ttf', 'svg']` (filtered to requested `types`)
- Description: Order of `@font-face` `src` entries in generated CSS. All values must also appear in `types`.

### `css`

- Type: `boolean`
- Default: `true`
- Description: Whether to generate a CSS file.

### `html`

- Type: `boolean`
- Default: `false`
- Description: Whether to generate an HTML preview file.

### `writeFiles`

- Type: `boolean`
- Default: `true`
- Description: Whether to write generated files to disk. Set to `false` for in-memory usage.

### `cssTemplate`

- Type: `string`
- Description: Path to a custom Handlebars template for CSS generation. The template receives the context described in [Templates](#templates).

### `htmlTemplate`

- Type: `string`
- Description: Path to a custom Handlebars template for HTML preview generation.

### `cssFontsUrl`

- Type: `string`
- Description: URL prefix for font files in the generated CSS. Defaults to the relative path from `cssDest` to `dest`.

### `cssDest`

- Type: `string`
- Default: `path.join(dest, fontName + '.css')`
- Description: Output path for the generated CSS file.

### `htmlDest`

- Type: `string`
- Default: `path.join(dest, fontName + '.html')`
- Description: Output path for the generated HTML file.

### `codepoints`

- Type: `Record<string, number>`
- Description: Explicit Unicode codepoints for specific glyphs, keyed by glyph name.

### `startCodepoint`

- Type: `number`
- Default: `0xF101`
- Description: Starting codepoint for auto-assigned glyphs.

### `fontHeight`

- Type: `number`
- Description: Explicit output font height (units per em).

### `ascent`

- Type: `number`
- Description: Font ascent value.

### `descent`

- Type: `number`
- Description: Font descent value.

### `normalize`

- Type: `boolean`
- Default: `true`
- Description: Scale icons to the height of the tallest icon.

### `fixedWidth`

- Type: `boolean`
- Description: Create a monospace font based on the widest icon.

### `centerHorizontally`

- Type: `boolean`
- Description: Center glyphs horizontally based on their bounding box.

### `centerVertically`

- Type: `boolean`
- Description: Center glyphs vertically based on their bounding box. This is a convenience alias for `formatOptions.svg.centerVertically`.

### `ligature`

- Type: `boolean`
- Default: `true`
- Description: Enable ligature support. When enabled, each glyph can be referenced by its name as a text ligature.

### `round`

- Type: `number`
- Description: SVG path coordinate rounding precision.

### `preserveAspectRatio`

- Type: `boolean`
- Description: Preserve the aspect ratio of SVG icons. This is a convenience alias for `formatOptions.svg.preserveAspectRatio`.

### `optimizeOutput`

- Type: `boolean`
- Description: Optimize SVG output paths. This is a convenience alias for `formatOptions.svg.optimizeOutput`.

### `fontStyle`

- Type: `string`
- Description: CSS `font-style` value for the generated `@font-face` rule.

### `fontWeight`

- Type: `string`
- Description: CSS `font-weight` value for the generated `@font-face` rule.

### `formatOptions`

- Type: `FormatOptions`
- Description: Per-format configuration object with keys `svg`, `ttf`, and `woff`.

::: details FormatOptions type definition

```ts
interface FormatOptions {
    svg?: SvgFormatOptions;
    ttf?: TtfFormatOptions;
    woff?: WoffFormatOptions;
}

interface SvgFormatOptions {
    centerVertically?: boolean;
    fontId?: string;
    metadata?: string;
    optimizeOutput?: boolean;
    preserveAspectRatio?: boolean;
}

interface TtfFormatOptions {
    copyright?: string;
    description?: string;
    ts?: number; // Unix timestamp for reproducible builds
    url?: string;
    version?: string;
}

interface WoffFormatOptions {
    metadata?: string; // WOFF metadata XML string
}
```

:::

### `cssContext`

- Type: `(context: Record<string, any>) => void`
- Description: Callback to mutate the Handlebars template context before CSS rendering. Receives the context object; modify it in-place.

### `htmlContext`

- Type: `(context: Record<string, any>) => void`
- Description: Callback to mutate the Handlebars template context before HTML rendering.

### `rename`

- Type: `(name: string) => string`
- Description: Custom function to derive glyph names from file paths. Receives the file path; must return the glyph name.

### `templateOptions`

- Type: `Record<string, any>`
- Description: Additional key-value pairs merged into the Handlebars template context. This is where `classPrefix` and `baseSelector` are typically set.

## Result type

`generateWebfonts()` returns a `Promise<GenerateWebfontsResult>` with font data and template methods.

### Font data properties

Each font format is available as a property on the result. Formats that were not requested return `null`.

| Property | Type                 | Description          |
| -------- | -------------------- | -------------------- |
| `svg`    | `string \| null`     | SVG font XML string  |
| `ttf`    | `Uint8Array \| null` | TrueType font binary |
| `eot`    | `Uint8Array \| null` | EOT font binary      |
| `woff`   | `Uint8Array \| null` | WOFF font binary     |
| `woff2`  | `Uint8Array \| null` | WOFF2 font binary    |

### `generateCss(urls?)`

- Type: `(urls?: Partial<Record<FontType, string>>) => string`
- Description: Returns the rendered CSS string. Pass `urls` to override the default font URLs in `@font-face src`.

### `generateHtml(urls?)`

- Type: `(urls?: Partial<Record<FontType, string>>) => string`
- Description: Returns the rendered HTML preview string. Pass `urls` to override font URLs in the embedded stylesheet.

## Templates

The package exports default Handlebars template paths via a subpath export:

```ts
import { templates } from '@atlowchemi/webfont-generator/templates';

console.log(templates.css); // absolute path to default CSS template
console.log(templates.scss); // absolute path to default SCSS template
console.log(templates.html); // absolute path to default HTML template
```

These paths can be passed to `cssTemplate` or `htmlTemplate` when you want to use the built-in templates as a starting point for customization.

The templates namespace is also available on the function itself:

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

console.log(generateWebfonts.templates.css);
```

## See also

- [Overview](./) -- architecture and design
- [Rust usage](./rust) -- crate API reference
- [CLI usage](./cli) -- command-line interface
