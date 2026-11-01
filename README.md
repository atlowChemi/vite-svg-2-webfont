<p align="center">
  <img src="./packages/docs/assets/brand/fonte.svg" alt="fonte" height="96" />
</p>

<p align="center">SVG icons, cast into fonts.</p>

<p align="center">
  <a href="https://fonte.atlow.dev/"><img src="https://img.shields.io/badge/docs-fonte.atlow.dev-ff6a13?style=flat-square" alt="docs" /></a>
  <a href="https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE"><img src="https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square" alt="license" /></a>
</p>

fonte turns SVG icons into icon fonts: SVG, TTF, EOT, WOFF, and WOFF2, with the CSS, an HTML preview, and TypeScript names to go with them. Every tool runs the same Rust engine, so the fonts they produce are the same; they differ in where they run.

## Tools

| Tool                                         | Package                                                                  | Pick it when                                                                     |
| -------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| [Vite plugin](https://fonte.atlow.dev/vite/) | [`vite-svg-2-webfont`](./packages/vite-svg-2-webfont)                    | Your app uses Vite and the icons live in your repository                         |
| [Node.js](https://fonte.atlow.dev/node/)     | [`@atlowchemi/webfont-generator`](./packages/webfont-generator)          | You need fonts from a Node.js build script, another bundler, or your own tooling |
| [Browser](https://fonte.atlow.dev/browser/)  | [`@atlowchemi/webfont-browser-generator`](./packages/webfont-browser)    | Users create fonts in your web app, such as an icon editor or a font builder     |
| [CLI](https://fonte.atlow.dev/cli/)          | [`webfont-generator`](./crates/webfont-generator) with the `cli` feature | You want fonts from a shell script or CI job, without writing code               |
| [Rust](https://fonte.atlow.dev/rust/)        | [`webfont-generator`](./crates/webfont-generator)                        | You are writing Rust, or want generation without a Node.js runtime               |

Not sure which one? See [choosing a tool](https://fonte.atlow.dev/guide/).

## Documentation

Everything is documented at [fonte.atlow.dev](https://fonte.atlow.dev/): a getting-started page, usage, examples, and a configuration reference for each tool, and a [shared guide](https://fonte.atlow.dev/guide/) for what they have in common, such as [color glyphs](https://fonte.atlow.dev/guide/features/color-glyphs), [multi-weight families](https://fonte.atlow.dev/guide/features/variants), and the [engine cache](https://fonte.atlow.dev/guide/features/caching). The [Playground](https://fonte.atlow.dev/playground) runs the engine in your browser.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

[MIT](./LICENSE)
