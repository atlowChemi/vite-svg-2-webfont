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

<!-- prettier-ignore -->
```html [index.html]
<link rel="stylesheet" href="my-icons.css" />
<i class="icon icon-star"></i>
```

<!-- prettier-ignore -->
```css [my-icons.css]
@font-face {
	font-family: "my-icons";
	font-display: block;
	src: url("my-icons.woff2?…") format("woff2");
}

.icon {
	line-height: 1;
}

.icon:before {
	font-family: my-icons !important;
	font-style: normal;
	font-weight: normal !important;
	vertical-align: top;
}

.icon-star:before {
	content: "\f101";
}
.icon-bell:before {
	content: "\f102";
}
```

```ts [my-icons.ts]
export const icons = ['star', 'bell'] as const;
export type IconName = (typeof icons)[number];
```

:::

</template>
</FonteLanding>
