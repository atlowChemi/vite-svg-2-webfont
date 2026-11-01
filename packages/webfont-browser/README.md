# @atlowchemi/webfont-browser-generator

<p align="center">
  <img src="../docs/assets/brand/browser.svg" alt="fonte for the browser" width="160" />
</p>

[![npm](https://img.shields.io/npm/v/@atlowchemi/webfont-browser-generator.svg?style=flat-square)](https://www.npmjs.com/package/@atlowchemi/webfont-browser-generator)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/atlowChemi/vite-svg-2-webfont/main.yaml?branch=main&style=flat-square)
[![docs](https://img.shields.io/badge/docs-fonte.atlow.dev-ff6a13?style=flat-square)](https://fonte.atlow.dev/browser/)
[![license](https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square)](https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE)

The [fonte](https://fonte.atlow.dev/) engine in the browser. It generates icon fonts from SVG text in dedicated module workers, powered by precompiled WebAssembly, so applications need no Rust tools or install-time compilation.

Use it when your users make the fonts, for example in an icon editor or a font builder. Everything runs on their device, with no server and no file system. The [Playground](https://fonte.atlow.dev/playground) runs this package.

## Installation

```bash
npm install @atlowchemi/webfont-browser-generator
```

## Quick start

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

using result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
});
const bytes = await result.font('woff2');
const css = await result.generateCss();
```

Bundlers such as Vite and webpack emit the package's worker and WebAssembly files for you, and it also works without a bundler; see [how to use](https://fonte.atlow.dev/browser/usage#how-to-use).

## Documentation

- [Getting started](https://fonte.atlow.dev/browser/)
- [Usage](https://fonte.atlow.dev/browser/usage): using the output, the [persistent cache](https://fonte.atlow.dev/browser/usage#persistent-cache), [incremental updates](https://fonte.atlow.dev/browser/usage#incremental-updates), [worker hosting](https://fonte.atlow.dev/browser/usage#worker-lifetime-and-hosting), and how to [reuse one worker](https://fonte.atlow.dev/browser/usage#reuse-one-worker)
- [Examples](https://fonte.atlow.dev/browser/examples): dropped files, a live preview, live editing, and an IndexedDB cache
- [Configuration](https://fonte.atlow.dev/browser/configuration): every option
- [API](https://fonte.atlow.dev/browser/api): the result, its methods, and the TypeScript types

## License

[MIT](https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE)
