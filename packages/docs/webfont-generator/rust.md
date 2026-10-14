---
description: API reference for the webfont-generator Rust crate, including async and sync APIs, types, and examples.
---

# Rust Usage

## Installation

```sh
cargo add webfont-generator
```

## Feature flags

| Feature     | Default | Description                                                                                                             |
| ----------- | ------- | ----------------------------------------------------------------------------------------------------------------------- |
| `fs`        | yes     | File/path generation, hooks, file templates, writes and file regeneration; implies `async` and Tokio filesystem support |
| `in_memory` | yes     | Synchronous SVG text generation, inline templates, output collection and source regeneration                            |
| `async`     | yes     | Optional Tokio dependency and asynchronous APIs                                                                         |
| `cli`       | no      | CLI with JSON manifest support; implies `fs` and adds `clap` and `serde_path_to_error`                                  |
| `bench`     | no      | Internal Criterion benchmark helpers; implies `fs` and `in_memory`                                                      |

The `async` feature enables `generate_from_sources`, consuming asynchronous `regenerate`, and
`SourceRegenerateError`. With only `in_memory`, Tokio is absent from the engine's normal
dependencies. Select `fs` for a file-only integration. `generate_sync` creates a
Tokio runtime internally, so `fs` implies `async` even for synchronous callers.

Features are additive: another dependency can enable additional capabilities in the same
Cargo build. Disabling file APIs is groundwork for browser builds, not a WASM compatibility guarantee.

