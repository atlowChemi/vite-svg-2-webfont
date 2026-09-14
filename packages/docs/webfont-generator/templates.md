---
description: Shared reference for generated CSS, HTML previews, Handlebars template context, and the SCSS icon mixin.
---

# Templates

The generator can render CSS and HTML with Handlebars templates. These templates work with
fonts generated through Node.js, Rust, or the CLI. Use the built-in CSS for ready-made icon
classes, the SCSS template to attach icons to your own selectors, or a custom template for
your application's markup.

::: tip
Custom Handlebars templates can generate more than CSS and HTML — for example,
React or Vue component source files, etc. Use the template context to produce the
content you need, and set `cssDest` or `htmlDest` to an appropriate filename.
Your application's build tools handle compilation of the generated components.
:::

## Built-in Templates

Choose a stylesheet template for displaying icons in your application, and optionally generate
an HTML preview to browse the family:

| Template               | What it generates                                                         | When to use it                                                                                                         |
| ---------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| [CSS](#generated-css)  | Font-face declarations and ready-made icon classes.                       | Use icons directly in your markup without a Sass compilation step. This is the default stylesheet template.            |
| [SCSS](#scss)          | Font-face declarations, an icon map, and the `webfont-icon($name)` mixin. | Attach icons to your own component selectors in a project that compiles Sass. Select this instead of the CSS template. |
| [HTML](#html-previews) | An icon grid with embedded styles.                                        | Browse or inspect the generated icons. Enable preview output with `html: true`.                                        |

### Generated CSS

With `fontName: 'my-icons'`, the default CSS output is `my-icons.css`. Load it and apply the
base `icon` class together with a glyph class:

```html
<link rel="stylesheet" href="/fonts/my-icons.css" /> <span class="icon icon-add" aria-hidden="true"></span>
```

`templateOptions.classPrefix` defaults to `icon-`; `templateOptions.baseSelector` defaults to
`.icon`. Use these options to customize the glyph class prefix and the selector receiving
the shared icon styles.

#### Multi-variant styles

For a family with a default `light` design and a `bold` design (for example), add the variant modifier to
the same element:

```html
<span class="icon icon-add" aria-hidden="true"></span> <span class="icon icon-add icon--bold" aria-hidden="true"></span>
```

The icon class selects the glyph; the modifier selects its design. A modifier by itself
does not display an icon. `variantClassPrefix` controls the modifier prefix (default `icon--`).

The stylesheet declares one exact-weight `@font-face` per design, all sharing the same font
URLs. Icon pseudo-elements use the default weight and
[`font-synthesis: none`](https://developer.mozilla.org/en-US/docs/Web/CSS/font-synthesis)
to prevent the browser from creating a synthetic bold or italic design.
Modifiers override the pseudo-element's weight; setting an inherited weight on a parent
does not select a different design.

When writing your own weight rules, the browser uses CSS font matching to choose an available
face. For faces at 300, 400, and 700, requests for 100/350 select 300, 450/500 select 400,
and 600/900 select 700. Use generated modifiers when you want a particular supplied design.

### SCSS

The SCSS template emits font-face declarations, an icon map, and a `webfont-icon($name)` mixin.
The mixin adds an icon in the caller's `:before` pseudo-element, letting you use your own
component selectors instead of the generated CSS icon classes. Compile the generated SCSS
with Sass as part of your stylesheet build.

Generate it from Node by selecting `templates.scss` and an `.scss` destination:

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';
import * as templates from '@atlowchemi/webfont-generator/templates';

await generateWebfonts({
    files: ['icons/add.svg'],
    dest: './styles/fonts',
    fontName: 'my-icons',
    cssTemplate: templates.scss,
    cssDest: './styles/fonts/my-icons.scss',
});
```

Then consume the generated file:

::: code-group

```scss [styles/app.scss]
@use './fonts/my-icons' as icons;

.add-button {
    @include icons.webfont-icon('add');
}
```

:::

The Rust and CLI template-path options also accept a copy of the SCSS Handlebars template.

#### Multi-variant SCSS

Use `variants` instead of `files` in the generation options above to generate a multi-variant
stylesheet. The same mixin uses each icon family's default design and emits variant modifiers
scoped to its caller. With a design named `bold`, the example becomes:

```html
<button class="add-button">Add</button> <button class="add-button icon--bold">Add in bold</button>
```

The generated map stores `(family, codepoint)` for single-variant icons and
`(family, codepoint, weight, style, variantsMap)` for multi-variant icons. The fifth entry maps
CSS-escaped modifier identifiers to numeric weights. This lets the mixin select modifiers
from the icon's own family when multiple generated families are combined.

### HTML previews

Enable `html` with the default template to write a preview containing an icon grid and embedded CSS.
For multi-variant families, the built-in grid displays the default design (e.g., `light`).
Custom previews can use the `variants` context to offer other views. Default font URLs are relative
to the HTML destination; explicit URL overrides are used as supplied.

## Custom templates

Set `cssTemplate` or `htmlTemplate` to a Handlebars file. In Rust these options are
`css_template` and `html_template`; CLI manifests use camelCase names. See the API-specific
references for [Node template paths](./node#templates), [Rust options](./rust#generatewebfontsoptions),
or [CLI configuration](./cli#json-manifest).

For example, this custom HTML template lists the icon names:

::: code-group

```handlebars [icon-list.hbs]
<h1>{{fontName}}</h1>
<ul>
    {{#each names}}
        <li>{{this}}</li>
    {{/each}}
</ul>
```

:::

Pass extra values through `templateOptions` (`template_options` in Rust). Node also supports
callbacks that mutate the context before rendering. Handlebars context names are camelCase
regardless of the API used to generate the font.

### Generate Type Safe Component

Another useful example could be a template to export icon names and a TypeScript union type.
This could allow a React or Vue component to accept only valid icon names as a prop.
Use the HTML context for this because it supplies the ordered `names` array:

::: code-group

```handlebars [icons.ts.hbs]
export const iconBaseSelector = '{{baseSelector}}' as const; export const iconClassPrefix = '{{classPrefix}}' as const; export const icons = [
{{#each names}}
    '{{this}}',
{{/each}}
] as const; export type IconOption = typeof icons[number];
```

```ts [generate-icons.ts]
import { generateWebfonts } from '@atlowchemi/webfont-generator';

await generateWebfonts({
    files: ['icons/add.svg', 'icons/remove.svg'],
    dest: './dist/fonts',
    html: true,
    htmlTemplate: './icons.ts.hbs',
    htmlDest: './src/generated/icons.ts',
});
```

```ts [src/generated/icons.ts]
export const iconBaseSelector = '.icon' as const;
export const iconClassPrefix = 'icon-' as const;
export const icons = ['add', 'remove'] as const;

export type IconOption = (typeof icons)[number]; // 'add' | 'remove'
```

```tsx [src/components/Icon.tsx]
import type { FC } from 'react';
import { iconBaseSelector, iconClassPrefix, type IconOption } from '../generated/icons';

export const Icon: FC<{ name: IconOption }> = ({ name }) => <span className={`${iconBaseSelector.slice(1)} ${iconClassPrefix}${name}`} aria-hidden="true" />;
```

```vue [src/components/Icon.vue]
<script setup lang="ts">
import { iconBaseSelector, iconClassPrefix, type IconOption } from '../generated/icons';

defineProps<{ name: IconOption }>();
</script>

<template>
    <span :class="[iconBaseSelector.slice(1), `${iconClassPrefix}${name}`]" aria-hidden="true" />
</template>
```

:::

Import `icons` to iterate over the available names, or `IconOption` to type an icon component's
name prop. Here `htmlDest` writes TypeScript source instead of an HTML preview; CSS and font
generation still use their configured destinations. This simple template assumes names and
selector settings that can be embedded directly in single-quoted TypeScript strings.

Both components assume the default `.icon` base selector: `.slice(1)` turns that selector into
the `icon` class name. If you customize `baseSelector`, adapt the component's classes to match.
Load the generated CSS in your application. These examples render decorative icons alongside
visible text, so they use `aria-hidden="true"`.

## Template context

Single-variant generation uses `files` and provides one design per icon. Multi-variant
generation uses `variants` and supplies additional design metadata.

### Context shapes

These TypeScript declarations describe the fields supplied to templates before customization;
they are reference examples, not additional package exports. The **Shared types** tab defines
the fields used by both CSS and HTML. Multi-variant contexts add family metadata to the
single-variant shape.

::: code-group

```ts [CSS]
interface SingleVariantCssContext extends CommonTemplateFields {
    // Ready to insert into an @font-face src descriptor.
    src: string;
    // Glyph name → hex string, for example { add: 'f101' }.
    codepoints: Record<string, string>;
}

type MultiVariantCssContext = SingleVariantCssContext & VariantMetadata;
```

```ts [HTML]
interface SingleVariantHtmlContext extends CommonTemplateFields {
    // Glyph names in declaration order, for example ['add', 'remove'].
    names: string[];
    // Rendered CSS for embedding in the preview.
    styles: string;
    // Glyph name → numeric codepoint, for example { add: 0xf101 }.
    codepoints: Record<string, number>;
}

type MultiVariantHtmlContext = SingleVariantHtmlContext & VariantMetadata;
```

```ts [Shared types]
interface CommonTemplateFields {
    fontName: string; // Font family name.
    classPrefix: string; // Glyph class prefix; default 'icon-'.
    baseSelector: string; // Shared icon selector; default '.icon'.
}

interface VariantMetadata {
    variants: TemplateVariant[]; // Designs in input order, with resolved weights.
    variantClassPrefix: string; // Modifier prefix; default 'icon--'.
    defaultWeight: number; // Resolved default design weight.
    fontStyle: string; // Resolved style; default 'normal'.
}

interface TemplateVariant {
    name: string; // Input design name.
    weight: number; // Explicit or automatically assigned weight.
    default: boolean; // Whether this is the default design.
    className: string; // Modifier class name for HTML.
    selector: string; // CSS-escaped modifier identifier, without a leading dot.
}
```

:::

| Template | Single-variant (`files`)                                  | Multi-variant (`variants`)      |
| -------- | --------------------------------------------------------- | ------------------------------- |
| CSS      | Common fields + `src` + hex `codepoints`                  | Same fields + `VariantMetadata` |
| HTML     | Common fields + `names` + `styles` + numeric `codepoints` | Same fields + `VariantMetadata` |

Family metadata describes designs within the shared font resource, not separate font files.
The generator does not supply these metadata fields for single-variant input. Additional
`templateOptions` values are available in either mode and can customize the context;
see [Node callbacks](#node-callbacks) for the exported callback types and their handling of
user-supplied fields.

### Node callbacks

`cssContext` and `htmlContext` mutate the context in place. Multi-variant options infer
`CssContext<true>` and `HtmlContext<true>`, so their family metadata is typed:

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

await generateWebfonts({
    dest: './dist/fonts',
    variants: [
        { name: 'light', files: ['icons/light/add.svg'], default: true },
        { name: 'bold', files: ['icons/bold/add.svg'] },
    ],
    cssContext(context) {
        context.designNames = context.variants.map(variant => variant.name);
    },
});
```

Plain `CssContext` and `HtmlContext` treat family metadata as `unknown`: single-variant
`templateOptions` may supply arbitrary values under those names. Validate those values before
using them, or annotate a callback as variant-only when appropriate:

```ts
import type { CssContext } from '@atlowchemi/webfont-generator';

function customize(context: CssContext) {
    if (typeof context.defaultWeight === 'number') {
        console.log(context.defaultWeight);
    }
}

function customizeVariants(context: CssContext<true>) {
    console.log(context.defaultWeight); // number
    console.log(context.variants[0].name); // string
}
```

Results created with either callback cannot currently be regenerated.

### Rust context types

`CssContext` and `HtmlContext` describe the context metadata using snake_case Rust fields.
Family fields are optional: `variants`, `variant_class_prefix`, `default_weight`, and
`font_style`. `TemplateVariant` contains `name`, `weight`, `default`, `class_name`, and
`selector`. Templates themselves use the camelCase names in the declarations above.
