<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vitepress';
import { useLayout } from 'vitepress/theme';

// Above each page title, like the landing page's section rails: the page's number within its sidebar group, and
// the group's name. Pages outside the sidebar show nothing.
const { sidebar } = useLayout();
const route = useRoute();

const normalize = (path: string) => path.replace(/(\.html|\/index)$/, '').replace(/\/$/, '') || '/';

const position = computed(() => {
    const current = normalize(route.path);
    for (const group of sidebar.value) {
        const index = group.items?.findIndex(item => item.link && normalize(item.link) === current) ?? -1;
        if (index >= 0) return { number: String(index + 1).padStart(2, '0'), group: group.text };
    }
    return undefined;
});
</script>

<template>
    <p v-if="position" class="fonte-eyebrow">
        <span>No.</span><span class="number">{{ position.number }}</span
        ><span class="group">{{ position.group }}</span>
    </p>
</template>

<style scoped>
.fonte-eyebrow {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 0 0 10px;
    font-family: var(--vp-font-family-mono);
    font-size: 12px;
    letter-spacing: 0.03em;
    color: var(--vp-c-text-3);
}

.number {
    font-family: var(--fonte-font-display);
    font-size: 15px;
    font-weight: 600;
    color: var(--vp-c-brand-1);
}

.group::before {
    content: '·';
    margin-right: 6px;
}
</style>
