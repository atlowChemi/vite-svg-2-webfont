# Changelog

## [0.12.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-engine-v0.11.0...webfont-engine-v0.12.0) (2026-11-01)

### Features

- **webfont-generator:** persist incremental cache updates automatically ([#658](https://github.com/atlowChemi/vite-svg-2-webfont/issues/658)) ([ccec5e2](https://github.com/atlowChemi/vite-svg-2-webfont/commit/ccec5e26db59780fc474bfad8166f1979ce09f29))

### Bug Fixes

- **deps:** update rust crate brotli to v9.1.1 ([#651](https://github.com/atlowChemi/vite-svg-2-webfont/issues/651)) ([a7bb626](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a7bb62652d06b7f238678dfcc6e53003b47ef9ef))
- **docs:** serve the docs site from fonte.atlow.dev ([#663](https://github.com/atlowChemi/vite-svg-2-webfont/issues/663)) ([3e565f0](https://github.com/atlowChemi/vite-svg-2-webfont/commit/3e565f04c8f49c3879e6eebb39916276c36c2c1e))

## [0.11.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-engine-v0.10.0...webfont-engine-v0.11.0) (2026-10-28)

### ⚠ BREAKING CHANGES

- **webfont-generator:** CLI inputs must be explicit paths. Bare names such as `icons`, `icons/` or `plus.svg` are rejected; write `./icons`, `./icons/` or `./plus.svg` instead. `clear-cache` and `help` are subcommands.

### Features

- **webfont-generator:** add a threads option ([#633](https://github.com/atlowChemi/vite-svg-2-webfont/issues/633)) ([a1264c3](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a1264c31917dea40623aa9ff8d40bd2674af9aa6))
- **webfont-generator:** add clear-cache CLI subcommand ([#610](https://github.com/atlowChemi/vite-svg-2-webfont/issues/610)) ([c6ca79a](https://github.com/atlowChemi/vite-svg-2-webfont/commit/c6ca79a8ff0565f4f2d72973b4e4884809196103))
- **webfont-generator:** add fixed TypeScript output to the engine and CLI ([#640](https://github.com/atlowChemi/vite-svg-2-webfont/issues/640)) ([689530e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/689530ed524f083e9303d0b84f43a42d718a4771))
- **webfont-generator:** add persistent generation cache ([#609](https://github.com/atlowChemi/vite-svg-2-webfont/issues/609)) ([17837c2](https://github.com/atlowChemi/vite-svg-2-webfont/commit/17837c247a524500c22d32d8176e14ba07ea2fd2))
- **webfont-generator:** cache color glyphs ([#618](https://github.com/atlowChemi/vite-svg-2-webfont/issues/618)) ([565d9f1](https://github.com/atlowChemi/vite-svg-2-webfont/commit/565d9f159a34c6461afb9f4dedd13a95ce1fb3c9))
- **webfont-generator:** cache variant families ([#614](https://github.com/atlowChemi/vite-svg-2-webfont/issues/614)) ([8b813a2](https://github.com/atlowChemi/vite-svg-2-webfont/commit/8b813a2dca51ed78df1be9d4c13c0d72d09f6504))
- **webfont-generator:** reuse cache bytes in in-memory and browser generation ([#612](https://github.com/atlowChemi/vite-svg-2-webfont/issues/612)) ([327b1a8](https://github.com/atlowChemi/vite-svg-2-webfont/commit/327b1a86a3a719e73063ad23b6354528c4f0de13))

### Bug Fixes

- **webfont-generator:** update font dependencies ([#628](https://github.com/atlowChemi/vite-svg-2-webfont/issues/628)) ([173074e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/173074ef3e383ae0b0bfa194c233e72080d9c675))

### Performance Improvements

- **webfont-generator:** read SVG files in bounded chunks ([#631](https://github.com/atlowChemi/vite-svg-2-webfont/issues/631)) ([89c8b59](https://github.com/atlowChemi/vite-svg-2-webfont/commit/89c8b59483f7f87dad539e07d5aa70fccaeea453))
- **webfont-generator:** reduce optimized glyph finalization time by 15-21% in CI benchmarks ([edf8a5f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/edf8a5f786b23f8b760690c3594f6b95522c3a46))
- **webfont-generator:** speed up warm cache runs by decoding saved glyphs lazily ([#619](https://github.com/atlowChemi/vite-svg-2-webfont/issues/619)) ([190e9e2](https://github.com/atlowChemi/vite-svg-2-webfont/commit/190e9e29ec1a183c3c59d6d26eee8222f37ffc2b))
- **webfont-generator:** use zlib-rs for WOFF compression ([#639](https://github.com/atlowChemi/vite-svg-2-webfont/issues/639)) ([e1c5046](https://github.com/atlowChemi/vite-svg-2-webfont/commit/e1c50469e1ec36b5e4bf85ea6dd77d659b3c660c))

## [0.10.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-engine-v0.9.0...webfont-engine-v0.10.0) (2026-10-18)

### Features

- **webfont-generator:** add browser WASM generator ([#568](https://github.com/atlowChemi/vite-svg-2-webfont/issues/568)) ([50450c6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/50450c69d8b2d261ebeaa7e0e3eac59354d59772))
- **webfont-generator:** add filesystem-free Rust generation and updates ([#561](https://github.com/atlowChemi/vite-svg-2-webfont/issues/561)) ([ffb54d4](https://github.com/atlowChemi/vite-svg-2-webfont/commit/ffb54d4bff76e421836964b36cf48ad7da9680dc))
- **webfont-generator:** add incremental source helpers ([#573](https://github.com/atlowChemi/vite-svg-2-webfont/issues/573)) ([040e808](https://github.com/atlowChemi/vite-svg-2-webfont/commit/040e8084e5cb54d72c1ea02a6699d57991a45ae7))
- **webfont-generator:** add named color slots for theme palettes ([#591](https://github.com/atlowChemi/vite-svg-2-webfont/issues/591)) ([75faee6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/75faee6a178614f4c7e3710a36ab736064894934))
- **webfont-generator:** split engine API features ([#565](https://github.com/atlowChemi/vite-svg-2-webfont/issues/565)) ([4a0fc76](https://github.com/atlowChemi/vite-svg-2-webfont/commit/4a0fc765aeeda0752a109e3153150d07c57e8fd6))

### Bug Fixes

- **webfont-generator:** include woff2 compression quality in asset hash ([#566](https://github.com/atlowChemi/vite-svg-2-webfont/issues/566)) ([923d9ee](https://github.com/atlowChemi/vite-svg-2-webfont/commit/923d9eea0805e4665f47cdb0efa031525e9f1421))

## [0.9.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-engine-v0.8.0...webfont-engine-v0.9.0) (2026-10-09)

### Features

- **webfont-generator:** add internal color glyph paint pipeline ([#530](https://github.com/atlowChemi/vite-svg-2-webfont/issues/530)) ([17a99de](https://github.com/atlowChemi/vite-svg-2-webfont/commit/17a99de4dc8144521f1c0ff1f32f2dabf618c878))
- **webfont-generator:** emit COLR v1 fonts with paint-aware glyph identity ([#542](https://github.com/atlowChemi/vite-svg-2-webfont/issues/542)) ([d85448f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/d85448f2698f93d758e75a70568d4b2ed026e376))
- **webfont-generator:** expose selective color glyph generation ([#546](https://github.com/atlowChemi/vite-svg-2-webfont/issues/546)) ([e311528](https://github.com/atlowChemi/vite-svg-2-webfont/commit/e3115286d0cc5dec3921fe9fb6204dffc8e6c4bc))

### Bug Fixes

- **deps:** update rust crate oxvg_path to 0.0.9 ([#534](https://github.com/atlowChemi/vite-svg-2-webfont/issues/534)) ([753e002](https://github.com/atlowChemi/vite-svg-2-webfont/commit/753e002242fee8040c39902d7e90ca4cc3d77ee7))
- **deps:** update rust crate write-fonts to 0.54.0 ([#528](https://github.com/atlowChemi/vite-svg-2-webfont/issues/528)) ([0b6ec76](https://github.com/atlowChemi/vite-svg-2-webfont/commit/0b6ec762a5e6c1c315ef157519f964d841c3c3e4))

## [0.8.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-engine-v0.7.0...webfont-engine-v0.8.0) (2026-10-03)

### ⚠ BREAKING CHANGES

- **webfont-generator:** the webfont-generator crate no longer has the napi feature or the Node-specific generate_webfonts entry point. The npm package API, exports, binary names, and platform packages are unchanged.

### Bug Fixes

- enable NAPI and docs build caching and upgrade tooling ([#516](https://github.com/atlowChemi/vite-svg-2-webfont/issues/516)) ([a5f1bb0](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a5f1bb0bd00154a306f01f444b85ba3115a73467))
- resolve adapter packaging and strengthen workflow coverage ([#520](https://github.com/atlowChemi/vite-svg-2-webfont/issues/520)) ([d49ffcb](https://github.com/atlowChemi/vite-svg-2-webfont/commit/d49ffcbeb5f9e874e92b6f020aae8f62f8dde4c9))

### Code Refactoring

- **webfont-generator:** extract Rust engine from NAPI adapter ([#503](https://github.com/atlowChemi/vite-svg-2-webfont/issues/503)) ([9d305b6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/9d305b6f7a34860f2114d48b5ec85e413de8224c))

## [0.7.0]

Changes to the Rust engine and CLI are recorded here starting with the workspace extraction.
Earlier releases are documented in the [shared generator changelog](https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/packages/webfont-generator/CHANGELOG.md).

The engine has its own GitHub releases under `webfont-engine-v*`; its crates.io name remains
`webfont-generator`. Engine and npm adapter versions remain synchronized.
