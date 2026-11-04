---
description: Generate icon fonts in the browser with @atlowchemi/webfont-browser-generator, the fonte engine compiled to WebAssembly.
---

# Browser

`@atlowchemi/webfont-browser-generator` wraps the Rust engine with wasm-bindgen and runs
generation inside dedicated module workers. The package ships compiled JavaScript, types,
worker code and WASM; applications need no Rust tools or install-time compilation.

Use it when your users make the fonts, for example in an icon editor or a font builder. Everything runs on their device, with no server and no file system.

::: tip Try it
The [Playground](/playground) runs this package in your browser: drop in SVGs, adjust the options, and download the fonts.
:::

## Installation

::: code-group

```sh [npm]
npm install @atlowchemi/webfont-browser-generator
```

```sh [pnpm]
pnpm add @atlowchemi/webfont-browser-generator
```

```sh [yarn]
yarn add @atlowchemi/webfont-browser-generator
```

```sh [bun]
bun add @atlowchemi/webfont-browser-generator
```

:::

## Your first font

Import `generateWebfonts` in your application's client code. Bundlers such as Vite and webpack emit the package's worker and WebAssembly files for you, and it also works without a bundler; see [how to use](/browser/usage#how-to-use).

```ts [src/main.ts]
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

using result = await generateWebfonts({
    sources: [{ id: 'square', name: 'square', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['woff2'],
});
const bytes = await result.font('woff2');
const css = await result.generateCss();
```

## Next steps

- [Usage](/browser/usage): using the output, caching between sessions, updates, and hosting the worker.
- [Examples](/browser/examples): dropped files, a live preview, live editing, and an IndexedDB cache.
- [Configuration](/browser/configuration): every option.
- [API](/browser/api): the result, its methods, and the TypeScript types.
- [Playground](/playground)
- [Changelog](/browser/changelog)
