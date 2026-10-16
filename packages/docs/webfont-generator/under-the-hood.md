---
description: How the webfont-generator engine is built — pipeline architecture, adapters, and benchmark results.
---

# Under the Hood

This page explains how the engine works internally. You don't need it to use any of the packages; see the [overview](./) for that.

## Adapters

The shared Rust engine owns font generation, template rendering, and caches. Each distribution is a thin adapter around it:

- The **npm package** owns the JavaScript API, callback transport, and conversion between JavaScript and Rust values. Its `webfont-generator-napi` crate is an internal workspace implementation, not a separate package for consumers to install.
- The **browser package** compiles the engine's filesystem-free API to WebAssembly with wasm-bindgen and runs it in a dedicated module worker.
- The **CLI** is a binary inside the Rust crate, behind the `cli` feature.

## Architecture

Owning the full pipeline made a new parallel architecture possible. The engine parses and normalizes glyphs once, then shares their prepared geometry between SVG output and binary font compilation. SVG serialization and font-table compilation can run in parallel, followed by parallel WOFF, WOFF2, and EOT generation. WOFF and WOFF2 consume the compiled tables directly, avoiding an intermediate TTF serialization and reparse. Together with parallel glyph processing, this reduces repeated work and makes better use of multiple CPU cores.

### Single-face fonts

For a single-face font, the generation pipeline works as follows:

