<script setup lang="ts">
import { computed } from 'vue';
import { iconsFor } from './common';

/** The input SVGs of one example, as a thumbnail grid labelled with their path inside `icons/`. */
const props = defineProps<{ example: string }>();

const icons = computed(() => iconsFor(props.example));
</script>

<template>
    <div class="example-icons">
        <figure v-for="icon in icons" :key="icon.file">
            <div class="example-icons-svg" v-html="icon.svg" />
            <figcaption>{{ icon.file }}</figcaption>
        </figure>
    </div>
</template>

<style scoped>
.example-icons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 16px;
    padding: 20px 24px;
}

.example-icons figure {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
}

/* Source SVGs use black fills, so keep them on a light chip in both themes. */
.example-icons-svg {
    width: 48px;
    height: 48px;
    padding: 8px;
    border-radius: 8px;
    background: #fff;
    box-shadow: inset 0 0 0 1px #d0d0d7;
}

.example-icons-svg :deep(svg) {
    display: block;
    width: 100%;
    height: 100%;
}

.example-icons figcaption {
    font-family: var(--vp-font-family-mono);
    font-size: 12px;
    color: var(--vp-c-text-2);
}
</style>
