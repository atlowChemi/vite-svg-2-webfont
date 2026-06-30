# Changelog

## [0.5.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.4.0...webfont-generator-v0.5.0) (2026-06-30)


### Features

* **webfont-generator:** add incremental regenerate ([222d929](https://github.com/atlowChemi/vite-svg-2-webfont/commit/222d9292f1a6e67af5a46861af9b4a311e7db303))
* **webfont-generator:** configurable WOFF2 compression quality ([3e5c250](https://github.com/atlowChemi/vite-svg-2-webfont/commit/3e5c250bec4e920186df59d3a66c33d4f6fa1eed))
* **webfont-generator:** reuse processed and compiled glyphs during recalc ([#192](https://github.com/atlowChemi/vite-svg-2-webfont/issues/192)) ([a80aecf](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a80aecf080f9434abac195485fbba1f2c7854f19))
* **webfont-generator:** support regenerate rediff ([#158](https://github.com/atlowChemi/vite-svg-2-webfont/issues/158)) ([a74d120](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a74d1200c55998a8c72e881c96b27a905a7afe81))


### Bug Fixes

* **webfont-generator:** normalize nested contour winding ([#155](https://github.com/atlowChemi/vite-svg-2-webfont/issues/155)) ([afcb1be](https://github.com/atlowChemi/vite-svg-2-webfont/commit/afcb1befe1884ae4229bc17f42b7fa16d9651544))


### Performance Improvements

* **vite-svg-2-webfont:** use faster WOFF2 compression in dev ([0edd41a](https://github.com/atlowChemi/vite-svg-2-webfont/commit/0edd41ac20f8e864622a645ba2b94ee00d626625))
* **webfont-generator:** make no-op incremental rebuilds up to 1078x faster and changed rebuilds up to 1.17x faster ([222d929](https://github.com/atlowChemi/vite-svg-2-webfont/commit/222d9292f1a6e67af5a46861af9b4a311e7db303))
* **webfont-generator:** reuse renders by template dependencies ([#161](https://github.com/atlowChemi/vite-svg-2-webfont/issues/161)) ([94abd39](https://github.com/atlowChemi/vite-svg-2-webfont/commit/94abd39e937445bb14e460009bfd59eba21755c1))
* **webfont-generator:** simplify glyf contours to shrink TTF ([#144](https://github.com/atlowChemi/vite-svg-2-webfont/issues/144)) ([ec0a919](https://github.com/atlowChemi/vite-svg-2-webfont/commit/ec0a919918d012cbeac10907cc546e632af211f9))

## [0.4.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.3.1...webfont-generator-v0.4.0) (2026-05-28)

### ⚠ BREAKING CHANGES

- **webfont-generator:** direct CJS consumers using `require('@atlowchemi/webfont-generator')` need to switch to `await import(...)` or rely on `require(esm)` (stable in Node 22.12+).

### Features

- **webfont-generator:** drop CJS entry points and ship ESM-only ([#131](https://github.com/atlowChemi/vite-svg-2-webfont/issues/131)) ([702486e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/702486ed58301fec9c23f23163518465e5148980))

### Bug Fixes

- **webfont-generator:** set repository.directory so npm resolves README images ([#133](https://github.com/atlowChemi/vite-svg-2-webfont/issues/133)) ([6f9b8c6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/6f9b8c64c85f78f06b3915b011bb2684fc43e6c1))

## [0.3.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.3.0...webfont-generator-v0.3.1) (2026-05-25)

### Bug Fixes

- **webfont-generator:** preserve leading slash when cssFontsUrl trims to root ([a82674b](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a82674b692d4e5bd1fea875629a33352d1ede78b))

## [0.3.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.2.4...webfont-generator-v0.3.0) (2026-04-27)

### Features

- **webfont-generator:** type cssContext and htmlContext callback context ([#113](https://github.com/atlowChemi/vite-svg-2-webfont/issues/113)) ([1be93fc](https://github.com/atlowChemi/vite-svg-2-webfont/commit/1be93fcab0accb96fe98a771efe031bd91331934))

### Bug Fixes

- **webfont-generator:** ship CJS entrypoints for require() consumers ([#116](https://github.com/atlowChemi/vite-svg-2-webfont/issues/116)) ([0d217bd](https://github.com/atlowChemi/vite-svg-2-webfont/commit/0d217bd61ce8a32b6f0324866e8d7275fb7bd61e))

## [0.2.4](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.2.3...webfont-generator-v0.2.4) (2026-04-16)

### Bug Fixes

- **ci:** disable GitHub release creation in napi pre-publish ([#108](https://github.com/atlowChemi/vite-svg-2-webfont/issues/108)) ([b30362a](https://github.com/atlowChemi/vite-svg-2-webfont/commit/b30362a814b57a82af317fca5aa02426905b70aa))

## [0.2.3](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.2.2...webfont-generator-v0.2.3) (2026-04-16)

### Bug Fixes

- **ci:** correct binding versions ([24048ee](https://github.com/atlowChemi/vite-svg-2-webfont/commit/24048eeb862ff3e92e7b85a75a5c771ecc7b66dc))
- **ci:** use heredoc for binding.js version sync, add README badges ([#107](https://github.com/atlowChemi/vite-svg-2-webfont/issues/107)) ([b802b9d](https://github.com/atlowChemi/vite-svg-2-webfont/commit/b802b9d08dfa3cff0995fffd1973a934dd51bb1d))
- **webfont-generator:** add repository field and sync binding.js version on release ([#105](https://github.com/atlowChemi/vite-svg-2-webfont/issues/105)) ([e70cef6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/e70cef651653fb3d574565b642e1a8cef5d0a95b))

## [0.2.2](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.2.1...webfont-generator-v0.2.2) (2026-04-15)

### Bug Fixes

- **webfont-generator:** fix npm release ([#101](https://github.com/atlowChemi/vite-svg-2-webfont/issues/101)) ([01684c1](https://github.com/atlowChemi/vite-svg-2-webfont/commit/01684c18c0b23c1a5bafa17ec743f792d96b6552))

## [0.2.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.2.0...webfont-generator-v0.2.1) (2026-04-15)

### Bug Fixes

- **ci:** sync Cargo.lock on release PR ([#100](https://github.com/atlowChemi/vite-svg-2-webfont/issues/100)) ([17af687](https://github.com/atlowChemi/vite-svg-2-webfont/commit/17af687ee37743a0610580ddebf0dd4940e26033))

## [0.2.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.1.0...webfont-generator-v0.2.0) (2026-04-15)

### Features

- publish webfont-generator crate with library API, CLI, and docs ([#98](https://github.com/atlowChemi/vite-svg-2-webfont/issues/98)) ([c2c8c1b](https://github.com/atlowChemi/vite-svg-2-webfont/commit/c2c8c1b786509d0506755cdcb435cad3d05137b8))
