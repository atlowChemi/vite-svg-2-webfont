<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { withBase } from 'vitepress';
import { trayIcons } from './landing/icons';

// A type case of cast glyphs with one empty compartment: the sort for this page. A highlight scans the case once and
// stops on the gap; with reduced motion, the gap is marked straight away.
const icons = Object.values(trayIcons);
const missing = 10;
const cells = Array.from({ length: 18 }, (_, i) => ({
    paths: icons[(i * 5) % icons.length]!,
    code: (0xe001 + i).toString(16).toUpperCase(),
}));

const scan = ref<number | null>(null);
const found = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;

onMounted(() => {
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
        found.value = true;
        return;
    }
    const step = (i: number) => {
        scan.value = i;
        if (i === missing) {
            timer = setTimeout(() => {
                scan.value = null;
                found.value = true;
            }, 240);
            return;
        }
        timer = setTimeout(() => step(i + 1), 170);
    };
    timer = setTimeout(() => step(0), 400);
});
onBeforeUnmount(() => clearTimeout(timer));
</script>

<template>
    <div class="not-found">
        <div class="copy">
            <h1>
                <span class="code">404</span>
                <span class="line">This sort isn’t in the case.</span>
            </h1>
            <p class="lede">A sort is one piece of cast type. The page you asked for isn’t in this case: it may have moved when the docs were reorganized by tool.</p>
            <p class="mono where">U+E404 · not found</p>
            <div class="actions">
                <a class="button brand" :href="withBase('/guide/')">Go to the guide</a>
                <a class="button" :href="withBase('/')">Home</a>
            </div>
        </div>
        <div class="case" aria-hidden="true">
            <div v-for="(cell, i) in cells" :key="cell.code" class="cell" :class="{ hole: i === missing, scan: scan === i, found: found && i === missing }">
                <template v-if="i !== missing">
                    <svg viewBox="0 0 24 24"><path v-for="d in cell.paths" :key="d" :d /></svg>
                    <span class="mono">{{ cell.code }}</span>
                </template>
            </div>
        </div>
    </div>
</template>

<style scoped>
.not-found {
    display: grid;
    gap: 48px;
    align-items: center;
    max-width: 1080px;
    margin: 0 auto;
    padding: 64px 24px 96px;
}

@media (min-width: 960px) {
    .not-found {
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
        padding: 120px 32px 160px;
    }
}

.mono {
    font-family: var(--vp-font-family-mono);
}

h1 {
    display: grid;
    gap: 8px;
    font-family: var(--fonte-font-display);
    font-weight: 600;
}

.code {
    font-size: clamp(72px, 14vw, 120px);
    line-height: 1;
    letter-spacing: -0.02em;
}

.line {
    font-size: 24px;
    line-height: 1.3;
    color: var(--vp-c-text-2);
}

.lede {
    max-width: 30rem;
    margin-top: 16px;
    color: var(--vp-c-text-2);
    line-height: 1.6;
}

.where {
    margin-top: 16px;
    font-size: 13px;
    color: var(--vp-c-brand-1);
}

.actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 28px;
}

.button {
    display: inline-block;
    border: 1px solid var(--vp-c-divider);
    border-radius: 6px;
    padding: 8px 18px;
    font-size: 14px;
    font-weight: 600;
    color: var(--vp-c-text-1);
    text-decoration: none;
    transition:
        border-color 0.25s,
        background-color 0.25s;
}

.button:hover {
    border-color: var(--vp-c-brand-1);
}

.button.brand {
    border-color: var(--vp-c-brand-1);
    background: var(--vp-c-brand-1);
    color: var(--vp-c-white);
}

.button.brand:hover {
    border-color: var(--vp-c-brand-2);
    background: var(--vp-c-brand-2);
}

.case {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--vp-c-divider);
    border-radius: 8px;
    background: var(--vp-c-bg-soft);
}

.cell {
    display: grid;
    place-items: center;
    gap: 4px;
    aspect-ratio: 4 / 5;
    padding: 10% 0 8%;
    border: 1px solid var(--vp-c-divider);
    border-radius: 4px;
    background: var(--vp-c-bg);
    transition: border-color 0.12s;
}

.cell svg {
    width: 46%;
    fill: none;
    stroke: var(--vp-c-text-2);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
}

.cell span {
    font-size: 10px;
    color: var(--vp-c-text-3);
}

.cell.scan {
    border-color: var(--fonte-ember);
}

.cell.hole {
    border-style: dashed;
    border-color: var(--vp-c-text-3);
}

.cell.hole.scan,
.cell.found {
    border: 2px solid var(--fonte-molten);
}
</style>
