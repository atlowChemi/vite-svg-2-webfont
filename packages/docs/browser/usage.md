---
description: Generate icon fonts in the browser with @atlowchemi/webfont-browser-generator, use the output, cache between sessions, update results, and host the worker.
---

# Browser Usage

Install the package first; see [Getting Started](/browser/). Every option is listed in [configuration](/browser/configuration), and every method and type in the [API reference](/browser/api).

## How to use

Import `generateWebfonts` in your Vite application's client code. Vite automatically bundles
the worker and WASM assets; no additional Vite configuration is needed.

```ts [src/main.ts]
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

using result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
    incremental: true,
    // Pin the font's creation timestamp so the same input always produces the same bytes.
    // Without it, the worker records the current browser time.
    formatOptions: { ttf: { ts: 0 } },
});
const bytes = await result.font('woff2');
const css = await result.generateCss();
```

The options match the other tools', except that icons arrive as SVG text in `sources` and templates are text rather than file paths. Nothing is read from or written to disk, and there are no JavaScript callbacks. See [configuration](/browser/configuration).

Without `formatOptions.ttf.ts`, generation captures the browser clock once and retains that
timestamp during updates; fonts reused from a [persistent cache](#persistent-cache) keep the
timestamp of the session that created them. A fixed Unix timestamp produces reproducible font
metadata.
WOFF uses the pure-Rust zlib-rs compression backend on both native and wasm32 targets.
WOFF2 uses single-threaded Brotli at every supported compression quality (0–11).

### Using the output

The result holds plain bytes and text, so you can use the font on the current page, offer it as a download, or upload it.

::: code-group

```ts [Use on the page]
const fontUrl = URL.createObjectURL(new Blob([bytes!], { type: 'font/woff2' }));
// Point the generated CSS at the in-memory font, then apply it.
const style = document.createElement('style');
style.textContent = await result.generateCss();
style.textContent = style.textContent.replace(/url\("?[^)"]*\.woff2[^)"]*"?\)/g, `url("${fontUrl}")`);
document.head.append(style);

document.body.insertAdjacentHTML('beforeend', '<i class="icon icon-square"></i>');
```

```ts [Download]
function download(name: string, data: BlobPart, type: string) {
    const link = document.createElement('a');
    link.href = URL.createObjectURL(new Blob([data], { type }));
    link.download = name;
    link.click();
    URL.revokeObjectURL(link.href);
}

download('iconfont.woff2', bytes!, 'font/woff2');
download('iconfont.css', await result.generateCss(), 'text/css');
```

```ts [Upload]
const outputs = await result.collectOutputs();
const form = new FormData();
for (const [kind, data] of Object.entries(outputs)) {
    form.append(kind, new Blob([data]), `iconfont.${kind}`);
}
await fetch('/api/icon-fonts', { method: 'POST', body: form });
```

:::

To pass explicit font URLs instead of rewriting the CSS, set `cssFontsUrl` to the URL prefix where you will serve the files.

## Persistent cache

Pass `cache` to reuse work from an earlier session with the [engine cache](/guide/features/caching).
Start with an empty array, store what `exportCache()` returns, and pass it back next time.
Unchanged icons skip parsing and path processing, and unchanged formats are reused; output is
byte-identical.

The package only accepts and returns the bytes (a `Uint8Array`); storing them is up to your app.
Keep them anywhere that holds binary data: IndexedDB (stores `Uint8Array` natively), the Origin
Private File System, Cache Storage, or your own server. Expect several MB for hundreds of icons.
In the example, `loadCache` and `saveCache` stand for your own storage code; see [keep the cache in IndexedDB](/browser/examples#keep-the-cache-in-indexeddb) for one implementation:

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';
// Your own module, not part of the package: for example an IndexedDB wrapper.
import { loadCache, saveCache } from './icon-cache';

const stored = await loadCache('icons'); // Uint8Array | undefined
using result = await generateWebfonts({
    sources,
    types: ['woff2'],
    cache: stored ?? new Uint8Array(),
});
const updated = await result.exportCache();
if (updated) await saveCache('icons', updated);
```

- Without a fixed `formatOptions.ttf.ts`, reused fonts keep the timestamp of the session that
  created them, so output matches that session rather than the current clock. Set `formatOptions.ttf.ts` for
  reproducible output.
- `exportCache()` reflects incremental updates and resolves `undefined` when nothing changed since
  the cache was loaded or last exported, so you can skip writes.
- Bytes from another engine version or other settings are ignored, and corrupted or truncated
  bytes fall back to a correct uncached run. Saved fonts and saved per-icon work carry separate
  checksums; sessions whose fonts all come from the cache don't read the per-icon work, so damage
  limited to it is detected (and a fresh cache exported) by the next session that needs it.

::: warning Only pass trusted cache bytes
Cached fonts are reused as-is, so only pass bytes your own app stored. Don't accept cache bytes
from users or other origins, and clear the stored copy if in doubt.
:::

## Incremental updates

With `incremental: true`, a result can be [updated](/guide/features/incremental) in place as icons change, without sending the unchanged SVGs again:

```ts
await result.add({ id: 'search', name: 'search', svg: searchSvg });
await result.update('search', replacementSvg);
await result.remove('search');
```

See [updating a result](/browser/api#updating-a-result) for families, batches with `regenerate()`, and failures, and [live editing](/browser/examples#live-editing) for a complete editor.

## Worker lifetime and hosting

Each result from `generateWebfonts()` has its own worker and WebAssembly instance, and disposing the result terminates its worker. Keep a result for incremental work, and dispose it when finished. The package needs neither SharedArrayBuffer nor cross-origin isolation.

`generateWebfonts(options, { workerUrl })` accepts an optional string or URL for a separately hosted worker. By default, bundlers resolve the worker relative to the package entry. Serve `worker.js` and `webfont_browser_bg.wasm` together from the same origin; see [self-hosted worker](/browser/examples#self-hosted-worker).

Vite emits the worker and WebAssembly assets. Serve the WebAssembly file as `application/wasm`, and permit worker execution and WebAssembly compilation in your Content Security Policy.

### Reuse one worker

Starting a worker loads and compiles the WebAssembly, which can take longer than generating a small font. When an app generates repeatedly with changing options, such as an editor that rebuilds on every settings change, create one generator and generate every result in its worker:

```ts
import { createWebfontGenerator } from '@atlowchemi/webfont-browser-generator';

// The worker starts loading the WebAssembly now, before the first generation.
using generator = createWebfontGenerator();

let result = await generator.generateWebfonts({ sources, types: ['woff2'] });
// Later, with other options: the WebAssembly is already loaded.
const next = await generator.generateWebfonts({ sources, types: ['woff2', 'woff'] });
result.dispose();
result = next;
```

`createWebfontGenerator({ workerUrl })` takes the same optional worker URL, and `generator.generateWebfonts(options)` takes the same options and returns the same result as the top-level function.

- **Several results at once.** A generator can host any number of live results. Disposing one frees its memory and rejects its pending calls; the others keep working.
- **One queue.** Calls from every result of a generator run one at a time, in submission order, so a long build delays the others. To build in parallel, create one generator per parallel task.
- **Failures.** A generation with invalid options rejects without affecting the generator. If the WebAssembly fails to load or crashes, every result of that generator rejects; create a new one.
- **Disposal.** Disposing the generator terminates its worker and rejects the pending and future calls of all its results. Results don't keep the generator alive, so keep it as long as you use them.
