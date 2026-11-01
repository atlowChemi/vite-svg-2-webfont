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

Import `generateWebfonts` in your application's client code. With Vite, the worker and WASM assets are bundled automatically.

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

- [Usage](/browser/usage): using the output, result methods, persistent caching, incremental updates, and hosting the worker.
- [Playground](/playground)
- [Changelog](/browser/changelog)