With `cli` enabled, `GenerateWebfontsOptions` and its input types implement `serde::Deserialize` using camelCase field names and rejecting unknown fields. Deserialization alone does not expand directories or rebase paths; those operations belong to the [CLI manifest loader](./cli#json-manifest).

### Migrating from the embedded Node binding

The former `napi` Cargo feature and Node-specific Rust entry point have moved out of
this crate into the internal `webfont-generator-napi` workspace crate. Remove `napi`
from the features requested for `webfont-generator`. Rust callers use the engine APIs
below; integrations that need callbacks use [`GenerationHooks`](#adapter-hooks).

This is a Cargo API breaking change. The published Rust crate and CLI remain named
`webfont-generator`. JavaScript consumers continue to install and import
`@atlowchemi/webfont-generator`; they do not install the internal adapter crate.

## Async API

The file APIs in this section require the [`fs` feature](#feature-flags) (enabled by default).

### Adapter hooks

`generate_with_hooks(options, &hooks)` accepts an implementation of `GenerationHooks`.
It returns `Result<GenerateWebfontsResult, H::Error>`, where the hook error type implements
`From<std::io::Error> + Send`. Implementations must be `Send + Sync` and return `Send` futures.

| Method                  | Default            | Contract                                                                                                                                                      |
| ----------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rename(paths)`         | `Ok(None)`         | Borrows `&[String]` after source loading; return one name per ordered path, or `None` for file stems. Variant paths are flattened in configured design order. |
| `has_css_context()`     | `false`            | Enable CSS context mutation.                                                                                                                                  |
| `has_html_context()`    | `false`            | Enable HTML context mutation.                                                                                                                                 |
| `css_context(context)`  | Unmodified context | Runs before HTML; engine-owned variant mode cannot be overridden.                                                                                             |
| `html_context(context)` | Unmodified context | Receives the HTML context, with variant styles derived from the finalized CSS context.                                                                        |

The engine owns generation, rendering caches and output writes. Adapters own callback
transport and boundary types. `()` implements the trait with no callbacks. Results made
with context mutation enabled reject incremental regeneration, matching the Node API.

### Generation

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

Requires the [`fs` feature](#feature-flags) (enabled by default).

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

## SVG text API

Requires the [`in_memory` feature](#feature-flags) (enabled by default); asynchronous entry points additionally require the [`async` feature](#feature-flags).

`generate_from_sources_sync(options)` returns `std::io::Result<GenerateWebfontsFromSourcesResult>`
without constructing a runtime, reading sources/templates, or writing output files.
`generate_from_sources(options).await` runs the same work on Tokio's blocking pool;
both entry points retain Rayon CPU parallelism. The async API requires an active Tokio runtime.

```rust
use webfont_generator::{generate_from_sources_sync, GenerateWebfontsFromSourcesOptions, SvgSource};

let mut result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
    sources: vec![SvgSource {
        id: "icon-42".into(),
        name: "square".into(),
        svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#.into(),
    }],
    incremental: Some(true),
    ..Default::default()
})?;
let css = result.generate_css()?;
let outputs = result.collect_outputs()?;
# Ok::<(), std::io::Error>(())
```

### Text options and identity

`GenerateWebfontsFromSourcesOptions` uses the same applicable settings and defaults as
`GenerateWebfontsOptions`, with these differences:

| Field                                      | Meaning                                                                                                  |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `sources: Vec<SvgSource>`                  | Ordered `{ id, name, svg }` records; empty when using variants.                                          |
| `variants: Option<Vec<SourceFontVariant>>` | Ordered `{ name, sources, weight, default }` designs, using the existing weight and missing-glyph rules. |
| `css_template`, `html_template`            | Optional Handlebars **text**. `None` uses the built-in renderer; `Some("")` renders empty text.          |
| `css_fonts_url`                            | Optional URL prefix. Both CSS and HTML default to relative font filenames with a generation fingerprint. |
| `incremental`                              | Retain caches for source updates; default false.                                                         |

There are no destination, writing, or path-based rename controls. Font formats, metrics,
codepoints, normalization, alignment, ligatures, color selection, template data and format
options follow the file API. Variant output supports TTF, WOFF and WOFF2.

IDs are opaque, nonempty strings, even if they look like paths. Names are supplied explicitly
and must contain only Unicode letters/numbers, hyphens or underscores (at least one character).
The same restriction applies to explicit codepoint-map names and rename updates, keeping names
safe in built-in SVG/CSS/HTML output. Quotes, markup, whitespace and selector punctuation are rejected.
Each design requires unique IDs and glyph names. Repeated IDs across
variants must have identical names and SVG text; use different IDs for different drawings
of the same glyph name. Input order controls automatic codepoint assignment.

External SVG image paths are ignored by the parser's text-mode resolver, including when
parsing embedded SVG data. No filesystem-resource scan or custom rejection diagnostic is
performed. This native API does not establish browser/WASM compatibility.

### Result methods

| Method                                                        | Result                                                                                       |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `svg_string()`                                                | `Option<&str>`                                                                               |
| `ttf_bytes()`, `eot_bytes()`, `woff_bytes()`, `woff2_bytes()` | `Option<&[u8]>`                                                                              |
| `generate_css()`, `generate_html()`                           | `std::io::Result<Arc<String>>`; renders once and shares the cached text.                     |
| `collect_outputs()`                                           | `std::io::Result<Vec<CollectedOutput>>`; shares font payloads and enabled companion renders. |

`CollectedOutput` contains `kind: OutputKind` (`Font(FontType)`, `Css`, or `Html`) and
`contents: OutputContents` (`Bytes(Arc<Vec<u8>>)` or `Text(Arc<String>)`). Use `as_bytes()`
to consume either representation. The caller chooses download names or other destinations.

Font getters never initialize templates. CSS requests never compile HTML. HTML embeds the
cached CSS. Collection respects `css`/`html`; explicit render methods work even when disabled.
Template compile/render errors are deferred until that companion is requested.

Memory fingerprints include ordered IDs, names, content, variant membership, resolved settings,
and supplied template text. They need not match legacy file-mode URL hashes.

### Incremental text updates

```rust
# use webfont_generator::{GenerateWebfontsFromSourcesResult, SourceChange, SourceMembership};
# fn update(result: &mut GenerateWebfontsFromSourcesResult) -> std::io::Result<()> {
result.regenerate_sync(
    SourceMembership::Sources(vec!["icon-42".into()]),
    vec![SourceChange::Changed {
        id: "icon-42".into(),
        name: None, // Keep the name; Some(...) renames without changing identity.
        svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0L24 24H0Z"/></svg>"#.into(),
    }],
)?;
# Ok(())
# }
```

- `SourceChange::Added(SvgSource)` introduces an unknown ID.
- `Changed { id, svg, name }` replaces a known source.
- `Removed { id }` removes a known source from all designs.
- `SourceMembership::Sources(ids)` supplies complete ordinary membership and order.
- `SourceMembership::Variants(Vec<VariantSourceSet { name, ids }>)` supplies every configured
  variant in its original order. A shared-ID update affects every membership.

Unchanged source text stays in Rust; submit only changed payloads. Complete membership also
permits reorder or removal without resending retained text. Added/changed IDs must appear in
final membership, removed IDs cannot remain, and each design must remain nonempty.
Duplicate changes, conflicting modes and unknown IDs return errors.

Generation settings, named color selection, variant configuration and templates remain fixed.
Invalidating a named color selection by removal or rename returns an error. No-op updates keep
font/render caches. Changed builds reuse geometry/encoder caches and invalidate companion renders.
Companions independent of changed fields retain their cached strings using the shared template
dependency rules. Failed builds retain old outputs and sources but discard partially mutated build caches before retry.

The consuming async method is `result.regenerate(membership, changes).await`. It returns
`Result<GenerateWebfontsFromSourcesResult, SourceRegenerateError>`. Recover ordinary failures
with `into_result()` or `into_parts()`; worker panic/cancellation returns no recoverable result.
For synchronous failures, the borrowed receiver remains usable.

Dropping an async future does not cancel already-started blocking work and does not return
the consumed result. CPU work may continue, with no memory-mode filesystem side effects.

These functions are Rust-only; the npm/NAPI exports and CLI still accept files.

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

`dest` is required. Use `files` for a single design of each icon or `variants` for a
[multi-variant family](#multi-variant-fonts). Set optional fields with `Some(...)`, or use
`..Default::default()` for their defaults.

| Field                   | Type                           | Default                                                                    | Description                                                  |
| ----------------------- | ------------------------------ | -------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `dest`                  | `String`                       | --                                                                         | Output directory (required)                                  |
| `files`                 | `Vec<String>`                  | Empty                                                                      | SVG paths for a single design                                |
| `color_glyphs`          | `Option<ColorGlyphSelection>`  | `None` (monochrome)                                                        | All or named color glyphs; see [color glyphs](#color-glyphs) |
| `font_name`             | `Option<String>`               | `"iconfont"`                                                               | Font family name                                             |
| `types`                 | `Option<Vec<FontType>>`        | `[Eot, Woff, Woff2]` for single-variant; `[Woff, Woff2]` for multi-variant | Font formats to generate                                     |
| `order`                 | `Option<Vec<FontType>>`        | Filtered default order                                                     | `@font-face` src order                                       |
| `css`                   | `Option<bool>`                 | `true`                                                                     | Generate CSS file                                            |
| `html`                  | `Option<bool>`                 | `false`                                                                    | Generate HTML preview                                        |
| `write_files`           | `Option<bool>`                 | `true`                                                                     | Write output to disk                                         |
| `css_template`          | `Option<String>`               | Built-in template                                                          | Custom Handlebars CSS template path                          |
| `html_template`         | `Option<String>`               | Built-in template                                                          | Custom Handlebars HTML template path                         |
| `css_fonts_url`         | `Option<String>`               | Relative path                                                              | URL prefix for fonts in CSS                                  |
| `css_dest`              | `Option<String>`               | `dest/fontName.css`                                                        | CSS output path                                              |
| `html_dest`             | `Option<String>`               | `dest/fontName.html`                                                       | HTML output path                                             |
| `codepoints`            | `Option<HashMap<String, u32>>` | Empty                                                                      | Explicit glyph codepoints                                    |
| `start_codepoint`       | `Option<u32>`                  | `0xF101`                                                                   | Starting auto-codepoint                                      |
| `font_height`           | `Option<f64>`                  | --                                                                         | Explicit font height                                         |
| `ascent`                | `Option<f64>`                  | --                                                                         | Font ascent                                                  |
| `descent`               | `Option<f64>`                  | --                                                                         | Font descent                                                 |
| `normalize`             | `Option<bool>`                 | `true`                                                                     | Normalize glyph heights                                      |
| `incremental`           | `Option<bool>`                 | `false`                                                                    | Retain parsed glyphs for `regenerate`                        |
| `fixed_width`           | `Option<bool>`                 | --                                                                         | Monospace font                                               |
| `center_horizontally`   | `Option<bool>`                 | --                                                                         | Center glyphs horizontally                                   |
| `center_vertically`     | `Option<bool>`                 | --                                                                         | Center glyphs vertically                                     |
| `ligature`              | `Option<bool>`                 | `true`                                                                     | Enable ligatures                                             |
| `round`                 | `Option<f64>`                  | --                                                                         | Path rounding precision                                      |
| `preserve_aspect_ratio` | `Option<bool>`                 | --                                                                         | Preserve SVG aspect ratio                                    |
| `optimize_output`       | `Option<bool>`                 | `false`                                                                    | Optimize SVG paths; font output is not guaranteed to shrink  |
| `font_style`            | `Option<String>`               | --                                                                         | CSS `font-style` value                                       |
| `font_weight`           | `Option<String>`               | --                                                                         | CSS `font-weight` value                                      |
| `missing_glyphs`        | `Option<MissingGlyphOptions>`  | `blank` in variant mode                                                    | Missing-glyph policy                                         |
| `format_options`        | `Option<FormatOptions>`        | --                                                                         | Per-format options                                           |
| `template_options`      | `Option<Map<String, Value>>`   | --                                                                         | Extra template context                                       |
| `variant_class_prefix`  | `Option<String>`               | `"icon--"`                                                                 | CSS variant modifier prefix                                  |
| `variants`              | `Option<Vec<FontVariant>>`     | --                                                                         | Ordered designs; see [`FontVariant`](#fontvariant)           |

## Color glyphs

By default, each SVG icon is flattened into a monochrome glyph. To preserve SVG colors, set `color_glyphs: Some(ColorGlyphSelection::All)` for all glyphs or `Some(ColorGlyphSelection::Named(vec!["logo".into()]))` for selected names on `GenerateWebfontsOptions`.

Named selection uses final post-rename logical names across every variant, including fallback states; blank states have no color record. `None` or an empty named list keeps monochrome output. Duplicates are harmless; unknown names are reported together in first-occurrence input order. Set `types` to TTF, WOFF, and/or WOFF2; ordinary defaults include incompatible EOT. See [color glyphs](./#color-glyphs) for SVG semantics and platform compatibility.

```rust
use webfont_generator::{generate_sync, ColorGlyphSelection, FontType, GenerateWebfontsOptions};

let result = generate_sync(GenerateWebfontsOptions {
    files: vec!["icons/logo.svg".into(), "icons/add.svg".into()],
    dest: "dist/fonts".into(),
    types: Some(vec![FontType::Woff2, FontType::Woff]),
    color_glyphs: Some(ColorGlyphSelection::Named(vec!["logo".into()])),
    incremental: Some(true),
    ..Default::default()
}, None)?;
```

The same option works with `variants` and the existing sync/async generation and regeneration APIs. Paint-only edits propagate to fallback consumers. Fresh and incremental builds of the same final inputs produce matching output. Paint and selection changes affect CSS font-URL hashes; name order and duplicates do not, and an empty list retains the omitted-selection hash.

Regeneration validates selected names after rename and membership changes. Removing the last source for a selected name fails. Selection, parsing, and font-build failures preserve the previous output for retry. Ordinary regeneration restores the previous in-memory result after a write failure; retry the changes or perform a full re-diff. Variant regeneration retains the newly committed in-memory result and pending writes; a no-op retry completes them. See [`GenerateWebfontsResult`](#generatewebfontsresult) for regeneration methods.

## Multi-variant fonts

A multi-variant family groups different designs of the same icons, such as light and bold,
into one font file per requested format. SVGs with matching filenames represent the same
logical icon across designs. Generated CSS lets you select a design using a modifier class.

```rust
use webfont_generator::{FontVariant, GenerateWebfontsOptions};

let result = webfont_generator::generate_sync(GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    font_name: Some("my-icons".to_owned()),
    variants: Some(vec![
        FontVariant {
            name: "light".to_owned(),
            files: vec!["icons/light/add.svg".to_owned()],
            weight: Some(300),
            default: Some(true),
        },
        FontVariant {
            name: "bold".to_owned(),
            files: vec!["icons/bold/add.svg".to_owned()],
            weight: Some(700),
            default: None,
        },
    ]),
    ..Default::default()
}, None)?;
```

This generates `my-icons.woff` and `my-icons.woff2`, plus `my-icons.css`. With that CSS loaded,
`class="icon icon-add"` uses the light design and `class="icon icon-add icon--bold"` uses bold.
TTF is also supported; SVG and EOT are not available for multi-variant families.
For sparse designs, set [`missing_glyphs`](#missingglyphoptions). Incremental regeneration is
available with `incremental: Some(true)` through the [variant methods](#variant-regeneration).

See [Templates](./templates) for CSS customization, SCSS, and the
[additional template context fields](./templates#template-context).

## `FontVariant`

One design in `GenerateWebfontsOptions::variants`. Supply at least two designs, each with
a unique name and a nonempty file list; exactly one must set `default: Some(true)`.
Leave the top-level `files` empty when using `variants`.

```rust
pub struct FontVariant {
    pub name: String,
    pub files: Vec<String>,
    pub weight: Option<u16>,
    pub default: Option<bool>,
}
```

| Field     | Meaning                                                                              |
| --------- | ------------------------------------------------------------------------------------ |
| `name`    | Design name, also used in its CSS modifier class. Whitespace and NUL are rejected.   |
| `files`   | SVG paths for this design. Match filenames across designs to identify the same icon. |
| `weight`  | Optional weight from 1–1000. Explicit weights must increase in variant order.        |
| `default` | Whether this design is used without a modifier class.                                |

With `weight: None`, the default resolves to 400. Other automatic weights are assigned outward
in steps of 100, or evenly within crowded explicit-weight intervals. All resolved weights must
be unique and increasing. Variant names do not change output filenames.

## `MissingGlyphBehavior`

Controls what happens when an icon exists in the family but is absent from one design.

```rust
pub enum MissingGlyphBehavior {
    Blank,
    Error,
    Fallback,
}
```

| Value      | Behavior                                                                       |
| ---------- | ------------------------------------------------------------------------------ |
| `Blank`    | Use an empty outline while retaining the icon's advance width. Default policy. |
| `Error`    | Reject the family and report missing design/icon pairs.                        |
| `Fallback` | Reuse the outline from a named design containing every icon.                   |

## `MissingGlyphOptions`

Sets the family-wide missing-icon policy through `GenerateWebfontsOptions::missing_glyphs`.

```rust
pub struct MissingGlyphOptions {
    pub behavior: MissingGlyphBehavior,
    pub variant: Option<String>,
}
```

Set `variant` to the fallback design's name only with `Fallback`; use `None` for `Blank` or `Error`.

```rust
use webfont_generator::{MissingGlyphBehavior, MissingGlyphOptions};

