# vite-svg-2-webfont

<p align="center">
  <img src="../docs/public/logo.svg" alt="vite-svg-2-webfont logo" width="144" />
</p>

[![npm](https://img.shields.io/npm/v/vite-svg-2-webfont.svg?style=flat-square)](https://www.npmjs.com/package/vite-svg-2-webfont)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/atlowChemi/vite-svg-2-webfont/main.yaml?branch=main&style=flat-square)
[![docs](https://img.shields.io/badge/docs-online-646cff?style=flat-square)](https://atlowChemi.github.io/vite-svg-2-webfont/)
[![npm](https://img.shields.io/npm/dm/vite-svg-2-webfont.svg?style=flat-square)](https://www.npmjs.com/package/vite-svg-2-webfont)
[![license](https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square)](https://github.com/atlowChemi/vite-svg-2-webfont/blob/master/LICENSE)
[![npm bundle size](https://img.shields.io/bundlephobia/minzip/vite-svg-2-webfont?style=flat-square)](https://img.shields.io/bundlephobia/minzip/vite-svg-2-webfont?style=flat-square)
[![node engine](https://img.shields.io/node/v/vite-svg-2-webfont?style=flat-square)](https://img.shields.io/node/v/vite-svg-2-webfont?style=flat-square)
[![Package Quality](https://packagequality.com/shield/vite-svg-2-webfont.svg)](https://packagequality.com/#?package=vite-svg-2-webfont)

A Vite plugin that generates webfonts from SVG icon files and exposes a virtual stylesheet you can import into your application.

## Documentation

Full documentation lives at [atlowChemi.github.io/vite-svg-2-webfont](https://atlowChemi.github.io/vite-svg-2-webfont/).

- [Getting Started](https://atlowChemi.github.io/vite-svg-2-webfont/getting-started)
- [Usage](https://atlowChemi.github.io/vite-svg-2-webfont/usage)
- [Configuration](https://atlowChemi.github.io/vite-svg-2-webfont/configuration)
- [Public API](https://atlowChemi.github.io/vite-svg-2-webfont/public-api)

## Installation

```bash
npm install --save-dev vite-svg-2-webfont
```

## Quick Start

```ts
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

```ts
import 'virtual:vite-svg-2-webfont.css';
```

```html
<i class="icon icon-add"></i>
```

In that example, `icon-add` is controlled by `classPrefix` and `icon` is controlled by `baseSelector`. See the [configuration reference](https://atlowChemi.github.io/vite-svg-2-webfont/configuration#classprefix) and [base selector option](https://atlowChemi.github.io/vite-svg-2-webfont/configuration#baseselector).
