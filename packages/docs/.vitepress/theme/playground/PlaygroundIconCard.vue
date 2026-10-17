<script setup lang="ts">
import { ref, watch } from 'vue';
import { formatCodepoint, isPrivateUse } from './codepoints';
import type { PlaygroundIcon } from './options';

/**
 * `previewFamily` renders the glyph from the built font when it contains this icon (`built`). Without a previewable
 * font, `showSource` shows the source SVG instead; otherwise an empty placeholder keeps the card's size.
 */
const props = defineProps<{
    icon: PlaygroundIcon;
    previewFamily?: string;
    built?: boolean;
    showSource?: boolean;
    size: number;
    classPrefix: string;
    nameError?: string;
    codepointError?: string;
}>();
const emit = defineEmits<{ rename: [name: string]; codepoint: [text: string]; remove: [] }>();

const name = ref(props.icon.name);
const codepoint = ref(formatCodepoint(props.icon.codepoint));
watch(
    () => props.icon.name,
    value => (name.value = value),
);
watch(
    () => props.icon.codepoint,
    value => (codepoint.value = formatCodepoint(value)),
);
</script>

<template>
    <figure class="pg-card" :class="{ 'pg-card-invalid': nameError || codepointError }">
        <button class="pg-card-remove" type="button" :aria-label="`Remove ${icon.name}`" title="Remove" @click="emit('remove')">×</button>
        <span v-if="previewFamily && built" class="pg-card-glyph" :style="{ fontFamily: previewFamily, fontSize: `${size}px` }">{{ String.fromCodePoint(icon.codepoint) }}</span>
        <span v-else-if="showSource" class="pg-card-glyph pg-card-source" :style="{ width: `${size}px`, height: `${size}px` }" v-html="icon.svg" />
        <span v-else class="pg-card-glyph" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true" />
        <label class="pg-card-field">
            <span class="pg-visually-hidden">Name</span>
            <input v-model="name" spellcheck="false" :aria-invalid="Boolean(nameError)" @change="emit('rename', name)" />
        </label>
        <code class="pg-card-class">{{ classPrefix }}{{ icon.name }}</code>
        <label class="pg-card-field">
            <span class="pg-visually-hidden">Codepoint</span>
            <input v-model="codepoint" class="pg-card-codepoint" spellcheck="false" :aria-invalid="Boolean(codepointError)" @change="emit('codepoint', codepoint)" />
        </label>
        <small v-if="nameError || codepointError" class="pg-card-error">{{ nameError ?? codepointError }}</small>
        <small v-else-if="!isPrivateUse(icon.codepoint)" class="pg-card-warning">Outside the Private Use Area</small>
    </figure>
</template>

<style scoped>
.pg-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 16px 10px 12px;
    border: 1px solid var(--vp-c-divider);
    border-radius: var(--pg-radius);
    background: var(--vp-c-bg-soft);
}

.pg-card-invalid {
    border-color: var(--vp-c-danger-1);
}

.pg-card-remove {
    position: absolute;
    top: 4px;
    right: 8px;
    color: var(--vp-c-text-3);
    font-size: 18px;
}

.pg-card-remove:hover {
    color: var(--vp-c-danger-1);
}

.pg-card-glyph {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 56px;
    line-height: 1;
    color: var(--vp-c-text-1);
}

.pg-card-source {
    min-height: 0;
    padding: 4px;
    border-radius: 6px;
    background: #fff;
}

.pg-card-source :deep(svg) {
    width: 100%;
    height: 100%;
}

.pg-card-field {
    width: 100%;
}

/* Outranks the shared `.pg input` rule. */
.pg-card .pg-card-field input {
    padding: 2px 6px;
    text-align: center;
    font-size: 13px;
}

.pg-card-codepoint {
    font-family: var(--vp-font-family-mono);
}

.pg-card-class {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--vp-c-text-2);
}

.pg-card-error,
.pg-card-warning {
    font-size: 12px;
    text-align: center;
}

.pg-card-error {
    color: var(--vp-c-danger-1);
}

.pg-card-warning {
    color: var(--vp-c-warning-1);
}
</style>
