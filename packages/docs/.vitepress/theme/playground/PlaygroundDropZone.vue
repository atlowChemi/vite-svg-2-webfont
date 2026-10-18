<script setup lang="ts">
import { ref } from 'vue';

const emit = defineEmits<{ files: [files: Iterable<File>]; samples: [] }>();
const dragging = ref(false);

function onDrop(event: DragEvent) {
    dragging.value = false;
    if (event.dataTransfer) emit('files', event.dataTransfer.files);
}

function onPick(event: Event) {
    const input = event.target as HTMLInputElement;
    // The FileList is live: clearing the input below empties it, so hand over a copy.
    if (input.files) emit('files', [...input.files]);
    input.value = '';
}
</script>

<template>
    <div class="pg-drop" :class="{ 'pg-drop-active': dragging }" @dragover.prevent="dragging = true" @dragleave="dragging = false" @drop.prevent="onDrop">
        <p><strong>Drop SVG files here</strong></p>
        <div class="pg-drop-actions">
            <label class="pg-button">
                Choose files
                <input class="pg-visually-hidden" type="file" accept=".svg,image/svg+xml" multiple @change="onPick" />
            </label>
            <button type="button" class="pg-button pg-button-alt" @click="emit('samples')">Load sample icons</button>
        </div>
        <p class="pg-hint">Everything runs in your browser. Your SVGs are never uploaded.</p>
    </div>
</template>

<style scoped>
.pg-drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 32px 16px;
    border: 2px dashed var(--vp-c-divider);
    border-radius: var(--pg-radius);
    text-align: center;
    transition:
        border-color 0.2s,
        background-color 0.2s;
}

.pg-drop p {
    margin: 0;
}

.pg-drop-active {
    border-color: var(--vp-c-brand-1);
    background: var(--vp-c-brand-soft);
}

.pg-drop-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 12px;
}
</style>