let blank = MissingGlyphOptions { behavior: MissingGlyphBehavior::Blank, variant: None };
let error = MissingGlyphOptions { behavior: MissingGlyphBehavior::Error, variant: None };
let fallback = MissingGlyphOptions {
    behavior: MissingGlyphBehavior::Fallback,
    variant: Some("Regular".to_owned()),
};
```

Set `missing_glyphs: Some(fallback)` (or another policy) in the generation options. The fallback
design must contain every logical icon; blank cells retain the logical icon's advance.

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

Unrequested formats return `None`. Multi-variant families return one shared resource per requested
format; their SVG/EOT getters return `None`. Output writes are non-transactional and can leave
a partial bundle if a write fails.

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

`None` uses generated URLs. `Some` replaces all defaults, leaving omitted entries empty;
multi-variant results reject SVG/EOT URL overrides. See the shared [Templates reference](./templates)
for context fields, generated CSS, HTML previews, and SCSS usage.

### Variant regeneration

Enable `incremental: Some(true)` when generating a family to retain per-design parse/process
caches. The same [incremental methods](#incremental-rebuild) support ordinary fonts and families.
Select the matching source mode with `RegenerationFiles`:

```rust
use webfont_generator::{GlyphChange, RegenerationFiles, VariantFileSet};

