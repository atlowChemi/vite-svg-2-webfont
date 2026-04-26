---
description: Upgrade vite-svg-2-webfont across major versions — what changes, who needs to act, and step-by-step before/after examples.
---

# Migration

This page collects the breaking changes between major versions of `vite-svg-2-webfont`. When upgrading, read the section that matches your starting version. Newer entries appear first.

## v6 → v7

v7 swaps the unmaintained [`@vusion/webfonts-generator`](https://www.npmjs.com/package/@vusion/webfonts-generator) JS dependency for a native Rust engine, [`@atlowchemi/webfont-generator`](/webfont-generator/). The plugin's surface is mostly unchanged — the same options, the same virtual CSS module, the same generated assets — but a small number of advanced touchpoints have moved. See the [Webfont Generator overview](/webfont-generator/) for background on the new engine.

### What's new

- [`optimizeOutput`](/configuration#optimizeoutput) — a new top-level plugin option that runs an SVG path optimizer over each glyph before assembling the font. Defaults to `false`; opt in for smaller output bytes at the cost of a small amount of build time.

### Do you need to migrate?

If your config only uses the documented [plugin options](/configuration) and you don't pass a `cssContext` callback, **you don't need to change anything**. Bump the version and reinstall.

If you do any of the following, read on:

- Pass a `cssContext` callback.
- Pass per-format options through `formatOptions`.
- Diff generated font binaries between builds in CI.

### `cssContext` callback signature

The callback now receives a single, typed argument. The second `options` and third `handlebars` arguments are gone — the native engine renders templates directly and no longer exposes a Handlebars instance.

::: code-group

```ts [vite.config.v6.ts]
viteSvgToWebfont({
    context: 'icons',
    // [!code focus]
    cssContext(context, options, handlebars) {
        context.extra = 'value';
    },
});
```

```ts [vite.config.v7.ts]
viteSvgToWebfont({
    context: 'icons',
    // [!code focus:2]
    cssContext(context) {
        // options and handlebars are no longer exposed
        context.extra = 'value';
    },
});
```

:::

If you relied on the removed parameters, please reach out on GitHub, by opening an issue, so we can find a solution that works for your use case.

The `context` parameter is typed as [`CssContext`](/webfont-generator/node#csscontext), which exposes the named fields the engine guarantees (`fontName`, `src`, `codepoints`) plus an open-ended index signature for the keys forwarded from `baseSelector` and `classPrefix`.

### `formatOptions` is now typed

`formatOptions` was previously `{ [format]?: unknown }` — any key, any shape. v7 narrows it to [`FormatOptions`](/webfont-generator/node#formatoptions), which only accepts `svg`, `ttf`, and `woff` keys, each with its own typed per-format options ([`SvgFormatOptions`](/webfont-generator/node#formatoptions), [`TtfFormatOptions`](/webfont-generator/node#formatoptions), [`WoffFormatOptions`](/webfont-generator/node#formatoptions)). Two practical consequences:

- `formatOptions.woff2` and `formatOptions.eot` are no longer accepted — `woff2` has no format-specific options and `eot` is derived from the TTF output.
- Free-form keys on `formatOptions.svg` / `.ttf` / `.woff` will fail to typecheck. Replace them with the documented fields, or drop them if they were no-ops in v6.

### Generated font binaries differ at byte level

The native engine produces different — but valid — bytes for the same input SVG set. If a CI step compares generated `.eot` / `.woff` / `.woff2` / `.ttf` / `.svg` files against committed fixtures, the comparison will fail on the first v7 build. Regenerate the fixtures once on the upgrade commit; downstream builds will be deterministic again.

### Step by step

1. Bump the dev dependency to v7 and reinstall.

    ::: code-group

    ```sh [npm]
    npm install --save-dev vite-svg-2-webfont@^7
    ```

    ```sh [pnpm]
    pnpm add --save-dev vite-svg-2-webfont@^7
    ```

    ```sh [yarn]
    yarn add --dev vite-svg-2-webfont@^7
    ```

    ```sh [bun]
    bun add -D vite-svg-2-webfont@^7
    ```

    :::

2. If you pass a `cssContext` callback, drop the second and third parameters.
3. If you pass `formatOptions`, drop any `woff2` / `eot` keys and confirm the remaining fields match the typed `FormatOptions` shape.
4. If your CI compares font binaries to committed fixtures, regenerate them on this commit.
5. Run your build. The plugin will emit the same virtual CSS module and the same icon classes you had before.
