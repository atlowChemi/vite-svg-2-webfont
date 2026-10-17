---
title: Playground
description: Turn SVG icons into a webfont in your browser — tweak every option, preview the result, and download fonts, CSS, and HTML.
layout: page
sidebar: false
---

<script setup>
import Playground from './.vitepress/theme/playground/Playground.vue';
</script>

<div class="playground-page">

# Playground

Drop in SVG icons to build a webfont with [`@atlowchemi/webfont-browser-generator`](/webfont-generator/browser), the same Rust engine compiled to WebAssembly. Adjust the options, check the preview, and download the result.

<ClientOnly>
<Playground />
</ClientOnly>

</div>

<style>
.playground-page {
    max-width: 1280px;
    margin: 0 auto;
    padding: 32px 24px 96px;
}

.playground-page > h1 {
    margin-bottom: 12px;
    font-size: 32px;
    font-weight: 600;
}

.playground-page > p {
    max-width: 720px;
    margin-bottom: 24px;
    color: var(--vp-c-text-2);
}
</style>
