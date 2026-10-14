---
description: Reference for the private browser workspace package and its worker-owned in-memory font API.
---

# Browser workspace API

`@atlowchemi/webfont-browser-generator` is a **private workspace package**, not an npm release.
It wraps the Rust engine with wasm-bindgen and runs generation inside dedicated module workers.

## Workspace setup

```sh
vp install
vp run @atlowchemi/webfont-browser-generator#build
vp run @atlowchemi/webfont-browser-generator#test
```

The build requires Rust/rustup and installs the WASM target and a matching, repository-local
wasm-bindgen CLI. Use `WASM_BINDGEN` to select an existing CLI of the required version.
The package exports TypeScript source for Vite workspace consumers; build its WASM before
starting Vite. Browser tests execute generation in Chromium, Firefox and WebKit.

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

Each result has its own worker and WASM instance. Keep a result for incremental work, and
dispose it when finished. The package needs neither SharedArrayBuffer nor cross-origin isolation.
Vite emits the worker and WASM assets. Serve WASM with `application/wasm`, and permit worker
execution and WASM compilation in your Content Security Policy.
