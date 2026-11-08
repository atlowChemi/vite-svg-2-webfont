---
description: Install vite-svg-2-webfont and add it to your Vite app with the default virtual stylesheet workflow.
---

# Vite Plugin

`vite-svg-2-webfont` turns a folder of SVG icons into an icon font as part of your Vite dev server and build, and serves it through a virtual stylesheet. It runs the fonte engine through the [Node.js package](/node/).

Use it when your app is built with Vite and its icons live in the repository. Editing an icon reloads it in the dev server.

## Installation

Install `vite-svg-2-webfont` as a development dependency in the app that will consume the plugin.

::: code-group

```sh [npm]
npm install --save-dev vite-svg-2-webfont
```

```sh [pnpm]
pnpm add --save-dev vite-svg-2-webfont
```

```sh [yarn]
yarn add --dev vite-svg-2-webfont
```

```sh [bun]
bun add -D vite-svg-2-webfont
```

:::

## Add the plugin to Vite

Register the plugin in your `vite.config.ts` and point `context` at the directory that contains the SVG icons you want to include in the webfont.

```ts{7-9} [vite.config.ts]
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default defineConfig({
    plugins: [
        viteSvgToWebfont({
            context: resolve(import.meta.dirname, 'icons'),
        }),
    ],
});
```

## Import the virtual stylesheet

Import the generated virtual CSS module once in your app entrypoint.

```ts [main.ts]
import 'virtual:vite-svg-2-webfont.css';
```

## Use generated icon classes

SVG file names become icon class names. With the default settings, `add.svg` becomes `icon-add`, the class prefix comes from [`classPrefix`](/vite/configuration#classprefix), and the base font selector comes from [`baseSelector`](/vite/configuration#baseselector).

```html [index.html]
<i class="icon icon-add"></i>
```

## Next steps

- Continue to [Usage](/vite/usage) for the full flow
- Review [Configuration](/vite/configuration) for all available options
- Review [Public API](/vite/public-api) if you want to consume generated font metadata from another plugin
