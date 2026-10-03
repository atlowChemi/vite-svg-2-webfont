# Changelog

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
