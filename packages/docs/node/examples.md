---
description: Examples for @atlowchemi/webfont-generator — a build script, in-memory serving, watch mode, multi-weight families, custom templates, and caching in CI.
---

# Node.js Examples

Each example is a complete script. See [configuration](/node/configuration) for every option and the [API reference](/node/api) for the result.

## Build script

Find the icons with a glob and write the fonts and CSS. Sort the file list: icons get [codepoints](/guide/features/codepoints-and-ligatures#how-codepoints-are-assigned) in input order, and glob results aren't guaranteed to be sorted.

```ts [scripts/build-icons.ts]
import { glob } from 'node:fs/promises';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const files = (await Array.fromAsync(glob('icons/*.svg'))).sort();

await generateWebfonts({
    files,
    dest: 'dist/fonts',
    fontName: 'app-icons',
    types: ['woff2', 'woff'],
    cssDest: 'dist/app-icons.css',
    cssFontsUrl: '/fonts/',
});
```

This writes `dist/fonts/app-icons.woff2`, `dist/fonts/app-icons.woff`, and `dist/app-icons.css`, whose font URLs start with `/fonts/`.

## Serve fonts from memory

Set `writeFiles: false` to keep everything on the result, and serve it from your own server. This server builds the font on the first request instead of at startup, keeps the result in memory, and rebuilds it when the icons change:

- **Lazy:** the first request for the CSS or the font starts generation. Requests that arrive while it runs wait for the same build, instead of starting their own.
- **Cached:** responses carry an `ETag`, so browsers revalidate with a `304 Not Modified` instead of downloading the font again.
- **Fresh:** a watcher marks the result stale, and the next request [updates it incrementally](/guide/features/incremental), processing only the changed icons. A failed update keeps serving the last good font.

```ts [server.ts]
import { createHash } from 'node:crypto';
import { glob, watch } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const listIcons = async () => ({ files: (await Array.fromAsync(glob('icons/*.svg'))).sort() });

const build = async () =>
    generateWebfonts({
        ...(await listIcons()),
        dest: 'unused', // Required, but nothing is written to it.
        fontName: 'icons',
        types: ['woff2'],
        writeFiles: false,
        incremental: true,
        cssFontsUrl: '/fonts/',
    });

let current: ReturnType<typeof build> | undefined;
let stale = false;

function icons() {
    if (!current) {
        current = build();
        // A failed first build is retried by the next request.
        current.catch(() => (current = undefined));
    } else if (stale) {
        stale = false;
        current = current.then(async result => {
            try {
                return await result.regenerateAsync(await listIcons());
            } catch (error) {
                console.error(error);
                return result; // Keep serving the last good font.
            }
        });
    }
    return current;
}

function send(request: IncomingMessage, response: ServerResponse, type: string, body: string | Uint8Array) {
    const etag = `"${createHash('sha256').update(body).digest('base64url')}"`;
    if (request.headers['if-none-match'] === etag) {
        response.writeHead(304, { ETag: etag }).end();
    } else {
        response.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-cache', ETag: etag }).end(body);
    }
}

createServer(async (request, response) => {
    const { pathname } = new URL(request.url ?? '/', 'http://localhost');
    if (pathname !== '/icons.css' && pathname !== '/fonts/icons.woff2') {
        response.writeHead(404).end();
        return;
    }
    try {
        const result = await icons();
        if (pathname === '/icons.css') {
            send(request, response, 'text/css', result.generateCss());
        } else {
            send(request, response, 'font/woff2', result.woff2);
        }
    } catch (error) {
        console.error(error);
        response.writeHead(500).end();
    }
}).listen(3000);

for await (const event of watch('icons')) {
    if (event.filename?.endsWith('.svg')) stale = true;
}
```

The generated CSS points at `/fonts/icons.woff2` through [`cssFontsUrl`](/node/configuration#cssfontsurl), with a content hash in the query, which the server ignores. The engine has no streaming output: a result holds each font as one complete `Uint8Array`, which is small enough to send in one response.

### On-demand subsets

Generation is fast enough to run per request. This variant builds a font with only the icons a page asks for, such as `/icons.css?set=add,search`, and keeps the most recent subsets in memory:

```ts [server.ts]
import { readdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

// Only icons that exist can be requested, so a name never reaches the file system unchecked.
const available = new Set((await readdir('icons')).filter(file => file.endsWith('.svg')).map(file => file.slice(0, -4)));
const subsets = new Map<string, ReturnType<typeof buildSubset>>();
const MAX_SUBSETS = 100;

const buildSubset = (names: string[]) =>
    generateWebfonts({
        files: names.map(name => `icons/${name}.svg`),
        dest: 'unused',
        fontName: 'icons',
        types: ['woff2'],
        writeFiles: false,
    });

/** The subset for a comma-separated list of icon names, or `undefined` when the list is empty or names an unknown icon. */
function subset(set: string) {
    const names = [...new Set(set.split(','))].sort();
    if (!names.every(name => available.has(name))) return undefined;
    const key = names.join(',');
    let font = subsets.get(key);
    if (!font) {
        if (subsets.size >= MAX_SUBSETS) subsets.delete(subsets.keys().next().value!); // Evict the oldest.
        font = buildSubset(names);
        font.catch(() => subsets.delete(key));
        subsets.set(key, font);
    }
    return { key, font };
}

createServer(async (request, response) => {
    const url = new URL(request.url ?? '/', 'http://localhost');
    if (url.pathname !== '/icons.css' && url.pathname !== '/icons.woff2') {
        response.writeHead(404).end();
        return;
    }
    const requested = subset(url.searchParams.get('set') ?? '');
    if (!requested) {
        response.writeHead(400, { 'Content-Type': 'text/plain' }).end('`set` must list existing icons, separated by commas.');
        return;
    }
    try {
        const result = await requested.font;
        if (url.pathname === '/icons.css') {
            const css = result.generateCss({ woff2: `/icons.woff2?set=${requested.key}` });
            response.writeHead(200, { 'Content-Type': 'text/css' }).end(css);
        } else {
            response.writeHead(200, { 'Content-Type': 'font/woff2', 'Cache-Control': 'public, max-age=31536000, immutable' }).end(result.woff2);
        }
    } catch (error) {
        console.error(error);
        response.writeHead(500).end();
    }
}).listen(3000);
```

- The subset's sorted names are both the cache key and the font URL, so the same set always maps to the same bytes, and the font can be cached as `immutable`.
- Every subset uses the family name `icons`, so load one subset per page.
- [Codepoints](/guide/features/codepoints-and-ligatures#how-codepoints-are-assigned) depend on the set, so the CSS and the font must come from the same subset, and codepoints stored elsewhere don't carry over between subsets.

## Watch mode

Generate once with `incremental: true`, then update the result whenever the folder changes. Only the changed icons are processed again. Without change hints, `regenerateAsync()` re-reads the files and works out what changed, including additions and removals.

```ts [scripts/watch-icons.ts]
import { glob, watch } from 'node:fs/promises';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const listIcons = async () => ({ files: (await Array.fromAsync(glob('icons/*.svg'))).sort() });

let result = await generateWebfonts({ ...(await listIcons()), dest: 'dist/fonts', types: ['woff2'], incremental: true });

for await (const event of watch('icons')) {
    if (!event.filename?.endsWith('.svg')) continue;
    try {
        result = await result.regenerateAsync(await listIcons());
        console.log(`Rebuilt after ${event.eventType} of ${event.filename}`);
    } catch (error) {
        // The previous result stays valid; the next change retries.
        console.error(error);
    }
}
```

Each update waits for the previous one, since a result can only have one update in flight. See [updating a result](/node/api#updating-a-result) for the single-file methods and change hints.

## Multi-weight family

Combine light and bold drawings into one font per format. Matching file names identify the same icon in each design.

```ts [scripts/build-icons.ts]
import { glob } from 'node:fs/promises';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const icons = async (dir: string) => (await Array.fromAsync(glob(`icons/${dir}/*.svg`))).sort();

await generateWebfonts({
    dest: 'dist/fonts',
    fontName: 'app-icons',
    variants: [
        { name: 'light', files: await icons('light'), weight: 300, default: true },
        { name: 'bold', files: await icons('bold'), weight: 700 },
    ],
    missingGlyphs: { behavior: 'fallback', variant: 'light' },
});
```

```html
<i class="icon icon-add"></i> <i class="icon icon-add icon--bold"></i>
```

An icon missing from the bold design uses its light drawing there, so the light design must contain every icon. See [variants](/guide/features/variants).

## Custom class names

Change the base selector and class prefix through `templateOptions`, without writing a template:

```ts
await generateWebfonts({
    files,
    dest: 'dist/fonts',
    templateOptions: { baseSelector: '.ui-icon', classPrefix: 'ui-icon-' },
});
```

```html
<i class="ui-icon ui-icon-add"></i>
```

## Custom template

Start from a built-in template: copy it into your project, edit it, and point `cssTemplate` at the copy.

```ts [scripts/build-icons.ts]
import { copyFile } from 'node:fs/promises';
import * as templates from '@atlowchemi/webfont-generator/templates';

// Once, to start from the built-in SCSS template:
await copyFile(templates.scss, 'icons.scss.hbs');
```

```ts
await generateWebfonts({
    files,
    dest: 'dist/fonts',
    cssTemplate: 'icons.scss.hbs',
    cssDest: 'src/styles/_icons.scss',
});
```

See [Templates](/guide/templates) for the context every template receives.

## Type-safe icon names

Write a TypeScript module with the icon names and their union type next to the fonts. See [TypeScript output](/guide/templates#typescript) for a React and a Vue component that use it.

```ts
await generateWebfonts({
    files,
    dest: 'dist/fonts',
    ts: true,
    tsDest: 'src/generated/icons.ts',
});
```

## Cache in CI

Keep the [engine cache](/guide/features/caching) between CI runs, so unchanged icons and fonts aren't rebuilt:

```ts
await generateWebfonts({ files, dest: 'dist/fonts', cacheLocation: '.webfont-cache' });
```

Then restore `.webfont-cache` in your CI's cache step; see [keep it between CI runs](/guide/features/caching#keep-it-between-ci-runs) for GitHub Actions, GitLab CI, and Docker.
