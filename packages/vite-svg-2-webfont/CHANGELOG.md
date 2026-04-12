# Changelog

## [6.1.2](https://github.com/atlowChemi/vite-svg-2-webfont/compare/vite-svg-2-webfont-v6.1.1...vite-svg-2-webfont-v6.1.2) (2026-04-12)

### Bug Fixes

- **ci:** auto-format release-please changelog on PR branch ([#89](https://github.com/atlowChemi/vite-svg-2-webfont/issues/89)) ([f94d10e](https://github.com/atlowChemi/vite-svg-2-webfont/commit/f94d10eb25a27674ca162a94dd51c784b88498c8))

## [6.1.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/vite-svg-2-webfont-v6.1.0...vite-svg-2-webfont-v6.1.1) (2026-04-12)

### Bug Fixes

- use platform-independent path separators in option parsing ([#87](https://github.com/atlowChemi/vite-svg-2-webfont/issues/87)) ([f5f3f49](https://github.com/atlowChemi/vite-svg-2-webfont/commit/f5f3f49e61892068834a0ecc5df1265a0e149bb4))

## [6.1.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/vite-svg-2-webfont-v6.0.0...vite-svg-2-webfont-v6.1.0) (2026-04-12)

### Features

- add preloadFormats support to preload generated webfont assets ([#75](https://github.com/atlowChemi/vite-svg-2-webfont/pull/75))
- add centerVertically alias support for SVG format options ([#79](https://github.com/atlowChemi/vite-svg-2-webfont/pull/79))
- add release-please for automated releases and changelog ([#83](https://github.com/atlowChemi/vite-svg-2-webfont/issues/83))

### Documentation

- add VitePress docs site with GitHub Pages deployment ([#76](https://github.com/atlowChemi/vite-svg-2-webfont/pull/76))

### Miscellaneous Chores

- restructure as monorepo with packages dir ([#82](https://github.com/atlowChemi/vite-svg-2-webfont/pull/82))

## [6.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v5.0.0...v6.0.0) (2026-03-19)

### ⚠ BREAKING CHANGES

- Change `vite` supported versions range based on https://vite.dev/releases#supported-versions. (drop support for `vite` versions older than 6, add `vite` 8)

### Miscellaneous Chores

- use tsdown ([#57](https://github.com/atlowChemi/vite-svg-2-webfont/pull/57))
- use pnpm & update vitest ([#67](https://github.com/atlowChemi/vite-svg-2-webfont/pull/67))
- use oxlint & oxfmt ([#68](https://github.com/atlowChemi/vite-svg-2-webfont/pull/68))
- upgrade tsdown ([#69](https://github.com/atlowChemi/vite-svg-2-webfont/pull/69))
- add vite 8 support ([#70](https://github.com/atlowChemi/vite-svg-2-webfont/pull/70))
- finish pnpm migration, vite <5 cleanup, add contribution guide ([#71](https://github.com/atlowChemi/vite-svg-2-webfont/pull/71))
- try vite+ ([#72](https://github.com/atlowChemi/vite-svg-2-webfont/pull/72))
- run UT across vite versions ([#73](https://github.com/atlowChemi/vite-svg-2-webfont/pull/73))
- package audit ([#74](https://github.com/atlowChemi/vite-svg-2-webfont/pull/74))

## [5.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v4.0.0...v5.0.0) (2025-07-08)

### ⚠ BREAKING CHANGES

- drop support for Node v18

### Features

- bump dependencies and allow usage with vite 7 ([#54](https://github.com/atlowChemi/vite-svg-2-webfont/pull/54))

## [4.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v3.4.0...v4.0.0) (2025-02-02)

### ⚠ BREAKING CHANGES

- drop support for Node v21

### Features

- support vite v6 ([#43](https://github.com/atlowChemi/vite-svg-2-webfont/pull/43))

## [3.4.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v3.3.0...v3.4.0) (2024-07-03)

### Features

- introduce new option `allowWriteFilesInBuild` ([#35](https://github.com/atlowChemi/vite-svg-2-webfont/pull/35))

## [3.3.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v3.2.0...v3.3.0) (2024-04-27)

### Features

- added support for cssContext ([#33](https://github.com/atlowChemi/vite-svg-2-webfont/pull/33))

## [3.2.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v3.1.0...v3.2.0) (2024-04-15)

### Features

- use a hash based on content instead of `guid` ([#30](https://github.com/atlowChemi/vite-svg-2-webfont/pull/30))

## [3.1.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v3.0.0...v3.1.0) (2024-03-02)

### Features

- implement font inlining ([#25](https://github.com/atlowChemi/vite-svg-2-webfont/pull/25))
- cleanup Node 16 support & fix `vite v4` compatibility

## [3.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v2.2.0...v3.0.0) (2024-02-22)

### ⚠ BREAKING CHANGES

- drop support for Node v16

## [2.2.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v2.1.0...v2.2.0) (2024-02-22)

### Features

- add `moduleId` param to options that allow customization of `virtualModuleId` ([#18](https://github.com/atlowChemi/vite-svg-2-webfont/pull/18))

## [2.1.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v2.0.0...v2.1.0) (2024-02-21)

### Features

- add public API ([#20](https://github.com/atlowChemi/vite-svg-2-webfont/pull/20))

## [2.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v1.0.1...v2.0.0) (2024-02-06)

### ⚠ BREAKING CHANGES

- upgrade to Vite 5 ([#14](https://github.com/atlowChemi/vite-svg-2-webfont/pull/14))

## [1.0.1](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v1.0.0...v1.0.1) (2023-07-20)

### Bug Fixes

- dependency updates

## [1.0.0](https://github.com/atlowChemi/vite-svg-2-webfont/compare/v0.1.0...v1.0.0) (2023-01-31)

### ⚠ BREAKING CHANGES

- change input options to allow saving html/css without saving fonts ([#1](https://github.com/atlowChemi/vite-svg-2-webfont/pull/1))
