<script setup lang="ts">
import { useId } from 'vue';
import { letters, moltenSurface, tPath, tTransform, viewBox } from './wordmark';

// Inline rather than an <img>, so the letters and the mold follow the site theme instead of the OS color scheme.
const clip = useId();
</script>

<template>
    <svg class="fonte-wordmark" :viewBox role="img" aria-label="fonte">
        <path class="ink" :d="letters" />
        <g :transform="tTransform">
            <clipPath :id="clip"><path :d="tPath" /></clipPath>
            <g :clip-path="`url(#${clip})`">
                <path class="molten" :d="`${moltenSurface} L100 100 L0 100 Z`" />
                <path class="surface" :d="moltenSurface" />
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
    stroke-width: 2.4;
    stroke-linejoin: round;
}
</style>
