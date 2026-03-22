---
description: Install vite-svg-2-webfont and add it to your Vite app with the default virtual stylesheet workflow.
---

# Getting Started

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
            context: resolve(__dirname, 'icons'),
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

SVG file names become icon class names. With the default settings, `add.svg` becomes `icon-add`, the class prefix comes from [`classPrefix`](/configuration#classprefix), and the base font selector comes from [`baseSelector`](/configuration#baseselector).

```html [index.html]
<i class="icon icon-add"></i>
```

## Next steps

- Continue to [Usage](/usage) for the full flow
- Review [Configuration](/configuration) for all available options
- Review [Public API](/public-api) if you want to consume generated font metadata from another plugin
