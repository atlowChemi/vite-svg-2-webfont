---
description: Every field of the webfont-generator crate's GenerateWebfontsOptions, grouped by what it controls, plus the in-memory options.
---

# Rust Configuration

Fields of `GenerateWebfontsOptions`, the options for [`generate`](/rust/api#generate) and [`generate_sync`](/rust/api#generate-sync). `dest` and either `files` or `variants` are required; set the others with `Some(…)`, or leave them to `..Default::default()`:

```rust
use webfont_generator::{FontType, GenerateWebfontsOptions};

let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    files: vec!["icons/add.svg".to_owned()],
    types: Some(vec![FontType::Woff2]),
    ..Default::default()
};
```

Descriptions shared with the other tools name fields in camelCase, as the [CLI manifest](/cli/usage#json-manifest) does: `formatOptions.svg` is `format_options.svg` in Rust. The in-memory API takes [similar options](#in-memory-options).

## Input

### `files`

<!--@include: @/_shared/options/files.md-->

- Type: `Vec<String>`
- Default: empty; required unless `variants` is set

### `variants`

<!--@include: @/_shared/options/variants.md-->

- Type: `Option<Vec<FontVariant>>`
- Default: `None` (a single-design font)

Each [`FontVariant`](/rust/api#fontvariant) has a `name`, its `files`, and optional `weight` and `default`:

```rust
use webfont_generator::FontVariant;

let light = FontVariant {
    name: "light".to_owned(),
    files: vec!["icons/light/add.svg".to_owned()],
    weight: Some(300),
    default: Some(true),
};
```

### `variant_class_prefix`

<!--@include: @/_shared/options/variant-class-prefix.md-->

- Type: `Option<String>`
- Default: `"icon--"`

### `missing_glyphs`

<!--@include: @/_shared/options/missing-glyphs.md-->

- Type: `Option<MissingGlyphOptions>`
- Default: `MissingGlyphBehavior::Blank` for families

See [`MissingGlyphOptions`](/rust/api#missingglyphoptions).

## Output

### `dest`

<!--@include: @/_shared/options/dest.md-->

- Type: `String`
- Required

### `font_name`

<!--@include: @/_shared/options/font-name.md-->

- Type: `Option<String>`
- Default: `"iconfont"`

### `types`

<!--@include: @/_shared/options/types.md-->

- Type: `Option<Vec<FontType>>`
- Default: `[Eot, Woff, Woff2]`, or `[Woff, Woff2]` for families

### `order`

<!--@include: @/_shared/options/order.md-->

- Type: `Option<Vec<FontType>>`
- Default: `[Eot, Woff2, Woff, Ttf, Svg]`, filtered to `types`

### `write_files`

<!--@include: @/_shared/options/write-files.md-->

- Type: `Option<bool>`
- Default: `true`

The result holds every output either way.

### `css`

<!--@include: @/_shared/options/css.md-->

- Type: `Option<bool>`
- Default: `true`

### `css_dest`

<!--@include: @/_shared/options/css-dest.md-->

- Type: `Option<String>`
- Default: `<dest>/<font_name>.css`

### `css_fonts_url`

<!--@include: @/_shared/options/css-fonts-url.md-->

- Type: `Option<String>`
- Default: the relative path from `css_dest` to `dest`

### `html`

<!--@include: @/_shared/options/html.md-->

- Type: `Option<bool>`
- Default: `false`

### `html_dest`

<!--@include: @/_shared/options/html-dest.md-->

- Type: `Option<String>`
- Default: `<dest>/<font_name>.html`

### `ts`

<!--@include: @/_shared/options/ts.md-->

- Type: `Option<bool>`
- Default: `false`

`generate_ts()` renders the module even when this is off; see [`generate_ts()`](/rust/api#generate-ts).

### `ts_dest`

<!--@include: @/_shared/options/ts-dest.md-->

- Type: `Option<String>`
- Default: `<dest>/<font_name>.ts`

Setting it alone doesn't enable the output; set `ts` too.

## Templates

See [Templates](/guide/templates) for the built-in templates and the context they receive.

### `css_template`

<!--@include: @/_shared/options/css-template.md-->

- Type: `Option<String>`
- Default: the built-in CSS template

### `html_template`

<!--@include: @/_shared/options/html-template.md-->

- Type: `Option<String>`
- Default: the built-in HTML template

### `template_options`

<!--@include: @/_shared/options/template-options.md-->

- Type: `Option<serde_json::Map<String, Value>>`
- Default: `None`

## Glyphs

### `codepoints`

<!--@include: @/_shared/options/codepoints.md-->

- Type: `Option<HashMap<String, u32>>`
- Default: `None`

### `start_codepoint`

<!--@include: @/_shared/options/start-codepoint.md-->

- Type: `Option<u32>`
- Default: `0xF101`

### `ligature`

<!--@include: @/_shared/options/ligature.md-->

- Type: `Option<bool>`
- Default: `true`

### `font_height`

<!--@include: @/_shared/options/font-height.md-->

- Type: `Option<f64>`
- Default: the tallest icon's height

### `ascent`

<!--@include: @/_shared/options/ascent.md-->

- Type: `Option<f64>`

### `descent`

<!--@include: @/_shared/options/descent.md-->

- Type: `Option<f64>`

### `normalize`

<!--@include: @/_shared/options/normalize.md-->

- Type: `Option<bool>`
- Default: `true`

### `fixed_width`

<!--@include: @/_shared/options/fixed-width.md-->

- Type: `Option<bool>`
- Default: `false`

### `center_horizontally`

<!--@include: @/_shared/options/center-horizontally.md-->

- Type: `Option<bool>`
- Default: `false`

### `center_vertically`

<!--@include: @/_shared/options/center-vertically.md-->

- Type: `Option<bool>`
- Default: `false`

### `preserve_aspect_ratio`

<!--@include: @/_shared/options/preserve-aspect-ratio.md-->

- Type: `Option<bool>`

### `round`

<!--@include: @/_shared/options/round.md-->

- Type: `Option<f64>`

### `optimize_output`

<!--@include: @/_shared/options/optimize-output.md-->

- Type: `Option<bool>`
- Default: `false`

## Color

### `color_glyphs`

<!--@include: @/_shared/options/color-glyphs.md-->

- Type: `Option<ColorGlyphSelection>`
- Default: `None` (single-color)

`ColorGlyphSelection::All` selects every icon, and `ColorGlyphSelection::Named(names)` the listed ones.

### `color_slots`

<!--@include: @/_shared/options/color-slots.md-->

- Type: `Option<HashMap<String, String>>`
- Default: `None`

## Font metadata

### `font_style`

<!--@include: @/_shared/options/font-style.md-->

- Type: `Option<String>`

### `font_weight`

<!--@include: @/_shared/options/font-weight.md-->

- Type: `Option<String>`

### `format_options`

<!--@include: @/_shared/options/format-options.md-->

- Type: `Option<FormatOptions>`

The Rust types are `FormatOptions` with `svg`, `ttf`, `woff`, and `woff2` fields of `SvgFormatOptions`, `TtfFormatOptions`, `WoffFormatOptions`, and `Woff2FormatOptions`, with snake_case fields such as `compression_quality`. Without `ttf.ts`, each run records the current time in the font.

## Performance

### `incremental`

<!--@include: @/_shared/options/incremental.md-->

- Type: `Option<bool>`
- Default: `false`

Update the result with the [`regenerate` methods](/rust/api#updating-a-file-result).

### `cache_location`

<!--@include: @/_shared/options/cache-location.md-->

- Type: `Option<String>`
- Default: `None` (no persistent cache)

Remove the files the engine created with [`clear_cache()`](/rust/api#clear-cache). With `incremental`, call [`flush_cache()`](/rust/api#flush-cache) before shutdown. Cached fonts are reused as-is, so only restore trusted caches.

### `threads`

<!--@include: @/_shared/options/threads.md-->

- Type: `Option<NonZeroUsize>`
- Default: one per CPU core

## In-memory options

[`generate_from_sources`](/rust/api#generate-from-sources-sync) takes `GenerateWebfontsFromSourcesOptions`. It has the same fields and defaults as `GenerateWebfontsOptions` where they apply, with these differences:

| Field                                      | Meaning                                                                                                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sources: Vec<SvgSource>`                  | The icons as `{ id, name, svg }` records, in order; empty when using `variants`.                                                                        |
| `variants: Option<Vec<SourceFontVariant>>` | Designs as `{ name, sources, weight, default }`, with the same weight and missing-glyph rules.                                                          |
| `css_template`, `html_template`            | Handlebars **text**, not paths. `None` uses the built-in template; `Some("")` renders empty text.                                                       |
| `css_fonts_url`                            | URL prefix. Without it, CSS and HTML refer to the bare file names, with a content hash.                                                                 |
| `cache: Option<Vec<u8>>`                   | [Engine cache](/guide/features/caching) bytes from an earlier [`cache_bytes()`](/rust/api#cache-bytes), or empty to start cold.                         |
| `fallback_ts: Option<i64>`                 | TTF timestamp when `format_options.ttf.ts` is unset, for targets without a system clock, such as WebAssembly. Unlike `ts`, it isn't part of cache keys. |

There are no destination, file-writing, or path-based renaming options.

- **IDs** are opaque, nonempty strings, even when they look like paths.
- **Names** are given explicitly and use only Unicode letters, digits, hyphens, and underscores, so they're safe in the built-in SVG, CSS, and HTML. The same rule applies to names in `codepoints` and renames.
- **Within a design**, IDs and names are unique. An ID repeated across designs must have the same name and SVG text; give different drawings different IDs.
- **External images** referenced by an SVG are ignored; nothing is read from disk.
