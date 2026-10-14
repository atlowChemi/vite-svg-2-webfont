# webfont-generator

[![crates.io](https://img.shields.io/crates/v/webfont-generator.svg?style=flat-square)](https://crates.io/crates/webfont-generator)
[![docs.rs](https://img.shields.io/docsrs/webfont-generator?style=flat-square)](https://docs.rs/webfont-generator)
[![license](https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square)](https://github.com/atlowChemi/vite-svg-2-webfont/blob/master/LICENSE)

[Changelog](./CHANGELOG.md) · [Engine releases](https://github.com/atlowChemi/vite-svg-2-webfont/releases?q=webfont-engine-v)

Rust library and optional CLI for generating SVG, TTF, EOT, WOFF and WOFF2 fonts from SVG icons.

```rust,no_run
use webfont_generator::{generate_sync, GenerateWebfontsOptions};

let result = generate_sync(GenerateWebfontsOptions {
    files: vec!["icons/add.svg".into()],
    dest: "dist/fonts".into(),
    write_files: Some(false),
    ..Default::default()
}, None)?;
# Ok::<(), std::io::Error>(())
```

`generate` is the Tokio async entry point. `generate_with_hooks` accepts runtime-independent
`GenerationHooks` for asynchronous batch renaming and CSS/HTML context mutation.
`generate` delegates to that same pipeline through a synchronous rename hook; input paths
are borrowed, and option conversion does not clone their strings or template JSON values.
`GenerateWebfontsResult` owns fonts, rendering caches and incremental regeneration state.
Use consuming `regenerate_async`/`regenerate_all_async` in Rust, or the borrowing
`regenerate_snapshot_async` when an adapter must retain a readable old result.

## SVG text input

`generate_from_sources_sync` accepts named SVG text without files or a Tokio runtime:

```rust
use webfont_generator::{generate_from_sources_sync, GenerateWebfontsFromSourcesOptions, SvgSource};

let result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
    sources: vec![SvgSource {
        id: "icon-42".into(),
        name: "square".into(),
        svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#.into(),
    }],
    ..Default::default()
})?;
let css = result.generate_css()?; // Shared, lazy render; no output files.
let outputs = result.collect_outputs()?; // Reuses CSS and font payloads.
# Ok::<(), std::io::Error>(())
```

`generate_from_sources` runs the same pipeline on Tokio's blocking pool. Text options
support ordinary and variant designs, color glyphs, and inline CSS/HTML templates.
IDs are opaque and must be nonempty; shared IDs across designs must agree on name and text.
Glyph names accept Unicode letters/numbers, hyphens and underscores; empty or unsafe names are rejected.
External SVG image paths are not loaded.

Set `incremental: Some(true)` to use `regenerate_sync` or consuming async `regenerate`.
Supply `SourceChange` entries only for changed sources and complete `SourceMembership`
for the final order. Settings and templates stay fixed. Failed synchronous updates leave
the previous result usable; async failures expose it through `SourceRegenerateError::into_result`
unless the worker itself failed. See the [text-input reference](https://atlowChemi.github.io/vite-svg-2-webfont/webfont-generator/rust.html#svg-text-api).

## CLI

```sh
cargo install webfont-generator --features cli
webfont-generator --dest ./dist/fonts ./icons/
```

## Cargo features

| Feature     | Default | Enables                                                                                                |
| ----------- | ------- | ------------------------------------------------------------------------------------------------------ |
| `fs`        | yes     | File/path APIs, hooks, file templates and output writers; implies `async` and Tokio filesystem support |
| `in_memory` | yes     | SVG text APIs, inline templates, output collection and source updates                                  |
| `async`     | yes     | Tokio-backed asynchronous APIs; synchronous text APIs do not need it                                   |
| `cli`       | no      | Command-line binary and JSON configuration; implies `fs`                                               |
| `bench`     | no      | Internal benchmark helpers; implies `fs` and `in_memory`                                               |

The `async` feature enables text generation and regeneration on Tokio's blocking pool. File-only
consumers can select `fs`. Even synchronous file generation uses Tokio internally,
so `fs` always enables `async`. Features are additive when Cargo unifies dependencies.

The previous `napi` Cargo feature and Node-specific
Rust entry point have moved to the unpublished `webfont-generator-napi` workspace crate.
This is a Cargo API breaking change; existing npm imports remain unchanged.

Remove `napi` from your Cargo dependency features when migrating. Rust integrations
use the engine APIs above and `GenerationHooks` for callback behavior. JavaScript users
install `@atlowchemi/webfont-generator` from npm, which supplies the platform addon;
they do not need the internal adapter crate or a Rust toolchain on supported platforms.

Templates live in `packages/webfont-generator/templates/` and ship with the npm package.
The Rust engine renders defaults directly. Adapter tests verify parity with the shipped
templates; engine tests use engine-owned inputs without reading npm package assets.

[Full Rust reference](https://atlowChemi.github.io/vite-svg-2-webfont/webfont-generator/rust.html) · [CLI reference](https://atlowChemi.github.io/vite-svg-2-webfont/webfont-generator/cli)
