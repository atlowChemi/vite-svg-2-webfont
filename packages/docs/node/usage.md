---
description: Generate icon fonts from Node.js with @atlowchemi/webfont-generator, including color glyphs, multi-weight families, updates, and caching.
---

# Node.js Usage

Install the package first; see [Getting Started](/node/). Every option is listed in [configuration](/node/configuration), and every function and type in the [API reference](/node/api).

## Basic usage

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    files: ['icons/add.svg', 'icons/remove.svg', 'icons/settings.svg'],
    dest: './dist/fonts',
    fontName: 'my-icons',
    types: ['woff2', 'woff'],
});

// Font data is available directly on the result
console.log(result.woff2); // Uint8Array
console.log(result.woff); // Uint8Array

// Generate CSS with default font URLs
const css = result.generateCss();

// Generate CSS with custom URLs
const cssCustom = result.generateCss({ woff2: '/fonts/icons.woff2' });
```

## Color selection

Set `colorGlyphs: true` to keep the SVG colors of every icon, or `colorGlyphs: ['logo']` for selected icons. See [color glyphs](/guide/features/color-glyphs) for the selection rules, theme palettes with [`colorSlots`](/node/configuration#colorslots), supported SVG paint, and browser support.

```ts
const result = await generateWebfonts({
    files: ['icons/logo.svg', 'icons/add.svg'],
    dest: 'dist/fonts',
    types: ['woff2', 'woff'],
    colorGlyphs: ['logo'],
    incremental: true,
});

// After changing only logo.svg's fill, update the existing result.
result.regenerate({ files: ['icons/logo.svg', 'icons/add.svg'] }, [{ path: 'icons/logo.svg', changeType: 'changed' }]);
```

The unselected `add` glyph remains monochrome. In the example above, replace `colorGlyphs: ['logo']` with `colorGlyphs: true` to preserve colors for both icons.

For a multi-variant family, the selected name applies across every variant. Each design can use different SVG colors:

```ts
const family = await generateWebfonts({
    dest: 'dist/fonts',
    variants: [
        { name: 'light', files: ['light/logo.svg'], weight: 300, default: true },
        { name: 'bold', files: ['bold/logo.svg'], weight: 700 },
    ],
    colorGlyphs: ['logo'],
    incremental: true,
});
// Each SVG can use different fixed colors. Weight selection chooses its paint and outline.
family.regenerate(
    {
        variants: [
            { variant: 'light', files: ['light/logo.svg'] },
            { variant: 'bold', files: ['bold/logo.svg'] },
        ],
    },
    [{ path: 'bold/logo.svg', changeType: 'changed' }],
);
```

## Multi-variant fonts

A multi-variant family groups several designs of the same icons, such as light and bold,
into one font file per output format. Match SVG filenames across designs so that `add.svg`
refers to the same icon in each variant. You can then choose a design with a CSS modifier class.

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

const result = await generateWebfonts({
    dest: './dist/fonts',
    fontName: 'my-icons',
    variants: [
        { name: 'light', files: ['icons/light/add.svg'], weight: 300, default: true },
        { name: 'bold', files: ['icons/bold/add.svg'], weight: 700 },
    ],
});
```

Load the generated `my-icons.css`, then use the icon class with an optional variant modifier:

```html
<span class="icon icon-add" aria-hidden="true"></span> <span class="icon icon-add icon--bold" aria-hidden="true"></span>
```

The first icon uses the default design (`light`); the second uses `bold`. See [variants](/guide/features/variants)
for the rules on designs and weights, supported formats, and icons missing from a design
([`missingGlyphs`](/node/configuration#missingglyphs)). To update a family, use the [variant methods](/node/api#variant-regeneration).

Custom templates receive extra family metadata; see the [template context comparison](/guide/templates#template-context).
For stylesheet customization and SCSS, see [Templates](/guide/templates).

## Updating after changes

For watch mode or a dev server, generate with `incremental: true` and update the result when icons change, instead of generating again:

```ts
let result = await generateWebfonts({ files, dest: './dist/fonts', incremental: true });

// On a watch event:
result = await result.updateAsync('icons/search.svg');
```

See [incremental updates](/guide/features/incremental) and [updating a result](/node/api#updating-a-result).

## Caching between runs

Set [`cacheLocation`](/node/configuration#cachelocation) to reuse work across processes, such as CI jobs or restarts. See the [engine cache](/guide/features/caching).

## Templates

The CSS, SCSS, and HTML templates, their context, and the `cssContext` and `htmlContext` callbacks are described in [Templates](/guide/templates). The built-in templates' paths are exported; see [template paths](/node/api#template-paths).

## See also

- [The Engine](/guide/engine): architecture and design
- [Rust usage](/rust/usage): the same engine as a crate
- [CLI usage](/cli/usage): the engine from a terminal
