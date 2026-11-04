<script setup lang="ts">
import { useId } from 'vue';
import { letters, moltenSurface, tPath, tTransform, viewBox } from './wordmark';

// Inline rather than an <img>, so the letters and the mold follow the site theme instead of the OS color scheme.
// `surface` replaces the still molten surface (in the T's 100×86 box), e.g. with a frame of the landing page's pour.
const { surface = moltenSurface } = defineProps<{ surface?: string }>();
const clip = useId();
</script>

<template>
    <svg class="fonte-wordmark" :viewBox role="img" aria-label="fonte">
        <path class="ink" :d="letters" />
        <g :transform="tTransform">
            <clipPath :id="clip"><path :d="tPath" /></clipPath>
            <g :clip-path="`url(#${clip})`">
                <path class="molten" :d="`${surface} L100 100 L0 100 Z`" />
                <path class="surface" :d="surface" />
            </g>
            <path class="mold" :d="tPath" />
        </g>
    </svg>
</template>

<style scoped>
.fonte-wordmark {
    display: block;
    color: var(--vp-c-text-1);
}

.ink {
    fill: currentColor;
}

.molten {
    fill: var(--fonte-molten);
}

.surface {
    fill: none;
    stroke: var(--fonte-ember);
    stroke-width: 1.9;
}

.mold {
    fill: none;
    stroke: var(--fonte-mold);
    stroke-width: var(--fonte-mold-width, 2.4);
    stroke-linejoin: round;
}
</style>
