<script setup lang="ts">
import 'virtual:docs-examples';
import { computed } from 'vue';
import { iconsFor } from './common';

/**
 * Live output of one example: every input icon with its class, then any slot content (for example a
 * template's utilities). `rows` repeats the icons once per extra class, such as a design modifier.
 * The generated CSS is scoped to this container, so markup uses real class names.
 */
const props = withDefaults(defineProps<{ example: string; baseClass?: string; classPrefix?: string; rows?: { label: string; class?: string }[] }>(), {
    baseClass: 'icon',
    classPrefix: 'icon-',
    rows: () => [{ label: '' }],
});

const names = computed(() => [
    ...new Set(
        iconsFor(props.example).map(icon =>
            icon.file
                .split('/')
                .at(-1)!
                .replace(/\.svg$/, ''),
        ),
    ),
]);
</script>

<template>
    <div class="example-output" :class="`vp-example-${example}`">
        <template v-for="row in rows" :key="row.label">
            <p v-if="row.label" class="example-output-label">{{ row.label }}</p>
            <div class="example-output-grid">
                <figure v-for="name in names" :key="name">
                    <i :class="[baseClass, `${classPrefix}${name}`, row.class]" aria-hidden="true" />
                    <figcaption>{{ classPrefix }}{{ name }}</figcaption>
                </figure>
            </div>
        </template>
        <div v-if="$slots.default" class="example-output-grid example-output-extra">
            <slot />
        </div>
    </div>
</template>
