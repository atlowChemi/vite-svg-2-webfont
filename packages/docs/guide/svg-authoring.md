---
description: Prepare SVG icons for webfont generation and fix common problems — backgrounds, holes, clip paths, text, artboards, alignment, precision, file names, and codepoints — with live before/after fonts.
---

<script setup>
import SvgAuthoringCompare from '../.vitepress/theme/components/SvgAuthoringCompare.vue';
</script>

# SVG Authoring & Troubleshooting

By default, each SVG becomes one single-color glyph in the font. Icons you opt into [color glyphs](/guide/engine#color-glyphs) are the exception: they keep their solid fills, so the [color rules](#colors-are-ignored-nested-shapes-become-holes) below don't apply to them. Most icons exported from a design tool work as they are. This page explains what changes when an SVG becomes a glyph, and how to fix the surprises.

Every comparison below is live: the docs site builds real fonts from the SVGs shown, using `vite-svg-2-webfont`. In each card, the white chip on the left is the SVG as a browser draws it, and the glyph on the right is what the generated font renders. The dashed outline is the glyph's box.

These rules come from the [engine](/guide/engine), so they apply to every package: the Vite plugin, Node.js, the browser, Rust, and the CLI.

## What works automatically

You don't need to prepare these. The engine converts them as drawn:

- **Strokes** are converted to filled outlines, including line caps and joins.
- **Transforms** on shapes and groups are applied.
- **Hidden elements** (`display="none"`) are skipped.
- **Basic shapes** (`rect`, `circle`, `ellipse`, `polygon`, `line`, `polyline`) work like paths.
- **`fill-rule="evenodd"`** holes are kept.
- **Light shapes drawn on top of a dark shape**, such as a white play triangle on a circle, become holes, which is usually what you want. The next section explains why.

<SvgAuthoringCompare :cases="[{ id: 'supported', label: 'Converted as drawn' }]" />

## Colors are ignored; nested shapes become holes

An ordinary glyph has a single color: wherever there is a shape, the text color shows. The engine ignores fill colors and decides what is filled by nesting:

- A shape drawn **inside** another shape becomes a **hole**.
- A shape inside that hole is filled again, and so on, alternating with each level.
- Shapes that only **partly overlap** merge into one.

This turns white details into cutouts, but it causes two common surprises:

- **Background rectangles invert the icon.** A white square behind the artwork counts as a shape, so the artwork becomes a hole in a solid square.
- **Colored details become holes.** A blue dot on a black badge is nested inside the badge, so it is cut out.

<SvgAuthoringCompare
    :cases="[
        { id: 'ink-before', label: 'As exported' },
        { id: 'ink-after', label: 'Background removed; colorGlyphs: [\'badge\']' },
    ]"
/>

**Fix:**

- Delete background rectangles and any other shape that exists only to fill the artboard.
- If a detail should stay solid, merge it into the shape around it (for example with "Union" in your design tool) instead of stacking it on top.
- If the icon really needs more than one color, make it a [color glyph](/guide/engine#color-glyphs). Color glyphs require TTF, WOFF, or WOFF2 output, and some platforms show the monochrome version instead.

## Clip paths, masks, and opacity are ignored

The engine reads the shapes themselves. It doesn't apply `clip-path` or `mask`, and it ignores `opacity`. Clipped or masked artwork appears in full, and a faded shape is as solid as the rest.

<SvgAuthoringCompare
    :cases="[
        { id: 'effects-before', label: 'clip-path, mask, opacity' },
        { id: 'effects-after', label: 'Flattened to plain shapes' },
    ]"
/>

**Fix:** flatten the effect into real geometry before exporting:

- **Figma:** select the layers and use _Flatten_ (and _Outline stroke_ if needed).
- **Illustrator:** use _Object › Expand_ and _Object › Clipping Mask › Release_, then combine the shapes with the _Pathfinder_.
- **Inkscape:** use _Path › Object to Path_, then _Path › Intersection_ or _Difference_.

Remove faded shapes or draw them as separate icons. With [`colorGlyphs`](/vite/configuration#colorglyphs), solid fills with `fill-opacity` keep their opacity.

## Convert text to outlines

`<text>` elements are dropped, so an icon made only of text comes out empty. Convert text to outlines (paths) in your design tool before exporting.

<SvgAuthoringCompare
    :cases="[
        { id: 'text-before', label: '<text>' },
        { id: 'text-after', label: 'Converted to outlines' },
    ]"
/>

## Keep artboards consistent

A glyph's size comes from the SVG's `viewBox`, not from the drawing inside it. An icon drawn small in the middle of its artboard stays small in the font, with the empty space around it.

<SvgAuthoringCompare
    :cases="[
        { id: 'artboard-before', label: 'Different padding' },
        { id: 'artboard-after', label: 'Same padding' },
    ]"
/>

**Fix:** draw every icon on the same artboard size (for example 24×24) with the same padding around the artwork, and export the full artboard.

## Mixed sizes and alignment

When icons have different `viewBox` sizes or sit at different heights, these options control how they line up. The comparisons show the same four icons next to text: a square, a low bar, a 16×16 square, and a wide arrow.

