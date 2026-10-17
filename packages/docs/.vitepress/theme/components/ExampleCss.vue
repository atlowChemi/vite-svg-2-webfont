<script setup lang="ts">
import generatedCss from 'virtual:docs-examples-css';
import { computed } from 'vue';

/**
 * The CSS the plugin generated for one example, highlighted at build time. Lines containing any `highlight`
 * snippet get VitePress's highlighted-line style, matching the config tab's highlights.
 */
const props = defineProps<{ example: string; highlight?: string[] }>();

const ENTITIES: Record<string, string> = { '&quot;': '"', '&#x27;': "'", '&#39;': "'", '&lt;': '<', '&gt;': '>', '&amp;': '&' };

function lineText(line: string): string {
    return line.replaceAll(/<[^>]+>/g, '').replaceAll(/&(?:quot|#x27|#39|lt|gt|amp);/g, entity => ENTITIES[entity]!);
}

const html = computed(() => {
    const source = generatedCss[props.example] ?? '';
    const snippets = props.highlight ?? [];
    if (!snippets.length) return source;
    return source
        .split('\n')
        .map(line =>
            line.includes('<span class="line">') && snippets.some(snippet => lineText(line).includes(snippet))
                ? line.replace('<span class="line">', '<span class="line highlighted">')
                : line,
        )
        .join('\n');
});
</script>

<template>
    <div v-html="html" />
</template>

<style scoped>
/* One rule per icon makes generated CSS long; keep the tab compact and scrollable. */
:deep(pre) {
    max-height: 420px;
    overflow: auto;
}
</style>
