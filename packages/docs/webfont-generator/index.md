---
description: Overview of webfont-generator, the Rust engine behind vite-svg-2-webfont, available as a Rust crate, CLI, Node.js package, and browser package.
---

<p align="center">
  <img src="/webfont-generator-logo.png" alt="webfont-generator logo" width="200" />
</p>

# Webfont Generator

`webfont-generator` is a Rust engine that generates webfonts (SVG, TTF, EOT, WOFF, WOFF2) from SVG icons, together with matching CSS and an HTML preview. It is the engine that powers `vite-svg-2-webfont`, and you can also use it directly from Rust, a terminal, Node.js, or the browser.

## Packages and runtimes

| Use case                    | Package                                                                                        | Install                                             | Runtime                                       |
| --------------------------- | ---------------------------------------------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------- |
| Vite integration            | [`vite-svg-2-webfont`](https://www.npmjs.com/package/vite-svg-2-webfont)                       | `npm install -D vite-svg-2-webfont`                 | Node.js; uses the npm package below           |
| JavaScript / TypeScript API | [`@atlowchemi/webfont-generator`](https://www.npmjs.com/package/@atlowchemi/webfont-generator) | `npm install @atlowchemi/webfont-generator`         | Node.js with a platform-specific native addon |
| Browser API                 | `@atlowchemi/webfont-browser-generator`                                                        | `npm install @atlowchemi/webfont-browser-generator` | Browsers; WebAssembly in a module worker      |
| Rust library                | [`webfont-generator`](https://crates.io/crates/webfont-generator)                              | `cargo add webfont-generator`                       | Rust; no Node.js dependency                   |
| Standalone command          | [`webfont-generator`](https://crates.io/crates/webfont-generator)                              | `cargo install webfont-generator --features cli`    | Native CLI; no Node.js dependency             |

Not sure which one fits? See [Which package do I need?](/which-package). Usage for each interface: [Node.js](./node), [Browser](./browser), [Rust](./rust), [CLI](./cli).

## Highlights

- **Every common format:** SVG, TTF, EOT, WOFF, and WOFF2 from one set of SVG icons, plus CSS, SCSS, and HTML preview templates.
- **Multi-weight families:** light, regular, and bold designs of the same icons combined into one font per format. See [multi-variant fonts](./node#multi-variant-fonts).
- **Color glyphs:** optionally keep solid SVG colors. See [color glyphs](#color-glyphs).
- **Fast rebuilds:** [incremental generation](./node#incremental) reuses work from the previous build when only a few icons change.
- **No filesystem required:** generate from SVG text in memory, which is what the [browser package](./browser) uses.
- **Fast:** over 6× faster on average than `@vusion/webfonts-generator` in our benchmarks. See [performance](./under-the-hood#performance).

## Why a new engine

The JavaScript lineage started with [`webfonts-generator`](https://github.com/sunflowerdeath/webfonts-generator) by sunflowerdeath, which the Vusion team forked as [`@vusion/webfonts-generator`](https://github.com/vusion/webfonts-generator). Neither project has received updates in years, and both rely on a JavaScript pipeline spanning several separate packages. `webfont-generator` is a ground-up rewrite in Rust that keeps the familiar API while delivering better performance and long-term maintainability. [Under the hood](./under-the-hood) describes the architecture and benchmarks.

::: tip Attribution
The API design and Handlebars template system build on sunflowerdeath's original `webfonts-generator` and the Vusion team's `@vusion/webfonts-generator` fork. Credit to sunflowerdeath, the Vusion team, and the contributors to both projects for that foundation.
:::

## Compatibility

The API is largely compatible with upstream `@vusion/webfonts-generator`, with a few documented differences:

- The `cssContext` and `htmlContext` callbacks receive only the context object; the additional `options` and Handlebars instance arguments are no longer available.
- `formatOptions` is strictly typed, and the `eot` key is no longer accepted; EOT is derived from the TTF output.
- [`optimizeOutput`](./node#optimizeoutput) adds optional SVG path optimization, disabled by default.
- [`formatOptions.woff2.compressionQuality`](./node#formatoptions) controls WOFF2 compression quality from 0–11 (default 11), allowing faster encoding at the cost of slightly larger output.
- [`incremental`](./node#incremental) enables regeneration that reuses work from the previous build, disabled by default.
- Font binaries differ at the byte level (different TTF compiler, different path normalization) but are valid and render identically
- CSS, HTML, and template output is identical.
- [`variants`](./node#multi-variant-fonts) adds multi-weight families with discrete designs.
- [`colorGlyphs`](./node#colorglyphs) preserves solid SVG colors for selected glyphs in TTF, WOFF, and WOFF2, disabled by default.

Like upstream, the npm package always writes the same fixed creation timestamp into TTF output and ignores `formatOptions.ttf.ts`, so repeated builds produce identical bytes. The [Rust crate](./rust) and the [browser package](./browser) honor `ts`.

::: warning
If you are migrating from `@vusion/webfonts-generator`, review the [Node.js usage](./node) page for the full options reference.
:::

## Color glyphs

Color generation is opt-in. It preserves solid SVG fills in COLR v1/CPAL tables and retains a monochrome `glyf` fallback in the same font. It supports ordinary fonts and discrete multi-weight families. This option is available in the [Node API](./node#color-selection), [Rust API](./rust#color-glyphs), and [CLI manifest](./cli#json-manifest).

Active color supports **TTF, WOFF, and WOFF2**. SVG-font and EOT output are rejected during option resolution. Ordinary defaults include EOT, so ordinary color callers must specify compatible `types`. Variant defaults are already WOFF and WOFF2.

### SVG paint

| SVG source                                               | Color glyph paint                         |
| -------------------------------------------------------- | ----------------------------------------- |
| No authored effective fill                               | Host text color                           |
| Authored solid fill, including inherited root/group fill | Fixed SVG color                           |
| `fill="currentColor"` without effective authored `color` | Host text color                           |
| `fill="currentColor"` with effective authored `color`    | Fixed resolved SVG color                  |
| Solid fill with `fill-opacity`                           | That paint with independent layer opacity |

Source paint order, curves, nonzero winding, and ordinary nested evenodd holes are supported. Color layers and monochrome fallback share positioning and normalization. Authored inline styles and internal stylesheets use the SVG parser's cascade.

Advanced input is **best-effort**: gradients, strokes as color paint, text, images, clipping, masks, filters, paint-dependent selectors, CSS resets/invalid declarations, and intersecting evenodd contours can produce incomplete or incorrect results. There is no dedicated unsupported-input detection, warning, or rejection. Please report a minimal SVG reproduction when a supported case renders incorrectly.

### Browser and platform compatibility

Chromium and Firefox render fixed colors, host foreground colors, and per-layer opacity. Linux WebKit also renders color; tested macOS WebKit uses the monochrome fallback.

WebKit behavior depends on the operating system and platform text-rendering libraries. In the tested macOS configuration, the font still loads: the fallback retains host text color, selected weight, ligatures, and advances, but loses fixed colors and independent per-layer opacity. It uses the same font URL; no separate API or alternate resource is needed. These observations do not imply a permanent limitation in future Safari, WebKit, or macOS versions. See [WebKit issue 233496](https://bugs.webkit.org/show_bug.cgi?id=233496).

Color layers add physical glyphs and increase font size. Fonts must fit the 65,535-glyph limit and the `post` table's 65,278 unique custom-name limit; exceeding either returns a build error.

## Links

- [npm package](https://www.npmjs.com/package/@atlowchemi/webfont-generator)
- [crates.io](https://crates.io/crates/webfont-generator)
- [docs.rs](https://docs.rs/webfont-generator)
- [GitHub repository](https://github.com/atlowChemi/vite-svg-2-webfont)

## Next steps

- [Node.js usage](./node) -- npm package API reference
- [Browser usage](./browser) -- WebAssembly package API reference
- [Rust usage](./rust) -- crate API reference
- [CLI usage](./cli) -- command-line interface
- [Under the hood](./under-the-hood) -- architecture and performance
- [Changelog](./changelog) -- release history
