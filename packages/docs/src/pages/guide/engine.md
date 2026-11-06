---
description: The Rust engine behind every fonte tool, what it supports, and why it was written.
---

# The Engine

Every fonte tool runs the same engine: `webfont-generator`, written in Rust. It generates webfonts (SVG, TTF, EOT, WOFF, WOFF2) from SVG icons, together with matching CSS, an HTML preview, and TypeScript metadata.

## Ways to use it

The engine ships as five tools: the [Vite plugin](/vite/), the [Node.js package](/node/), the [browser package](/browser/), the [Rust crate](/rust/), and the [CLI](/cli/). Each tool's page covers installing it.

Not sure which one fits? [Choosing a tool](/guide/) compares what each one supports and what it runs on.

## Highlights

- **Every common format:** SVG, TTF, EOT, WOFF, and WOFF2 from one set of SVG icons, plus CSS, SCSS, and an HTML preview from templates, and [TypeScript](/guide/templates#typescript) with the icon names and their union type.
- **Multi-weight families:** light, regular, and bold designs of the same icons combined into one font per format. See [variants](/guide/features/variants).
- **Color glyphs:** optionally keep solid SVG colors. See [color glyphs](#color-glyphs).
- **Fast rebuilds:** [incremental generation](/guide/features/incremental) reuses work from the previous build when only a few icons change.
- **Warm starts:** an opt-in [persistent cache](/guide/features/caching) lets a new process reuse work from earlier runs for unchanged icons and formats.
- **No filesystem required:** generate from SVG text in memory, which is what the [browser package](/browser/) uses.
- **Fast:** average speedups of 6.5× on macOS and 7.6× on Linux over `@vusion/webfonts-generator` in our measured default-setting workloads without caching. With a warm [persistent cache](/guide/under-the-hood#persistent-cache), unchanged rebuilds are 43× (macOS) and 113× (Linux) faster than Vusion. See [performance](/guide/under-the-hood#performance).

## Why a new engine

The JavaScript lineage started with [`webfonts-generator`](https://github.com/sunflowerdeath/webfonts-generator) by sunflowerdeath, which the Vusion team forked as [`@vusion/webfonts-generator`](https://github.com/vusion/webfonts-generator). Neither project has received updates in years, and both rely on a JavaScript pipeline spanning several separate packages. `webfont-generator` is a ground-up rewrite in Rust that keeps the familiar API while delivering better performance and long-term maintainability. [Under the hood](/guide/under-the-hood) describes the architecture and benchmarks.

::: tip Attribution
The API design and Handlebars template system build on sunflowerdeath's original `webfonts-generator` and the Vusion team's `@vusion/webfonts-generator` fork. Credit to sunflowerdeath, the Vusion team, and the contributors to both projects for that foundation.
:::

## Compatibility

The API is largely compatible with upstream `@vusion/webfonts-generator`, with a few documented differences:

- The `cssContext` and `htmlContext` callbacks receive only the context object; the additional `options` and Handlebars instance arguments are no longer available.
- `formatOptions` is strictly typed, and the `eot` key is no longer accepted; EOT is derived from the TTF output.
- [`optimizeOutput`](/node/configuration#optimizeoutput) adds optional SVG path optimization, disabled by default.
- [`formatOptions.woff2.compressionQuality`](/node/configuration#formatoptions) controls WOFF2 compression quality from 0–11 (default 11), allowing faster encoding at the cost of slightly larger output.
- [`incremental`](/guide/features/incremental) enables regeneration that reuses work from the previous build, disabled by default.
- [`cacheLocation`](/guide/features/caching) enables a persistent cache shared across runs, disabled by default.
- Font binaries differ at the byte level (different TTF compiler, different path normalization) but are valid and render identically
- CSS, HTML, and template output is identical.
- [`variants`](/guide/features/variants) adds multi-weight families with discrete designs.
- [`colorGlyphs`](/guide/features/color-glyphs) preserves solid SVG colors for selected glyphs in TTF, WOFF, and WOFF2, disabled by default.

Like upstream, the npm package always writes the same fixed creation timestamp into TTF output and ignores `formatOptions.ttf.ts`, so repeated builds produce identical bytes. The [Rust crate](/rust/) and the [browser package](/browser/) honor `ts`.

::: warning
If you are migrating from `@vusion/webfonts-generator`, review the [Node.js usage](/node/) page for the full options reference.
:::

## Color glyphs

The engine can keep the solid colors of selected icons as COLR/CPAL color glyphs, with a single-color fallback in the same font. See [color glyphs](/guide/features/color-glyphs) for selecting icons, theme palettes, and browser support.

## Links

- [npm package](https://www.npmjs.com/package/@atlowchemi/webfont-generator)
- [crates.io](https://crates.io/crates/webfont-generator)
- [docs.rs](https://docs.rs/webfont-generator)
- [GitHub repository](https://github.com/atlowChemi/vite-svg-2-webfont)

## Next steps

- [Node.js usage](/node/) -- npm package API reference
- [Browser usage](/browser/) -- WebAssembly package API reference
- [Rust usage](/rust/) -- crate API reference
- [CLI usage](/cli/) -- command-line interface
- [Under the hood](/guide/under-the-hood) -- architecture and performance
- [Changelog](/node/changelog) -- release history
