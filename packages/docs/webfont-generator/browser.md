---
description: Browser font generation with precompiled WebAssembly and a worker-owned in-memory API.
---

# Browser API

`@atlowchemi/webfont-browser-generator` wraps the Rust engine with wasm-bindgen and runs
generation inside dedicated module workers. The package ships compiled JavaScript, types,
worker code and WASM; applications need no Rust tools or install-time compilation.

## Installation

::: code-group

```sh [npm]
npm install @atlowchemi/webfont-browser-generator
```

```sh [pnpm]
pnpm add @atlowchemi/webfont-browser-generator
```

```sh [yarn]
yarn add @atlowchemi/webfont-browser-generator
```

```sh [bun]
bun add @atlowchemi/webfont-browser-generator
```

:::

## How to use

Import `generateWebfonts` in your Vite application's client code. Vite automatically bundles
the worker and WASM assets; no additional Vite configuration is needed.

```ts [src/main.ts]
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

using result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
    incremental: true,
    formatOptions: { ttf: { ts: 0 } },
});
const bytes = await result.font('woff2');
const css = await result.generateCss();
```

Options follow the [Rust SVG text API](./rust#svg-text-api) in camelCase, including variants,
color selection, missing-glyph policies, format options and inline templates. Unknown fields,
including file input/output options, are rejected. There are no JavaScript callbacks.
Variant families support TTF, WOFF and WOFF2; ordinary fonts additionally support SVG and EOT.

Without `formatOptions.ttf.ts`, generation captures the browser clock once and retains that
timestamp during updates. A fixed Unix timestamp produces reproducible font metadata.
WOFF uses pure-Rust compression; native zlib output may differ at the byte level.
WOFF2 uses single-threaded Brotli at every supported compression quality (0–11).

## Result methods

::: tip
An explicit `dispose()` method is available for manually managed lifetimes instead of using Explicit Resource Management. It is idempotent: calling it more than once has no additional effect and does not throw.
:::

| Method                            | Result                             | Behavior                                                                                                             |
| --------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `font(format)`                    | `Promise<Uint8Array \| undefined>` | Copy a requested font; does not render templates                                                                     |
| `generateCss()`                   | `Promise<string>`                  | Lazy CSS render, even if CSS collection is disabled                                                                  |
| `generateHtml()`                  | `Promise<string>`                  | Lazy HTML render, reusing CSS                                                                                        |
| `collectOutputs()`                | `Promise<CollectedOutputs>`        | Object keyed by output kind, with independently owned bytes; unrequested outputs are omitted                         |
| `regenerate(membership, changes)` | `Promise<void>`                    | Apply source updates; requires `incremental: true`                                                                   |
| `add(source, variant?)`           | `Promise<void>`                    | Append a new source; families require a target variant. Requires `incremental: true`.                                |
| `update(id, svg, name?)`          | `Promise<void>`                    | Replace an existing source, preserving its position. Requires `incremental: true`.                                   |
| `remove(id)`                      | `Promise<void>`                    | Remove an ID from all designs without emptying a design. Requires `incremental: true`.                               |
| `dispose()`                       | `void`                             | Terminate the worker and reject pending calls. Idempotent: repeated calls have no additional effect and do not throw |

`CollectedOutputs` is `Partial<Record<FontType | 'css' | 'html', Uint8Array<ArrayBuffer>>>`.
Access outputs directly by kind, for example `(await result.collectOutputs()).woff2`.

Calls on one result execute in submission order. Returned byte arrays remain valid after updates
and disposal and cannot mutate the engine's retained buffers. CSS and HTML are compiled lazily;
an invalid HTML template does not prevent accessing a font or rendering CSS.

## Incremental updates

With `incremental: true`, single actions derive membership from the worker's current result:

```ts
await result.add({ id: 'search', name: 'search', svg: searchSvg });
await result.update('search', replacementSvg, 'renamed');
await result.remove('search');
// For a family, additions require a target variant:
await family.add({ id: 'bold:search', name: 'search', svg: searchSvg }, 'bold');
```

`add(source, variant?)` appends a new ID. `update(id, svg, name?)` preserves its position and,
when omitted, its name. `remove(id)` preserves remaining order. Updates and removals apply
to every design consuming the ID. Duplicate additions, unknown IDs, incorrect target variants,
and removals that empty a design reject without replacing the result. Calls execute in submission
order, including after recoverable failures. None of these methods requires resending unchanged SVGs.

For batches, explicit ordering, or membership reassignment, use `regenerate`:

```ts
await result.regenerate({ sources: ['square'] }, [{ type: 'changed', id: 'square', svg: replacementSvg }]);
```

Supply complete final IDs in order, with only changed source content. For variants, membership
is `{ variants: [{ name: 'light', ids: [...] }, { name: 'bold', ids: [...] }] }` in original design order.
Changes use `type: 'added'` with id/name/svg, `type: 'changed'` with id/svg and optional name,
or `type: 'removed'` with id. Options and variant configuration stay fixed.

Ordinary validation/build failures reject the call and preserve the existing result for retry.
Worker failures or WASM traps invalidate the session and reject pending calls. Create a new
result after a fatal failure. `dispose()` is idempotent; all subsequent calls reject.

## Worker lifetime and hosting

`generateWebfonts(options, { workerUrl })` accepts an optional string or URL for a separately
hosted worker. By default, bundlers resolve the worker relative to the package entry.

For separately hosted workers, serve `worker.js` and `webfont_browser_bg.wasm` together from the same origin.

Each result has its own worker and WASM instance. Keep a result for incremental work, and
dispose it when finished. The package needs neither SharedArrayBuffer nor cross-origin isolation.
Vite emits the worker and WASM assets. Serve WASM with `application/wasm`, and permit worker
execution and WASM compilation in your Content Security Policy.
