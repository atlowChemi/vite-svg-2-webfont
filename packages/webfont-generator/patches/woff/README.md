# WOFF [![Package][package-img]][package-url] [![Documentation][documentation-img]][documentation-url] [![Build][build-img]][build-url]

The package provides a converter for Web Open Font Format.

## Installation

```shell
cargo install --features binary woff
```

## Usage

```
Usage: woff <source> <destination> [options]

Either the source or destination should end with either .woff or .woff2.

Options for WOFF:
    --major-version <number> — set the major version (1 by default)
    --minor-version <number> — set the minor version (0 by default)

Options for WOFF2:
    --metadata <string> — append metadata (empty by default)
    --quality <number>  — set the compression quality (8 by default)
    --no-transform      — disallow transforms
```

## Contribution

Your contribution is highly appreciated. Do not hesitate to open an issue or a
pull request. Note that any contribution submitted for inclusion in the project
will be licensed according to the terms given in [LICENSE.md](LICENSE.md).

[build-img]: https://github.com/bodoni/woff/workflows/build/badge.svg
[build-url]: https://github.com/bodoni/woff/actions/workflows/build.yml
[documentation-img]: https://docs.rs/woff/badge.svg
[documentation-url]: https://docs.rs/woff
[package-img]: https://img.shields.io/crates/v/woff.svg
[package-url]: https://crates.io/crates/woff
