# @atlowchemi/webfont-generator

A native Rust [NAPI](https://napi.rs) addon that generates webfonts (SVG, TTF, EOT, WOFF, WOFF2) and their companion CSS/HTML from a set of SVG icon files.

This is a ground-up rewrite of [`@vusion/webfonts-generator`](https://github.com/vusion/webfonts-generator) in Rust — the original package and its authors deserve credit for the API design and template system that this project builds on. The JS implementation is unmaintained, so this package reimplements the same pipeline natively for better performance and long-term maintainability.

The API is largely compatible with `@vusion/webfonts-generator`, with a few differences:

- The `cssContext` and `htmlContext` callbacks receive only the context object. The original also passed `options` and the `handlebars` instance as additional arguments — those are no longer available.
- Generated font binaries (TTF, WOFF, etc.) may differ at the byte level because a different encoder is used, but the fonts are equally valid.
- CSS, HTML, and template output is identical.

Performance scales better with glyph count — for larger icon sets the native pipeline is significantly faster.

## Node.js (npm)

```bash
npm install @atlowchemi/webfont-generator
```

Pre-built binaries are published for the following targets:

| Platform       | Architecture      |
| -------------- | ----------------- |
| macOS          | x64, arm64        |
| Linux (glibc)  | x64, arm64, armv7 |
| Linux (musl)   | x64, arm64        |
| Windows (MSVC) | x64, arm64        |

```js
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    files: ['./icons/home.svg', './icons/search.svg'],
    dest: './dist/fonts',
    fontName: 'my-icons',
    types: ['woff2', 'woff'],
});

const css = result.generateCss();
const html = result.generateHtml();
```

## Rust library (crates.io)

```bash
cargo add webfont-generator
```

```rust
use webfont_generator::{GenerateWebfontsOptions, FontType};

let result = webfont_generator::generate_sync(
    GenerateWebfontsOptions {
        dest: "dist/fonts".to_owned(),
        files: vec!["icons/home.svg".to_owned(), "icons/search.svg".to_owned()],
        font_name: Some("my-icons".to_owned()),
        types: Some(vec![FontType::Woff2, FontType::Woff]),
        ..Default::default()
    },
    None,
).unwrap();

let css = result.generate_css_pure(None).unwrap();
```

An async API (`webfont_generator::generate`) is also available for use with tokio.

## CLI

The CLI is available as an opt-in feature (to avoid pulling in `clap` for library users):

```bash
cargo install webfont-generator --features cli
```

### Usage

```
webfont-generator [OPTIONS] --dest <DEST> <FILES>...
```

### Examples

```bash
# Generate default formats (eot, woff, woff2) from a directory of SVGs
webfont-generator --dest ./dist/fonts ./icons/

# Generate specific formats with a custom font name
webfont-generator --dest ./dist/fonts --types woff2,woff --font-name my-icons ./icons/

# Generate fonts with an HTML preview page
webfont-generator --dest ./dist/fonts --html ./icons/*.svg
```

### Options

```
Arguments:
  <FILES>...  SVG files or directories containing SVG files

Options:
  -d, --dest <DEST>                        Output directory
  -n, --font-name <FONT_NAME>              Font name [default: iconfont]
  -t, --types <TYPES>                      Font types to generate [possible values: svg, ttf, eot, woff, woff2]
      --css                                Generate CSS (default)
      --no-css                             Skip CSS generation
      --html                               Generate HTML preview
      --no-html                            Skip HTML generation (default)
      --css-template <CSS_TEMPLATE>        Custom CSS template path
      --html-template <HTML_TEMPLATE>      Custom HTML template path
      --css-fonts-url <CSS_FONTS_URL>      CSS fonts URL prefix
      --write                               Write output files to disk (default)
      --no-write                           Do not write output files (dry run)
      --ligature                           Enable ligatures (default)
      --no-ligature                        Disable ligatures
      --font-height <FONT_HEIGHT>          Font height
      --ascent <ASCENT>                    Ascent value
      --descent <DESCENT>                  Descent value
      --start-codepoint <START_CODEPOINT>  Start codepoint (hex, e.g. 0xF101)
  -h, --help                               Print help
  -V, --version                            Print version
```

## Templates

Default CSS, SCSS, and HTML templates are available via the `/templates` export:

```js
import { templates } from '@atlowchemi/webfont-generator/templates';

console.log(templates.css); // path to default CSS template
console.log(templates.scss); // path to default SCSS template
console.log(templates.html); // path to default HTML template
```

## License

[MIT](../../LICENSE)
