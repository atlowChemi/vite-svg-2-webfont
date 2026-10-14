---
description: Browser font generation with precompiled WebAssembly and a worker-owned in-memory API.
---

# Browser API

`@atlowchemi/webfont-browser-generator` wraps the Rust engine with wasm-bindgen and runs
generation inside dedicated module workers. The package ships compiled JavaScript, types,
worker code and WASM; applications need no Rust tools or install-time compilation.

## Installation

```sh
npm install @atlowchemi/webfont-browser-generator
```

Vite apps, including React, automatically bundle the worker and WASM. Invoke generation in
the browser rather than during server rendering.

For local package development, `vp run @atlowchemi/webfont-browser-generator#pack` builds
the complete distribution; `#pack:tgz` creates an installable tarball for demo applications.

## Generation

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

const result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
    incremental: true,
    formatOptions: { ttf: { ts: 0 } },
});
try {
    const bytes = await result.font('woff2');
    const css = await result.generateCss();
} finally {
    result.dispose();
}
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

With Explicit Resource Management, `using result = await generateWebfonts(options)` automatically
cleans up at scope exit, including exceptions. `Symbol.dispose` terminates the worker synchronously,
so `await using` is unnecessary. Use `dispose()` for framework-managed lifetimes that span scopes.

| Method                            | Result                             | Behavior                                                                 |
| --------------------------------- | ---------------------------------- | ------------------------------------------------------------------------ |
| `font(format)`                    | `Promise<Uint8Array \| undefined>` | Copy a requested font; does not render templates                         |
| `generateCss()`                   | `Promise<string>`                  | Lazy CSS render, even if CSS collection is disabled                      |
| `generateHtml()`                  | `Promise<string>`                  | Lazy HTML render, reusing CSS                                            |
| `collectOutputs()`                | `Promise<{ kind, bytes }[]>`       | Requested fonts and enabled CSS/HTML, with independently owned bytes     |
| `regenerate(membership, changes)` | `Promise<void>`                    | Apply source updates; requires `incremental: true`                       |
| `dispose()`                       | `void`                             | Terminate the worker, releasing its instance and rejecting pending calls |

Calls on one result execute in submission order. Returned arrays remain valid after updates
and disposal and cannot mutate the engine's retained buffers. CSS and HTML are compiled lazily;
an invalid HTML template does not prevent accessing a font or rendering CSS.

## Incremental updates

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
