<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue';
import PlaygroundIconCard from './PlaygroundIconCard.vue';
import PlaygroundOptions from './PlaygroundOptions.vue';
import { codepointsSnippet, nextCodepoint, parseCodepoint } from './codepoints';
import { iconName, isValidName, uniqueName } from './names';
import { defaultSettings, selectedTypes, type PlaygroundIcon, type PlaygroundSettings } from './options';
import { useWebfontBuild } from './useWebfontBuild';
import { outputFileName, zipOutputs } from './zip';

const SETTINGS_KEY = 'vite-svg-2-webfont:playground-settings';
const samples = import.meta.glob<string>('../../../examples/icons/*.svg', { query: '?raw', import: 'default', eager: true });

function loadSettings(): PlaygroundSettings {
    try {
        const saved = localStorage.getItem(SETTINGS_KEY);
        return saved ? { ...defaultSettings, ...(JSON.parse(saved) as Partial<PlaygroundSettings>) } : { ...defaultSettings };
    } catch {
        return { ...defaultSettings };
    }
}

const settings = ref<PlaygroundSettings>(loadSettings());
const icons = ref<PlaygroundIcon[]>([]);
const errors = ref<Record<string, { name?: string; codepoint?: string }>>({});
const size = ref(40);
const dragging = ref(false);
const viewer = ref<'css' | 'html'>('css');
const copied = shallowRef<string>();

watch(
    settings,
    value => {
        try {
            localStorage.setItem(SETTINGS_KEY, JSON.stringify(value));
        } catch {
            // Storage can be unavailable (private mode, blocked site data); settings then last for this visit only.
        }
    },
    { deep: true },
);

const { build, error, building } = useWebfontBuild(icons, settings);

const iconNames = computed(() => icons.value.map(icon => icon.name));

/** Add SVGs; a file whose name matches an existing icon replaces that icon's drawing and keeps its codepoint. */
function addSources(sources: { fileName: string; svg: string }[]) {
    const next = [...icons.value];
    for (const { fileName, svg } of sources) {
        const name = iconName(fileName);
        const existing = next.find(icon => icon.name === name);
        if (existing) {
            existing.svg = svg;
            continue;
        }
        const codepoint = nextCodepoint(settings.value.startCodepoint, new Set(next.map(icon => icon.codepoint)));
        next.push({ id: crypto.randomUUID(), name: uniqueName(name, new Set(next.map(icon => icon.name))), svg, codepoint });
    }
    icons.value = next;
}

async function addFiles(files: Iterable<File>) {
    const svgs = [...files].filter(file => file.type === 'image/svg+xml' || file.name.toLowerCase().endsWith('.svg'));
    addSources(await Promise.all(svgs.map(async file => ({ fileName: file.name, svg: await file.text() }))));
}

function onDrop(event: DragEvent) {
    dragging.value = false;
    if (event.dataTransfer) void addFiles(event.dataTransfer.files);
}

function onPick(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files) void addFiles(input.files);
    input.value = '';
}

function loadSamples() {
    addSources(Object.entries(samples).map(([path, svg]) => ({ fileName: path.split('/').at(-1)!, svg })));
}

function setError(id: string, field: 'name' | 'codepoint', message?: string) {
    errors.value = { ...errors.value, [id]: { ...errors.value[id], [field]: message } };
}

function rename(icon: PlaygroundIcon, name: string) {
    const taken = icons.value.some(other => other.id !== icon.id && other.name === name);
    const message = !isValidName(name) ? 'Use letters, numbers, - and _ only' : taken ? 'Another icon has this name' : undefined;
    setError(icon.id, 'name', message);
    if (message) return;
    settings.value = { ...settings.value, colorNames: settings.value.colorNames.map(item => (item === icon.name ? name : item)) };
    icons.value = icons.value.map(item => (item.id === icon.id ? { ...item, name } : item));
}

function setCodepoint(icon: PlaygroundIcon, text: string) {
    const codepoint = parseCodepoint(text);
    const taken = codepoint !== undefined && icons.value.some(other => other.id !== icon.id && other.codepoint === codepoint);
    const message = codepoint === undefined ? 'Enter a hexadecimal codepoint, such as F101' : taken ? 'Another icon uses this codepoint' : undefined;
    setError(icon.id, 'codepoint', message);
    if (message || codepoint === undefined) return;
    icons.value = icons.value.map(item => (item.id === icon.id ? { ...item, codepoint } : item));
}

function remove(icon: PlaygroundIcon) {
    icons.value = icons.value.filter(item => item.id !== icon.id);
    settings.value = { ...settings.value, colorNames: settings.value.colorNames.filter(name => name !== icon.name) };
}

