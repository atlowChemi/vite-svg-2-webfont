---
description: The functions, result methods, and TypeScript types of the @atlowchemi/webfont-browser-generator package.
---

# Browser API

## `generateWebfonts(options, runtime?)`

- Type: `(options: GenerateWebfontsOptions, runtime?: WorkerRuntime) => Promise<WebfontResult>`

Starts a dedicated module worker, generates the fonts in it, and resolves with a [`WebfontResult`](#webfontresult) once they're ready. The result owns the worker. To generate many results without starting a worker for each, use [`createWebfontGenerator()`](#createwebfontgenerator-runtime). See [configuration](/browser/configuration) for every option.

`runtime.workerUrl` loads a separately hosted worker instead of the one your bundler emits; serve `webfont_browser_bg.wasm` beside it. See [worker lifetime and hosting](/browser/usage#worker-lifetime-and-hosting).

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

using result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
});
```

## `createWebfontGenerator(runtime?)`

- Type: `(runtime?: WorkerRuntime) => WebfontGenerator`

Starts one module worker that can host any number of results, and returns a [`WebfontGenerator`](#webfontgenerator). The worker starts loading the WebAssembly right away, so every generation, including the first, skips that wait. See [reuse one worker](/browser/usage#reuse-one-worker).

```ts
import { createWebfontGenerator } from '@atlowchemi/webfont-browser-generator';

using generator = createWebfontGenerator();
using result = await generator.generateWebfonts({ sources, types: ['woff2'] });
```

## `WebfontGenerator`

### `generateWebfonts(options)`

- Type: `(options: GenerateWebfontsOptions) => Promise<WebfontResult>`

Generates a new result in the generator's worker. It takes the same [options](/browser/configuration) as the top-level function and returns the same [`WebfontResult`](#webfontresult). Invalid options reject without affecting the generator or its other results.

### `dispose()` {#generator-dispose}

- Type: `() => void`

Terminates the worker and rejects the pending and future calls of every result it hosts. Calling it again does nothing. `using generator = createWebfontGenerator()` calls it when the scope ends.

Results don't keep their generator alive, so keep the generator as long as you use any of its results.

## `WebfontResult`

A handle to the generated fonts in a worker. Every method is asynchronous, except `dispose()`, and calls on one result run in the order you make them, even after a recoverable error. Results of one [generator](#webfontgenerator) share a single queue, so a long call on one delays the others.

Returned bytes are copies: they stay valid after updates and disposal, and changing them doesn't affect the result. CSS and HTML render on demand, so an invalid HTML template doesn't stop you from reading a font or the CSS.

### `font(format)`

- Type: `(format: FontType) => Promise<Uint8Array | undefined>`

A requested font's bytes, or `undefined` for a format that wasn't in [`types`](/browser/configuration#types).

### `generateCss()`

- Type: `() => Promise<string>`

Renders the CSS, even when [`css`](/browser/configuration#css) is `false`.

### `generateHtml()`

- Type: `() => Promise<string>`

Renders the HTML preview, reusing the CSS, even when [`html`](/browser/configuration#html) is `false`.

### `generateTs()`

- Type: `() => Promise<string>`

Renders the fixed [TypeScript metadata](/guide/templates#typescript), even when [`ts`](/browser/configuration#ts) or `css` is `false`:

<!-- prettier-ignore -->
```ts
export const iconBaseSelector = ".icon" as const;
export const iconClassPrefix = "icon-" as const;
export const icons = [
  "square"
] as const;

export type IconName = (typeof icons)[number];
```

The tuple follows the engine's icon order, and the selectors come from `templateOptions.baseSelector` and `templateOptions.classPrefix`. It describes those settings, not the classes a custom CSS template emits. Updates that add, remove, or rename icons refresh it.

### `collectOutputs()`

- Type: `() => Promise<CollectedOutputs>`

Every output in one object, keyed by kind: each requested font, plus `css`, `html`, and `ts` when their options are on. Each value is its own `Uint8Array`, for example `(await result.collectOutputs()).woff2`.

### `exportCache()`

- Type: `() => Promise<Uint8Array | undefined>`

The [engine cache](/guide/features/caching) bytes to store, including incremental updates. Resolves `undefined` when [`cache`](/browser/configuration#cache) wasn't passed, or when nothing changed since the bytes were loaded or last exported, so you can skip the write. See [persistent cache](/browser/usage#persistent-cache).

### `dispose()`

- Type: `() => void`

Rejects pending calls and releases the result; later calls reject too. A result from `generateWebfonts()` also terminates its worker, while a result from a generator frees its memory and leaves the generator's other results running. Calling it again does nothing. `using result = await generateWebfonts(…)` calls it when the scope ends, through `[Symbol.dispose]`.

## Updating a result

A result created with [`incremental: true`](/browser/configuration#incremental) can be [updated](/guide/features/incremental) in place. None of the methods need the unchanged SVGs again. Options and the designs of a family stay fixed; to change them, generate a new result.

### `add(source, variant?)`

- Type: `(source: SvgSource, variant?: string) => Promise<void>`

Appends a new icon. In a family, `variant` names the design it joins. Adding an `id` that already exists rejects.

### `update(id, svg, name?)`

- Type: `(id: string, svg: string, name?: string) => Promise<void>`

Replaces an icon's SVG in every design that uses it, keeping its position, and its name unless you pass a new one.

### `remove(id)`

- Type: `(id: string) => Promise<void>`

Removes an icon from every design, keeping the order of the rest. Removing a design's last icon rejects.

### `regenerate(membership, changes)`

- Type: `(membership: SourceMembership, changes: SourceChange[]) => Promise<void>`

Applies several changes at once, or reorders icons. `membership` lists every final `id` in order: `{ sources: [...] }`, or `{ variants: [{ name: 'light', ids: [...] }, …] }` for a family, in its original design order. `changes` carries only the changed SVGs:

```ts
await result.regenerate({ sources: ['square', 'circle'] }, [
    { type: 'changed', id: 'square', svg: squareSvg },
    { type: 'added', id: 'circle', name: 'circle', svg: circleSvg },
]);
```

### When an update fails

Invalid input rejects the call and keeps the previous result, so you can retry. A worker crash or a WebAssembly trap ends the worker instead: every pending call of every result in it rejects, and you need a new result, or a new generator.

## Types

All types are exported from the package entry.

| Type                      | Shape                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------- |
| `GenerateWebfontsOptions` | The [options](/browser/configuration).                                                                  |
| `WebfontResult`           | The [result](#webfontresult).                                                                           |
| `WebfontGenerator`        | A [generator](#webfontgenerator) that hosts many results in one worker.                                 |
| `WorkerRuntime`           | `{ workerUrl?: string \| URL }`, where to load the worker from.                                         |
| `FontType`                | `'svg' \| 'ttf' \| 'eot' \| 'woff' \| 'woff2'`                                                          |
| `SvgSource`               | `{ id: string; name: string; svg: string }`                                                             |
| `SourceFontVariant`       | `{ name: string; sources: SvgSource[]; weight?: number; default?: boolean }`                            |
| `SourceMembership`        | `{ sources: string[] }` or `{ variants: { name: string; ids: string[] }[] }`                            |
| `SourceChange`            | `{ type: 'added'; id; name; svg }`, `{ type: 'changed'; id; svg; name? }`, or `{ type: 'removed'; id }` |
| `CollectedOutputs`        | `Partial<Record<FontType \| 'css' \| 'html' \| 'ts', Uint8Array>>`                                      |
| `JsonValue`               | Any JSON value, as in [`templateOptions`](/browser/configuration#templateoptions).                      |
