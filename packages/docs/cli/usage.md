---
description: Reference for the webfont-generator command-line tool, including usage examples, the JSON configuration file, and all options.
---

# CLI Usage

Install the CLI first; see [Getting Started](/cli/).

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

Generate a fixed TypeScript module with icon names, their `IconName` union, and selector settings:

```sh
webfont-generator --dest ./dist/fonts --ts --ts-dest ./dist/icons.ts ./icons/
```

Without `--ts-dest`, TS is written to `<dest>/<font-name>.ts`. A destination alone does not
enable TS output. See the [TypeScript output contract](/rust/usage#typescript-output).

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

`--config <PATH>` loads a complete JSON configuration using the camelCase [generator options](/node/usage#options-reference). It cannot be combined with positional inputs or any generation flag, even a flag set to its default. The exception is `--threads`, which is machine-specific and overrides the config's `threads`. `--help` and `--version` still work.

[Color glyphs](/guide/features/color-glyphs) are only available in a manifest: `"colorGlyphs": true` or `"colorGlyphs": ["logo"]`, with compatible formats such as `"types": ["woff2", "woff"]`, and optionally `"colorSlots": { "secondary": "#ff9900" }` for theme palettes. There are no color flags.

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

- `dest` is required. Supply `files` for a single design or ordered `variants` for a [family](/guide/features/variants).
- Relative input paths, `dest`, `cssDest`, `htmlDest`, `tsDest`, `cssTemplate`, `htmlTemplate`, and `cacheLocation` resolve against the manifest directory, independent of the working directory. URL values such as `cssFontsUrl` are not rebased.
- `"ts": true` enables fixed TypeScript source output; `"tsDest": "generated/icons.ts"` overrides its destination.
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

| Flag                           | Default                 | Description                                     |
| ------------------------------ | ----------------------- | ----------------------------------------------- |
| `--css` / `--no-css`           | `--css`                 | Generate or skip CSS output                     |
| `--html` / `--no-html`         | `--no-html`             | Generate or skip HTML preview                   |
| `--ts` / `--no-ts`             | `--no-ts`               | Generate or skip fixed TypeScript source        |
| `--ts-dest <PATH>`             | `<dest>/<font-name>.ts` | TypeScript destination; requires `--ts` to emit |
| `--write` / `--no-write`       | `--write`               | Write files to disk or dry run                  |
| `--ligature` / `--no-ligature` | `--ligature`            | Enable or disable ligature support              |

### Cache options

| Flag                     | Description                                                                                                                                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--cache-location <DIR>` | Persistent [engine cache](/guide/features/caching). The CLI doesn't set `formatOptions.ttf.ts`, so cached formats keep the TTF timestamp of the run that created them unless your config sets it |

### Thread options

| Flag            | Default      | Description                                                                                                                                                                                                                                 |
| --------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--threads <N>` | One per core | Threads reading SVG files and doing glyph work at once (glyph work stops at the core count); see [`threads`](/node/usage#threads). Also allowed with `--config`, where it overrides the config's value. Output is identical for every value |

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

- [Overview](/guide/engine) -- architecture and design
- [Node.js usage](/node/) -- npm package API reference
- [Rust usage](/rust/) -- crate API reference