function reassign() {
    const used = new Set<number>();
    for (const icon of icons.value) {
        icon.codepoint = nextCodepoint(settings.value.startCodepoint, used);
        used.add(icon.codepoint);
    }
    errors.value = {};
}

function resetSettings() {
    settings.value = { ...defaultSettings };
}

function download(fileName: string, bytes: BlobPart) {
    const link = document.createElement('a');
    link.href = URL.createObjectURL(new Blob([bytes]));
    link.download = fileName;
    link.click();
    URL.revokeObjectURL(link.href);
}

/** Fonts in their CSS order, then the CSS and HTML. */
const OUTPUT_ORDER = computed(() => [...selectedTypes(settings.value), 'css', 'html']);

const files = computed(() =>
    Object.entries(build.value?.outputs ?? {})
        .toSorted(([a], [b]) => OUTPUT_ORDER.value.indexOf(a) - OUTPUT_ORDER.value.indexOf(b))
        .map(([kind, bytes]) => ({
            kind,
            fileName: outputFileName(settings.value.fontName, kind),
            bytes,
            size: `${(bytes.byteLength / 1024).toFixed(1)} kB`,
        })),
);

function downloadZip() {
    if (build.value) download(`${settings.value.fontName}.zip`, zipOutputs(settings.value.fontName, build.value.outputs) as Uint8Array<ArrayBuffer>);
}

async function copy(label: string, text: string) {
    try {
        await navigator.clipboard.writeText(text);
        copied.value = label;
        setTimeout(() => (copied.value = undefined), 1500);
    } catch {
        copied.value = undefined;
    }
}
</script>

<template>
    <div class="pg">
        <div class="pg-drop" :class="{ 'pg-drop-active': dragging }" @dragover.prevent="dragging = true" @dragleave="dragging = false" @drop.prevent="onDrop">
            <p><strong>Drop SVG files here</strong></p>
            <div class="pg-drop-actions">
                <label class="pg-button">
                    Choose files
                    <input class="pg-visually-hidden" type="file" accept=".svg,image/svg+xml" multiple @change="onPick" />
                </label>
                <button type="button" class="pg-button pg-button-alt" @click="loadSamples">Load sample icons</button>
            </div>
            <p class="pg-hint">Everything runs in your browser. Your SVGs are never uploaded.</p>
        </div>

        <div class="pg-layout">
            <section class="pg-main">
                <div class="pg-toolbar">
                    <h2>Preview</h2>
                    <span class="pg-status" role="status">
                        <template v-if="building">Building…</template>
                        <template v-else-if="build">Built in {{ build.milliseconds }} ms</template>
                    </span>
                    <label class="pg-size">Size <input v-model.number="size" type="range" min="16" max="96" /></label>
                    <button v-if="icons.length" type="button" class="pg-link" @click="icons = []">Remove all</button>
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
                        :size="size"
                        :class-prefix="settings.classPrefix"
                        :name-error="errors[icon.id]?.name"
                        :codepoint-error="errors[icon.id]?.codepoint"
                        @rename="rename(icon, $event)"
                        @codepoint="setCodepoint(icon, $event)"
                        @remove="remove(icon)"
                    />
                </div>

                <section v-if="build" class="pg-outputs">
                    <h2>Outputs</h2>
                    <div class="pg-files">
                        <button v-for="file in files" :key="file.kind" type="button" class="pg-file" @click="download(file.fileName, file.bytes)">
                            <span>{{ file.fileName }}</span>
                            <small>{{ file.size }}</small>
                        </button>
                        <button type="button" class="pg-button" @click="downloadZip">Download all (.zip)</button>
                        <button type="button" class="pg-button pg-button-alt" @click="copy('codepoints', codepointsSnippet(icons))">
                            {{ copied === 'codepoints' ? 'Copied!' : 'Copy as codepoints' }}
                        </button>
                    </div>
                    <p class="pg-hint">Codepoints are assigned in the order icons were added. Paste the copied option into your config to keep them stable.</p>

                    <div class="pg-viewer">
                        <div class="pg-viewer-tabs" role="tablist">
                            <button type="button" role="tab" :aria-selected="viewer === 'css'" @click="viewer = 'css'">CSS</button>
                            <button type="button" role="tab" :aria-selected="viewer === 'html'" @click="viewer = 'html'">HTML</button>
                            <button type="button" class="pg-link" @click="copy('source', viewer === 'css' ? build.css : build.html)">
                                {{ copied === 'source' ? 'Copied!' : 'Copy' }}
                            </button>
                        </div>
                        <pre class="pg-code"><code>{{ viewer === 'css' ? build.css : build.html }}</code></pre>
                    </div>
                </section>
            </section>

            <aside class="pg-aside">
                <PlaygroundOptions v-model="settings" :icon-names="iconNames" @reassign="reassign" @reset="resetSettings" />
            </aside>
        </div>
    </div>
