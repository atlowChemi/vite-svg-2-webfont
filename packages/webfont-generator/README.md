# @atlowchemi/webfont-generator

<p align="center">
  <img src="../docs/public/webfont-generator-logo.png" alt="webfont-generator logo" width="200" />
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@atlowchemi/webfont-generator"><img src="https://img.shields.io/npm/v/@atlowchemi/webfont-generator.svg?style=flat-square" alt="npm" /></a>
  <a href="https://crates.io/crates/webfont-generator"><img src="https://img.shields.io/crates/v/webfont-generator.svg?style=flat-square" alt="crates.io" /></a>
  <a href="https://docs.rs/webfont-generator"><img src="https://img.shields.io/docsrs/webfont-generator?style=flat-square" alt="docs.rs" /></a>
  <a href="https://github.com/atlowChemi/vite-svg-2-webfont/blob/master/LICENSE"><img src="https://img.shields.io/github/license/atlowChemi/vite-svg-2-webfont.svg?style=flat-square" alt="license" /></a>
</p>

A native Rust [NAPI](https://napi.rs) addon that generates webfonts (SVG, TTF, EOT, WOFF, WOFF2) and their companion CSS/HTML from a set of SVG icon files.

This is a ground-up rewrite of [`@vusion/webfonts-generator`](https://github.com/vusion/webfonts-generator) in Rust — the original package and its authors deserve credit for the API design and template system that this project builds on. The JS implementation is unmaintained, so this package reimplements the same pipeline natively for better performance and long-term maintainability.

The API is largely compatible with `@vusion/webfonts-generator`, with a few differences:

- The `cssContext` and `htmlContext` callbacks receive only the context object. The original also passed `options` and the `handlebars` instance as additional arguments — those are no longer available.
- `formatOptions` is now strictly typed as `{ svg?: SvgFormatOptions; ttf?: TtfFormatOptions; woff?: WoffFormatOptions; woff2?: Woff2FormatOptions }`. The original accepted arbitrary `{ [format]: unknown }`; the `eot` key is no longer accepted (EOT is derived from the TTF output).
- A new `optimizeOutput` option runs an SVG path optimizer over each glyph before assembling the font. Defaults to `false`; opt in for smaller output bytes at a small build-time cost. Also available as `formatOptions.svg.optimizeOutput`.
- `formatOptions.woff2.compressionQuality` sets the Brotli compression quality (0–11) for WOFF2 output. Defaults to `11` (smallest output); lower it (e.g. to `10`) for faster encoding at a marginal size cost.
- A new `incremental` option (default `false`) retains parsed glyph data on the result so `result.regenerateAsync(files, changes?)` can rebuild after file changes without re-parsing the glyphs that didn't change or blocking the Node.js event loop. It refreshes the outputs in memory and, when the result was generated with `writeFiles`, writes refreshed fonts to disk too while skipping unchanged CSS/HTML companion files. You pass the full file set (in the order a fresh build would use) plus what changed, or omit `changes` to re-read/hash the full set and infer changes; the result is byte-identical to a fresh `generateWebfonts()` of that set, additions included.
- Generated font binaries (TTF, WOFF, etc.) may differ at the byte level because a different encoder is used, but the fonts are equally valid.
- CSS, HTML, and template output is identical.

Performance scales better with glyph count — for larger icon sets the native pipeline is significantly faster.

### Multi-weight variants

Variant formats default to WOFF/WOFF2; TTF is also supported. Ordinary format defaults are unchanged.

The Rust and Node APIs support multi-variant input through
`FontVariant`, `MissingGlyphBehavior`, and `MissingGlyphOptions`. Set either ordinary `files` or
`variants`, not both; variant mode requires at least two uniquely named variants and exactly one
default. Explicit weights are anchors in the range 1–1000. An automatic default resolves to 400;
other automatic weights resolve outward in steps of 100, or evenly within crowded anchor
intervals. Final weights must follow variant order and remain unique. Variant names determine CSS
modifier classes, not output filenames. The default
missing-glyph behavior is `blank`; `fallback` requires an existing variant that contains every
logical glyph in the family. SVG/EOT output and incremental mode are invalid with variants.

Generation returns one shared variable font per requested modern format through the existing
TTF/WOFF/WOFF2 getters and writes `fontName.ttf`, `fontName.woff`, and `fontName.woff2`.
SVG/EOT getters are empty for variant results. Writes are non-transactional: a failure can leave a
partial bundle. Variant regeneration, including async methods, returns an unsupported error.

The existing CSS/HTML render methods accept shared flat URL maps. A supplied map completely
overrides generated URLs; missing entries become empty. Variant SVG/EOT overrides are rejected.
Custom-template contexts receive ordered `variants` entries with `name`, `weight`, `default`,
`className`, and `selector`, plus `variantClassPrefix`. No extra result getters are needed.
Default CSS emits one exact-weight `@font-face` per variant, sharing the modern URLs. Icon
pseudo-elements use the default weight and `font-synthesis: none`; add a modifier such as
`icon--bold` alongside the glyph class to select another variant. A modifier alone emits no glyph.
CSS/HTML companion files are written when enabled; HTML shows one grid in the default variant.
Contexts also expose `defaultWeight` and `fontStyle` (default `normal`). The SCSS
`webfont-icon($name)` mixin uses each icon family's default weight; modifier classes select variants.
Variant icon-map entries contain `(family, codepoint, weight, style, variantsMap)`. The fifth item
maps CSS-escaped modifier identifiers (without a leading dot) to numeric weights, which the mixin
uses to emit modifiers scoped to its caller's selector. Ordinary family/codepoint pairs remain supported.

Non-exact weights use CSS font matching, not interpolation: with faces at 300, 400, and 700,
requests for 100/350 select 300, 450/500 select 400, and 600/900 select 700. Generated
pseudo-elements explicitly set their weight, so inherited weights do not override them.

### TypeScript inference and callback metadata

TypeScript infers output getters from literal `types` lists. With `types` omitted, ordinary
calls expose non-null EOT/WOFF/WOFF2, and variant calls expose non-null WOFF/WOFF2; other getters
are `null`. A dynamically selected format array has nullable getters because its contents are
not known statically. Variables typed as the ordinary/variant options union are also accepted.

Explicit single-format generics, such as `generateWebfonts<'svg'>({ files, dest, types: ['svg'] })`,
retain a non-null getter with a nonempty `types` tuple. Modern variant formats behave the same;
explicit union generics and widened arrays retain conservative nullable getters.

Variant options infer `CssContext<true>` / `HtmlContext<true>` callbacks with typed `variants: TemplateVariant[]`, `variantClassPrefix`,
`defaultWeight`, and `fontStyle`. Plain `CssContext` / `HtmlContext` keep these fields `unknown` because ordinary template options may supply arbitrary values. Variant generation supplies these fields; ordinary generation
does not supply them by default. Each `TemplateVariant` has `name`, `weight`, `default`,
`className`, and `selector` (an escaped CSS identifier without a leading dot).

### Incremental regeneration

```js
let files = ['./icons/home.svg', './icons/search.svg'];
let result = await generateWebfonts({ files, dest, fontName: 'my-icons', incremental: true });

// On a watch event, rebuild reusing cached geometry for unchanged glyphs. Pass the full file set
// (in fresh-build order) so additions land in the right position, plus what changed:
result = await result.regenerateAsync(files, [{ path: './icons/home.svg', changeType: 'changed' }]);
// Or omit changes when watcher hints are unavailable/untrusted:
result = await result.regenerateAsync(files);
result.woff2; // refreshed bytes
```

The first argument is the complete file set after the change, in the order a fresh build would use (e.g. your glob result) — any file omitted from it is dropped. Each change is `{ path, changeType: 'added' | 'changed' | 'removed', name? }`, where `name` is the resolved glyph name if you apply a custom rename; otherwise added files derive their name from the file stem, changed files keep their current name, and removed files ignore it. When `changes` is omitted or `null`, every current file is re-read and hashed to detect added/changed/removed paths automatically. `regenerateAsync()` returns a replacement result; the receiver stays readable and unchanged while work runs and after failure. Assign the replacement before starting another rebuild because overlapping calls from the same result lineage are rejected. Disk writes are not transactional. The synchronous, mutating `regenerate()` method remains available when blocking the event loop is acceptable. Results generated with `cssContext` or `htmlContext` callbacks cannot be regenerated because those JavaScript callbacks cannot be re-run during a rebuild.

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

Async generation and incremental regeneration are available for Tokio applications. Async
regeneration consumes the old result, preventing stale-result reuse, and returns the next result:

```rust
let result = result.regenerate_async(files.clone(), changes).await?;
// Or re-diff the complete file set:
let result = result.regenerate_all_async(files).await?;
```

On failure, `RegenerateError::into_result()` recovers the consumed result for retry when the
blocking task returned normally. These consuming futures are not cancellation-safe: dropping one
does not stop already-started blocking work and the consumed result cannot be recovered.

## CLI

Use a complete JSON configuration for ordinary or multi-weight generation:

```sh
webfont-generator --config icons.webfont.json
```

```json
{
    "dest": "dist/fonts",
    "fontName": "icons",
    "variants": [
        { "name": "outline", "files": ["icons/outline"], "weight": 300, "default": true },
        { "name": "filled", "files": ["icons/filled"], "weight": 700 }
    ]
}
```

Manifest keys use the generator's camelCase option names. Relative input, output, and template paths resolve against the manifest directory. Input directories expand non-recursively into sorted lowercase `.svg` files at their array position; globs are not expanded. Duplicate normalized paths within a variant and nonexistent inputs are errors. Unknown fields and invalid values produce manifest-specific diagnostics.

`--config` cannot be combined with positional inputs or generation flags; only `--help` and `--version` coexist. Ordinary manifests use `files` and retain SVG/EOT support. Variant manifests default to WOFF/WOFF2 and accept only TTF/WOFF/WOFF2. JSON callbacks are unsupported; use the Node API for callbacks. See the [CLI reference](https://atlowchemi.github.io/vite-svg-2-webfont/webfont-generator/cli) for the complete manifest contract.

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