let files = RegenerationFiles::Variants(vec![
    VariantFileSet { variant: "light".into(), files: vec!["light/add.svg".into()] },
    VariantFileSet { variant: "bold".into(), files: vec!["bold/add.svg".into()] },
]);
let changes = vec![("bold/add.svg".into(), GlyphChange::Changed { name: None })];
result = result.regenerate_async(files, changes).await?;
```

#### `RegenerationFiles`

| Case       | Payload               | Meaning                                                |
| ---------- | --------------------- | ------------------------------------------------------ |
| `Single`   | `Vec<String>`         | Complete ordered inputs for an ordinary font           |
| `Variants` | `Vec<VariantFileSet>` | Complete ordered file sets for every configured design |

#### `VariantFileSet`

| Field     | Type          | Meaning                                                          |
| --------- | ------------- | ---------------------------------------------------------------- |
| `variant` | `String`      | Existing design name, present exactly once per update            |
| `files`   | `Vec<String>` | Nonempty, duplicate-free authoritative file list for this design |

Every update supplies every configured design. The update's design order does not change the
configured order; file ordering is independent within each design. Regeneration cannot change
the design list or weights themselves. Unknown/duplicate design names and invalid lists fail
before reading sources.

`GlyphChange` hints apply to paths across the family. `Added` introduces a path not previously
used by any design; `Changed` updates every final consumer of an existing path; `Removed`
requires that no design retain the path. Optional names apply to all consumers too. Membership
changes, including moving or sharing an existing path, need only changes to the file lists.
New memberships are loaded even without a hint. Use `regenerate_all` or `regenerate_all_async`
to read every file and infer additions, changes, and removals.

Successful updates match a fresh build of the final inputs, including union/codepoint assignment,
shared metrics, and fallback consumers. Unchanged parse/process work is reused; effective changes
reassemble the shared font bundle. No-op updates retain in-memory output and can retry writes.

Validation, source loading, and font-build failures preserve the previous state. State commits
before writes: a write failure leaves the new in-memory output available and may leave partial
disk files. Retry to complete output writes. The consuming async methods take `self`; recover the result
with `RegenerateError::into_result` on failure. Dropping the future does not cancel started
blocking work or its writes. Results created through Node context callbacks reject regeneration.

### Incremental rebuild

| Method                                      | Return type                     | Description                                                                 |
| ------------------------------------------- | ------------------------------- | --------------------------------------------------------------------------- |
| `regenerate(files, changes)`                | `io::Result<()>`                | Rebuild after known file changes, reusing unchanged glyphs                  |
| `regenerate_all(files)`                     | `io::Result<()>`                | Re-read/hash the full file set and infer added/changed/removed paths        |
| `regenerate_async(files, changes)`          | `Result<Self, RegenerateError>` | Consume the result and rebuild on Tokio's blocking pool                     |
| `regenerate_all_async(files)`               | `Result<Self, RegenerateError>` | Consume the result, re-diff, and rebuild on Tokio's blocking pool           |
| `regenerate_snapshot_async(files, changes)` | `io::Result<Self>`              | Borrow the result, preserving readable output while returning a replacement |

Requires the result to have been generated with `incremental: Some(true)` (errors otherwise).
Synchronous methods take `files: &RegenerationFiles`; async methods take an owned `RegenerationFiles`.
Each contained file list is the complete file set after the changes, in the order a fresh build
would use (e.g. the glob result); the rebuilt glyphs are ordered to match it, so the result is
byte-identical to a fresh build of that set — additions included, even when they sort before
existing glyphs. Any path absent from its list is dropped. `changes: &[(String, GlyphChange)]`
names the affected files: added/changed files are re-read from disk. Every format is refreshed in
memory, and — when the result was built with `write_files` — refreshed fonts are written to disk
too, while unchanged CSS/HTML companion files are skipped. Rendered
CSS/HTML is reused when glyph names and codepoints are unchanged. Use `regenerate_all` when you have
the fresh ordered file set but no reliable watcher change batch; existing glyph names are preserved,
and added paths derive their glyph name from the file stem.

`regenerate_async` and `regenerate_all_async` take owned inputs and consume the result, so Rust rejects stale-result
reuse after a successful rebuild. Assign the returned generation:

```rust
result = result.regenerate_async(files, changes).await?;
```

For ordinary regeneration failures, `RegenerateError::into_result()` returns the consumed result
so it can be retried. It returns `None` only if Tokio cancelled the blocking task before the result
could be returned. The consuming futures are not cancellation-safe: dropping one does not stop an
already-started blocking task, filesystem writes may continue, and the consumed result cannot be
recovered. Panics resume unwinding on the awaiting task.

`regenerate_snapshot_async` supports adapters that need the receiver to remain readable.
It borrows `&self` and takes `changes: Option<Vec<(String, GlyphChange)>>`: `Some(changes)`
supplies explicit changes; `None` re-diffs the complete membership. Success returns a new
result and prevents further regeneration of the old snapshot. Overlapping attempts are
rejected. Failure restores the old snapshot's regeneration state for retry, including
reconciliation after partial variant writes. Worker panics are returned as I/O errors.
Dropping this future does not cancel blocking work or writes: if the work succeeds, the
old snapshot remains replaced, but the new result is lost. Prefer the consuming methods
when Rust ownership can enforce replacement.

For Node.js results, `regenerate()` errors if the initial build used `cssContext` or `htmlContext`
callbacks because the synchronous method cannot re-run JavaScript callbacks during the rebuild.

```rust
pub enum GlyphChange {
    Added { name: Option<String> },   // new file; `name` overrides the file-stem glyph name
    Changed { name: Option<String> }, // content changed; `name` overrides the glyph name
    Removed,                          // file deleted
}

// result built with `incremental: Some(true)`
let files = RegenerationFiles::Single(vec!["icons/add.svg".to_owned(), "icons/remove.svg".to_owned()]);
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
