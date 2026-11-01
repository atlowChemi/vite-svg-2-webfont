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

Set `writeFiles: false` to keep everything on the result, for example to serve the fonts from your own server. `generateCss()` takes the URLs the fonts are served at.

```ts [server.ts]
import { createServer } from 'node:http';
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    files: ['icons/add.svg', 'icons/remove.svg'],
    dest: 'unused',
    types: ['woff2'],
    writeFiles: false,
});
const css = result.generateCss({ woff2: '/icons.woff2' });

createServer((request, response) => {
    if (request.url === '/icons.woff2') {
        response.writeHead(200, { 'Content-Type': 'font/woff2' }).end(result.woff2);
    } else if (request.url === '/icons.css') {
        response.writeHead(200, { 'Content-Type': 'text/css' }).end(css);
    } else {
        response.writeHead(404).end();
    }
}).listen(3000);
```

`dest` is still required, but nothing is written to it.

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
