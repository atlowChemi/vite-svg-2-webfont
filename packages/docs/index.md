---
title: fonte
titleTemplate: SVG icons, cast into fonts
description: fonte casts SVG icons into fonts with a Rust engine, available as a CLI, Node and browser bindings, and a Vite plugin.
layout: page
footer: false
sidebar: false
---

<FonteLanding>
<template #cli>

```sh
cargo install webfont-generator
webfont-generator --dest ./dist/fonts ./icons/
```

</template>
<template #node>

```ts
import { generateWebfonts } from '@atlowchemi/webfont-generator';

await generateWebfonts({ files, dest: './dist/fonts', fontName: 'my-icons' });
```

</template>
<template #wasm>

```ts
import { generateWebfonts } from '@atlowchemi/webfont-browser-generator';

const { woff2 } = await generateWebfonts({ files, fontName: 'my-icons' });
```

</template>
<template #vite>

```ts
import viteSvgToWebfont from 'vite-svg-2-webfont';

export default { plugins: [viteSvgToWebfont({ context: './icons' })] };
```

</template>
<template #outputs>

::: code-group

```html [index.html]
<link rel="stylesheet" href="my-icons.css" /> <i class="icon icon-star"></i>
```

```ts [my-icons.ts]
export const icons = ['star', 'bell'] as const;
export type IconName = (typeof icons)[number];
```

:::

</template>
</FonteLanding>
