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

Multi-variant generation returns one shared variable TTF/WOFF/WOFF2 per requested format through
the existing getters. SVG/EOT getters return `null`. Files use the existing `fontName.extension`
names. Writes are non-transactional and may leave a partial bundle on failure.

Use the existing `generateCss(urls?)` and `generateHtml(urls?)` methods with a flat shared URL map.
Omitting the map uses generated URLs; supplying one is a complete override, with omitted entries
empty. Variant SVG/EOT URL overrides are rejected. Regeneration methods reject variant results.

Custom-template contexts receive ordered `variants` entries (`name`, `weight`, `default`,
`className`, `selector`) and `variantClassPrefix`. These values describe resolved weights/classes,
not additional font resources. `defaultWeight` and `fontStyle` expose the resolved default weight
and style (default `normal`). Default CSS emits one exact-weight face per variant sharing the
modern URLs. Icon pseudo-elements use the default weight and `font-synthesis: none`; add a
modifier such as `icon--bold` alongside the glyph class to select a variant. A modifier alone
emits no glyph. CSS/HTML companion files are written when enabled; HTML shows one default grid.

The SCSS `webfont-icon($name)` mixin retains its signature and reads each family's default weight
and style from its five-item icon-map entry `(family, codepoint, weight, style, variantsMap)`.
`variantsMap` maps CSS-escaped modifier identifiers (without a leading dot) to numeric weights;
the mixin reads this fifth item to emit modifiers scoped to its caller's selector. Ordinary two-item entries
remain supported. Generated modifier classes select other variants.

Non-exact weights follow CSS font matching: for faces at 300, 400, and 700, requests for 100/350
select 300, 450/500 select 400, and 600/900 select 700. Generated pseudo-elements set their own
weight; inherited weights do not override it.

### TypeScript output inference

Literal `types` lists infer non-null getters for the requested formats and `null` for the rest.
Omitting `types` infers EOT/WOFF/WOFF2 for ordinary calls and WOFF/WOFF2 for variant calls.
Widened arrays, such as `FontType[]`, produce nullable getters for possible formats. Options
variables typed as `GenerateWebfontsOptions` (the ordinary/variant union) are accepted with
conservative nullable getters.

`GenerateWebfontsResult<Possible, Guaranteed>` describes these two sets of formats; the second
parameter defaults to the first for explicitly known outputs. Async regeneration preserves both.

`CssContext` and `HtmlContext` include optional `variants: TemplateVariant[]`,
`variantClassPrefix: string`, `defaultWeight: number`, and `fontStyle: string`. These fields are
provided in variant mode and absent by default in ordinary mode. `TemplateVariant` contains
`name: string`, `weight: number`, `default: boolean`, `className: string`, and `selector: string`.
The selector is an escaped CSS identifier without a leading dot.

### `files`

- **Required for ordinary generation**
- Type: `string[]`
- Description: Array of paths to SVG files to include in the font.

Use a non-empty array for ordinary generation. Omit this field when `variants` is provided.

### `variants`

- **Required for multi-variant input**
- Type: `FontVariant[]`
- Description: Ordered SVG designs for one logical icon family. Requires at least two uniquely
  named variants, non-empty `files` in each variant, and exactly one `default: true`. Explicit
  `weight` values are ordered anchors from 1 through 1000. An automatic default resolves to 400;
  other automatic values resolve outward in steps of 100, or evenly when an anchor interval is too
  crowded. Every final weight is unique and strictly follows variant order.

Variant names produce CSS modifier classes with CSSOM-escaped selectors. Names do not form output
filenames; all variants share one resource per requested modern format.

### `variantClassPrefix`

- Type: `string`
- Default: `'icon--'` in variant mode
- Description: Prefix for generated variant modifier classes. Whitespace, NUL, and the legacy
  `templateOptions.variantClassPrefix` location are rejected.

### `missingGlyphs`

- Type: `{ behavior: 'blank' | 'error' | 'fallback'; variant?: string }`
- Default: `{ behavior: 'blank' }` in variant mode
- Description: Family-wide policy for glyphs missing from a variant. `fallback` requires the name
  of an existing variant that contains every logical glyph in the family; other behaviors reject
  `variant`. Import `MissingGlyphBehavior` from the package for the enum values.

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
- Default: `['eot', 'woff', 'woff2']` for ordinary input; `['woff', 'woff2']` for variants
- Description: Font formats to generate. Ordinary input accepts `'svg'`, `'ttf'`, `'eot'`, `'woff'`, `'woff2'`. Variant input accepts only `'ttf'`, `'woff'`, and `'woff2'`.

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

