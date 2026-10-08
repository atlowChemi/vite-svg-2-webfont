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

## Color glyphs

Use `color_glyphs: Some(webfont_generator::ColorGlyphSelection::All)` or `Some(webfont_generator::ColorGlyphSelection::Named(vec!["logo".into()]))` to preserve solid SVG paint. Omission or an empty list preserves monochrome output. Selection uses final post-rename logical names in ordinary and multi-variant fonts, including incremental regeneration.

Active color requires TTF, WOFF, or WOFF2; set `types` explicitly because ordinary defaults include EOT. Fonts retain a monochrome fallback. Tested macOS WebKit uses that fallback, while Chromium, Firefox, and Linux WebKit render color. Advanced SVG is best-effort. See the [color reference](https://atlowchemi.github.io/vite-svg-2-webfont/webfont-generator/color) for paint semantics, lifecycle behavior, and platform limitations.

## CLI

```sh
cargo install webfont-generator --features cli
webfont-generator --dest ./dist/fonts ./icons/
```

Default features are empty. `cli` enables the command-line binary and JSON configuration;
`bench` exposes benchmark helpers. The previous `napi` Cargo feature and Node-specific
Rust entry point have moved to the unpublished `webfont-generator-napi` workspace crate.
This is a Cargo API breaking change; existing npm imports remain unchanged.

Remove `napi` from your Cargo dependency features when migrating. Rust integrations
use the engine APIs above and `GenerationHooks` for callback behavior. JavaScript users
install `@atlowchemi/webfont-generator` from npm, which supplies the platform addon;
they do not need the internal adapter crate or a Rust toolchain on supported platforms.

Templates live in `packages/webfont-generator/templates/` and ship with the npm package.
The Rust engine renders defaults directly. Adapter tests verify parity with the shipped
templates; engine tests use engine-owned inputs without reading npm package assets.

[Full Rust reference](https://atlowChemi.github.io/vite-svg-2-webfont/webfont-generator/rust.html)
