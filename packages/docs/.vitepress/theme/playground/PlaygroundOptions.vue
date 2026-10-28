<script setup lang="ts">
import type { FontType } from '@atlowchemi/webfont-browser-generator';
import { computed, ref } from 'vue';
import builtInCss from '../../../../webfont-generator/templates/css.hbs?raw';
import builtInHtml from '../../../../webfont-generator/templates/html.hbs?raw';
import { formatCodepoint, parseCodepoint } from './codepoints';
import { ALL_TYPES, selectedTypes, unavailableTypes, type PlaygroundSettings } from './options';

const props = defineProps<{ iconNames: string[] }>();
const settings = defineModel<PlaygroundSettings>({ required: true });
const emit = defineEmits<{ reassign: []; reset: [] }>();

const unavailable = computed(() => unavailableTypes(settings.value));
const ordered = computed(() => selectedTypes(settings.value));

type NumberKey = 'fontHeight' | 'ascent' | 'descent' | 'round' | 'ttfTimestamp';
function numberValue(key: NumberKey): string {
    return settings.value[key]?.toString() ?? '';
}
function setNumber(key: NumberKey, text: string) {
    const value = text.trim() === '' ? undefined : Number(text);
    settings.value[key] = value !== undefined && Number.isFinite(value) ? value : undefined;
}

function toggle<T>(list: T[], item: T, enabled: boolean) {
    const index = list.indexOf(item);
    if (enabled && index === -1) list.push(item);
    else if (!enabled && index !== -1) list.splice(index, 1);
}

/** Move a selected format one step within the full order, past any unselected formats between them. */
function move(type: FontType, step: -1 | 1) {
    const visible = ordered.value;
    const neighbour = visible[visible.indexOf(type) + step];
    if (!neighbour) return;
    const { order } = settings.value;
    const from = order.indexOf(type);
    const to = order.indexOf(neighbour);
    [order[from], order[to]] = [order[to]!, order[from]!];
}

const startText = ref(formatCodepoint(settings.value.startCodepoint));
const startError = ref<string>();
function setStart() {
    const value = parseCodepoint(startText.value);
    startError.value = value === undefined ? 'Enter a hexadecimal codepoint, such as F101' : undefined;
    if (value !== undefined) settings.value.startCodepoint = value;
}

function customize(key: 'cssTemplate' | 'htmlTemplate') {
    settings.value[key] = key === 'cssTemplate' ? builtInCss : builtInHtml;
}
function resetTemplate(key: 'cssTemplate' | 'htmlTemplate') {
    settings.value[key] = undefined;
}
</script>

