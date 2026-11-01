# vite-svg-2-webfont

<p align="center">
  <img src="../docs/assets/brand/vite.svg" alt="fonte for Vite" width="160" />
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/vite-svg-2-webfont"><img src="https://img.shields.io/npm/v/vite-svg-2-webfont.svg?style=flat-square" alt="npm" /></a>
  <a href="https://github.com/atlowChemi/vite-svg-2-webfont/actions/workflows/main.yaml"><img src="https://img.shields.io/github/actions/workflow/status/atlowChemi/vite-svg-2-webfont/main.yaml?branch=main&amp;style=flat-square" alt="build" /></a>
  <a href="https://fonte.atlow.dev/vite/"><img src="https://img.shields.io/badge/docs-fonte.atlow.dev-ff6a13?style=flat-square" alt="docs" /></a>
  <a href="https://www.npmjs.com/package/vite-svg-2-webfont"><img src="https://img.shields.io/npm/dm/vite-svg-2-webfont.svg?style=flat-square" alt="downloads" /></a>
  <a href="https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE"><img src="https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square" alt="license" /></a>
  <a href="https://bundlephobia.com/package/vite-svg-2-webfont"><img src="https://img.shields.io/bundlephobia/minzip/vite-svg-2-webfont?style=flat-square" alt="npm bundle size" /></a>
  <a href="https://www.npmjs.com/package/vite-svg-2-webfont"><img src="https://img.shields.io/node/v/vite-svg-2-webfont?style=flat-square" alt="node engine" /></a>
  <a href="https://packagequality.com/#?package=vite-svg-2-webfont"><img src="https://packagequality.com/shield/vite-svg-2-webfont.svg" alt="Package Quality" /></a>
</p>

The [fonte](https://fonte.atlow.dev/) Vite plugin. It turns a folder of SVG icons into an icon font as part of your Vite dev server and build, and serves it through a virtual stylesheet. Editing an icon reloads it in the dev server.

## Installation

```bash
npm install --save-dev vite-svg-2-webfont
```

## Quick start

Register the plugin in `vite.config.ts` and point `context` at the directory with your SVG icons:

```ts
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

Import the generated stylesheet once in your app's entrypoint:

```ts
import 'virtual:vite-svg-2-webfont.css';
```

SVG file names become icon class names, so `add.svg` becomes `icon-add`:

```html
<i class="icon icon-add"></i>
```

The `icon-` prefix comes from [`classPrefix`](https://fonte.atlow.dev/vite/configuration#classprefix) and the `icon` class from [`baseSelector`](https://fonte.atlow.dev/vite/configuration#baseselector).

## Documentation

- [Getting started](https://fonte.atlow.dev/vite/)
- [Usage](https://fonte.atlow.dev/vite/usage), including [multi-weight icon families](https://fonte.atlow.dev/vite/usage#multi-weight-icon-families) and [multiple plugin instances](https://fonte.atlow.dev/vite/usage#multiple-plugin-instances)
- [Examples](https://fonte.atlow.dev/vite/examples), each with its generated output
- [Configuration](https://fonte.atlow.dev/vite/configuration): every option
- [How it works](https://fonte.atlow.dev/vite/how-it-works): development, builds, and caching
- [Public API](https://fonte.atlow.dev/vite/public-api) for other plugins
- [Migration](https://fonte.atlow.dev/vite/migration) between major versions

## License

[MIT](https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE)
