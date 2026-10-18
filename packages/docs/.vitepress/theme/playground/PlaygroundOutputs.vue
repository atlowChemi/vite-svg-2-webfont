<script setup lang="ts">
import { computed, ref } from 'vue';
import PlaygroundViewer from './PlaygroundViewer.vue';
import { codepointsSnippet } from './codepoints';
import { downloadFile } from './download';
import { selectedTypes, type PlaygroundIcon, type PlaygroundSettings } from './options';
import { useClipboard } from './useClipboard';
import type { WebfontBuild } from './useWebfontBuild';
import { outputFileName, zipOutputs } from './zip';

const props = defineProps<{ build: WebfontBuild; settings: PlaygroundSettings; icons: PlaygroundIcon[] }>();

const { copied, copy } = useClipboard();

/** Fonts in their CSS order, then the CSS and HTML. */
const outputOrder = computed(() => [...selectedTypes(props.settings), 'css', 'html']);

const files = computed(() =>
    Object.entries(props.build.outputs)
        .toSorted(([a], [b]) => outputOrder.value.indexOf(a) - outputOrder.value.indexOf(b))
        .map(([kind, bytes]) => ({
            kind,
            fileName: outputFileName(props.settings.fontName, kind),
            bytes,
            size: `${(bytes.byteLength / 1024).toFixed(1)} kB`,
        })),
);

const zipping = ref(false);
const zipError = ref<string>();

async function downloadZip() {
    if (zipping.value) return;
    // Capture the current build: a rebuild can finish while the archive is being compressed.
    const { fontName } = props.settings;
    const { outputs } = props.build;
    zipping.value = true;
    zipError.value = undefined;
    try {
        downloadFile(`${fontName}.zip`, await zipOutputs(fontName, outputs));
    } catch (cause) {
        zipError.value = `Could not create the ZIP: ${cause instanceof Error ? cause.message : String(cause)}`;
    } finally {
        zipping.value = false;
    }
}
</script>

<template>
    <section class="pg-outputs">
        <h2>Outputs</h2>
        <div class="pg-files">
            <button v-for="file in files" :key="file.kind" type="button" class="pg-file" @click="downloadFile(file.fileName, file.bytes)">
                <span>{{ file.fileName }}</span>
                <small>{{ file.size }}</small>
            </button>
            <button type="button" class="pg-button" :disabled="zipping" @click="downloadZip">{{ zipping ? 'Zipping…' : 'Download all (.zip)' }}</button>
            <button type="button" class="pg-button pg-button-alt" @click="copy('codepoints', codepointsSnippet(icons))">
                {{ copied === 'codepoints' ? 'Copied!' : 'Copy as codepoints' }}
            </button>
        </div>
        <p v-if="zipError" class="pg-error" role="alert">{{ zipError }}</p>
        <p class="pg-hint">Codepoints are assigned in the order icons were added. Paste the copied option into your config to keep them stable.</p>
        <PlaygroundViewer :build="build" />
    </section>
</template>

<style scoped>
.pg-outputs {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 32px;
}

.pg-files {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.pg-file {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 6px 12px;
    border: 1px solid var(--vp-c-divider);
    border-radius: 6px;
    font-family: var(--vp-font-family-mono);
    font-size: 13px;
}

.pg-file:hover {
    border-color: var(--vp-c-brand-1);
}

.pg-file small {
    color: var(--vp-c-text-2);
}
</style>