<template>
    <div class="pg-options">
        <div class="pg-options-header">
            <h2>Options</h2>
            <button type="button" class="pg-link" @click="emit('reset')">Reset all</button>
        </div>

        <details open>
            <summary>Font</summary>
            <label>Font name <input v-model="settings.fontName" spellcheck="false" /></label>
            <label>Font style <input v-model="settings.fontStyle" placeholder="normal" /></label>
            <label>Font weight <input v-model="settings.fontWeight" placeholder="normal" /></label>
        </details>

        <details open>
            <summary>Formats</summary>
            <div class="pg-checks">
                <label v-for="type in ALL_TYPES" :key="type" :title="unavailable.includes(type) ? 'Color glyphs need TTF, WOFF, or WOFF2' : undefined">
                    <input
                        type="checkbox"
                        :checked="settings.types.includes(type)"
                        :disabled="unavailable.includes(type)"
                        @change="toggle(settings.types, type, ($event.target as HTMLInputElement).checked)"
                    />
                    {{ type.toUpperCase() }}
                </label>
            </div>
            <p class="pg-hint">Order in the generated CSS:</p>
            <ol class="pg-order">
                <li v-for="(type, index) in ordered" :key="type">
                    {{ type.toUpperCase() }}
                    <span>
                        <button type="button" :disabled="index === 0" :aria-label="`Move ${type} up`" @click="move(type, -1)">↑</button>
                        <button type="button" :disabled="index === ordered.length - 1" :aria-label="`Move ${type} down`" @click="move(type, 1)">↓</button>
                    </span>
                </li>
            </ol>
            <label>
                WOFF2 compression quality: {{ settings.woff2CompressionQuality }}
                <input v-model.number="settings.woff2CompressionQuality" type="range" min="0" max="11" />
            </label>
        </details>

        <details>
            <summary>Glyph size and alignment</summary>
            <label
                >Font height
                <input type="number" :value="numberValue('fontHeight')" placeholder="Tallest icon" @change="setNumber('fontHeight', ($event.target as HTMLInputElement).value)"
            /></label>
            <label
                >Ascent <input type="number" :value="numberValue('ascent')" placeholder="Automatic" @change="setNumber('ascent', ($event.target as HTMLInputElement).value)"
            /></label>
            <label>Descent <input type="number" :value="numberValue('descent')" placeholder="0" @change="setNumber('descent', ($event.target as HTMLInputElement).value)" /></label>
            <label
                >Rounding <input type="number" :value="numberValue('round')" placeholder="10e12" @change="setNumber('round', ($event.target as HTMLInputElement).value)"
            /></label>
            <label class="pg-toggle"><input v-model="settings.normalize" type="checkbox" /> Normalize to the same height</label>
            <label class="pg-toggle"><input v-model="settings.fixedWidth" type="checkbox" /> Fixed width</label>
            <label class="pg-toggle"><input v-model="settings.centerHorizontally" type="checkbox" /> Center horizontally</label>
            <label class="pg-toggle"><input v-model="settings.centerVertically" type="checkbox" /> Center vertically</label>
            <label class="pg-toggle"><input v-model="settings.preserveAspectRatio" type="checkbox" /> Preserve aspect ratio</label>
            <label class="pg-toggle"><input v-model="settings.optimizeOutput" type="checkbox" /> Optimize SVG paths</label>
        </details>

        <details>
            <summary>Class names and CSS</summary>
            <label>Base selector <input v-model="settings.baseSelector" spellcheck="false" /></label>
            <label>Class prefix <input v-model="settings.classPrefix" spellcheck="false" /></label>
            <label>Font URL prefix <input v-model="settings.cssFontsUrl" placeholder="Relative to the CSS file" spellcheck="false" /></label>
            <label class="pg-toggle"><input v-model="settings.css" type="checkbox" /> Include CSS in downloads</label>
            <label class="pg-toggle"><input v-model="settings.html" type="checkbox" /> Include HTML preview in downloads</label>
            <label class="pg-toggle"><input v-model="settings.ts" type="checkbox" /> Include TypeScript metadata in downloads</label>
            <label>
                Extra template data (JSON)
                <textarea v-model="settings.templateData" rows="3" spellcheck="false" placeholder='{ "brand": "acme" }' />
            </label>
        </details>

        <details>
            <summary>Codepoints and ligatures</summary>
            <label class="pg-toggle"><input v-model="settings.ligature" type="checkbox" /> Ligatures</label>
            <label>
                Start codepoint
                <input v-model="startText" spellcheck="false" :aria-invalid="Boolean(startError)" @change="setStart" />
            </label>
            <small v-if="startError" class="pg-options-error">{{ startError }}</small>
            <p class="pg-hint">New icons get the next free codepoint from here. Edit any icon's codepoint on its card.</p>
            <button type="button" class="pg-button" @click="emit('reassign')">Reassign all from the start codepoint</button>
        </details>

        <details>
            <summary>Color glyphs</summary>
            <div class="pg-checks">
                <label><input v-model="settings.colorMode" type="radio" value="off" /> Off</label>
                <label><input v-model="settings.colorMode" type="radio" value="all" /> All icons</label>
                <label><input v-model="settings.colorMode" type="radio" value="selected" /> Selected</label>
            </div>
            <div v-if="settings.colorMode === 'selected'" class="pg-checks pg-color-names">
                <label v-for="name in props.iconNames" :key="name">
                    <input type="checkbox" :checked="settings.colorNames.includes(name)" @change="toggle(settings.colorNames, name, ($event.target as HTMLInputElement).checked)" />
                    {{ name }}
                </label>
            </div>
            <p class="pg-hint">Keeps solid SVG colors. Some platforms show the single-color version instead.</p>
        </details>

        <details>
            <summary>Templates</summary>
            <template v-for="key in ['cssTemplate', 'htmlTemplate'] as const" :key="key">
                <div class="pg-template">
                    <span>{{ key === 'cssTemplate' ? 'CSS' : 'HTML' }} template</span>
                    <button v-if="settings[key] === undefined" type="button" class="pg-link" @click="customize(key)">Customize</button>
                    <button v-else type="button" class="pg-link" @click="resetTemplate(key)">Use built-in</button>
                </div>
                <textarea v-if="settings[key] !== undefined" v-model="settings[key]" rows="10" spellcheck="false" class="pg-code" />
            </template>
        </details>

        <details>
            <summary>Font metadata</summary>
            <label>SVG font ID <input v-model="settings.svgFontId" placeholder="Font name" /></label>
            <label>SVG metadata <input v-model="settings.svgMetadata" /></label>
            <label>TTF copyright <input v-model="settings.ttfCopyright" /></label>
            <label>TTF description <input v-model="settings.ttfDescription" /></label>
            <label>TTF URL <input v-model="settings.ttfUrl" /></label>
            <label>TTF version <input v-model="settings.ttfVersion" placeholder="1.0" /></label>
            <label>
                TTF timestamp (Unix seconds)
                <input type="number" :value="numberValue('ttfTimestamp')" placeholder="Now" @change="setNumber('ttfTimestamp', ($event.target as HTMLInputElement).value)" />
            </label>
            <label>WOFF metadata <input v-model="settings.woffMetadata" /></label>
        </details>
    </div>
</template>

<style scoped>
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

.pg-options-error {
    font-size: 12px;
    text-align: center;
    color: var(--vp-c-danger-1);
}
</style>
