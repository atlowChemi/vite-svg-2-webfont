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
