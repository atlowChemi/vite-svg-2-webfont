---
description: Definitions of the terms used across the vite-svg-2-webfont and webfont-generator docs.
---

# Glossary

Terms used across these docs, in alphabetical order. Where the docs use more than one word for the same thing, the alternatives are listed.

### Adapter

A package that exposes the [engine](#engine) to a particular runtime: the Node.js package, the browser package, and the CLI. The Vite plugin builds on the Node.js package. See [Under the hood](/guide/under-the-hood#adapters).

### Base selector

The CSS selector that applies the icon font, `.icon` by default. An element needs the base class and an [icon class](#class-prefix): `<i class="icon icon-add">`. See [`baseSelector`](/vite/configuration#baseselector).

### Class prefix

The prefix added to each icon's name to form its CSS class, `icon-` by default, so `add.svg` becomes `icon-add`. See [`classPrefix`](/vite/configuration#classprefix).

### Codepoint

The Unicode number that identifies a glyph in the font, such as `U+F101`. The generated CSS maps each icon class to its codepoint. Codepoints are assigned automatically from `U+F101` unless you [pin them](/guide/svg-authoring#pin-codepoints-for-stable-output).

### Color glyph

A glyph that keeps the solid colors from its SVG, stored in the font's COLR/CPAL tables, instead of following the text color. Each color glyph also has a [monochrome fallback](#monochrome-fallback). See [color glyphs](/guide/engine#color-glyphs).

### Default design

In a [family](#family), the design that renders when no [modifier class](#modifier-class) is set. Exactly one design must be the default.

### Design

_Also: variant._ One complete drawing of an icon set, such as _light_ or _bold_, with its own folder of SVGs. Designs are combined into a [family](#family). The configuration option is called `variants`; the docs use "design" for the drawings and "variant" for the option and its entries.

### Engine

The Rust library that turns SVGs into fonts, CSS, and HTML. Every package in this project uses it. See [The Engine](/guide/engine).

### Family

_Also: multi-weight family, multi-variant font._ Several [designs](#design) of the same icons combined into one font file per format, with each design mapped to a font weight. See [multi-weight icon families](/vite/usage#multi-weight-icon-families).

### Font height

The size of the font's design grid, in font units. Glyph coordinates are rounded to this grid, so a small value makes curves faceted. The Vite plugin uses `1000`; the engine alone uses the tallest icon's height. See [`fontHeight`](/vite/configuration#fontheight).

### fonte

The name of this project and its family of tools, written in lowercase. In French, _fonte_ is a casting: metal melted and poured into a mold. A set of type cast in one size and style was _une fonte_, which gave English the word _font_. See [where the name comes from](/#colophon).

### Glyph

One drawing inside a font. Each SVG icon becomes one glyph (or, with a [family](#family), one glyph per design).

### Incremental generation

Rebuilding a font while reusing work from the previous build, so that only changed icons are processed again. The Vite plugin uses it automatically during development. See [`incremental`](/node/usage#incremental).

### Inline mode

Embedding fonts in the CSS as base64 data URLs instead of separate files. See [`inline`](/vite/configuration#inline).

### Ligature

A rule in the font that replaces a word with a glyph, so typing `add` inside an icon element can render the _add_ icon. Enabled by default. See [`ligature`](/vite/configuration#ligature).

### Missing glyph policy

What a [family](#family) does when a design doesn't have an icon that other designs have: fail, leave it blank, or borrow it from another design. See [`missingGlyphs`](/vite/configuration#missingglyphs).

### Modifier class

_Also: variant class._ The class that selects a non-default [design](#design), such as `icon--bold`. It is added next to the base and icon classes. See [`variantClassPrefix`](/vite/configuration#variantclassprefix).

### Monochrome fallback

The single-color version of a [color glyph](#color-glyph), stored in the same font. Platforms that can't render color fonts show it instead.

### Ordinary mode

_Also: single design, single-face font._ Generating a font from one set of SVGs, configured with `files`. The alternative is a [family](#family), configured with `variants`. Some features, such as SVG and EOT output, are available only in ordinary mode.

### Virtual module

The module the Vite plugin creates in memory, `virtual:vite-svg-2-webfont.css` by default. Importing it adds the generated CSS to your app. See [`moduleId`](/vite/configuration#moduleid).

### Webfont formats

- **WOFF2**: the smallest format, supported by all current browsers. Use it by default.
- **WOFF**: an older compressed format, useful as a fallback.
- **TTF**: an uncompressed TrueType font, also usable outside browsers.
- **EOT**: a legacy format for old Internet Explorer versions.
- **SVG font**: a legacy format that most browsers no longer support.

See [`types`](/vite/configuration#types).
