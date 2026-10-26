---
description: CLI reference for the webfont-generator command-line tool, including installation, usage examples, and all options.
---

# CLI Usage

Use the CLI to turn SVG files into icon fonts from a terminal, shell script, or CI job.
Pass a directory for a one-off build, or keep a JSON configuration in your project for
repeatable builds with custom options. No Node.js application code is required.

## Installation

```sh
cargo install webfont-generator --features cli
```

::: tip
The `cli` feature is opt-in and not included in the default feature set. It enables `fs` (and therefore `async`) along with the CLI dependencies. Library users do not pull in `clap` unless they enable `cli`; see [Cargo feature flags](./rust#feature-flags).
:::

## Usage

```sh
webfont-generator [OPTIONS] --dest <DEST> <FILES>...
webfont-generator --config <PATH> [--threads <N>]
webfont-generator clear-cache --cache-location <DIR>
```

`<FILES>` accepts individual SVG file paths or directories, written as explicit paths: `./icons/`, `../shared/plus.svg`, or an absolute path. Bare names such as `icons` are rejected, because they are reserved for [subcommands](#subcommands). When a directory is given, all `.svg` files inside it are included (non-recursive, sorted alphabetically).

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

Reuse work from previous runs, then clear the cache:

```sh
webfont-generator --dest ./dist/fonts --cache-location .webfont-cache ./icons/
webfont-generator clear-cache --cache-location .webfont-cache
```

## Options reference

### JSON manifest

`--config <PATH>` loads a complete JSON configuration using the camelCase [generator options](./node#options-reference). It cannot be combined with positional inputs or any generation flag, even a flag set to its default. The exception is `--threads`, which is machine-specific and overrides the config's `threads`. `--help` and `--version` still work.

The manifest accepts `"colorGlyphs": true` or `"colorGlyphs": ["logo"]`. Omit it or use `[]` for monochrome output; explicit `null`, `false`, strings, and mixed-type arrays are invalid. Select compatible formats explicitly in ordinary mode, for example `"types": ["woff2", "woff"]`; the ordinary default includes incompatible EOT. Names are logical glyph names, and selection applies across all variants, including fallbacks. Duplicates are harmless; unknown names are rejected in first-occurrence input order. There is no separate color CLI flag. See [`colorGlyphs`](./node#colorglyphs) for option details and [color glyphs](./#color-glyphs) for the supported SVG subset and browser/platform behavior.

With active `colorGlyphs`, the manifest also accepts `"colorSlots": { "secondary": "#ff9900" }`. Named slots reserve palette indices in ASCII name order, including unused colors, and expose them as `colorSlots` in the CSS template context. Values must be distinct `#RRGGBB` colors; names use ASCII letters, digits, hyphens or underscores. There is no separate slot CLI flag. See [slot semantics](./node#colorslots) and the [live theme example](../examples#theme-colors-in-one-glyph).

::: code-group

```json [icons.webfont.json]
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

:::

Run it with `webfont-generator --config icons.webfont.json`.

- `dest` is required. Supply `files` for a single design or ordered `variants` for a [multi-variant family](./node#multi-variant-fonts).
- Relative input paths, `dest`, `cssDest`, `htmlDest`, `cssTemplate`, `htmlTemplate`, and `cacheLocation` resolve against the manifest directory, independent of the working directory. URL values such as `cssFontsUrl` are not rebased.
- Input arrays preserve explicit entry order. Each directory expands at its position into sorted lowercase `.svg` files, non-recursively. JSON paths do not expand wildcards or globs.
- Missing entries and duplicate normalized input paths within one variant (or the ordinary file list) are errors. The same source may appear in different variants.
- Unknown option fields and invalid JSON types are rejected, including nested options. Diagnostics include the manifest path and the affected option field; malformed JSON includes line and column information.
- JSON uses numeric codepoints: for example, `"startCodepoint": 57344` instead of hexadecimal syntax. Callbacks (`rename`, `cssContext`, `htmlContext`) cannot be represented in a manifest; use the Node API for callbacks.
- Library defaults apply: ordinary output is EOT/WOFF/WOFF2; variants default to WOFF/WOFF2 and accept only TTF/WOFF/WOFF2. Use `"writeFiles": false` for a dry run.

### Positional arguments

| Argument     | Description                                                                                                       |
| ------------ | ----------------------------------------------------------------------------------------------------------------- |
| `<FILES>...` | SVG files or directories containing SVG files, as explicit paths such as `./icons/` (required without `--config`) |

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

### Cache options

| Flag                     | Description                                                                                                                                                                                                                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--cache-location <DIR>` | Persistent generation cache; see [`cacheLocation`](./node#cachelocation) for how it behaves (the CLI doesn't set `formatOptions.ttf.ts`, so cached formats keep the TTF timestamp of the run that created them unless your config sets it) and [caching across builds](./node#caching-across-builds) for CI and Docker setups |

### Thread options

| Flag            | Default      | Description                                                                                                                                                                                                                            |
| --------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--threads <N>` | One per core | Threads reading SVG files and doing glyph work at once (glyph work stops at the core count); see [`threads`](./node#threads). Also allowed with `--config`, where it overrides the config's value. Output is identical for every value |

### Subcommands

| Command                              | Description                                                                                                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `clear-cache --cache-location <DIR>` | Remove the files the generation cache created in `<DIR>`, then print how many files and bytes were removed. Other files stay. A missing directory is not an error. |

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
