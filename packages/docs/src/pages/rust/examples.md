---
description: Examples for the webfont-generator crate — fonts at build time, fonts from SVG text, custom names, multi-weight families, watch mode, async generation, and caching.
---

# Rust Examples

Each example is a complete program. See [configuration](/rust/configuration) for every option and the [API reference](/rust/api) for the results.

## Fonts at build time

Generate the font in a build script and embed it in your binary, so the icons ship with the program and are rebuilt only when they change:

```toml [Cargo.toml]
[build-dependencies]
webfont-generator = "0.11"
```

```rust [build.rs]
use webfont_generator::{generate_sync, FontType, GenerateWebfontsOptions};

fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-changed=icons");

    let mut files: Vec<String> = std::fs::read_dir("icons")?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "svg"))
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    // Codepoints follow input order, and read_dir's order isn't stable.
    files.sort();

    generate_sync(
        GenerateWebfontsOptions {
            dest: std::env::var("OUT_DIR").expect("set by Cargo"),
            files,
            font_name: Some("icons".to_owned()),
            types: Some(vec![FontType::Woff2]),
            ..Default::default()
        },
        None,
    )?;
    Ok(())
}
```

```rust [src/main.rs]
static ICONS_WOFF2: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons.woff2"));
static ICONS_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/icons.css"));

fn main() {
    println!("{} bytes of font, {} bytes of CSS", ICONS_WOFF2.len(), ICONS_CSS.len());
}
```

## Fonts from SVG text

When the icons don't live in files, such as rows in a database or uploads, pass their text to the in-memory API. Nothing touches the file system until you decide where the outputs go:

```rust [src/main.rs]
use webfont_generator::{generate_from_sources_sync, FontType, GenerateWebfontsFromSourcesOptions, OutputKind, SvgSource};

fn main() -> std::io::Result<()> {
    // For example, rows loaded from a database.
    let rows = [
        ("add", r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M11 4h2v16h-2zM4 11h16v2H4z"/></svg>"#),
        ("remove", r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 11h16v2H4z"/></svg>"#),
    ];

    let result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        sources: rows
            .iter()
            .map(|(name, svg)| SvgSource { id: (*name).to_owned(), name: (*name).to_owned(), svg: (*svg).to_owned() })
            .collect(),
        types: Some(vec![FontType::Woff2]),
        css_fonts_url: Some("/fonts/".to_owned()),
        ..Default::default()
    })?;

    std::fs::create_dir_all("public/fonts")?;
    for output in result.collect_outputs()? {
        let path = match output.kind {
            OutputKind::Font(font_type) => format!("public/fonts/iconfont.{}", font_type.as_extension()),
            OutputKind::Css => "public/iconfont.css".to_owned(),
            OutputKind::Html | OutputKind::Ts => continue,
        };
        std::fs::write(path, output.contents.as_bytes())?;
    }
    Ok(())
}
```

## Custom icon names

