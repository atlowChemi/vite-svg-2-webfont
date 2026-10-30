# webfont-generator

<p align="center">
  <img src="../../packages/docs/public/webfont-generator-logo.png" alt="webfont-generator Rust engine logo" width="200" />
</p>

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

Text options and source-update types implement `serde::Deserialize` with camelCase fields
and reject unknown fields. Updates use `type`-tagged `added`, `changed`, and `removed` objects;
membership uses either `{ "sources": [...] }` or `{ "variants": [...] }`.
WOFF uses flate2's pure-Rust zlib-rs backend on both native and wasm32 targets.

Set `incremental: Some(true)` to use `regenerate_sync` or consuming async `regenerate`.
For single text actions, use `add_sync(source, variant)`, `update_sync(id, svg, name)`,
and `remove_sync(id)`, or consuming async `add`/`update`/`remove`. File results expose
`add(path, name, variant)`, `update(path, name)`, `remove(path)` and their `_async`
equivalents. Additions append; families require a target variant. Updates preserve order,
and removals apply across consuming designs without deleting files. All helpers reuse
regeneration's validation and failure semantics; full regeneration remains available for batches.
Supply `SourceChange` entries only for changed sources and complete `SourceMembership`
for the final order. Settings and templates stay fixed. Failed synchronous updates leave
the previous result usable; async failures expose it through `SourceRegenerateError::into_result`
unless the worker itself failed. See the [text-input reference](https://fonte.atlow.dev/webfont-generator/rust.html#svg-text-api).

An optional [persistent cache](https://fonte.atlow.dev/webfont-generator/rust#persistent-cache) (`cache_location`) lets later runs reuse unchanged work.

For color glyphs and named theme palettes, see the [color documentation](https://fonte.atlow.dev/webfont-generator/rust#color-glyphs).

## TypeScript output

Both file and SVG-text results expose `generate_ts() -> std::io::Result<Arc<String>>`.
The fixed module exports `icons` (a readonly tuple), `IconName` (its literal union),
`iconBaseSelector`, and `iconClassPrefix`. Strings are escaped as TypeScript literals.
Rendering does not load or render CSS/HTML templates and has no custom template option.
Names follow input order (deduplicated across variants); selectors reflect template options
and, for file results, the effective CSS context hook. Custom CSS need not use those settings.

Set `ts: Some(true)` to include TS in collected/written outputs. File output defaults to
`<dest>/<font_name>.ts`; override it with `ts_dest`. `write_files: Some(false)` still suppresses
all writes. Explicit `generate_ts()` works regardless of `ts` and shares repeated renders;
incremental membership updates refresh the output. SVG-text collection uses `OutputKind::Ts`.

## CLI

```sh
cargo install webfont-generator --features cli
webfont-generator --dest ./dist/fonts ./icons/
webfont-generator --dest ./dist/fonts --ts --ts-dest ./dist/icons.ts ./icons/
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

[Full Rust reference](https://fonte.atlow.dev/webfont-generator/rust.html) · [CLI reference](https://fonte.atlow.dev/webfont-generator/cli)
