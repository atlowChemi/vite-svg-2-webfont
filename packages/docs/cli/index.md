---
description: Generate icon fonts from a terminal, shell script, or CI job with the webfont-generator command.
---

# CLI

Use the CLI to turn SVG files into icon fonts from a terminal, shell script, or CI job.
Pass a directory for a one-off build, or keep a JSON configuration in your project for
repeatable builds with custom options. No Node.js application code is required.

## Installation

```sh
cargo install webfont-generator --features cli
```

::: tip
The `cli` feature is opt-in and not included in the default feature set. It enables `fs` (and therefore `async`) along with the CLI dependencies. Library users do not pull in `clap` unless they enable `cli`; see [Cargo feature flags](/rust/usage#feature-flags).
:::

## Your first font

Generate the default formats (EOT, WOFF, WOFF2) from a folder of SVGs:

```sh
webfont-generator --dest ./dist/fonts ./icons/
```

Pick the font name and formats:

```sh
webfont-generator --dest ./dist/fonts --font-name my-icons --types woff2,woff ./icons/
```

## Next steps

- [Usage](/cli/usage): every option, the JSON configuration file, and subcommands.
- [Templates](/guide/templates): customize the generated CSS and HTML.
- [Changelog](/rust/changelog): the CLI ships with the Rust engine.