Icon names come from file names by default. A [`RenameFn`](/rust/api#renamefn) can derive them some other way, here by prefixing each icon with its folder, so `icons/brand/github.svg` becomes `brand-github`:

```rust [src/main.rs]
use std::path::Path;
use webfont_generator::{generate_sync, GenerateWebfontsOptions, RenameFn};

fn main() -> std::io::Result<()> {
    let rename: RenameFn = Box::new(|path| {
        let path = Path::new(path);
        let folder = path.parent().and_then(Path::file_name).and_then(|name| name.to_str()).unwrap_or("icon");
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        format!("{folder}-{stem}")
    });

    generate_sync(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            files: vec!["icons/brand/github.svg".to_owned(), "icons/ui/add.svg".to_owned()],
            ..Default::default()
        },
        Some(rename),
    )?;
    Ok(())
}
```

Names become class names, so return only letters, digits, hyphens, and underscores.

## Multi-weight family

Combine light and bold drawings into one font per format. Matching file names identify the same icon in each design:

```rust [src/main.rs]
use webfont_generator::{generate_sync, FontVariant, GenerateWebfontsOptions, MissingGlyphBehavior, MissingGlyphOptions};

fn main() -> std::io::Result<()> {
    let design = |name: &str, weight: u16, default: bool| FontVariant {
        name: name.to_owned(),
        files: vec![format!("icons/{name}/add.svg"), format!("icons/{name}/search.svg")],
        weight: Some(weight),
        default: default.then_some(true),
    };

    generate_sync(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            font_name: Some("app-icons".to_owned()),
            variants: Some(vec![design("light", 300, true), design("bold", 700, false)]),
            missing_glyphs: Some(MissingGlyphOptions {
                behavior: MissingGlyphBehavior::Fallback,
                variant: Some("light".to_owned()),
            }),
            ..Default::default()
        },
        None,
    )?;
    Ok(())
}
```

With `app-icons.css` loaded, `class="icon icon-add icon--bold"` uses the bold design. See [variants](/guide/features/variants).

## Watch mode

Keep one incremental result and update it when the icons change, so only the changed icons are processed again. This loop polls the folder with the standard library; a file-watching crate can call the same update.

```rust [src/main.rs]
use std::time::{Duration, SystemTime};
use webfont_generator::{generate_sync, GenerateWebfontsOptions, RegenerationFiles};

fn list_icons() -> std::io::Result<(Vec<String>, SystemTime)> {
    let mut files = Vec::new();
    let mut newest = SystemTime::UNIX_EPOCH;
    for entry in std::fs::read_dir("icons")? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|extension| extension == "svg") {
            newest = newest.max(entry.metadata()?.modified()?);
            files.push(entry.path().to_string_lossy().into_owned());
        }
    }
    files.sort();
    Ok((files, newest))
}

fn main() -> std::io::Result<()> {
    let (files, mut seen) = list_icons()?;
    let mut result = generate_sync(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            files: files.clone(),
            incremental: Some(true),
            ..Default::default()
        },
        None,
    )?;
    let mut last_files = files;

    loop {
        std::thread::sleep(Duration::from_millis(500));
        let (files, newest) = list_icons()?;
        if files == last_files && newest <= seen {
            continue;
        }
        // Re-reads the files, works out what changed, and rebuilds only those icons.
        match result.regenerate_all(&RegenerationFiles::Single(files.clone())) {
            Ok(()) => println!("rebuilt {} icons", files.len()),
            Err(error) => eprintln!("kept the last good font: {error}"),
        }
        (last_files, seen) = (files, newest);
    }
}
```

See [updating a file result](/rust/api#updating-a-file-result) for change hints and the async variants.

## Async generation

Inside a Tokio application, `generate` runs on the blocking pool, so it doesn't stall other tasks:

```toml [Cargo.toml]
[dependencies]
webfont-generator = "0.11"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust [src/main.rs]
use webfont_generator::{generate, FontType, GenerateWebfontsOptions};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let result = generate(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            files: vec!["icons/add.svg".to_owned()],
            types: Some(vec![FontType::Woff2]),
            ..Default::default()
        },
        None,
    )
    .await?;
    println!("{} bytes", result.woff2_bytes().map_or(0, <[u8]>::len));
    Ok(())
}
```

## Cache between runs

Keep the [engine cache](/guide/features/caching) in a directory, so later runs reuse unchanged icons and fonts. A fixed TTF timestamp keeps the output identical whether fonts come from the cache or not:

```rust [src/main.rs]
use webfont_generator::{generate_sync, FormatOptions, GenerateWebfontsOptions, TtfFormatOptions};

fn main() -> std::io::Result<()> {
    generate_sync(
        GenerateWebfontsOptions {
            dest: "dist/fonts".to_owned(),
            files: vec!["icons/add.svg".to_owned()],
            cache_location: Some(".webfont-cache".to_owned()),
            format_options: Some(FormatOptions {
                ttf: Some(TtfFormatOptions {
                    ts: Some(0),
                    copyright: None,
                    description: None,
                    url: None,
                    version: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        None,
    )?;
    Ok(())
}
```

Remove the files the cache created with [`clear_cache(".webfont-cache")`](/rust/api#clear-cache).
