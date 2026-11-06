---
description: The functions, result type, and TypeScript types of the @atlowchemi/webfont-generator npm package.
---

# Node.js API

## `generateWebfonts(options)`

- Type: `(options: GenerateWebfontsOptions) => Promise<GenerateWebfontsResult>`

Generates the fonts and, unless [`writeFiles`](/node/configuration#writefiles) is `false`, writes them to disk. See [configuration](/node/configuration) for every option.

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    files: ['icons/add.svg', 'icons/remove.svg'],
    dest: './dist/fonts',
    types: ['woff2', 'woff'],
});
```

## `GenerateWebfontsResult`

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
are empty. Multi-variant results reject SVG/EOT URLs. See [generated CSS](/guide/templates#generated-css)
for icon selectors and variant modifiers.

### `generateHtml(urls?)`

- Type: `(urls?: Partial<Record<FontType, string>>) => string`
- Description: Returns the rendered HTML preview string. Pass `urls` to override font URLs in the embedded stylesheet.

URL overrides follow the same complete-replacement rule as `generateCss`. The built-in preview
shows one icon grid using the default design. See [HTML previews](/guide/templates#html-previews).

### `generateTs()`

`result.generateTs(): string` returns fixed TypeScript source with `icons` as a readonly
literal tuple, `IconName = (typeof icons)[number]`, and literal `iconBaseSelector` and
`iconClassPrefix` constants. It works with CSS disabled and includes effective `cssContext`
changes. It describes generation context, not the classes a custom CSS template happens to emit.
There is no TS-specific template, override, or callback.

Set `ts: true` (default: false) to write `<dest>/<fontName>.ts`; `tsDest` overrides the path.
`writeFiles: false` suppresses physical output, but does not disable `generateTs()`.
Regeneration refreshes metadata along with the fonts.

```ts
const result = await generateWebfonts({ files: ['icons/add.svg'], dest: './fonts', ts: true });
console.log(result.generateTs());
```

### `flushCache()`

- Type: `() => Promise<void>`
- Waits for automatic cache writes and retries a previously failed write once. Await outstanding incremental calls first, then call this before shutdown. Dropping a result does not wait, and abrupt process termination can lose pending writes.
- Rejects on persistence failure without affecting generated output. No-op without an incremental `cacheLocation`. Old results returned before `regenerateAsync` share the same writer: flushing one waits for the lineage and never saves its stale fonts.
- Initial generation still saves synchronously. No explicit save call is required after edits.

## Updating a result

Results created with [`incremental: true`](/node/configuration#incremental) can be updated after icons change. See [incremental updates](/guide/features/incremental) for when to use each method.

### `regenerate(files, changes?)`

- Type: `(files: RegenerationFiles, changes?: GlyphChangeEntry[] | null) => void`
- Requires: the result was produced with [`incremental: true`](/node/configuration#incremental) (throws otherwise).
- Supported input: `{ files: string[] }` for ordinary fonts, or `{ variants: VariantFileSet[] }` for multi-variant families. Supply exactly one source, matching the result's mode.
- Description: Rebuilds every requested font format after file changes, reusing cached geometry for the glyphs that didn't change (and reusing the rendered CSS/HTML when the glyph names and codepoints are unchanged). `files` is the complete file set after the change, in the order a fresh build would use (e.g. your glob result); the rebuilt glyphs are ordered to match it, so the result is byte-identical to a fresh `generateWebfonts()` of that set — additions included. Any file omitted from `files` is dropped; added/changed files named in `changes` are read from disk and re-parsed. Omit `changes` or pass `null` to re-read/hash every current file and infer added/changed/removed paths automatically. Outputs are refreshed in memory, and — when the result was created with [`writeFiles: true`](/node/configuration#writefiles) — refreshed fonts are written to disk too, while unchanged CSS/HTML companion files are skipped. Results generated with `cssContext` or `htmlContext` callbacks cannot be regenerated because those JavaScript callbacks cannot be re-run by the synchronous method. Intended for dev/watch rebuilds.

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
- Requires: the result was produced with [`incremental: true`](/node/configuration#incremental) (rejects otherwise).
- Supported input: the same complete source shape as `regenerate`.
- Description: Performs the same rebuild as [`regenerate()`](#regenerate-files-changes) off the Node.js event loop and resolves with a replacement result. The receiver remains readable and unchanged while the rebuild runs and after failure. Assign the replacement before starting another rebuild; overlapping calls from the same result lineage reject. In-memory state is replaced only on success, but writes made with [`writeFiles: true`](/node/configuration#writefiles) are not transactional.

```ts
let files = ['/icons/add.svg', '/icons/search.svg'];
let result = await generateWebfonts({ files, dest, incremental: true });
result = await result.regenerateAsync({ files }, [{ path: '/icons/add.svg', changeType: 'changed' }]);
```

### Single-file actions

Results generated with `incremental: true` support single actions without supplying the full file list:

| Synchronous                  | Async (returns replacement result) | Effect                                                                                                     |
| ---------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `add(path, name?, variant?)` | `addAsync(path, name?, variant?)`  | Read a new file and append it; optional `name` overrides the file stem. Families require a target variant. |
| `update(path, name?)`        | `updateAsync(path, name?)`         | Re-read an existing file in every consuming design, preserving its position and its name unless replaced.  |
| `remove(path)`               | `removeAsync(path)`                | Remove an existing path from every design, preserving remaining order. Does not delete the file on disk.   |

```ts
result = await result.addAsync('icons/search.svg');
result = await result.updateAsync('icons/search.svg', 'search');
result = await result.removeAsync('icons/old.svg');
// For a family, append to one configured design:
family = await family.addAsync('bold/search.svg', undefined, 'bold');
```

Duplicate additions, unknown update/remove paths, and removals that empty a design reject.
The helpers use the current retained membership, including changes made through `regenerate`.
They inherit regeneration's callback restrictions, output writing, failure handling, and async
snapshot/overlap rules. Assign async replacements before the next action.
Use `regenerate` for batches, explicit ordering, or assigning existing paths to other variants.

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

## `clearCache(cacheLocation)`

Remove the files the generation cache created in `cacheLocation`, including leftovers from interrupted writes. Other files in the directory are left untouched, and a missing directory resolves with zero counts.

Stop incremental updates and await `flushCache()` on each active result lineage before clearing. Otherwise an outstanding background write can recreate a removed cache file. Follow the same ordering before deleting the cache directory yourself.

```ts
import { clearCache } from '@atlowchemi/webfont-generator';

const { filesRemoved, bytesRemoved } = await clearCache('.webfont-cache');
```

## Template paths

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

## TypeScript types

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
See [template context and callback typing](/guide/templates#node-callbacks) for a comparison and examples.
