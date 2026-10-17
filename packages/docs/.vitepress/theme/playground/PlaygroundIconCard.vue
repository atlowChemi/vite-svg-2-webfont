<script setup lang="ts">
import { ref, watch } from 'vue';
import { formatCodepoint, isPrivateUse } from './codepoints';
import type { PlaygroundIcon } from './options';

const props = defineProps<{ icon: PlaygroundIcon; previewFamily?: string; size: number; classPrefix: string; nameError?: string; codepointError?: string }>();
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
        <span v-if="previewFamily" class="pg-card-glyph" :style="{ fontFamily: previewFamily, fontSize: `${size}px` }">{{ String.fromCodePoint(icon.codepoint) }}</span>
        <span v-else class="pg-card-glyph pg-card-source" :style="{ width: `${size}px`, height: `${size}px` }" v-html="icon.svg" />
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
