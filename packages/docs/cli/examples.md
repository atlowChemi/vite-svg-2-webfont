---
description: Examples for the webfont-generator command-line tool — quick builds, previews, TypeScript names, dry runs, manifests for families and color, npm scripts, and CI caching.
---

# CLI Examples

See [configuration](/cli/configuration) for every option and [usage](/cli/usage) for how flags and manifests work.

## Quick build

Build WOFF2 and WOFF fonts and their CSS from a folder of SVGs:

```sh
webfont-generator --dest ./dist/fonts --font-name app-icons --types woff2,woff ./icons/
```

This writes `app-icons.woff2`, `app-icons.woff`, and `app-icons.css` to `./dist/fonts`.

## Preview the icons

Add an HTML page that shows every icon with its class name, to check a set before using it:

```sh
webfont-generator --dest ./dist/fonts --html ./icons/
```

Open `./dist/fonts/iconfont.html` in a browser.

## TypeScript icon names

Write a TypeScript module with the icon names and their `IconName` union next to your source:

```sh
webfont-generator --dest ./dist/fonts --ts --ts-dest ./src/generated/icons.ts ./icons/
```

See [TypeScript output](/guide/templates#typescript) for components that use it.

## Check icons without writing

`--no-write` generates everything and writes nothing, so a CI step can fail on invalid SVGs before anything is published:

```sh
webfont-generator --dest ./dist/fonts --no-write ./icons/
```

## Multi-weight family

Combine designs into one font per format with a manifest. A directory in `files` adds its SVGs, so each design is one folder:

```json [icons.webfont.json]
{
    "dest": "dist/fonts",
    "fontName": "app-icons",
    "variants": [
        { "name": "light", "files": ["icons/light"], "weight": 300, "default": true },
        { "name": "bold", "files": ["icons/bold"], "weight": 700 }
    ],
    "missingGlyphs": { "behavior": "fallback", "variant": "light" }
}
```

```sh
webfont-generator --config icons.webfont.json
```

An icon missing from the bold design uses its light drawing there. See [variants](/guide/features/variants).

## Color glyphs with a theme color

Keep the colors of the `logo` icon, and reserve a palette slot for its secondary color so CSS can change it per theme:

```json [icons.webfont.json]
{
    "dest": "dist/fonts",
    "files": ["icons"],
    "types": ["woff2", "woff"],
    "colorGlyphs": ["logo"],
    "colorSlots": { "secondary": "#ff9900" }
}
```

Color needs TTF, WOFF, or WOFF2, so `types` leaves out the default EOT. See [color glyphs](/guide/features/color-glyphs) for the CSS side.

## npm script

Keep the manifest in your repository and run the CLI from a `package.json` script:

```json [package.json]
{
    "scripts": {
        "build:icons": "webfont-generator --config icons.webfont.json"
    }
}
```

The CLI is a native binary installed with Cargo, not an npm package. To generate fonts from npm without Rust, use the [Node.js package](/node/) instead.

## Reproducible builds with a cache in CI

Fix the font's timestamp so the same icons always produce the same bytes, and keep the [engine cache](/guide/features/caching) between runs:

```json [icons.webfont.json]
{
    "dest": "dist/fonts",
    "files": ["icons"],
    "types": ["woff2"],
    "cacheLocation": ".webfont-cache",
    "formatOptions": { "ttf": { "ts": 0 } }
}
```

```yaml [.github/workflows/icons.yaml]
- uses: actions/cache@v6
  with:
      path: .webfont-cache
      key: webfont-${{ runner.os }}-${{ hashFiles('icons/**/*.svg') }}
      restore-keys: webfont-${{ runner.os }}-
- run: webfont-generator --config icons.webfont.json
```

To start over, remove the files the cache created:

```sh
webfont-generator clear-cache --cache-location .webfont-cache
```
