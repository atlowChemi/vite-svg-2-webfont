---
description: Use the webfont-generator crate from Rust, from files or SVG text, synchronously or async, with caching, color glyphs, and families.
---

# Rust Usage

Add the crate first; see [Getting Started](/rust/). Every option is listed in [configuration](/rust/configuration), every function and type in the [API reference](/rust/api), and complete programs in [examples](/rust/examples).

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
Cargo build.

With `cli` enabled, `GenerateWebfontsOptions` and its input types implement `serde::Deserialize` using camelCase field names and rejecting unknown fields. Deserialization alone does not expand directories or rebase paths; those operations belong to the [CLI manifest loader](/cli/usage#json-manifest).

### Migrating from the embedded Node binding

The former `napi` Cargo feature and Node-specific Rust entry point have moved out of
this crate into the internal `webfont-generator-napi` workspace crate. Remove `napi`
from the features requested for `webfont-generator`. Rust callers use the engine APIs
below; integrations that need callbacks use [`GenerationHooks`](/rust/api#generate-with-hooks).

This is a Cargo API breaking change. The published Rust crate and CLI remain named
`webfont-generator`. JavaScript consumers continue to install and import
`@atlowchemi/webfont-generator`; they do not install the internal adapter crate.

## Choose an API

The crate generates fonts from SVG **files** or from SVG **text**, each synchronously or with Tokio:

|            | Files ([`fs`](#feature-flags))                                           | SVG text ([`in_memory`](#feature-flags))                                                |
| ---------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| Blocking   | [`generate_sync`](/rust/api#generate-sync), which starts its own runtime | [`generate_from_sources_sync`](/rust/api#generate-from-sources-sync), no runtime at all |
| With Tokio | [`generate`](/rust/api#generate)                                         | [`generate_from_sources`](/rust/api#generate-from-sources), with the `async` feature    |

Use the file API for build scripts and tools that read icons from disk and write fonts next to them. Use the text API when the icons come from somewhere else, such as a database, a network request, or a WebAssembly target without a file system; the [browser package](/browser/) is built on it.

```rust
use webfont_generator::{generate_sync, FontType, GenerateWebfontsOptions};

fn main() -> std::io::Result<()> {
    let result = generate_sync(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            files: vec!["icons/add.svg".to_owned(), "icons/remove.svg".to_owned()],
            types: Some(vec![FontType::Woff2]),
            ..Default::default()
        },
        None,
    )?;
    println!("{} bytes", result.woff2_bytes().map_or(0, <[u8]>::len));
    Ok(())
}
```

## Caching between runs

Set [`cache_location`](/rust/configuration#cache-location) to reuse work across processes with the [engine cache](/guide/features/caching). The text API takes the cache as bytes instead: pass them in [`cache`](/rust/configuration#in-memory-options) and store what [`cache_bytes()`](/rust/api#cache-bytes) returns. Remove the files the cache created with [`clear_cache()`](/rust/api#clear-cache).

## Color glyphs

Set [`color_glyphs`](/rust/configuration#color-glyphs) to keep the SVG colors of every icon, or of the icons you name, and `types` to TTF, WOFF, or WOFF2:

```rust
use webfont_generator::{ColorGlyphSelection, FontType, GenerateWebfontsOptions};

let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    files: vec!["icons/logo.svg".to_owned(), "icons/add.svg".to_owned()],
    types: Some(vec![FontType::Woff2, FontType::Woff]),
    color_glyphs: Some(ColorGlyphSelection::Named(vec!["logo".to_owned()])),
    ..Default::default()
};
```

See [color glyphs](/guide/features/color-glyphs) for theme palettes with `color_slots`, supported SVG paint, and browser support.

## Multi-weight families

Set [`variants`](/rust/configuration#variants) to combine designs of the same icons into one font per format. With the generated CSS loaded, `class="icon icon-add"` uses the default design and `class="icon icon-add icon--bold"` uses bold. See [multi-weight family](/rust/examples#multi-weight-family) for a complete program, and [variants](/guide/features/variants) for the rules.

## Updates

For watch mode, generate with [`incremental: Some(true)`](/rust/configuration#incremental) and update the result as files change, instead of generating again. See [incremental updates](/guide/features/incremental), [updating a file result](/rust/api#updating-a-file-result), and [updating an in-memory result](/rust/api#updating-an-in-memory-result).

## See also

- [The Engine](/guide/engine): architecture and design
- [CLI](/cli/): the engine as a command, from the same crate
- [docs.rs/webfont-generator](https://docs.rs/webfont-generator): the complete reference
