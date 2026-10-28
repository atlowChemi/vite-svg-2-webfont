<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue';
import { highlight } from './highlight';
import { useClipboard } from './useClipboard';
import type { WebfontBuild } from './useWebfontBuild';

const props = defineProps<{ build: WebfontBuild }>();

const viewer = ref<'css' | 'html' | 'svg' | 'ts'>('css');
const { copied, copy } = useClipboard();

/** The SVG font is the one text-based font format, so it can be shown alongside CSS, HTML and TypeScript. */
const svgFont = computed(() => {
    const bytes = props.build.outputs.svg;
    return bytes ? new TextDecoder().decode(bytes) : undefined;
});
watch(svgFont, text => {
    if (!text && viewer.value === 'svg') viewer.value = 'css';
});
const viewerText = computed(() => ({ css: props.build.css, html: props.build.html, ts: props.build.ts, svg: svgFont.value })[viewer.value] ?? '');
/** Highlighting very large output (an SVG font with many icons) would stall the page; it then stays plain text. */
const HIGHLIGHT_LIMIT = 200_000;
const highlighted = shallowRef<{ text: string; html: string }>();
let highlightRequest = 0;

// Highlight the visible output once Shiki loads; plain text shows until then, and stale results are dropped.
watch(
    viewerText,
    async text => {
        const request = ++highlightRequest;
        if (!text || text.length > HIGHLIGHT_LIMIT) return;
        try {
            const html = await highlight(text, viewer.value === 'svg' ? 'xml' : viewer.value);
            if (request === highlightRequest) highlighted.value = { text, html };
        } catch {
            // Highlighting is cosmetic; the plain text stays visible.
        }
    },
    { immediate: true },
);
</script>

<template>
    <div class="pg-viewer">
        <div class="pg-viewer-tabs" role="tablist">
            <button type="button" role="tab" data-title="output.css" :aria-selected="viewer === 'css'" @click="viewer = 'css'">CSS</button>
            <button type="button" role="tab" data-title="output.html" :aria-selected="viewer === 'html'" @click="viewer = 'html'">HTML</button>
            <button v-if="svgFont" type="button" role="tab" data-title="output.svg" :aria-selected="viewer === 'svg'" @click="viewer = 'svg'">SVG font</button>
            <button type="button" role="tab" data-title="output.ts" :aria-selected="viewer === 'ts'" @click="viewer = 'ts'">TypeScript</button>
            <button type="button" class="pg-link" @click="copy('source', viewerText)">
                {{ copied === 'source' ? 'Copied!' : 'Copy' }}
            </button>
        </div>
        <div v-if="highlighted?.text === viewerText" class="pg-code pg-code-highlighted" v-html="highlighted.html" />
        <pre v-else class="pg-code"><code>{{ viewerText }}</code></pre>
    </div>
</template>

<style scoped>
.pg-viewer {
    border-radius: var(--pg-radius);
    background: var(--vp-code-block-bg);
    overflow: hidden;
}

.pg-viewer-tabs {
    display: flex;
    gap: 16px;
    padding: 0 16px;
    box-shadow: inset 0 -1px var(--vp-code-tab-divider);
}

.pg-viewer-tabs [role='tab'] {
    padding: 10px 0;
    border-bottom: 2px solid transparent;
    color: var(--vp-code-tab-text-color);
    font-size: 14px;
    font-weight: 500;
}

.pg-viewer-tabs [aria-selected='true'] {
    border-color: var(--vp-code-tab-active-bar-color);
    color: var(--vp-code-tab-active-text-color);
}

.pg-viewer-tabs .pg-link {
    margin-left: auto;
}

/* File-type icons from vitepress-plugin-group-icons, which sets --icon per data-title. */
.pg-viewer-tabs [data-title]::before {
    display: inline-block;
    width: 1em;
    height: 1em;
    margin-right: 0.5em;
    margin-bottom: -0.2em;
    background: var(--icon) no-repeat center / contain;
}

.pg-code-highlighted {
    padding: 0;
}

.pg-code-highlighted :deep(pre) {
    margin: 0;
    font: inherit;
    padding: 16px;
    background: transparent !important;
}

.pg-code-highlighted :deep(span) {
    color: var(--shiki-light);
}

.dark .pg-code-highlighted :deep(span) {
    color: var(--shiki-dark);
}
</style>
