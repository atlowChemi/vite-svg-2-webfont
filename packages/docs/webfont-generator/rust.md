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

All fields except `dest` and `files` are optional and implement `Default`.

| Field                   | Type                           | Default                | Description                          |
| ----------------------- | ------------------------------ | ---------------------- | ------------------------------------ |
| `dest`                  | `String`                       | --                     | Output directory (required)          |
| `files`                 | `Vec<String>`                  | --                     | SVG file paths (required)            |
| `font_name`             | `Option<String>`               | `"iconfont"`           | Font family name                     |
| `types`                 | `Option<Vec<FontType>>`        | `[Eot, Woff, Woff2]`   | Font formats to generate             |
| `order`                 | `Option<Vec<FontType>>`        | Filtered default order | `@font-face` src order               |
| `css`                   | `Option<bool>`                 | `true`                 | Generate CSS file                    |
| `html`                  | `Option<bool>`                 | `false`                | Generate HTML preview                |
| `write_files`           | `Option<bool>`                 | `true`                 | Write output to disk                 |
| `css_template`          | `Option<String>`               | Built-in template      | Custom Handlebars CSS template path  |
| `html_template`         | `Option<String>`               | Built-in template      | Custom Handlebars HTML template path |
| `css_fonts_url`         | `Option<String>`               | Relative path          | URL prefix for fonts in CSS          |
| `css_dest`              | `Option<String>`               | `dest/fontName.css`    | CSS output path                      |
| `html_dest`             | `Option<String>`               | `dest/fontName.html`   | HTML output path                     |
| `codepoints`            | `Option<HashMap<String, u32>>` | Empty                  | Explicit glyph codepoints            |
| `start_codepoint`       | `Option<u32>`                  | `0xF101`               | Starting auto-codepoint              |
| `font_height`           | `Option<f64>`                  | --                     | Explicit font height                 |
| `ascent`                | `Option<f64>`                  | --                     | Font ascent                          |
| `descent`               | `Option<f64>`                  | --                     | Font descent                         |
| `normalize`             | `Option<bool>`                 | `true`                 | Normalize glyph heights              |
| `fixed_width`           | `Option<bool>`                 | --                     | Monospace font                       |
| `center_horizontally`   | `Option<bool>`                 | --                     | Center glyphs horizontally           |
| `center_vertically`     | `Option<bool>`                 | --                     | Center glyphs vertically             |
| `ligature`              | `Option<bool>`                 | `true`                 | Enable ligatures                     |
| `round`                 | `Option<f64>`                  | --                     | Path rounding precision              |
| `preserve_aspect_ratio` | `Option<bool>`                 | --                     | Preserve SVG aspect ratio            |
| `optimize_output`       | `Option<bool>`                 | --                     | Optimize SVG output                  |
| `font_style`            | `Option<String>`               | --                     | CSS `font-style` value               |
| `font_weight`           | `Option<String>`               | --                     | CSS `font-weight` value              |
| `format_options`        | `Option<FormatOptions>`        | --                     | Per-format options                   |
| `template_options`      | `Option<Map<String, Value>>`   | --                     | Extra template context               |

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

## Full API reference

For the complete API surface including all sub-types, see [docs.rs/webfont-generator](https://docs.rs/webfont-generator).

## See also

- [Overview](./) -- architecture and design
- [Node.js usage](./node) -- npm package API reference
- [CLI usage](./cli) -- command-line interface
