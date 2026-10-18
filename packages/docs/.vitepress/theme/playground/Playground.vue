<script setup lang="ts">
import { computed, ref } from 'vue';
import './playground.css';
import PlaygroundDropZone from './PlaygroundDropZone.vue';
import PlaygroundIconCard from './PlaygroundIconCard.vue';
import PlaygroundOptions from './PlaygroundOptions.vue';
import PlaygroundOutputs from './PlaygroundOutputs.vue';
import PlaygroundToast from './PlaygroundToast.vue';
import { readSvgFiles } from './files';
import { useIconList } from './useIconList';
import { useNotice } from './useNotice';
import { useStoredSettings } from './useStoredSettings';
import { useWebfontBuild } from './useWebfontBuild';

const samples = import.meta.glob<string>('../../../examples/icons/*.svg', { query: '?raw', import: 'default', eager: true });

const { settings, reset: resetSettings } = useStoredSettings();
const { icons, errors, addSources, rename, setCodepoint, remove, removeAll, reassign } = useIconList(settings);
const { build, error, building } = useWebfontBuild(icons, settings);
const { notice, notify, dismiss } = useNotice();
const size = ref(40);

const iconNames = computed(() => icons.value.map(icon => icon.name));

async function addFiles(files: Iterable<File>) {
    const { sources, skipped } = await readSvgFiles(files);
    if (skipped) notify(skipped);
    addSources(sources);
}

function loadSamples() {
    addSources(Object.entries(samples).map(([path, svg]) => ({ fileName: path.split('/').at(-1)!, svg })));
}
</script>

<template>
    <div class="pg">
        <PlaygroundToast :message="notice" @dismiss="dismiss" />
        <PlaygroundDropZone @files="addFiles" @samples="loadSamples" />

        <div class="pg-layout">
            <section class="pg-main">
                <div class="pg-toolbar">
                    <h2>Preview</h2>
                    <span class="pg-status" role="status">
                        <template v-if="building">Building…</template>
                        <template v-else-if="build">Built in {{ build.milliseconds }} ms</template>
                    </span>
                    <label class="pg-size">Size <input v-model.number="size" type="range" min="16" max="96" /></label>
                    <button v-if="icons.length" type="button" class="pg-link" @click="removeAll">Remove all</button>
                </div>

                <p v-if="error" class="pg-error" role="alert">{{ error }}</p>
                <p v-if="!icons.length" class="pg-empty">Add some SVG icons to see them as a font.</p>
                <p v-else-if="build && !build.previewFamily" class="pg-hint">Select WOFF2, WOFF, or TTF to preview the font. The cards show the source SVGs meanwhile.</p>

                <div class="pg-grid">
                    <PlaygroundIconCard
                        v-for="icon in icons"
                        :key="icon.id"
                        :icon="icon"
                        :preview-family="build?.previewFamily"
                        :built="build?.codepoints.has(icon.codepoint)"
                        :show-source="Boolean(build) && !build?.previewFamily"
                        :size="size"
                        :class-prefix="settings.classPrefix"
                        :name-error="errors[icon.id]?.name"
                        :codepoint-error="errors[icon.id]?.codepoint"
                        @rename="rename(icon, $event)"
                        @codepoint="setCodepoint(icon, $event)"
                        @remove="remove(icon)"
                    />
                </div>

                <PlaygroundOutputs v-if="build" :build="build" :settings="settings" :icons="icons" />
            </section>

            <aside class="pg-aside">
                <PlaygroundOptions v-model="settings" :icon-names="iconNames" @reassign="reassign" @reset="resetSettings" />
            </aside>
        </div>
    </div>
</template>

<style scoped>
.pg-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 32px;
}

@media (min-width: 960px) {
    .pg-layout {
        grid-template-columns: minmax(0, 1fr) 320px;
    }
}

.pg-toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px 20px;
    margin-bottom: 16px;
}

.pg-status {
    flex: 1;
    font-size: 13px;
    color: var(--vp-c-text-2);
}

.pg-size {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
}

.pg-empty {
    padding: 48px 0;
    text-align: center;
    color: var(--vp-c-text-2);
}

.pg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
}
</style>