- **[`normalize`](/vite/configuration#normalize)** (on by default) scales every icon to the same height, so a 16×16 icon matches 24×24 ones. When it is off, icons keep their size relative to the tallest `viewBox`.
- **[`centerVertically`](/vite/configuration#centervertically)** centers each glyph's shape vertically, ignoring where it sat on its artboard.
- **[`fixedWidth`](/vite/configuration#fixedwidth)** gives every glyph the width of the widest one. The dashed boxes show each glyph's width.

<SvgAuthoringCompare
    :text="['Save', 'now']"
    :cases="[
        { id: 'alignment-default', label: 'Defaults' },
        { id: 'alignment-no-normalize', label: 'normalize: false' },
        { id: 'alignment-center', label: 'normalize: false, centerVertically: true' },
        { id: 'alignment-fixed-width', label: 'fixedWidth: true' },
    ]"
/>

To move every icon up or down relative to the text baseline, adjust [`descent`](/vite/configuration#descent). A positive value moves glyphs down.

## Jagged curves outside the Vite plugin

The engine stores each glyph on a grid of whole units. By default the grid is as tall as the tallest icon's `viewBox`, so a set of 24×24 icons gets only 24 units, and curves come out visibly faceted.

<SvgAuthoringCompare
    :size="112"
    :cases="[
        { id: 'precision-low', label: 'fontHeight: 24 (generator default here)' },
        { id: 'precision-default', label: 'fontHeight: 1000 (plugin default)' },
    ]"
/>

**Fix:** the Vite plugin already uses [`fontHeight: 1000`](/vite/configuration#fontheight). With the [Node.js](/node/usage#fontheight), [browser](/browser/usage#how-to-use), [Rust](/rust/usage#generatewebfontsoptions), or [CLI](/cli/usage#font-options) interfaces, set `fontHeight` to `1000` or more.

## File names become class names

Each file name, without `.svg`, becomes the icon's name and CSS class. File-based interfaces use the name as it is, so characters that mean something in CSS break the class:

- `arrow.left.svg` produces the selector `.icon-arrow.left`, which needs two classes, `icon-arrow` and `left`.
- `my icon.svg` produces `.icon-my icon`, which matches an `icon` element inside `.icon-my`.

<SvgAuthoringCompare
    :cases="[
        { id: 'names-before', label: 'arrow.left.svg, my icon.svg' },
        { id: 'names-after', label: 'arrow-left.svg, my-icon.svg' },
    ]"
/>

**Fix:** name files with letters, digits, hyphens, and underscores only. Class names are case-sensitive, so `Add.svg` and `add.svg` produce different classes. With the [Node.js API](/node/usage#rename), a `rename` callback can turn paths into safe names. The [browser](/browser/) and in-memory [Rust](/rust/usage#text-options-and-identity) APIs reject unsafe names.

## Pin codepoints for stable output

Each icon gets a Unicode codepoint, assigned in order starting at `U+F101`. Adding an icon can shift the codepoints of the icons after it. That doesn't matter while CSS and font are always deployed together, but it breaks anything that stores codepoints separately: hand-written `content: "\f102"` rules, cached CSS served with a new font, or exports to design tools.

The comparison renders the raw codepoints `U+F101`–`U+F103` in three fonts. After `bell.svg` is added, `U+F102` changes from _remove_ to _bell_, unless codepoints are pinned.

<SvgAuthoringCompare
    :codepoints="[0xf101, 0xf102, 0xf103]"
    :cases="[
        { id: 'codepoints-v1', label: 'add, remove' },
        { id: 'codepoints-v2', label: 'add, bell, remove' },
        { id: 'codepoints-v2-pinned', label: 'add, bell, remove; codepoints pinned' },
    ]"
/>

**Fix:** pin the codepoints of icons you publish with [`codepoints`](/vite/configuration#codepoints). New icons continue from the start codepoint, skipping pinned values.

```ts [vite.config.ts]
viteSvgToWebfont({
    context: './src/icons',
    codepoints: { add: 0xf101, remove: 0xf102 },
});
```

## Troubleshooting checklist

| Symptom                                          | Likely cause                                         | Fix                                                                                                         |
| ------------------------------------------------ | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Icon is a solid square with the artwork cut out  | A background rectangle                               | [Remove backgrounds](#colors-are-ignored-nested-shapes-become-holes)                                        |
| A detail is a hole instead of a solid shape      | The detail is nested inside another shape            | [Merge it, or use `colorGlyphs`](#colors-are-ignored-nested-shapes-become-holes)                            |
| Parts that should be hidden or faded are visible | `clip-path`, `mask`, or `opacity`                    | [Flatten the effect](#clip-paths-masks-and-opacity-are-ignored)                                             |
| Icon is empty                                    | Text, or every shape is hidden                       | [Convert text to outlines](#convert-text-to-outlines)                                                       |
| Icons look different sizes                       | Different artboard padding or `viewBox` sizes        | [Keep artboards consistent](#keep-artboards-consistent), [`normalize`](#mixed-sizes-and-alignment)          |
| Icons sit too high or low next to text           | Artwork placement on the artboard                    | [`centerVertically` or `descent`](#mixed-sizes-and-alignment)                                               |
| Curves look faceted                              | Low `fontHeight` outside the plugin                  | [Set `fontHeight`](#jagged-curves-outside-the-vite-plugin)                                                  |
| A class has no effect                            | The file name contains `.`, spaces, or other symbols | [Rename the file](#file-names-become-class-names)                                                           |
| Wrong icon after adding new ones                 | Codepoints shifted                                   | [Pin codepoints](#pin-codepoints-for-stable-output)                                                         |
| No icon at all, or a placeholder box             | The CSS isn't loaded, or the base class is missing   | Import the [virtual stylesheet](/vite/#import-the-virtual-stylesheet) and use both classes: `icon icon-add` |
