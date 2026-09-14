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

## Multi-variant fonts

A multi-variant family groups several designs of the same icons, such as light and bold,
into one font file per output format. Match SVG filenames across designs so that `add.svg`
refers to the same icon in each variant. You can then choose a design with a CSS modifier class.

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    dest: './dist/fonts',
    fontName: 'my-icons',
    variants: [
        { name: 'light', files: ['icons/light/add.svg'], weight: 300, default: true },
        { name: 'bold', files: ['icons/bold/add.svg'], weight: 700 },
    ],
});
```

Load the generated `my-icons.css`, then use the icon class with an optional variant modifier:

```html
<span class="icon icon-add" aria-hidden="true"></span> <span class="icon icon-add icon--bold" aria-hidden="true"></span>
```

The first icon uses the default design (`light`); the second uses `bold`. Each design needs a
unique name and at least one SVG, and exactly one design must be the default. Omit `weight`
to assign weights automatically, or supply increasing weights from 1–1000.

Multi-variant output supports TTF, WOFF, and WOFF2, defaulting to WOFF/WOFF2. For icons that
appear in only some designs, choose a [missing-glyph policy](#missingglyphs). Incremental
regeneration is available with `incremental: true`; use the [variant methods](#variant-regeneration).

Custom templates receive extra family metadata; see the [template context comparison](./templates#template-context).
For stylesheet customization and SCSS, see [Templates](./templates).

## Options reference

### `files`

- **Required for single-variant generation**
- Type: `string[]`
- Description: Array of paths to SVG files to include in the font.

Use a non-empty array when generating one design of each icon. Omit this field when `variants` is provided.

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

```ts
import { MissingGlyphBehavior } from '@atlowchemi/webfont-generator';

const blank = { behavior: MissingGlyphBehavior.Blank };
const error = { behavior: MissingGlyphBehavior.Error };
const fallback = { behavior: MissingGlyphBehavior.Fallback, variant: 'Regular' };
```

Pass one object as `missingGlyphs`. Blank cells retain the logical icon's advance; error mode
rejects missing cells; fallback reuses `Regular` artwork, so `Regular` must contain every icon.

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
- Default: `['eot', 'woff', 'woff2']` for single-variant input; `['woff', 'woff2']` for variants
- Description: Font formats to generate. Single-variant input accepts `'svg'`, `'ttf'`, `'eot'`, `'woff'`, `'woff2'`. Multi-variant input accepts only `'ttf'`, `'woff'`, and `'woff2'`.

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
- Description: Whether to write generated files to disk. Set to `false` for in-memory usage. Writes are non-transactional and can leave a partial bundle if a write fails.

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
- Description: Retain parsed glyph data on the result so [`regenerateAsync()`](#regenerateasync-files-changes) or [`regenerate()`](#regenerate-files-changes) can rebuild after file changes without re-parsing the glyphs that didn't change. Enable for watch/dev; leave it off for one-shot builds so the parsed geometry isn't held in memory.

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

- Type: `(context: CssContext) => void`, or `(context: CssContext<true>) => void` for multi-variant options
- Description: Callback to mutate the Handlebars template context before CSS rendering. See [context fields and callback typing](./templates#node-callbacks).

### `htmlContext`

- Type: `(context: HtmlContext) => void`, or `(context: HtmlContext<true>) => void` for multi-variant options
- Description: Callback to mutate the Handlebars template context before HTML rendering. See [context fields and callback typing](./templates#node-callbacks).

### `rename`

- Type: `(name: string) => string`
- Description: Custom function to derive glyph names from file paths. Receives the file path; must return the glyph name.

### `templateOptions`

- Type: `Record<string, any>`
- Description: Additional key-value pairs merged into the Handlebars template context. This is where `classPrefix` and `baseSelector` are typically set.

## Input types and TypeScript inference

`GenerateWebfontsOptions` accepts either `files` for a single design or `variants` for a
multi-variant family. Use `satisfies` to check a reusable configuration without losing its
specific input mode. Keep `types` as a tuple to guarantee the requested outputs:

```ts
import { generateWebfonts, type GenerateWebfontsOptions } from '@atlowchemi/webfont-generator';

const options = {
    files: ['icons/add.svg'],
    dest: './dist/fonts',
    types: ['woff2'] as ['woff2'],
} satisfies GenerateWebfontsOptions;

