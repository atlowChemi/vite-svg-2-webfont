# Changelog

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
