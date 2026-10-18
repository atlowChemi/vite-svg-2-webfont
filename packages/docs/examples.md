---
description: Live vite-svg-2-webfont examples — each shows the Vite config, the input icons, any templates, and the font it generates.
---

<script setup>
import ExampleTabs from './.vitepress/theme/components/ExampleTabs.vue';
import ExampleIcons from './.vitepress/theme/components/ExampleIcons.vue';
import ExampleOutput from './.vitepress/theme/components/ExampleOutput.vue';
import ExampleCss from './.vitepress/theme/components/ExampleCss.vue';
import TypedIcon from './examples/type-safe-component/Icon.vue';
</script>

# Examples

Each example below is built by `vite-svg-2-webfont` when these docs are built. The **Generated CSS** tab shows the stylesheet the plugin produced, with font URLs shortened to file names; Vite rewrites them to the emitted assets. The **Output** tab renders the font with the example's real class names.

## Basic icon set

Point `context` at a folder of SVGs: each file becomes a glyph and an `icon-<name>` class. The icons scale with `font-size` and take the text `color`.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/basic/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="basic" />
</template>
<template #css>
<ExampleCss example="basic" />
</template>
<template #output>
<ExampleOutput example="basic" />
</template>
</ExampleTabs>

## Custom naming

Choose the font name, the base class, and the icon class prefix to match your project's conventions.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/custom-naming/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="custom-naming" />
</template>
<template #css>
<ExampleCss example="custom-naming" :highlight="['font-family: &quot;ui-icons&quot;', 'font-family: ui-icons', '.ui-icon {', '.ui-icon:before', '.ui-icon-add:before']" />
</template>
<template #output>
<ExampleOutput example="custom-naming" base-class="ui-icon" class-prefix="ui-icon-" />
</template>
</ExampleTabs>

## Ligatures

Ligatures are on by default: inside an element that uses the icon font, typing an icon's name renders its glyph, so the icon also has a readable text form.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/ligatures/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="ligatures" />
</template>
<template #css>
<ExampleCss example="ligatures" />
</template>
<template #output>
<ExampleOutput example="ligatures">
<figure style="grid-column: 1 / -1"><span style="font-family: 'ligature-icons'">add check home search star heart</span><figcaption>text: add check home search star heart</figcaption></figure>
</ExampleOutput>
</template>
</ExampleTabs>

## Type-safe icon component

An HTML template can generate any text file. This one writes a TypeScript module listing every icon name, so an `<Icon>` component only accepts icons that exist. The same module works with Vue, React, Angular, and Svelte.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', template: 'icons.ts.hbs', generated: 'icons.ts', components: 'Components', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/type-safe-component/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="type-safe-component" />
</template>
<template #template>

<<< ./examples/type-safe-component/icons.ts.hbs{handlebars}

</template>
<template #generated>

<<< ./examples/type-safe-component/icons.ts

</template>
<template #components>

::: code-group

<<< ./examples/type-safe-component/Icon.vue

<<< ./examples/type-safe-component/Icon.tsx

<<< ./examples/type-safe-component/icon.component.ts

<<< ./examples/type-safe-component/Icon.svelte

:::

</template>
<template #css>
<ExampleCss example="type-safe-component" />
</template>
<template #output>
<ExampleOutput example="type-safe-component">
<figure><TypedIcon name="bell" /><figcaption>&lt;Icon name="bell" /&gt;</figcaption></figure>
<figure><TypedIcon name="search" /><figcaption>&lt;Icon name="search" /&gt;</figcaption></figure>
</ExampleOutput>
</template>
</ExampleTabs>

## Custom CSS template

Add your own utilities to the generated stylesheet by starting from the built-in template and extending it. This template adds a large size and a spinning animation.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', template: 'icons.css.hbs', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/custom-css-template/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="custom-css-template" />
</template>
<template #template>

<<< ./examples/custom-css-template/icons.css.hbs{handlebars}

</template>
<template #css>
<ExampleCss example="custom-css-template" :highlight="['.icon--lg', 'font-size: 2em', '.icon--spin', 'animation:', '@keyframes', 'rotate(360deg)']" />
</template>
<template #output>
<ExampleOutput example="custom-css-template">
<figure><i class="icon icon-star icon--lg" aria-hidden="true"></i><figcaption>icon--lg</figcaption></figure>
<figure><i class="icon icon-spinner icon--spin" aria-hidden="true"></i><figcaption>icon--spin</figcaption></figure>
</ExampleOutput>
</template>
</ExampleTabs>

## Multi-weight family

Give each weight its own folder of matching file names. The designs share one font file, and a modifier class switches weight. `bold/` has no `heart.svg`, so the fallback policy borrows it from the regular design.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/multi-weight/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="multi-weight" />
</template>
<template #css>
<ExampleCss example="multi-weight" :highlight="['font-weight: 300', 'font-weight: 400', 'font-weight: 700', 'icon--light', 'icon--regular', 'icon--bold']" />
</template>
<template #output>
<ExampleOutput example="multi-weight" :rows="[{ label: 'icon--light', class: 'icon--light' }, { label: 'default (regular)' }, { label: 'icon--bold', class: 'icon--bold' }]" />
</template>
</ExampleTabs>

## Color glyphs

