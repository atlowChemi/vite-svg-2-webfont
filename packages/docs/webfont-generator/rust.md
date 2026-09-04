---
description: API reference for the webfont-generator Rust crate, including async and sync APIs, types, and examples.
---

# Rust Usage

## Installation

```sh
cargo add webfont-generator
```

## Feature flags

| Feature | Default | Description                                                        |
| ------- | ------- | ------------------------------------------------------------------ |
| (none)  | yes     | Library-only build                                                 |
| `cli`   | no      | Builds the `webfont-generator` CLI binary (adds `clap` dependency) |
| `napi`  | no      | Enables Node.js NAPI bindings for use as a native addon            |

## Async API

The primary entry point requires a [tokio](https://tokio.rs/) runtime:

```rust
pub async fn generate(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult>
```

### Example

```rust
use webfont_generator::{GenerateWebfontsOptions, FontType};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let options = GenerateWebfontsOptions {
        dest: "output".to_owned(),
        files: vec![
            "icons/add.svg".to_owned(),
            "icons/remove.svg".to_owned(),
        ],
        font_name: Some("my-icons".to_owned()),
        types: Some(vec![FontType::Woff2, FontType::Woff]),
        ..Default::default()
    };

    let result = webfont_generator::generate(options, None).await?;

    if let Some(woff2) = result.woff2_bytes() {
        println!("Generated WOFF2: {} bytes", woff2.len());
    }

    Ok(())
}
```

## Sync API

For contexts without a tokio runtime, `generate_sync` spawns one internally:

```rust
pub fn generate_sync(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult>
```

### Example

```rust
use webfont_generator::{GenerateWebfontsOptions, FontType};

let options = GenerateWebfontsOptions {
    dest: "output".to_owned(),
    files: vec!["icons/add.svg".to_owned()],
    write_files: Some(false),
    ..Default::default()
};

let result = webfont_generator::generate_sync(options, None).unwrap();

if let Some(svg) = result.svg_string() {
    println!("SVG font length: {}", svg.len());
}
```

## `RenameFn`

```rust
pub type RenameFn = Box<dyn Fn(&str) -> String + Send + Sync>;
```

An optional callback that maps file paths to custom glyph names. When `None`, glyph names are derived from the file stem.

```rust
let rename: webfont_generator::RenameFn = Box::new(|path| {
    // Use only the filename without extension, lowercased
    std::path::Path::new(path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_lowercase()
});

let result = webfont_generator::generate_sync(options, Some(rename)).unwrap();
```

## `GenerateWebfontsOptions`

`dest` is required. Use `files` for ordinary generation or `variants` for the multi-variant
contract; all other fields are optional and implement `Default`.

| Field                   | Type                           | Default                 | Description                           |
| ----------------------- | ------------------------------ | ----------------------- | ------------------------------------- |
| `dest`                  | `String`                       | --                      | Output directory (required)           |
| `files`                 | `Vec<String>`                  | Empty                   | Ordinary SVG file paths               |
| `font_name`             | `Option<String>`               | `"iconfont"`            | Font family name                      |
| `types`                 | `Option<Vec<FontType>>`        | `[Eot, Woff, Woff2]`    | Font formats to generate              |
| `order`                 | `Option<Vec<FontType>>`        | Filtered default order  | `@font-face` src order                |
| `css`                   | `Option<bool>`                 | `true`                  | Generate CSS file                     |
| `html`                  | `Option<bool>`                 | `false`                 | Generate HTML preview                 |
| `write_files`           | `Option<bool>`                 | `true`                  | Write output to disk                  |
| `css_template`          | `Option<String>`               | Built-in template       | Custom Handlebars CSS template path   |
| `html_template`         | `Option<String>`               | Built-in template       | Custom Handlebars HTML template path  |
| `css_fonts_url`         | `Option<String>`               | Relative path           | URL prefix for fonts in CSS           |
| `css_dest`              | `Option<String>`               | `dest/fontName.css`     | CSS output path                       |
| `html_dest`             | `Option<String>`               | `dest/fontName.html`    | HTML output path                      |
| `codepoints`            | `Option<HashMap<String, u32>>` | Empty                   | Explicit glyph codepoints             |
| `start_codepoint`       | `Option<u32>`                  | `0xF101`                | Starting auto-codepoint               |
| `font_height`           | `Option<f64>`                  | --                      | Explicit font height                  |
| `ascent`                | `Option<f64>`                  | --                      | Font ascent                           |
| `descent`               | `Option<f64>`                  | --                      | Font descent                          |
| `normalize`             | `Option<bool>`                 | `true`                  | Normalize glyph heights               |
| `incremental`           | `Option<bool>`                 | `false`                 | Retain parsed glyphs for `regenerate` |
| `fixed_width`           | `Option<bool>`                 | --                      | Monospace font                        |
| `center_horizontally`   | `Option<bool>`                 | --                      | Center glyphs horizontally            |
| `center_vertically`     | `Option<bool>`                 | --                      | Center glyphs vertically              |
| `ligature`              | `Option<bool>`                 | `true`                  | Enable ligatures                      |
| `round`                 | `Option<f64>`                  | --                      | Path rounding precision               |
| `preserve_aspect_ratio` | `Option<bool>`                 | --                      | Preserve SVG aspect ratio             |
| `optimize_output`       | `Option<bool>`                 | --                      | Optimize SVG output                   |
| `font_style`            | `Option<String>`               | --                      | CSS `font-style` value                |
| `font_weight`           | `Option<String>`               | --                      | CSS `font-weight` value               |
| `missing_glyphs`        | `Option<MissingGlyphOptions>`  | `blank` in variant mode | Missing-glyph policy                  |
| `format_options`        | `Option<FormatOptions>`        | --                      | Per-format options                    |
| `template_options`      | `Option<Map<String, Value>>`   | --                      | Extra template context                |
| `variant_class_prefix`  | `Option<String>`               | `"icon--"`              | CSS variant modifier prefix           |
| `variants`              | `Option<Vec<FontVariant>>`     | --                      | Multi-variant input contract          |

### Multi-variant contract preview

```rust
pub struct FontVariant {
    pub name: String,
    pub files: Vec<String>,
    pub weight: Option<u16>,
    pub default: Option<bool>,
}

pub enum MissingGlyphBehavior {
    Blank,
    Error,
    Fallback,
}

pub struct MissingGlyphOptions {
    pub behavior: MissingGlyphBehavior,
    pub variant: Option<String>,
}
```

Variant mode requires at least two uniquely named variants, exactly one default, and either all
ordinary `files` or `variants`, never both. Every variant needs at least one SVG file. Explicit
weights are ordered anchors in the range 1–1000. An automatic default resolves to 400; other
automatic values resolve outward in steps of 100, or evenly within a crowded anchor interval.
Resolved weights are unique and strictly follow variant order.

Variant names reject NUL and Unicode whitespace. `variant_class_prefix` follows CSSOM identifier
serialization, so punctuation is escaped instead of rejected. Filename components use a separate
encoding: ASCII letters, digits, `_`, and `-` remain readable, while every other UTF-8 byte becomes
`~HH`. Platform-special names are encoded, and case-insensitive filename collisions are rejected.

`MissingGlyphBehavior::Blank` is the default. `Fallback` requires an existing variant that contains
every logical glyph in the family; `Blank` and `Error` reject a fallback name. SVG output and
incremental mode are invalid with variants.

This release loads and resolves variant sources but does not generate variant fonts. Files load in
parallel while rename callbacks retain variant and file order. Matching names across variants join
one logical glyph and receive one shared codepoint; duplicate names within a variant are rejected.
Missing source cells then become explicit blanks, errors, or references to the configured fallback
variant. Every resolved source is then parsed and processed with shared family metrics and a stable
advance per logical glyph. `generate()` and `generate_sync()` return `io::ErrorKind::Unsupported`
after successful geometry preparation.

## `FontType`

```rust
pub enum FontType {
    Svg,
    Ttf,
    Eot,
    Woff,
    Woff2,
}
```

Methods:

- `css_format() -> &'static str` -- Returns the CSS `format()` value (e.g., `"woff2"`, `"truetype"`)
- `as_extension() -> &'static str` -- Returns the file extension (e.g., `"woff2"`, `"ttf"`)

## `GenerateWebfontsResult`

### Font data getters

| Method          | Return type     | Description         |
| --------------- | --------------- | ------------------- |
| `eot_bytes()`   | `Option<&[u8]>` | EOT font bytes      |
| `svg_string()`  | `Option<&str>`  | SVG font XML string |
| `ttf_bytes()`   | `Option<&[u8]>` | TTF font bytes      |
| `woff_bytes()`  | `Option<&[u8]>` | WOFF font bytes     |
| `woff2_bytes()` | `Option<&[u8]>` | WOFF2 font bytes    |

### Template methods

| Method                      | Return type          | Description                                     |
| --------------------------- | -------------------- | ----------------------------------------------- |
| `generate_css_pure(urls?)`  | `io::Result<String>` | Render CSS with optional URL overrides          |
| `generate_html_pure(urls?)` | `io::Result<String>` | Render HTML preview with optional URL overrides |

Both methods accept `Option<HashMap<FontType, String>>` for the `urls` parameter. Results are cached internally for repeated calls with the same arguments.

### Incremental rebuild

| Method                                     | Return type                     | Description                                                          |
| ------------------------------------------ | ------------------------------- | -------------------------------------------------------------------- |
| `regenerate(ordered_paths, changes)`       | `io::Result<()>`                | Rebuild after known file changes, reusing unchanged glyphs           |
| `regenerate_all(ordered_paths)`            | `io::Result<()>`                | Re-read/hash the full file set and infer added/changed/removed paths |
| `regenerate_async(ordered_paths, changes)` | `Result<Self, RegenerateError>` | Consume the result and rebuild on Tokio's blocking pool              |
| `regenerate_all_async(ordered_paths)`      | `Result<Self, RegenerateError>` | Consume the result, re-diff, and rebuild on Tokio's blocking pool    |

Requires the result to have been generated with `incremental: Some(true)` (errors otherwise).
`ordered_paths: &[String]` is the complete file set after the changes, in the order a fresh build
would use (e.g. the glob result); the rebuilt glyphs are ordered to match it, so the result is
byte-identical to a fresh build of that set — additions included, even when they sort before
existing glyphs. Any path absent from `ordered_paths` is dropped. `changes: &[(String, GlyphChange)]`
names the affected files: added/changed files are re-read from disk. Every format is refreshed in
memory, and — when the result was built with `write_files` — refreshed fonts are written to disk
too, while unchanged CSS/HTML companion files are skipped. Rendered
CSS/HTML is reused when glyph names and codepoints are unchanged. Use `regenerate_all` when you have
the fresh ordered file set but no reliable watcher change batch; existing glyph names are preserved,
and added paths derive their glyph name from the file stem.

The async methods take owned `Vec` inputs and consume the result, so Rust rejects stale-result
reuse after a successful rebuild. Assign the returned generation:

```rust
result = result.regenerate_async(files, changes).await?;
```

For ordinary regeneration failures, `RegenerateError::into_result()` returns the consumed result
so it can be retried. It returns `None` only if Tokio cancelled the blocking task before the result
could be returned. The consuming futures are not cancellation-safe: dropping one does not stop an
already-started blocking task, filesystem writes may continue, and the consumed result cannot be
recovered. Panics resume unwinding on the awaiting task.

For Node.js results, `regenerate()` errors if the initial build used `cssContext` or `htmlContext`
callbacks because the synchronous method cannot re-run JavaScript callbacks during the rebuild.

```rust
pub enum GlyphChange {
    Added { name: Option<String> },   // new file; `name` overrides the file-stem glyph name
    Changed { name: Option<String> }, // content changed; `name` overrides the glyph name
    Removed,                          // file deleted
}

// result built with `incremental: Some(true)`
let files = vec!["icons/add.svg".to_owned(), "icons/remove.svg".to_owned()];
result.regenerate(&files, &[("icons/add.svg".to_owned(), GlyphChange::Changed { name: None })])?;
// Or re-diff the full set when watcher hints are unavailable/untrusted:
result.regenerate_all(&files)?;
```

## Full API reference

For the complete API surface including all sub-types, see [docs.rs/webfont-generator](https://docs.rs/webfont-generator).

## See also

- [Overview](./) -- architecture and design
- [Node.js usage](./node) -- npm package API reference
- [CLI usage](./cli) -- command-line interface