const result = await generateWebfonts(options);
result.woff2; // Uint8Array
result.ttf; // null
```

### Requested formats

A literal `types` list guarantees the selected getters. With `types` omitted, single-variant
input guarantees EOT/WOFF/WOFF2 and multi-variant input guarantees WOFF/WOFF2.

```ts
const modern = await generateWebfonts({
    dest: './dist/fonts',
    variants: [
        { name: 'light', files: ['icons/light/add.svg'], default: true },
        { name: 'bold', files: ['icons/bold/add.svg'] },
    ],
});
modern.woff2; // Uint8Array
modern.svg; // null

const svg = await generateWebfonts<'svg'>({
    files: ['icons/add.svg'],
    dest: './dist/fonts',
    types: ['svg'],
});
svg.svg; // string
```

When formats are selected at runtime, TypeScript cannot guarantee which getters contain data:

```ts
import type { FontType } from '@atlowchemi/webfont-generator';

async function build(types: FontType[]) {
    const result = await generateWebfonts({ files: ['icons/add.svg'], dest: './dist/fonts', types });
    if (result.woff2 !== null) {
        console.log(result.woff2.byteLength);
    }
}
```

Widened option unions and explicit union generics also produce conservative nullable getters.
An explicit single-format generic guarantees its getter when `types` is a nonempty tuple;
an explicit union does not guarantee every member was requested.
`GenerateWebfontsResult<Possible, Guaranteed>` records the possible and guaranteed formats;
the second parameter defaults to the first. Async regeneration preserves both sets.

### Callback input types

Callbacks on multi-variant options infer `CssContext<true>` and `HtmlContext<true>`.
`GenerateWebfontsBaseOptions<true>` can describe shared options with those callback types.
See [template context and callback typing](./templates#node-callbacks) for a comparison and examples.

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

Omit `urls` to use generated URLs. A supplied map replaces all defaults, so omitted entries
are empty. Multi-variant results reject SVG/EOT URLs. See [generated CSS](./templates#generated-css)
for icon selectors and variant modifiers.

### `generateHtml(urls?)`

- Type: `(urls?: Partial<Record<FontType, string>>) => string`
- Description: Returns the rendered HTML preview string. Pass `urls` to override font URLs in the embedded stylesheet.

URL overrides follow the same complete-replacement rule as `generateCss`. The built-in preview
shows one icon grid using the default design. See [HTML previews](./templates#html-previews).

### `regenerate(files, changes?)`

- Type: `(files: RegenerationFiles, changes?: GlyphChangeEntry[] | null) => void`
- Requires: the result was produced with [`incremental: true`](#incremental) (throws otherwise).
- Supported input: `{ files: string[] }` for ordinary fonts, or `{ variants: VariantFileSet[] }` for multi-variant families. Supply exactly one source, matching the result's mode.
- Description: Rebuilds every requested font format after file changes, reusing cached geometry for the glyphs that didn't change (and reusing the rendered CSS/HTML when the glyph names and codepoints are unchanged). `files` is the complete file set after the change, in the order a fresh build would use (e.g. your glob result); the rebuilt glyphs are ordered to match it, so the result is byte-identical to a fresh `generateWebfonts()` of that set — additions included. Any file omitted from `files` is dropped; added/changed files named in `changes` are read from disk and re-parsed. Omit `changes` or pass `null` to re-read/hash every current file and infer added/changed/removed paths automatically. Outputs are refreshed in memory, and — when the result was created with [`writeFiles: true`](#writefiles) — refreshed fonts are written to disk too, while unchanged CSS/HTML companion files are skipped. Results generated with `cssContext` or `htmlContext` callbacks cannot be regenerated because those JavaScript callbacks cannot be re-run by the synchronous method. Intended for dev/watch rebuilds.

```ts
let files = ['/icons/add.svg', '/icons/search.svg'];
const result = await generateWebfonts({ files, dest, incremental: true });
// ...on a watch event:
result.regenerate({ files }, [{ path: '/icons/add.svg', changeType: 'changed' }]);
// ...or when watcher hints are unavailable/untrusted:
result.regenerate({ files });
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

