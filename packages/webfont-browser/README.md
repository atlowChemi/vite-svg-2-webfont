# @atlowchemi/webfont-browser-generator

Generate webfonts from SVG text in browser workers. The npm package includes compiled JavaScript,
TypeScript declarations and precompiled WASM; consumers need no Rust tools or install-time compilation.

```sh
npm install @atlowchemi/webfont-browser-generator
```

Vite apps (including React) bundle the worker and WASM automatically. Call the API in the browser,
not during server rendering.

## Build and test

From the repository root:

```sh
vp install
vp run @atlowchemi/webfont-browser-generator#build
vp run @atlowchemi/webfont-browser-generator#pack
vp run @atlowchemi/webfont-browser-generator#pack:tgz
vp run @atlowchemi/webfont-browser-generator#test
vp run @atlowchemi/webfont-browser-generator#test:coverage
```

The build installs the `wasm32-unknown-unknown` Rust target when needed and a version-matched
wasm-bindgen CLI in `target/wasm-bindgen-tools`. `WASM_BINDGEN` can point to an existing CLI
of that exact version. Rust and rustup must be installed. Tests use Playwright Chromium,
Firefox and WebKit (`vp exec playwright install chromium firefox webkit`).

`build` compiles the WASM binding; `pack` builds `dist/` with JS, types, worker and WASM.
`pack:tgz` also creates an installable npm tarball in this package directory. Install that tarball
in a demo app to test unpublished changes. Workspace consumers must run `pack` first.

Coverage writes `coverage/browser-js/lcov.info` and `coverage/browser-rust/lcov.info`.
JavaScript coverage combines Chromium client tests with worker protocol tests running the real
WASM in Node (only the message transport and asset fetch are replaced). Rust coverage executes
the adapter's WASM tests in Node through wasm-bindgen-test, not a native substitute.
It requires `cargo-llvm-cov` 0.9.0+, a WASM-capable `clang` on PATH, and:

```sh
rustup toolchain install nightly-2026-09-14 --profile minimal --component llvm-tools-preview --target wasm32-unknown-unknown
```

On macOS use Homebrew LLVM's `bin` directory on PATH; Apple clang lacks the WASM target.
`WEBFONT_COVERAGE_TOOLCHAIN` overrides the coverage-only toolchain. Normal builds use stable Rust.

## Use

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

const result = await generateWebfonts({
    sources: [{ id: 'square-id', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
    incremental: true,
    formatOptions: { ttf: { ts: 0 } },
});
try {
    const font = await result.font('woff2');
    const css = await result.generateCss();
    await result.regenerate({ sources: ['square-id'] }, [{ type: 'changed', id: 'square-id', svg: '<svg viewBox="0 0 24 24"><path d="M1 1H23V23H1Z"/></svg>' }]);
    const outputs = await result.collectOutputs();
} finally {
    result.dispose();
}
```

## Contract

`generateWebfonts(options, { workerUrl })` accepts an optional string or URL for a separately
hosted worker. Serve `worker.js` and `webfont_browser_bg.wasm` together from the same origin.

With Explicit Resource Management support, use `using result = await generateWebfonts(options)`
inside an async function. Leaving the scope calls `Symbol.dispose`, including on exceptions.
Cleanup terminates the worker synchronously; `await using` is unnecessary. Keep `dispose()`
for lifetimes managed by framework hooks or applications without `using` support.

CI reports the production smoke app as Codecov bundle `webfont-browser-generator-bundle`,
separately from the Vite plugin. This measures the emitted application JavaScript, worker and
WASM assets together, including the small smoke-app entry point.

- Each result owns one dedicated worker and WASM instance. Calls run in submission order.
  Different results are independent. No SharedArrayBuffer or cross-origin isolation is needed.
- All generation settings are the engine's in-memory settings in camelCase; see
  `GenerateWebfontsOptions` in the shipped declarations. Files, paths, output destinations and callbacks
  are not accepted. SVG image files are not loaded. Templates are inline Handlebars text.
- Ordinary generation supports SVG, TTF, EOT, WOFF and WOFF2; variants support TTF, WOFF and
  WOFF2. Use `variants` with named sources, weights and exactly one default design.
  `colorGlyphs: true` or a glyph-name list enables solid-color fonts in modern formats.
- `font(format)` returns copied bytes, or `undefined` for a format not requested. It does not
  render CSS or HTML. `generateCss()` and `generateHtml()` render independently on demand.
  `collectOutputs()` returns `{ kind, bytes }[]` for requested fonts and enabled companions.
- The worker copies Rust buffers into JS-owned arrays and transfers those arrays to the caller.
  Returned bytes remain usable after updates or disposal; changing them cannot affect the result.
- Updates require `incremental: true`, complete final membership, and only changed sources.
  Ordinary membership is `{ sources: string[] }`; variant membership is
  `{ variants: [{ name, ids }] }` in original design order. Changes are `added` (id/name/svg),
  `changed` (id/svg, optional name), or `removed` (id), tagged by `type`.
- Invalid input/update/rendering errors reject the operation; an existing result remains usable
  after ordinary engine errors. Worker failures and WASM traps reject pending calls and close
  that result. Create a new result after a fatal failure.
- `dispose()` terminates the worker, releases the instance, and rejects pending/future calls.
  It is idempotent. Always dispose results you no longer need.
- Omitted `formatOptions.ttf.ts` captures the browser's Unix time once per result, retaining it
  through updates. Supply a fixed timestamp for reproducible fonts.
- WOFF uses pure-Rust compression in this package. Its bytes need not match native zlib output.
  WOFF2 supports quality 0–11 using single-threaded Brotli.

Serve WASM with `application/wasm`. Content Security Policy must allow the worker and WASM
compilation for the deployment (for example, `worker-src 'self'` and the relevant
`script-src` sources plus `'wasm-unsafe-eval'`).
