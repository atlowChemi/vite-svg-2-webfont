<script setup lang="ts">
import 'virtual:svg-authoring-fonts';
import { computed } from 'vue';
import { svgAuthoringFonts } from '../../svgAuthoringCases';

const props = defineProps<{
    /** Fonts to compare, in display order. */
    cases: { id: string; label: string }[];
    /** Render each case's icons inline between these two words, to show alignment against text. */
    text?: [string, string];
    /** Render these raw codepoints instead of class names. */
    codepoints?: number[];
    /** Glyph size in pixels; larger sizes reveal outline detail. */
    size?: number;
}>();

const sources = import.meta.glob<string>('../../../svg-authoring-fixtures/**/*.svg', { query: '?raw', import: 'default', eager: true });

const columns = computed(() =>
    props.cases.map(({ id, label }) => {
        const dir = svgAuthoringFonts.find(font => font.id === id)?.dir ?? id;
        const icons = Object.entries(sources)
            .filter(([path]) => path.split('/').at(-2) === dir)
            .map(([path, svg]) => ({
                name: path
                    .split('/')
                    .at(-1)!
                    .replace(/\.svg$/, ''),
                svg,
            }))
            .toSorted((a, b) => a.name.localeCompare(b.name));
        return { id, label, icons };
    }),
);

/** Class names exactly as an app would write them, mapped onto this case's scoped prefix. */
const classes = (id: string, name: string) => `sa-${id} sa-${id}-${name}`;
const displayName = (name: string) => name.replace(/^[a-z]-/, '');
const hex = (codepoint: number) => `U+${codepoint.toString(16).toUpperCase()}`;
</script>

<template>
    <div class="sa-compare" :class="{ 'sa-stacked': text }" :style="{ '--sa-size': `${size ?? 48}px` }">
        <section v-for="column in columns" :key="column.id" class="sa-column">
            <h4 class="sa-label">{{ column.label }}</h4>

            <p v-if="text" class="sa-text">
                {{ text[0] }}
                <template v-for="icon in column.icons" :key="icon.name"><i :class="classes(column.id, icon.name)" class="sa-glyph-box" /> </template>
                {{ text[1] }}
            </p>

            <div v-else-if="codepoints" class="sa-grid">
                <figure v-for="codepoint in codepoints" :key="codepoint" class="sa-card">
                    <span class="sa-glyph" :style="{ fontFamily: `sa-${column.id}` }">{{ String.fromCodePoint(codepoint) }}</span>
                    <figcaption>
                        <code>{{ hex(codepoint) }}</code>
                    </figcaption>
                </figure>
            </div>

            <div v-else class="sa-grid">
                <figure v-for="icon in column.icons" :key="icon.name" class="sa-card">
                    <div class="sa-pair">
                        <div class="sa-source" :title="`${icon.name}.svg in the browser`" v-html="icon.svg" />
                        <span class="sa-arrow" aria-hidden="true">→</span>
                        <i :class="classes(column.id, icon.name)" class="sa-glyph" :title="`${icon.name} in the font`" />
                    </div>
                    <figcaption>
                        <code>icon-{{ displayName(icon.name) }}</code>
                    </figcaption>
                </figure>
            </div>
        </section>
    </div>
</template>

<style scoped>
.sa-compare {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 16px;
    margin: 16px 0;
}

.sa-column {
    min-width: 0;
    border: 1px solid var(--vp-c-divider);
    border-radius: 8px;
    padding: 12px 16px 16px;
    background: var(--vp-c-bg-soft);
}

.sa-label {
    margin: 0 0 12px;
    font-size: 14px;
    color: var(--vp-c-text-2);
}

.sa-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
}

.sa-card {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
}

.sa-card figcaption code {
    font-size: 12px;
}

.sa-pair {
    display: flex;
    align-items: center;
    gap: 8px;
}

/* Source SVGs use black fills, so keep them on a light chip in both themes. */
.sa-source {
    width: 40px;
    height: 40px;
    padding: 4px;
    border-radius: 6px;
    background: #fff;
    box-shadow: inset 0 0 0 1px #d0d0d7;
}

.sa-source :deep(svg) {
    display: block;
    width: 100%;
    height: 100%;
}

.sa-arrow {
    color: var(--vp-c-text-3);
}

.sa-glyph {
    font-size: var(--sa-size);
    color: var(--vp-c-text-1);
    outline: 1px dashed var(--vp-c-divider);
}

.sa-stacked {
    grid-template-columns: 1fr;
}

.sa-text {
    margin: 0;
    font-size: 40px;
    line-height: 1.4;
    white-space: nowrap;
    overflow-x: auto;
}

/* The dashed box is the glyph's advance width and line box, which shows spacing and alignment. */
.sa-glyph-box {
    outline: 1px dashed var(--vp-c-brand-1);
}
</style>
