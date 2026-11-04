---
description: How a page reaches each icon in the font, by codepoint through a CSS class or by ligature through its name, when to use each, and how to keep codepoints stable.
---

<script setup>
import SvgAuthoringCompare from '../../.vitepress/theme/components/SvgAuthoringCompare.vue';
</script>

# Codepoints and Ligatures

A page can reach an icon in the font in two ways:

- **By codepoint:** each icon gets a character in Unicode's Private Use Area, and its CSS class inserts that character. This is what `<i class="icon icon-add"></i>` does.
- **By ligature:** the font replaces the icon's name with its glyph, so the text `add` in the icon font renders the _add_ icon.

Every font has codepoints. Ligatures are on by default and can be turned off.

## Classes or ligatures?

|                                       | Classes (codepoints)                                 | Ligatures                                                                  |
| ------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------- |
| Markup                                | `<i class="icon icon-add"></i>`                      | `<span class="icons">add</span>`, with your own `font-family` rule         |
| Before the font loads, or if it fails | Nothing visible                                      | The word, `add`, which takes up its own width and can shift layout         |
| Screen readers                        | Nothing to read: add an `aria-label` or visible text | Read the word: hide it with `aria-hidden="true"` if the icon is decorative |
| Copy, paste, and search               | Copies a private-use character                       | Copies the word                                                            |
| Needs                                 | The generated CSS                                    | Only the font, and text you control                                        |

**Use classes by default.** They're what the generated CSS is built for, nothing shows before the font loads, and they work whatever text is around them.

**Use ligatures when you can write text but not classes or CSS:** content from a CMS, Markdown, or translation strings; text drawn with canvas `fillText`; native apps and design tools that only let you pick a font. The name is also readable on its own, so the content still makes sense if the font doesn't load.

Ligatures have a few catches:

- The generated CSS applies the font to the class's `::before` only. To use ligatures, set `font-family` to your `fontName` on the element yourself.
- Non-zero `letter-spacing` and `font-variant-ligatures: none` can turn ligatures off, and then the word shows.
- The name is the icon's final name, after any `rename`, so renaming an icon changes what text renders it.

Turn ligatures off with `ligature: false` (`--no-ligature` in the CLI) when you only use classes and want the font to skip its ligature rules.

## How codepoints are assigned

Icons get codepoints in input order, starting at `U+F101`, inside the Private Use Area, which no real character uses. Change the start with `startCodepoint` (`start_codepoint` in Rust).

Adding an icon can shift the codepoints of the icons after it. That doesn't matter while the CSS and the font are always deployed together, but it breaks anything that stores codepoints separately: hand-written `content: "\f102"` rules, cached CSS served with a new font, or exports to design tools.

The comparison renders the raw codepoints `U+F101`–`U+F103` in three fonts. After `bell.svg` is added, `U+F102` changes from _remove_ to _bell_, unless codepoints are pinned.

<SvgAuthoringCompare
    :codepoints="[0xf101, 0xf102, 0xf103]"
    :cases="[
        { id: 'codepoints-v1', label: 'add, remove' },
        { id: 'codepoints-v2', label: 'add, bell, remove' },
        { id: 'codepoints-v2-pinned', label: 'add, bell, remove; codepoints pinned' },
    ]"
/>

## Pin codepoints

Pin the codepoints of icons you publish with `codepoints`, a map from icon name to codepoint. Unpinned icons continue from the start codepoint, skipping pinned values.

::: code-group

```ts [Vite]
viteSvgToWebfont({
    context: './src/icons',
    codepoints: { add: 0xf101, remove: 0xf102 },
});
```

```ts [Node.js]
await generateWebfonts({
    files,
    dest: 'dist/fonts',
    codepoints: { add: 0xf101, remove: 0xf102 },
});
```

```ts [Browser]
using result = await generateWebfonts({
    sources,
    codepoints: { add: 0xf101, remove: 0xf102 },
});
```

```json [CLI]
{
    "dest": "dist/fonts",
    "files": ["icons"],
    "codepoints": { "add": 61697, "remove": 61698 }
}
```

```rust [Rust]
let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    files,
    codepoints: Some(HashMap::from([("add".to_owned(), 0xf101), ("remove".to_owned(), 0xf102)])),
    ..Default::default()
};
```

:::

JSON has no hexadecimal numbers, so the CLI manifest uses decimal codepoints: `61697` is `0xF101`.

## See also

- Live examples: [ligatures](/vite/examples#ligatures) and [pinned codepoints](/vite/examples#pinned-codepoints).
- [File names become class names](/guide/svg-authoring#file-names-become-class-names): which names are safe for classes and ligatures.
