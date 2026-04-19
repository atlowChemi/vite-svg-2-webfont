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

### Positional arguments

| Argument     | Description                                              |
| ------------ | -------------------------------------------------------- |
| `<FILES>...` | SVG files or directories containing SVG files (required) |

### Required options

| Flag                | Description      |
| ------------------- | ---------------- |
| `-d, --dest <DEST>` | Output directory |

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