- Type: `(files: RegenerationFiles, changes?: GlyphChangeEntry[] | null) => Promise<GenerateWebfontsResult>`
- Requires: the result was produced with [`incremental: true`](#incremental) (rejects otherwise).
- Supported input: the same complete source shape as `regenerate`.
- Description: Performs the same rebuild as [`regenerate()`](#regenerate-files-changes) off the Node.js event loop and resolves with a replacement result. The receiver remains readable and unchanged while the rebuild runs and after failure. Assign the replacement before starting another rebuild; overlapping calls from the same result lineage reject. In-memory state is replaced only on success, but writes made with [`writeFiles: true`](#writefiles) are not transactional.

```ts
let files = ['/icons/add.svg', '/icons/search.svg'];
let result = await generateWebfonts({ files, dest, incremental: true });
result = await result.regenerateAsync({ files }, [{ path: '/icons/add.svg', changeType: 'changed' }]);
```

### Variant regeneration

Generate with `incremental: true` to retain per-design geometry caches. Regeneration keeps the
configured designs and weights fixed; adding, removing, or reordering designs requires a fresh
generation. Changes to SVG membership, names, and contents are supported within each design.

```ts
type RegenerationFiles = { files: string[]; variants?: never } | { files?: never; variants: VariantFileSet[] };

interface VariantFileSet {
    variant: string;
    files: string[];
}
```

```ts
import { generateWebfonts, type RegenerationFiles } from '@atlowchemi/webfont-generator';

let result = await generateWebfonts({
    dest: './dist/fonts',
    incremental: true,
    variants: [
        { name: 'light', files: ['light/add.svg'], default: true },
        { name: 'bold', files: ['bold/add.svg'] },
    ],
});

const files: RegenerationFiles = {
    variants: [
        { variant: 'light', files: ['light/add.svg'] },
        { variant: 'bold', files: ['bold/add.svg'] },
    ],
};
result = await result.regenerateAsync(files, [{ path: 'bold/add.svg', changeType: 'changed' }]);
// Omit changes to read every file and infer changes, including additions and removals.
result = await result.regenerateAsync(files);
```

Every call requires every configured design exactly once. Each `files` list is authoritative
for that design's membership and order. Lists may share paths and use different orders; their
position in the update does not change configured design order. Unknown/duplicate design names,
empty lists, duplicate paths within a design, duplicate change hints, and inconsistent change
types are rejected before sources are read.

Change hints describe paths across the family: `added` means new to the family, `changed` means
an existing path still used in the final family, and `removed` means no design uses it afterward.
A content or `name` hint applies to every final design referencing that path. To add an existing
path to another design, move it, or remove only one membership, edit the per-design file lists;
no change hint is needed unless the contents or name also changed. New memberships are loaded
even with an explicit empty changes list. Existing unhinted files retain their contents and
names; full re-diff reads all files, preserves existing names, and derives new names from filenames.

Effective changes rebuild the requested shared font outputs while reusing eligible parsed and
processed geometry. Union membership, codepoints, shared metrics, and fallback dependencies are
recomputed. No-op updates reuse in-memory outputs; pending disk writes can still be retried.

Validation, loading, and font-build failures preserve the previous result. Synchronous write
failures leave the new in-memory result committed and may leave partial disk output; retry the
update or re-diff to finish writing. Async calls leave the receiver unchanged on failure and
return a replacement only on success; retry from the receiver after rejection. Overlapping
calls and regeneration from a replaced result are rejected. Writes are not transactional, so
an async rejection may still follow partial disk writes. Callback-created results cannot be
regenerated.

### Migrating regeneration calls

The regeneration input is a **breaking change** for both methods. Wrap ordinary file arrays in
`{ files }`; the optional second argument keeps its existing shape.

```ts
// Before
result.regenerate(files, changes);
result = await result.regenerateAsync(files);

// After
result.regenerate({ files }, changes);
result = await result.regenerateAsync({ files });
```

Use `{ variants: [...] }` for families as shown above. A bare array is no longer accepted.

## Templates

See the shared [Templates reference](./templates) for [context fields](./templates#template-context),
[custom templates](./templates#custom-templates), and the [SCSS mixin with examples](./templates#scss).

The package exports default Handlebars template paths via a subpath export:

```ts
import * as templates from '@atlowchemi/webfont-generator/templates';

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
