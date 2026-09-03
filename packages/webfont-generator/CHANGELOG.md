# Changelog

## [0.6.2](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.6.1...webfont-generator-v0.6.2) (2026-09-03)

### Performance Improvements

- **webfont-generator:** disable unused usvg features ([#398](https://github.com/atlowChemi/vite-svg-2-webfont/issues/398)) ([c628ce1](https://github.com/atlowChemi/vite-svg-2-webfont/commit/c628ce1705f7dddacd2d0ab41794567539be951f))

## [0.6.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.6.0...webfont-generator-v0.6.1) (2026-08-24)

### Performance Improvements

- **webfont-generator:** batch rename callbacks ([#368](https://github.com/atlowChemi/vite-svg-2-webfont/issues/368)) ([0046ddd](https://github.com/atlowChemi/vite-svg-2-webfont/commit/0046ddd3fd714ccc83bddc3fa16bdbf517976748))
- **webfont-generator:** carry structured TTF geometry ([#367](https://github.com/atlowChemi/vite-svg-2-webfont/issues/367)) ([cefee7b](https://github.com/atlowChemi/vite-svg-2-webfont/commit/cefee7bca898b19fb4844156cc86639ce3eed070))
- **webfont-generator:** cut SVG parse-stage time by 10-13% ([#347](https://github.com/atlowChemi/vite-svg-2-webfont/issues/347)) ([497e5f1](https://github.com/atlowChemi/vite-svg-2-webfont/commit/497e5f17a8166f4aa19034f8d2a021d383a3ad1b))
- **webfont-generator:** cut SVG transient allocations by 64% ([#344](https://github.com/atlowChemi/vite-svg-2-webfont/issues/344)) ([4f0f70e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/4f0f70e8fda0d8e4ba8143f0b863cb447bfb18fc))
- **webfont-generator:** defer cached SVG path clones ([#365](https://github.com/atlowChemi/vite-svg-2-webfont/issues/365)) ([1ee87c2](https://github.com/atlowChemi/vite-svg-2-webfont/commit/1ee87c2b3eb45dc64b1c24c7375a101b25ba8a30))
- **webfont-generator:** preallocate SVG font output ([#357](https://github.com/atlowChemi/vite-svg-2-webfont/issues/357)) ([d7a5868](https://github.com/atlowChemi/vite-svg-2-webfont/commit/d7a5868403c466cd831b6e27f80644becc0ff4cf))
- **webfont-generator:** reduce WOFF2 transient allocations ([#349](https://github.com/atlowChemi/vite-svg-2-webfont/issues/349)) ([3bf52b1](https://github.com/atlowChemi/vite-svg-2-webfont/commit/3bf52b10d3d9c7662f790af9e5ca9320cdbcc355))
- **webfont-generator:** remove two stored allocations per glyph ([#346](https://github.com/atlowChemi/vite-svg-2-webfont/issues/346)) ([82c07b6](https://github.com/atlowChemi/vite-svg-2-webfont/commit/82c07b639598a45a9a56d8f17d08746432bda56c))
- **webfont-generator:** reuse optimized path geometry ([#372](https://github.com/atlowChemi/vite-svg-2-webfont/issues/372)) ([fe0bae7](https://github.com/atlowChemi/vite-svg-2-webfont/commit/fe0bae70d03455a0d089d632fd2a1d2f87b43295))
- **webfont-generator:** reuse structured path hashes ([#369](https://github.com/atlowChemi/vite-svg-2-webfont/issues/369)) ([113b281](https://github.com/atlowChemi/vite-svg-2-webfont/commit/113b281bec8f7f12c465d3bf969883c328a9e633))
- **webfont-generator:** reuse WOFF2 glyph scratch buffers ([#355](https://github.com/atlowChemi/vite-svg-2-webfont/issues/355)) ([3149c2e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/3149c2e9b64514d6c119c78ca8d4fbd768c63f70))
- **webfont-generator:** share compiled TTF glyphs ([#364](https://github.com/atlowChemi/vite-svg-2-webfont/issues/364)) ([b95abfe](https://github.com/atlowChemi/vite-svg-2-webfont/commit/b95abfece526b9ad65b1f98d245d366b5fbd7b06))
- **webfont-generator:** share loaded SVG contents ([#360](https://github.com/atlowChemi/vite-svg-2-webfont/issues/360)) ([6315d75](https://github.com/atlowChemi/vite-svg-2-webfont/commit/6315d751e171ee875d650f1242392e8b30dec76b))
- **webfont-generator:** share parsed glyph cache entries ([#358](https://github.com/atlowChemi/vite-svg-2-webfont/issues/358)) ([b41707f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/b41707f591cd148100e2c0522b33fe6a970946b9))
- **webfont-generator:** share processed SVG paths ([#362](https://github.com/atlowChemi/vite-svg-2-webfont/issues/362)) ([77ce975](https://github.com/atlowChemi/vite-svg-2-webfont/commit/77ce9750837f14c51ae5663b7624ef11236f6aa4))
- **webfont-generator:** skip single-contour winding work ([#354](https://github.com/atlowChemi/vite-svg-2-webfont/issues/354)) ([4d7ed77](https://github.com/atlowChemi/vite-svg-2-webfont/commit/4d7ed775a252a52526fbc5d082175b40e9a91602))

## [0.6.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.5.1...webfont-generator-v0.6.0) (2026-08-09)

### Features

- **webfont-generator:** add async regeneration ([#317](https://github.com/atlowChemi/vite-svg-2-webfont/issues/317)) ([a038107](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a038107209f502c240972e3dc30beeaf27d4f272))

### Bug Fixes

- **deps:** update rust crate usvg to 0.48.0 ([#320](https://github.com/atlowChemi/vite-svg-2-webfont/issues/320)) ([0165125](https://github.com/atlowChemi/vite-svg-2-webfont/commit/0165125c7f7d02f550dc961266b326bedda5c108))
- **deps:** update rust crate write-fonts to 0.51.0 ([#285](https://github.com/atlowChemi/vite-svg-2-webfont/issues/285)) ([193b1c9](https://github.com/atlowChemi/vite-svg-2-webfont/commit/193b1c9bf5d63ad399decb7ce602084c664c52af))

### Performance Improvements

- Move regeneration state instead of cloning glyph caches, making unchanged incremental rebuilds 2-3x faster with negligible async overhead. ([a038107](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a038107209f502c240972e3dc30beeaf27d4f272))
- optimized font output is ~7% smaller and path optimization ~10% faster (thanks to oxvg_path 0.0.7) ([6b38435](https://github.com/atlowChemi/vite-svg-2-webfont/commit/6b38435e9550fe9c571a38ed1d01d94ff34fd03d))

## [0.5.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.5.0...webfont-generator-v0.5.1) (2026-07-23)

### Bug Fixes

- **webfont-generator:** clippy errors ([#231](https://github.com/atlowChemi/vite-svg-2-webfont/issues/231)) ([ad56248](https://github.com/atlowChemi/vite-svg-2-webfont/commit/ad562488f8c64eba84f2a6be81b3bfee5e61cbe7))

### Performance Improvements

- **webfont-generator:** make 300-glyph WOFF2-only generation approximately 44.5% faster ([9800712](https://github.com/atlowChemi/vite-svg-2-webfont/commit/980071209ffa22c50fcc12a9d1743f77add2aab7))
- **webfont-generator:** make incremental WOFF2 content edits approximately 38-41% faster ([9800712](https://github.com/atlowChemi/vite-svg-2-webfont/commit/980071209ffa22c50fcc12a9d1743f77add2aab7))
- **webfont-generator:** reduce cold WOFF2 transform preparation from 0.944 ms to 0.306 ms at 300 glyphs by decoding each glyph once ([9800712](https://github.com/atlowChemi/vite-svg-2-webfont/commit/980071209ffa22c50fcc12a9d1743f77add2aab7))
- **webfont-generator:** reduce integrated quality-11 generation from 14.082 ms to 7.248 ms at 100 glyphs, 61.065 ms to 24.830 ms at 300 glyphs, and 122.880 ms to 45.813 ms at 600 glyphs ([9800712](https://github.com/atlowChemi/vite-svg-2-webfont/commit/980071209ffa22c50fcc12a9d1743f77add2aab7))
- **webfont-generator:** speed up WOFF2 encoding with an internal transformed encoder ([#272](https://github.com/atlowChemi/vite-svg-2-webfont/issues/272)) ([9800712](https://github.com/atlowChemi/vite-svg-2-webfont/commit/980071209ffa22c50fcc12a9d1743f77add2aab7))

## [0.5.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/webfont-generator-v0.4.0...webfont-generator-v0.5.0) (2026-07-09)

### Features

- **webfont-generator:** add incremental regenerate ([222d929](https://github.com/atlowChemi/vite-svg-2-webfont/commit/222d9292f1a6e67af5a46861af9b4a311e7db303))
- **webfont-generator:** configurable WOFF2 compression quality ([3e5c250](https://github.com/atlowChemi/vite-svg-2-webfont/commit/3e5c250bec4e920186df59d3a66c33d4f6fa1eed))
- **webfont-generator:** reuse processed and compiled glyphs during recalc ([#192](https://github.com/atlowChemi/vite-svg-2-webfont/issues/192)) ([a80aecf](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a80aecf080f9434abac195485fbba1f2c7854f19))
- **webfont-generator:** reuse serialized font outputs ([#202](https://github.com/atlowChemi/vite-svg-2-webfont/issues/202)) ([63b426f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/63b426f2a00d71e3ceea17a96fb201a2655bfe7f))
- **webfont-generator:** support regenerate rediff ([#158](https://github.com/atlowChemi/vite-svg-2-webfont/issues/158)) ([a74d120](https://github.com/atlowChemi/vite-svg-2-webfont/commit/a74d1200c55998a8c72e881c96b27a905a7afe81))

### Bug Fixes

- **deps:** update rust crate write-fonts to 0.50.0 ([#223](https://github.com/atlowChemi/vite-svg-2-webfont/issues/223)) ([b2c6165](https://github.com/atlowChemi/vite-svg-2-webfont/commit/b2c6165a59d3a1bdbe4f0958edce14b60269981f))
- **webfont-generator:** normalize nested contour winding ([#155](https://github.com/atlowChemi/vite-svg-2-webfont/issues/155)) ([afcb1be](https://github.com/atlowChemi/vite-svg-2-webfont/commit/afcb1befe1884ae4229bc17f42b7fa16d9651544))

### Performance Improvements

- **webfont-generator:** incremental TTF-table regenerate ~15-25% faster (unchanged table bytes reused) ([63b426f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/63b426f2a00d71e3ceea17a96fb201a2655bfe7f))
- **webfont-generator:** make no-op incremental rebuilds up to 1078x faster and changed rebuilds up to 1.17x faster ([222d929](https://github.com/atlowChemi/vite-svg-2-webfont/commit/222d9292f1a6e67af5a46861af9b4a311e7db303))
- **webfont-generator:** raw SFNT assembly ~6-9% faster (direct writer + table-order match) ([63b426f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/63b426f2a00d71e3ceea17a96fb201a2655bfe7f))
- **webfont-generator:** rename-only WOFF ~20% faster (WOFF1 payload compression cached) ([63b426f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/63b426f2a00d71e3ceea17a96fb201a2655bfe7f))
- **webfont-generator:** reuse renders by template dependencies ([#161](https://github.com/atlowChemi/vite-svg-2-webfont/issues/161)) ([94abd39](https://github.com/atlowChemi/vite-svg-2-webfont/commit/94abd39e937445bb14e460009bfd59eba21755c1))
- **webfont-generator:** simplify glyf contours to shrink TTF ([#144](https://github.com/atlowChemi/vite-svg-2-webfont/issues/144)) ([ec0a919](https://github.com/atlowChemi/vite-svg-2-webfont/commit/ec0a919918d012cbeac10907cc546e632af211f9))
- **webfont-generator:** SVG parse ~15% faster (skips UTF-8 revalidation) ([63b426f](https://github.com/atlowChemi/vite-svg-2-webfont/commit/63b426f2a00d71e3ceea17a96fb201a2655bfe7f))

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
