---
title: vite-svg-2-webfont Docs
layout: home
description: Overview of vite-svg-2-webfont, a Vite plugin for generating icon webfonts from SVG files.

hero:
    name: vite-svg-2-webfont
    text: Generate icon webfonts from SVG files in Vite
    tagline: A Vite plugin that turns SVG icon folders into font assets, CSS classes, and a virtual stylesheet you can import into your app.
    image:
        src: /logo.svg
        alt: vite-svg-2-webfont logo
    actions:
        - theme: brand
          text: Get Started
          link: /getting-started
        - theme: alt
          text: View on GitHub
          link: https://github.com/atlowChemi/vite-svg-2-webfont

features:
    - title: Vite-native workflow
      icon:
          src: /native-workflow.svg
      details: Add one plugin to your Vite config, import the virtual stylesheet, and let the plugin generate font assets for your icons.
    - title: Flexible output
      icon:
          src: /flexible-output.svg
      details: Generate HTML, CSS, and multiple font formats during development, with support for inline assets and preload hints during build.
    - title: Highly configurable
      icon:
          src: /configurable.svg
      details: Control file globs, destinations, selectors, class prefixes, codepoints, font formats, template paths, and generator options.
---

## Why this plugin

`vite-svg-2-webfont` uses [`@atlowchemi/webfont-generator`](https://github.com/atlowChemi/vite-svg-2-webfont/tree/main/packages/webfont-generator) (Which is a native Rust implementation of [`@vusion/webfonts-generator`](https://github.com/vusion/webfonts-generator)) to transform SVG icon sets into webfont files and matching CSS classes. It is designed for projects that want icon fonts integrated directly into the Vite pipeline.

## What you get

- Generated font files in the formats you choose
- A stylesheet that maps SVG file names to icon class names
- A virtual module import for bringing the generated CSS into your app
- Optional HTML output for previewing generated icons during development

Continue with [Getting Started](/getting-started) for installation and setup details.
