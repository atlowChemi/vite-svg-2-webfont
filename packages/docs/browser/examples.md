---
description: Examples for @atlowchemi/webfont-browser-generator — fonts from dropped files, a live preview, live editing, an IndexedDB cache, and a self-hosted worker.
---

# Browser Examples

Each example runs in client-side code; with Vite, the worker and WebAssembly files are bundled for you. See [configuration](/browser/configuration) for every option and the [API reference](/browser/api) for the result. The [Playground](/playground) combines most of these into one app.

## Fonts from dropped files

Turn SVG files a user drops on the page into sources. File names become icon names, so replace anything other than letters, digits, hyphens, and underscores.

```ts [src/drop.ts]
import { generateWebfonts, type SvgSource } from '@atlowchemi/webfont-browser-generator';

async function toSources(files: Iterable<File>): Promise<SvgSource[]> {
    const svgs = [...files].filter(file => file.name.endsWith('.svg'));
    return Promise.all(
        svgs.map(async file => {
            const name = file.name.slice(0, -4).replace(/[^\p{L}\p{N}_-]/gu, '-');
            return { id: name, name, svg: await file.text() };
        }),
    );
}

document.addEventListener('dragover', event => event.preventDefault());
document.addEventListener('drop', async event => {
    event.preventDefault();
    const sources = await toSources(event.dataTransfer?.files ?? []);
    using result = await generateWebfonts({ sources, types: ['woff2'] });
    console.log(`${sources.length} icons, ${(await result.font('woff2'))?.byteLength} bytes`);
});
```

## Live preview

Show the generated font on the current page without serving it: wrap the bytes in a blob URL and point the generated CSS at it.

```ts [src/preview.ts]
import type { WebfontResult } from '@atlowchemi/webfont-browser-generator';

const style = document.head.appendChild(document.createElement('style'));
let fontUrl: string | undefined;

export async function showFont(result: WebfontResult) {
    const bytes = await result.font('woff2');
    if (!bytes) throw new Error('Generate with types: ["woff2"]');
    if (fontUrl) URL.revokeObjectURL(fontUrl);
    fontUrl = URL.createObjectURL(new Blob([bytes], { type: 'font/woff2' }));
    // The generated CSS refers to iconfont.woff2?<hash>; swap in the blob URL.
    const css = await result.generateCss();
    style.textContent = css.replace(/url\("[^"]*\.woff2[^"]*"\)/g, `url("${fontUrl}")`);
}
```

```html
<i class="icon icon-square"></i>
```

Call `showFont()` after generating, and again after every update; the old blob URL is released each time.

## Live editing

Keep one incremental result and update it as the user edits an icon, instead of generating the whole font again. Only the edited icon is processed.

```ts [src/editor.ts]
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';
import { showFont } from './preview';

const editor = document.querySelector('textarea')!;
const result = await generateWebfonts({
    sources: [{ id: 'logo', name: 'logo', svg: editor.value }],
    types: ['woff2'],
    incremental: true,
});
await showFont(result);

let timer: ReturnType<typeof setTimeout> | undefined;
editor.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(async () => {
        try {
            await result.update('logo', editor.value);
            await showFont(result);
        } catch (error) {
            // Invalid SVG: the result keeps the last good font, so keep showing it.
            console.warn(error);
        }
    }, 150);
});
```

The result stays alive for the editor's lifetime, so it isn't declared with `using`; call `result.dispose()` when the editor closes.

## Keep the cache in IndexedDB

Store the [engine cache](/guide/features/caching) between visits, so a returning user's unchanged icons aren't processed again. IndexedDB stores `Uint8Array` values as they are.

```ts [src/icon-cache.ts]
const open = () =>
    new Promise<IDBDatabase>((resolve, reject) => {
        const request = indexedDB.open('icon-cache', 1);
        request.onupgradeneeded = () => request.result.createObjectStore('caches');
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });

export async function loadCache(key: string): Promise<Uint8Array | undefined> {
    const db = await open();
    return new Promise((resolve, reject) => {
        const request = db.transaction('caches').objectStore('caches').get(key);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

export async function saveCache(key: string, bytes: Uint8Array): Promise<void> {
    const db = await open();
    return new Promise((resolve, reject) => {
        const transaction = db.transaction('caches', 'readwrite');
        transaction.objectStore('caches').put(bytes, key);
        transaction.oncomplete = () => resolve();
        transaction.onerror = () => reject(transaction.error);
    });
}
```

```ts [src/main.ts]
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';
import { loadCache, saveCache } from './icon-cache';

using result = await generateWebfonts({
    sources,
    types: ['woff2'],
    formatOptions: { ttf: { ts: 0 } },
    cache: (await loadCache('icons')) ?? new Uint8Array(),
});

const updated = await result.exportCache();
if (updated) await saveCache('icons', updated);
```

`exportCache()` resolves `undefined` when nothing changed, which skips the write. A fixed `ts` keeps the output identical whether fonts come from the cache or not.

## Self-hosted worker

Without a bundler that understands `new URL(…, import.meta.url)`, serve the worker and its WebAssembly file yourself and pass the worker's URL:

```ts
using result = await generateWebfonts({ sources, types: ['woff2'] }, { workerUrl: '/vendor/webfont/worker.js' });
```

Copy `worker.js` and `webfont_browser_bg.wasm` from the package's `dist` folder (also exported as `@atlowchemi/webfont-browser-generator/worker.js` and `/webfont_browser_bg.wasm`) to the same directory, and serve the WebAssembly file as `application/wasm`. See [worker lifetime and hosting](/browser/usage#worker-lifetime-and-hosting).
