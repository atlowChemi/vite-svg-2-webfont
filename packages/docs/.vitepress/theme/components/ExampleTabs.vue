<script setup lang="ts">
import { useId } from 'vue';

/**
 * A VitePress code group whose tabs can hold any content, not only code blocks. `tabs` maps each slot name
 * to its tab label, in display order; the first tab starts active, and VitePress's own code-group handler
 * switches tabs on click.
 */
defineProps<{ tabs: Record<string, string> }>();

const group = useId();
</script>

<template>
    <div class="vp-code-group example-tabs">
        <div class="tabs">
            <template v-for="(label, slot, index) in tabs" :key="slot">
                <input :id="`${group}-${index}`" type="radio" :name="group" :checked="index === 0" />
                <label :for="`${group}-${index}`" :data-title="label">{{ label }}</label>
            </template>
        </div>
        <div class="blocks">
            <div v-for="(_, slot, index) in tabs" :key="slot" class="vp-block" :class="{ active: index === 0 }">
                <slot :name="slot" />
            </div>
        </div>
    </div>
</template>

<style scoped>
/* Code blocks show whenever their tab is active; the tab label already names the file. */
.example-tabs .vp-block :deep(div[class*='language-']) {
    display: block;
    margin: 0 !important;
    border-radius: 0 0 8px 8px !important;
}

.example-tabs .vp-block :deep(.vp-code-block-title-bar) {
    display: none;
}

.example-tabs .vp-block :deep(.vp-code-block-title) {
    margin: 0;
}

.example-tabs .vp-block > :deep(:not(div[class*='language-'], .vp-code-block-title)) {
    border-radius: 0 0 8px 8px;
    background-color: var(--vp-code-block-bg);
}
</style>
