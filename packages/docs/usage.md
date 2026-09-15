---
description: Learn the standard integration flow for vite-svg-2-webfont, including plugin setup, virtual CSS import, and generated icon classes.
---

# Usage

## Standard setup

The typical integration has three parts:

1. Add the plugin to `vite.config.ts`

    ```ts{7-9} [vite.config.ts]
    import { resolve } from 'node:path';
    import { defineConfig } from 'vite';
    import viteSvgToWebfont from 'vite-svg-2-webfont';

    export default defineConfig({
        plugins: [
            viteSvgToWebfont({
                context: resolve(import.meta.dirname, 'icons'),
            }),
        ],
    });
    ```

2. Import `virtual:vite-svg-2-webfont.css`

    ```ts [main.ts]
    import 'virtual:vite-svg-2-webfont.css';
    ```

3. Use generated class names in markup, for example:

    ```html [index.html]
    <i class="icon icon-add"></i>
    ```

## How class names are generated

- The default base selector is `.icon`, see [`baseSelector`](./configuration#baseselector)
- The default class prefix is `icon-`, see [`classPrefix`](./configuration#classprefix)
- SVG file names become icon names (e.g. `add.svg` becomes `{classPrefix}-add`)

For example, if `context` contains `add.svg`, the generated CSS class is `icon-add`, and you would use it like this:

```html [index.html]
<i class="icon icon-add"></i>
```

If you want different class names, change [`classPrefix`](./configuration#classprefix) and [`baseSelector`](./configuration#baseselector).

## Development file output

See [multi-weight icon families](#multi-weight-icon-families) to configure multiple designs with the same output controls.

By default, the plugin does not write generated assets to disk during development. If you want preview artifacts while iterating, use [`generateFiles`](./configuration#generatefiles) to enable file output and specify which files to generate:

```ts{3} [vite.config.ts]
viteSvgToWebfont({
    context: './src/icons',
    generateFiles: ['css', 'fonts', 'html'],
});
```

## Build-time behavior

- Preload tags can be injected into built HTML with [`preloadFormats`](./configuration#preloadformats)
- Preload injection can be limited to selected HTML entrypoints with [`shouldProcessHtml`](./configuration#shouldprocesshtml)
- When [`inline`](./configuration#inline) is `true`, no preload tags are injected because assets are embedded in the CSS
- File output during build is disabled unless [`allowWriteFilesInBuild`](./configuration#allowwritefilesinbuild) is enabled

## Multi-weight icon families

To use light and bold versions of the same icons, put matching filenames in each design directory, for example `src/icons/light/add.svg` and `src/icons/bold/add.svg`.

1. Configure the designs instead of top-level `files`:

    ```ts
    viteSvgToWebfont({
        context: './src/icons',
        fontName: 'icons',
        variants: [
            { name: 'light', context: 'light', weight: 300, default: true },
            { name: 'bold', context: 'bold', weight: 700 },
        ],
    });
    ```

2. Keep the existing virtual CSS import:

    ```ts
    import 'virtual:vite-svg-2-webfont.css';
    ```

3. Add the modifier for a non-default design:

    ```html
    <span class="icon icon-add" aria-hidden="true"></span> <span class="icon icon-add icon--bold" aria-hidden="true"></span>
    ```

The first icon uses light artwork; the second uses bold artwork. The family produces one shared WOFF and one shared WOFF2 by default. Select `types: ['ttf', 'woff', 'woff2']` (any one of these 3 valid formats) to change the formats. SVG/EOT output is available only in ordinary mode.

For nested input folders, set a design's `files` to `['**/*.svg']`. To use sparse designs (ie. some variants have glyphs which other variants do not), configure [`missingGlyphs`](./configuration#missingglyphs); each design must still contain at least one SVG.

Edits across the design roots are batched and regenerated through one serialized queue. The plugin refreshes every design's membership, updates shared font bytes, and reloads virtual CSS, including in inline mode. Invalid edits leave the last successful in-memory output available; the plugin reports the error and retries on subsequent changes. Optional filesystem output remains non-transactional.

Custom CSS callbacks are invoked again through full generation. Existing CSS/SCSS/HTML templates, selectors, optional disk output, and preload controls also apply to families. Shared assets are emitted and preloaded once per selected format, not once per design.

## Related options

Common options you may want to adjust early:

- [`fontName`](./configuration#fontname)
- [`dest`](./configuration#dest)
- [`cssDest`](./configuration#cssdest)
- [`classPrefix`](./configuration#classprefix)
- [`baseSelector`](./configuration#baseselector)
- [`types`](./configuration#types)
- [`inline`](./configuration#inline)

See the full [Configuration](./configuration) reference for details.
