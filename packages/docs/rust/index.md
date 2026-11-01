---
description: Use the fonte engine directly from Rust with the webfont-generator crate, with no Node.js runtime.
---

# Rust

`webfont-generator` is the fonte engine itself, as a Rust crate. Every other fonte tool runs it: the [Node.js package](/node/) and the [Vite plugin](/vite/) through a native addon, and the [browser package](/browser/) as WebAssembly.

Use it in Rust programs, or anywhere you want font generation without a Node.js runtime. For a ready-made command, see the [CLI](/cli/), which ships in the same crate.

## Installation

```sh
cargo add webfont-generator
```

The default features cover file and in-memory generation with async APIs; see [feature flags](/rust/usage#feature-flags) for what each one enables.

## Your first font

The async API needs a [Tokio](https://tokio.rs/) runtime:

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

## Next steps

- [Usage](/rust/usage): the sync and SVG text APIs, persistent caching, incremental rebuilds, and every type.
- [API reference on docs.rs](https://docs.rs/webfont-generator)
- [Changelog](/rust/changelog)