### `incremental`

- Type: `boolean`
- Default: `false`
- Description: Retain parsed glyph data on the result so [`regenerateAsync()`](#regenerateasyncfiles-changes) or [`regenerate()`](#regeneratefiles-changes) can rebuild after file changes without re-parsing the glyphs that didn't change. Enable for watch/dev; leave it off for one-shot builds so the parsed geometry isn't held in memory.

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
- Description: Per-format configuration object with keys `svg`, `ttf`, `woff`, and `woff2`.

::: details FormatOptions type definition

```ts
interface FormatOptions {
    svg?: SvgFormatOptions;
    ttf?: TtfFormatOptions;
    woff?: WoffFormatOptions;
    woff2?: Woff2FormatOptions;
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

interface Woff2FormatOptions {
    // Brotli compression quality, 0 (fastest, largest) to 11 (slowest, smallest).
    // Tunes compression effort only — never changes glyph fidelity. Defaults to 11
    // (smallest output); lower it (e.g. to 10) for faster encoding at a marginal size
    // cost. Must be between 0 and 11; other values are rejected.
    compressionQuality?: number;
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

### `regenerate(files, changes?)`

- Type: `(files: string[], changes?: GlyphChangeEntry[] | null) => void`
- Requires: the result was produced with [`incremental: true`](#incremental) (throws otherwise).
- Description: Rebuilds every requested font format after file changes, reusing cached geometry for the glyphs that didn't change (and reusing the rendered CSS/HTML when the glyph names and codepoints are unchanged). `files` is the complete file set after the change, in the order a fresh build would use (e.g. your glob result); the rebuilt glyphs are ordered to match it, so the result is byte-identical to a fresh `generateWebfonts()` of that set — additions included. Any file omitted from `files` is dropped; added/changed files named in `changes` are read from disk and re-parsed. Omit `changes` or pass `null` to re-read/hash every current file and infer added/changed/removed paths automatically. Outputs are refreshed in memory, and — when the result was created with [`writeFiles: true`](#writefiles) — refreshed fonts are written to disk too, while unchanged CSS/HTML companion files are skipped. Results generated with `cssContext` or `htmlContext` callbacks cannot be regenerated because those JavaScript callbacks cannot be re-run by the synchronous method. Intended for dev/watch rebuilds.

```ts
let files = ['/icons/add.svg', '/icons/search.svg'];
const result = await generateWebfonts({ files, dest, incremental: true });
// ...on a watch event:
result.regenerate(files, [{ path: '/icons/add.svg', changeType: 'changed' }]);
// ...or when watcher hints are unavailable/untrusted:
result.regenerate(files);
result.woff2; // refreshed bytes
```

Each entry is a `GlyphChangeEntry`:

```ts
interface GlyphChangeEntry {
    path: string;
    changeType: 'added' | 'changed' | 'removed';
    // Resolved glyph name if you apply a custom rename. Added files default to
    // the file stem; changed files default to the current name; removed files ignore it.
    name?: string;
}
```

### `regenerateAsync(files, changes?)`

- Type: `(files: string[], changes?: GlyphChangeEntry[] | null) => Promise<GenerateWebfontsResult>`
- Requires: the result was produced with [`incremental: true`](#incremental) (rejects otherwise).
- Description: Performs the same rebuild as [`regenerate()`](#regeneratefiles-changes) off the Node.js event loop and resolves with a replacement result. The receiver remains readable and unchanged while the rebuild runs and after failure. Assign the replacement before starting another rebuild; overlapping calls from the same result lineage reject. In-memory state is replaced only on success, but writes made with [`writeFiles: true`](#writefiles) are not transactional.

```ts
let files = ['/icons/add.svg', '/icons/search.svg'];
let result = await generateWebfonts({ files, dest, incremental: true });
result = await result.regenerateAsync(files, [{ path: '/icons/add.svg', changeType: 'changed' }]);
```

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
