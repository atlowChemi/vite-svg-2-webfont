# webfont-generator

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

## CLI

```sh
cargo install webfont-generator --features cli
webfont-generator --dest ./dist/fonts ./icons/
```

Default features are empty. `cli` enables the command-line binary and JSON configuration;
`bench` exposes benchmark helpers. The previous `napi` Cargo feature and Node-specific
Rust entry point have moved to the unpublished `webfont-generator-napi` workspace crate.
This is a Cargo API breaking change; existing npm imports remain unchanged.

Templates live in `packages/webfont-generator/templates/` and ship with the npm package.
The Rust engine renders defaults directly. Adapter tests verify parity with the shipped
templates; engine tests use engine-owned inputs without reading npm package assets.

[Full Rust reference](https://atlowChemi.github.io/vite-svg-2-webfont/webfont-generator/rust.html)
