---
description: CLI reference for the webfont-generator command-line tool, including installation, usage examples, and all options.
---

# CLI Usage

## Installation

```sh
cargo install webfont-generator --features cli
```

::: tip
The `cli` feature is opt-in and not included in the default feature set. This avoids pulling in `clap` for library users.
:::

## Usage

```sh
webfont-generator [OPTIONS] --dest <DEST> <FILES>...
webfont-generator --config <PATH>
```

`<FILES>` accepts individual SVG file paths or directories. When a directory is given, all `.svg` files inside it are included (non-recursive, sorted alphabetically).

## Examples

Generate default formats (EOT, WOFF, WOFF2) from a directory:

```sh
webfont-generator --dest ./dist/fonts ./icons/
```

Custom font name and specific types:

```sh
webfont-generator --dest ./dist/fonts --font-name my-icons --types woff2,woff ./icons/
```

Generate with an HTML preview page:

```sh
webfont-generator --dest ./dist/fonts --html ./icons/
```

Dry run (no files written to disk):

```sh
webfont-generator --dest ./dist/fonts --no-write ./icons/
```

Custom start codepoint in hex:

```sh
webfont-generator --dest ./dist/fonts --start-codepoint 0xE000 ./icons/
```

## Options reference

### JSON manifest

`--config <PATH>` loads a complete JSON configuration using the camelCase [generator options](./node). It cannot be combined with positional inputs or any generation flag, even a flag set to its default. `--help` and `--version` still work.

```json
{
    "dest": "dist/fonts",
    "fontName": "icons",
    "html": true,
    "variants": [
        { "name": "outline", "files": ["icons/outline"], "weight": 300, "default": true },
        { "name": "filled", "files": ["icons/filled"], "weight": 700 }
    ],
    "missingGlyphs": { "behavior": "blank" }
}
```

Run it with `webfont-generator --config icons.webfont.json`.

- `dest` is required. Supply ordinary `files` or ordered `variants`; variant validation and automatic weights follow the generator API.
- Relative input paths, `dest`, `cssDest`, `htmlDest`, `cssTemplate`, and `htmlTemplate` resolve against the manifest directory, independent of the working directory. URL values such as `cssFontsUrl` are not rebased.
- Input arrays preserve explicit entry order. Each directory expands at its position into sorted lowercase `.svg` files, non-recursively. JSON paths do not expand wildcards or globs.
- Missing entries and duplicate normalized input paths within one variant (or the ordinary file list) are errors. The same source may appear in different variants.
- Unknown option fields and invalid JSON types are rejected, including nested options. Diagnostics include the manifest path and the affected option field; malformed JSON includes line and column information.
- JSON uses numeric codepoints: for example, `"startCodepoint": 57344` instead of hexadecimal syntax. Callbacks (`rename`, `cssContext`, `htmlContext`) cannot be represented in a manifest; use the Node API for callbacks.
- Library defaults apply: ordinary output is EOT/WOFF/WOFF2; variants default to WOFF/WOFF2 and accept only TTF/WOFF/WOFF2. Use `"writeFiles": false` for a dry run.

### Positional arguments

| Argument     | Description                                                                 |
| ------------ | --------------------------------------------------------------------------- |
| `<FILES>...` | SVG files or directories containing SVG files (required without `--config`) |

### Required options

| Flag                | Description                                    |
| ------------------- | ---------------------------------------------- |
| `-d, --dest <DEST>` | Output directory (required without `--config`) |

### Font options

| Flag                      | Default          | Description                                 |
| ------------------------- | ---------------- | ------------------------------------------- |
| `-n, --font-name <NAME>`  | `iconfont`       | Font family name                            |
| `-t, --types <TYPES>`     | `eot,woff,woff2` | Comma-separated font types to generate      |
| `--font-height <N>`       | --               | Explicit font height                        |
| `--ascent <N>`            | --               | Font ascent value                           |
| `--descent <N>`           | --               | Font descent value                          |
| `--start-codepoint <HEX>` | `0xF101`         | Starting codepoint for auto-assigned glyphs |

### Output control

| Flag                           | Default      | Description                        |
| ------------------------------ | ------------ | ---------------------------------- |
| `--css` / `--no-css`           | `--css`      | Generate or skip CSS output        |
| `--html` / `--no-html`         | `--no-html`  | Generate or skip HTML preview      |
| `--write` / `--no-write`       | `--write`    | Write files to disk or dry run     |
| `--ligature` / `--no-ligature` | `--ligature` | Enable or disable ligature support |

### Template options

| Flag                     | Description                     |
| ------------------------ | ------------------------------- |
| `--css-template <PATH>`  | Custom Handlebars CSS template  |
| `--html-template <PATH>` | Custom Handlebars HTML template |
| `--css-fonts-url <URL>`  | URL prefix for fonts in CSS     |

### Meta

| Flag            | Description   |
| --------------- | ------------- |
| `-h, --help`    | Print help    |
| `-V, --version` | Print version |

## See also

- [Overview](./) -- architecture and design
- [Node.js usage](./node) -- npm package API reference
- [Rust usage](./rust) -- crate API reference