</template>

<style>
.pg {
    --pg-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.pg h2 {
    margin: 0;
    padding: 0;
    border: 0;
    font-size: 18px;
}

.pg input:not([type='checkbox'], [type='radio'], [type='range']),
.pg textarea {
    width: 100%;
    padding: 6px 10px;
    border: 1px solid var(--vp-c-divider);
    border-radius: 6px;
    background: var(--vp-c-bg);
    color: var(--vp-c-text-1);
    font-size: 14px;
}

.pg input[aria-invalid='true'] {
    border-color: var(--vp-c-danger-1);
}

.pg .pg-visually-hidden {
    position: absolute;
    width: 1px !important;
    height: 1px;
    padding: 0 !important;
    border: 0 !important;
    overflow: hidden;
    clip-path: inset(50%);
}

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

.pg-button {
    display: inline-flex;
    align-items: center;
    padding: 6px 16px;
    border-radius: 20px;
    background: var(--vp-c-brand-3);
    color: var(--vp-c-white);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
}

.pg-button:hover {
    background: var(--vp-c-brand-2);
}

.pg-button-alt {
    background: var(--vp-c-default-soft);
    color: var(--vp-c-text-1);
}

.pg-button-alt:hover {
    background: var(--vp-c-default-2);
}

.pg-link {
    color: var(--vp-c-brand-1);
    font-size: 13px;
    font-weight: 500;
}

.pg-hint {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--vp-c-text-2);
}

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

.pg-error {
    padding: 8px 12px;
    white-space: pre-wrap;
    font-family: var(--vp-font-family-mono);
    font-size: 13px !important;
    border-radius: 6px;
    background: var(--vp-c-danger-soft);
    color: var(--vp-c-danger-1);
    font-size: 14px;
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

.pg-card-source svg {
    width: 100%;
    height: 100%;
}

.pg-card-field {
    width: 100%;
}

.pg .pg-card-field input {
    padding: 2px 6px;
    text-align: center;
    font-size: 13px;
}

.pg .pg-card-codepoint {
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

.pg-viewer {
    border-radius: var(--pg-radius);
    background: var(--vp-code-block-bg);
    overflow: hidden;
}

.pg-viewer-tabs {
    display: flex;
    gap: 16px;
    padding: 0 16px;
    box-shadow: inset 0 -1px var(--vp-code-tab-divider);
}

.pg-viewer-tabs [role='tab'] {
    padding: 10px 0;
    border-bottom: 2px solid transparent;
    color: var(--vp-code-tab-text-color);
    font-size: 14px;
    font-weight: 500;
}

.pg-viewer-tabs [aria-selected='true'] {
    border-color: var(--vp-code-tab-active-bar-color);
    color: var(--vp-code-tab-active-text-color);
}

.pg-viewer-tabs .pg-link {
    margin-left: auto;
}

.pg-code {
    margin: 0;
    max-height: 360px;
    padding: 16px;
    overflow: auto;
    font-family: var(--vp-font-family-mono);
    font-size: 13px;
    line-height: 1.6;
}

.pg-options {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.pg-options-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
}

.pg-options details {
    border-top: 1px solid var(--vp-c-divider);
    padding: 10px 0;
}

.pg-options summary {
    cursor: pointer;
    font-weight: 600;
    font-size: 14px;
}

.pg-options details > label,
.pg-options details > .pg-checks,
.pg-options details > .pg-template {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 10px;
    font-size: 13px;
}

.pg-options details > label.pg-toggle,
.pg-checks label {
    flex-direction: row;
    align-items: center;
    gap: 8px;
}

.pg-checks {
    flex-direction: row !important;
    flex-wrap: wrap;
    gap: 8px 16px !important;
}

.pg-checks label {
    display: flex;
}

.pg-color-names {
    max-height: 160px;
    overflow: auto;
}

.pg-order {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    font-size: 13px;
}

.pg-order li {
    display: flex;
    justify-content: space-between;
    padding: 2px 0;
    font-family: var(--vp-font-family-mono);
}

.pg-order button {
    padding: 0 6px;
}

.pg-order button:disabled {
    opacity: 0.3;
}

.pg-template {
    flex-direction: row !important;
    justify-content: space-between;
}

.pg-options .pg-button {
    align-self: flex-start;
    margin-top: 10px;
    font-size: 13px;
}
</style>
