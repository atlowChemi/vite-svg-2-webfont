---
description: Run the webfont-generator command-line tool with flags or a JSON manifest, and clear its cache.
---

# CLI Usage

Install the CLI first; see [Getting Started](/cli/). Every option is listed in [configuration](/cli/configuration), and complete commands are in [examples](/cli/examples).

## Usage

```sh
webfont-generator [OPTIONS] --dest <DEST> <FILES>...
webfont-generator --config <PATH> [--threads <N>]
webfont-generator clear-cache --cache-location <DIR>
```

`<FILES>` are SVG files or directories, written as explicit paths: `./icons/`, `../shared/plus.svg`, or an absolute path. Bare names such as `icons` are rejected, because they are reserved for [subcommands](#subcommands). A directory adds its `.svg` files, sorted by name, without subdirectories.

## Flags or a manifest

Flags cover the common options, for one-off builds and short scripts:

```sh
webfont-generator --dest ./dist/fonts --font-name my-icons --types woff2,woff ./icons/
```

For anything else, or to keep the settings in your repository, use a JSON manifest. It takes every option, including [families](/guide/features/variants), [color glyphs](/guide/features/color-glyphs), and [format options](/cli/configuration#formatoptions). The [configuration](/cli/configuration) page lists each option's flag, if it has one.

## JSON manifest

`--config <PATH>` loads every option from a JSON file, using the names in [configuration](/cli/configuration):

```json [icons.webfont.json]
{
    "dest": "dist/fonts",
    "fontName": "icons",
    "html": true,
    "variants": [
        { "name": "outline", "files": ["icons/outline"], "weight": 300, "default": true },
        { "name": "filled", "files": ["icons/filled"], "weight": 700 }
    ]
}
```

```sh
webfont-generator --config icons.webfont.json
```

- **No mixing with flags.** A manifest can't be combined with input paths or other flags, even ones set to their defaults. The exception is `--threads`, which depends on the machine and overrides the manifest's `threads`. `--help` and `--version` still work.
- **Paths are relative to the manifest**, not to the working directory: inputs, `dest`, `cssDest`, `htmlDest`, `tsDest`, `cssTemplate`, `htmlTemplate`, and `cacheLocation`. URLs such as `cssFontsUrl` are used as written.
- **Inputs keep their order.** Each directory expands in place into its `.svg` files, sorted by name, without subdirectories. Wildcards and globs aren't expanded. Missing paths and duplicate files within one design are errors; the same file may appear in several designs.
- **Strict.** Unknown options and values of the wrong type are rejected, including inside nested options. Errors name the manifest and the option, and malformed JSON reports the line and column.
- **Numbers are decimal.** JSON has no hexadecimal numbers, so write `"startCodepoint": 57344` for `0xE000`.
- **No callbacks.** `rename`, `cssContext`, and `htmlContext` need the [Node.js package](/node/).

## Subcommands

| Command                              | Description                                                                                                                                                                               |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clear-cache --cache-location <DIR>` | Remove the files the [engine cache](/guide/features/caching) created in `<DIR>`, then print how many files and bytes were removed. Other files stay. A missing directory is not an error. |

## Help and version

| Flag            | Description             |
| --------------- | ----------------------- |
| `-h, --help`    | Print every flag        |
| `-V, --version` | Print the CLI's version |

## See also

- [The Engine](/guide/engine): architecture and design
- [Rust usage](/rust/usage): the same engine as a library
- [Node.js usage](/node/usage): the same engine from Node.js