1. **SVG loading** -- Read and validate source SVG files in parallel
2. **Glyph preparation** -- Parse glyph paths and optional solid paint with `usvg`, process geometry with `oxvg_path`, and normalize shared layer/fallback geometry. Parsed and processed caches retain paint for selected glyphs.
3. **Parallel SVG and table assembly** -- Serialize the SVG font if requested and supported by the selected options. Compile paint-aware selectable glyphs and auxiliary layers into shared tables using [`write-fonts`](https://github.com/googlefonts/fontations): `glyf` fallback plus COLR/CPAL for selected glyphs.
4. **Requested binary outputs** -- Assemble TTF if requested; generate WOFF and WOFF2 from shared tables, and EOT (which embeds TTF) when supported by the selected options.
5. **Template rendering** -- Render CSS and HTML previews via Handlebars when writing those files or calling the result's rendering methods

```mermaid
flowchart TD
    A[SVG files + optional color selection] --> B[Parse geometry &amp; optional paint<br/>usvg + oxvg_path]
    B --> C[Shared layer/fallback geometry<br/>Parsed &amp; processed caches]
    C -.->|if supported and requested| D[SVG output]
    C -->|if binary formats requested| E[Paint-aware selectable IDs + layer glyphs<br/>glyf fallback + optional COLR/CPAL]
    E -.->|if requested| T[TTF output]
    E --> P
    subgraph P[Parallel generation - requested formats only]
        direction TD
        F[WOFF]
        G[WOFF2]
        H[EOT when supported<br/>embeds TTF]
    end
```

### Multi-variant fonts

For a family such as light, regular, and bold, the engine combines the designs into **one font file per requested format**:

1. **Family assembly** -- Load each variant's SVGs, match icons by name, assign shared codepoints, and apply the missing-glyph policy.
2. **Shared glyph preparation** -- Extract optional paint for selected logical names in every resolved variant. Cache shared layer/fallback geometry, normalized using family-wide metrics and shared advance widths.
3. **Variant compilation** -- Allocate paint-aware selectable IDs and auxiliary layer glyphs. Emit weight-axis metadata, `glyf` fallback, and optional COLR/CPAL tables. `rvrn` and conditioned `liga` select a painted design while retaining the shared advance; outlines are not interpolated.
4. **Output and templates** -- Assemble TTF if requested, encode requested WOFF and WOFF2 outputs in parallel, and render CSS/HTML with variant metadata and modifier classes.

```mermaid
flowchart TD
    A[SVG files for each variant + color selection] --> B[Match final names &amp; resolve missing glyphs]
    B --> C[Optional paint extraction + cached layer/fallback geometry<br/>Shared metrics and advance widths]
    C --> D[Paint-aware IDs + glyf + optional COLR/CPAL<br/>rvrn/liga select painted designs]
    D -.->|if requested| E[TTF]
    D --> P
    subgraph P[Parallel encoding - requested formats only]
        F[WOFF]
        G[WOFF2]
    end
```

This path supports TTF, WOFF, and WOFF2; SVG/EOT output is available only for color-disabled single-face fonts. Multi-variant families also support incremental regeneration, reusing cached geometry and paint for unchanged designs. See [multi-variant fonts](./node#multi-variant-fonts) and [color selection](./node#color-selection) for usage.

## Performance

The legacy `@vusion/webfonts-generator` engine does not support SVG path optimization. The Rust engine adds it through [`optimizeOutput`](./node#optimizeoutput) and is still faster with optimization enabled. The tables show the Rust engine with path optimization disabled and enabled side by side.

### All formats

Generating SVG, TTF, EOT, WOFF, and WOFF2:

| Glyphs | `@vusion/webfonts-generator` | Rust engine                                       | Rust engine (Optimized SVG) |
| ------ | ---------------------------- | ------------------------------------------------- | --------------------------- |
| 100    | 65.6 ms                      | <strong class="benchmark-winner">11.8 ms</strong> | 12.0 ms                     |
| 300    | 256.1 ms                     | <strong class="benchmark-winner">39.0 ms</strong> | 41.9 ms                     |
| 600    | 523.3 ms                     | <strong class="benchmark-winner">72.9 ms</strong> | 79.2 ms                     |

### Modern web formats

If you only need web delivery formats, the same comparison with **WOFF2 only** or **WOFF + WOFF2** gives:

| Formats      | Glyphs | `@vusion/webfonts-generator` | Rust engine                                       | Rust engine (Optimized SVG) |
| ------------ | ------ | ---------------------------- | ------------------------------------------------- | --------------------------- |
| WOFF2        | 100    | 64.0 ms                      | <strong class="benchmark-winner">11.6 ms</strong> | 12.6 ms                     |
| WOFF2        | 300    | 254.6 ms                     | <strong class="benchmark-winner">39.1 ms</strong> | 40.9 ms                     |
| WOFF2        | 600    | 510.1 ms                     | <strong class="benchmark-winner">71.5 ms</strong> | 77.9 ms                     |
| WOFF + WOFF2 | 100    | 64.1 ms                      | <strong class="benchmark-winner">11.5 ms</strong> | 11.6 ms                     |
| WOFF + WOFF2 | 300    | 280.2 ms                     | <strong class="benchmark-winner">38.6 ms</strong> | 40.9 ms                     |
| WOFF + WOFF2 | 600    | 514.3 ms                     | <strong class="benchmark-winner">75.8 ms</strong> | 77.1 ms                     |

With path optimization disabled (the default), the average speedup is **6.4× for WOFF2 only** and **6.5× for WOFF + WOFF2**. Enabling path optimization still delivers average speedups of **5.9×** and **6.3×**, respectively.

These builds skip SVG font serialization and TTF binary assembly in the Rust engine. In this benchmark, their total times remain close to the all-format builds: fewer output formats do not necessarily translate into a large reduction in elapsed time.

### Avoiding repeated work

The shared-geometry pipeline avoids reparsing an SVG font to compile binary formats. Requesting only the formats you need also skips unnecessary output work: a WOFF2-only build does not serialize an SVG font or assemble a TTF binary.

For repeated builds—such as a dev server regenerating your icon font whenever you edit an SVG—[incremental generation](./node#incremental) can reuse parsed glyphs, compiled glyphs, font tables, and WOFF compression or transform work from the previous build, giving you faster feedback while developing. In our 100–600-glyph benchmarks, a one-SVG edit rebuilt **1.5–1.8× faster than a full Rust build** and **12–17× faster than Vusion**, which always rebuilds the entire font.

Multi-weight families use a variant compilation path to produce TTF, WOFF, and WOFF2; see [multi-variant fonts](./node#multi-variant-fonts).

<style scoped>
.benchmark-winner {
    color: var(--vp-c-brand-1);
}
</style>