Keep the SVG colors for selected icons. Color glyphs need TTF, WOFF, or WOFF2 output; the other icons still follow the text color. Some platforms, such as Safari on macOS, show the single-color version instead.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/color-glyphs/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="color-glyphs" />
</template>
<template #css>
<ExampleCss example="color-glyphs" />
</template>
<template #output>
<ExampleOutput example="color-glyphs">
<figure style="color: #e8590c"><i class="icon icon-logo" aria-hidden="true"></i><figcaption>icon-logo (orange text)</figcaption></figure>
<figure style="color: #e8590c"><i class="icon icon-flag" aria-hidden="true"></i><figcaption>icon-flag (orange text)</figcaption></figure>
<figure style="color: #e8590c"><i class="icon icon-star" aria-hidden="true"></i><figcaption>icon-star (orange text)</figcaption></figure>
</ExampleOutput>
</template>
</ExampleTabs>

## Theme colors in one glyph

Use one charging `battery` glyph with three paths in both light and dark themes:

- **Primary:** the casing uses `currentColor`, following the element's text color (dark slate → light slate).
- **Secondary:** the charge area uses `#059669`, mapped to the `secondary` palette slot (emerald → mint).
- **Static:** the lightning bolt uses `#fbbf24`, staying amber in both themes.

CSS `@font-palette-values` overrides only the secondary slot, and `font-palette` selects the theme without loading a second font or switching glyphs.

The custom template reads `colorSlots.secondary`, the generated palette index. Slot indices follow ASCII name order and stay stable when icons change, including when a slot's source color is temporarily unused. Adding or renaming **slots** can change indices, so use the template mapping instead of hardcoding them.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', template: 'icons.css.hbs', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/theme-colors/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="theme-colors" />
</template>
<template #template>

<<< ./examples/theme-colors/icons.css.hbs{handlebars}

</template>
<template #css>
<ExampleCss example="theme-colors" :highlight="['@font-palette-values', 'override-colors', 'font-palette']" />
</template>
<template #output>
<ExampleOutput example="theme-colors" :rows="[{ label: 'Light: dark slate casing · emerald charge · amber bolt' }, { label: 'Dark: light slate casing · mint charge · amber bolt', class: 'theme-dark' }]" />
</template>
</ExampleTabs>

Palette overrides use explicit colors, not per-element `var(--secondary-color)` or `currentColor`. Select predefined palettes for your themes; only the foreground follows an inherited CSS variable here. Identical resolved RGB fills share a slot across selected icons and variants, with their layer opacity retained. Slots do not affect monochrome icons. Platforms using the monochrome fallback lose the secondary color; see [color compatibility](./webfont-generator/#browser-and-platform-compatibility).

## Pinned codepoints

Pin the codepoints of icons you publish, so CSS or content that refers to a codepoint keeps working when icons are added. Unpinned icons get the next free codepoint.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/pinned-codepoints/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="pinned-codepoints" />
</template>
<template #css>
<ExampleCss example="pinned-codepoints" :highlight="['e001', 'e002', 'e003']" />
</template>
<template #output>
<ExampleOutput example="pinned-codepoints">
<figure><span style="font-family: 'pinned-icons'">&#xe001;</span><figcaption>U+E001</figcaption></figure>
<figure><span style="font-family: 'pinned-icons'">&#xe002;</span><figcaption>U+E002</figcaption></figure>
<figure><span style="font-family: 'pinned-icons'">&#xe003;</span><figcaption>U+E003</figcaption></figure>
</ExampleOutput>
</template>
</ExampleTabs>

## Inline fonts

Embed the fonts in the generated CSS as data URLs. The page makes no separate font requests, at the cost of a larger stylesheet.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/inline-fonts/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="inline-fonts" />
</template>
<template #css>
<ExampleCss example="inline-fonts" :highlight="['url(&quot;data:']" />
</template>
<template #output>
<ExampleOutput example="inline-fonts" />
</template>
</ExampleTabs>

## Modern formats and preload

Generate only the formats current browsers need, and preload WOFF2 so the browser starts downloading it before the stylesheet asks for it. Preload tags are added to the built HTML only.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', html: 'dist/index.html', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/preload/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="preload" />
</template>
<template #html>

```html
<link rel="preload" href="/assets/preloaded-icons-[hash].woff2" as="font" type="font/woff2" crossorigin />
```

</template>
<template #css>
<ExampleCss example="preload" :highlight="['url(']" />
</template>
<template #output>
<ExampleOutput example="preload" />
</template>
</ExampleTabs>

## Writing files for other tools

Write the fonts, CSS, and HTML preview to a folder as well, for tools outside Vite. These files are extra copies; the app still uses the fonts Vite bundles.

<ExampleTabs :tabs="{ config: 'vite.config.ts', icons: 'icons/', files: 'generated/', css: 'Generated CSS', output: 'Output' }">
<template #config>

<<< ./examples/write-files/vite.config.ts

</template>
<template #icons>
<ExampleIcons example="write-files" />
</template>
<template #files>

```txt
generated/
├── written-icons.css
├── written-icons.html
├── written-icons.woff
└── written-icons.woff2
```

</template>
<template #css>
<ExampleCss example="write-files" :highlight="['url(']" />
</template>
<template #output>
<ExampleOutput example="write-files" />
</template>
</ExampleTabs>
