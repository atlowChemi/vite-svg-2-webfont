# webfont-generator

<p align="center">
  <img src="../../packages/docs/assets/brand/rust.svg" alt="fonte for Rust" width="160" />
</p>

<p align="center">
  <a href="https://crates.io/crates/webfont-generator"><img src="https://img.shields.io/crates/v/webfont-generator.svg?style=flat-square" alt="crates.io" /></a>
  <a href="https://docs.rs/webfont-generator"><img src="https://img.shields.io/docsrs/webfont-generator?style=flat-square" alt="docs.rs" /></a>
  <a href="https://fonte.atlow.dev/rust/"><img src="https://img.shields.io/badge/docs-fonte.atlow.dev-ff6a13?style=flat-square" alt="docs" /></a>
  <a href="https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE"><img src="https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square" alt="license" /></a>
</p>

The [fonte](https://fonte.atlow.dev/) engine as a Rust crate, with an optional CLI. It generates SVG, TTF, EOT, WOFF, and WOFF2 fonts, and their CSS, HTML preview, and TypeScript names, from SVG files or SVG text. Every other fonte tool runs it: the Node.js package and the Vite plugin through a native addon, and the browser package as WebAssembly.

## Installation

```sh
cargo add webfont-generator
```

## Quick start

`generate_sync` writes the font files and returns once they are done:

```rust,no_run
use webfont_generator::{GenerateWebfontsOptions, FontType};

fn main() -> std::io::Result<()> {
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

    let result = webfont_generator::generate_sync(options, None)?;

    if let Some(woff2) = result.woff2_bytes() {
        println!("Generated WOFF2: {} bytes", woff2.len());
    }

    Ok(())
}
```

In async code, use `generate` instead; it needs a [Tokio](https://tokio.rs/) runtime. To generate from SVG text without touching the file system, use `generate_from_sources_sync`.

## CLI

The crate also ships a command-line tool, behind the opt-in `cli` feature:

```sh
cargo install webfont-generator --features cli
webfont-generator --dest ./dist/fonts ./icons/
```

See the [CLI docs](https://fonte.atlow.dev/cli/) for every flag and the JSON manifest.

## Cargo features

| Feature     | Default | Enables                                                                                                |
| ----------- | ------- | ------------------------------------------------------------------------------------------------------ |
| `fs`        | yes     | File/path APIs, hooks, file templates and output writers; implies `async` and Tokio filesystem support |
| `in_memory` | yes     | SVG text APIs, inline templates, output collection and source updates                                  |
| `async`     | yes     | Tokio-backed asynchronous APIs; synchronous text APIs do not need it                                   |
| `cli`       | no      | Command-line binary and JSON configuration; implies `fs`                                               |
| `bench`     | no      | Internal benchmark helpers; implies `fs` and `in_memory`                                               |

The former `napi` feature has moved out of this crate; see [migrating from the embedded Node binding](https://fonte.atlow.dev/rust/usage#migrating-from-the-embedded-node-binding).

## Documentation

- [Getting started](https://fonte.atlow.dev/rust/)
- [Usage](https://fonte.atlow.dev/rust/usage): feature flags, choosing an API, caching, color glyphs, and families
- [Examples](https://fonte.atlow.dev/rust/examples): fonts at build time, from SVG text, watch mode, and more
- [Configuration](https://fonte.atlow.dev/rust/configuration): every field of the options
- [API](https://fonte.atlow.dev/rust/api): every function, result, and type
- [docs.rs](https://docs.rs/webfont-generator): the complete reference
- [Changelog](https://fonte.atlow.dev/rust/changelog)

## License

[MIT](https://github.com/atlowChemi/vite-svg-2-webfont/blob/main/LICENSE)
