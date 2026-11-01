---
description: Combine light, regular, and bold drawings of the same icons into one font family, and pick a design with a modifier class.
---

# Variants

A **family** combines several **designs** of the same icons, such as light and bold, into one font file per format. Each design becomes a font weight, and a modifier class picks it:

```html
<span class="icon icon-add" aria-hidden="true"></span> <span class="icon icon-add icon--bold" aria-hidden="true"></span>
```

The first icon uses the default design; the second uses bold. Because every design shares one file, switching designs loads nothing new, and each format is requested once rather than once per design.

::: tip Words used in the docs
A _design_ is one complete drawing of the icon set, with its own folder of SVGs. The option that configures designs is called `variants`, so the docs call its entries _variants_. A _family_ is the font that combines them. See the [glossary](/guide/glossary#design).
:::

## Configure the designs

Give each design its own folder and match the file names, so `light/add.svg` and `bold/add.svg` are the same icon. Then list the designs in `variants` instead of top-level `files`:

::: code-group

```ts [Vite]
viteSvgToWebfont({
    context: './src/icons',
    variants: [
        { name: 'light', context: 'light', weight: 300, default: true },
        { name: 'bold', context: 'bold', weight: 700 },
    ],
});
```

```ts [Node.js]
await generateWebfonts({
    dest: './dist/fonts',
    variants: [
        { name: 'light', files: ['icons/light/add.svg'], weight: 300, default: true },
        { name: 'bold', files: ['icons/bold/add.svg'], weight: 700 },
    ],
});
```

```ts [Browser]
using result = await generateWebfonts({
    variants: [
        { name: 'light', sources: [{ id: 'light:add', name: 'add', svg: lightAdd }], weight: 300, default: true },
        { name: 'bold', sources: [{ id: 'bold:add', name: 'add', svg: boldAdd }], weight: 700 },
    ],
});
```

```json [CLI]
{
    "dest": "dist/fonts",
    "variants": [
        { "name": "light", "files": ["icons/light"], "weight": 300, "default": true },
        { "name": "bold", "files": ["icons/bold"], "weight": 700 }
    ]
}
```

```rust [Rust]
let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    variants: Some(vec![
        FontVariant {
            name: "light".to_owned(),
            files: vec!["icons/light/add.svg".to_owned()],
            weight: Some(300),
            default: Some(true),
        },
        FontVariant {
            name: "bold".to_owned(),
            files: vec!["icons/bold/add.svg".to_owned()],
            weight: Some(700),
            default: None,
        },
    ]),
    ..Default::default()
};
```

:::

The CLI reads a JSON manifest: `webfont-generator --config icons.webfont.json`. In the browser, the icon's `name` matches designs, and each source keeps its own `id`.

## Rules

- **At least two designs**, each with a unique name and at least one SVG.
- **Exactly one default.** It renders when no modifier class is set.
- **Weights from 1 to 1000, increasing in list order.** Omit `weight` to assign it automatically: the default design gets 400, and the others move outward in steps of 100, or evenly between explicit weights that are too close for that. Every final weight is unique.
- **Formats:** TTF, WOFF, and WOFF2. The default is WOFF and WOFF2; SVG fonts and EOT are only available for single-design fonts.
- **Names don't change file names.** All designs share one file per format, named after `fontName`.

## Modifier classes

Each design except the default gets a modifier class, made from `variantClassPrefix` and the design name: `icon--bold` by default. The modifier goes next to the base and icon classes; on its own, it doesn't render an icon. Design names are escaped in the generated selectors, so names with special characters still work.

## Icons missing from a design

When an icon exists in one design but not another, `missingGlyphs` decides what the missing design shows:

| `behavior`        | What happens                                                                                          |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| `blank` (default) | An empty glyph that keeps the icon's width, so text around it doesn't move.                           |
| `error`           | Generation fails and lists every missing design and icon pair.                                        |
| `fallback`        | The icon is borrowed from the design named in `variant`, which must contain every icon in the family. |

```ts
missingGlyphs: { behavior: 'fallback', variant: 'regular' }
```

## Updating a family

[Incremental updates](/guide/features/incremental) work for families too: icons can be added, changed, renamed, and removed within each design. The designs themselves and their weights are fixed for the lifetime of a result; adding, removing, or reordering designs needs a fresh generation.

## See also

- [Color glyphs](/guide/features/color-glyphs): one color selection applies across every design.
- [Template context](/guide/templates#template-context): the extra family fields custom templates receive.
- [Under the hood](/guide/under-the-hood#multi-variant-fonts): how the engine compiles a family.
- Live example: [multi-weight family](/vite/examples#multi-weight-family).
