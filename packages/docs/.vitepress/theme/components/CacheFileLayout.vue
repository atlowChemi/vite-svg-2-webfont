<script setup lang="ts">
// The persistent cache file's layout, top to bottom, as the engine writes it (crates/webfont-generator/src/cache).
type Field = { name: string; size: string };
type Block = { title: string; note: string; fields: Field[]; repeat?: { label: string; fields: Field[] } };

const blocks: Block[] = [
    {
        title: 'Header',
        note: 'Any mismatch and the whole file is ignored.',
        fields: [
            { name: 'Magic: WFGCACHE', size: '8 bytes' },
            { name: 'Format version', size: '4 bytes' },
            { name: 'Engine version', size: 'text' },
            { name: 'Settings fingerprint', size: '16 bytes' },
        ],
    },
    {
        title: 'Per-icon work',
        note: 'Read only when an icon or format must be rebuilt.',
        fields: [
            { name: 'CRC32', size: '4 bytes' },
            { name: 'Length', size: '4 bytes' },
        ],
        repeat: {
            label: 'For each SVG, by content hash',
            fields: [
                { name: 'Parsed outlines and color layers', size: 'paths' },
                { name: 'Processed glyph', size: 'paths' },
            ],
        },
    },
    {
        title: 'Finished fonts',
        note: 'Checked on every load. Up to two entries per format.',
        fields: [
            { name: 'CRC32', size: '4 bytes' },
            { name: 'Length', size: '4 bytes' },
        ],
        repeat: {
            label: 'For each font',
            fields: [
                { name: 'Format', size: '1 byte' },
                { name: 'Key: format options and icons', size: '16 bytes' },
                { name: 'Icon set: names, codepoints, contents', size: '16 bytes' },
                { name: 'Font bytes', size: 'bytes' },
            ],
        },
    },
];
</script>

<template>
    <figure class="cache-layout" aria-label="Layout of a .wfcache file">
        <div v-for="block in blocks" :key="block.title" class="block">
            <div class="head">
                <span class="title">{{ block.title }}</span>
                <span class="note">{{ block.note }}</span>
            </div>
            <div class="fields">
                <div v-for="field in block.fields" :key="field.name" class="field">
                    <span>{{ field.name }}</span
                    ><code>{{ field.size }}</code>
                </div>
                <div v-if="block.repeat" class="repeat">
                    <span class="label">{{ block.repeat.label }}</span>
                    <div v-for="field in block.repeat.fields" :key="field.name" class="field">
                        <span>{{ field.name }}</span
                        ><code>{{ field.size }}</code>
                    </div>
                </div>
            </div>
        </div>
        <figcaption><code>icons.3f2a9c41d0b7e8a5.wfcache</code>, little-endian, top to bottom</figcaption>
    </figure>
</template>

<style scoped>
.cache-layout {
    margin: 24px 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.block {
    border: 1px solid var(--vp-c-divider);
    border-left: 4px solid var(--fonte-mold);
    border-radius: 8px;
    background: var(--vp-c-bg-soft);
    padding: 12px 16px;
}

.block:nth-child(2) {
    border-left-color: var(--fonte-ember);
}

.block:nth-child(3) {
    border-left-color: var(--fonte-molten);
}

.head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 4px 16px;
    margin-bottom: 8px;
}

.title {
    font-family: var(--fonte-font-display);
    font-weight: 600;
    font-size: 17px;
}

.note,
.label {
    font-size: 13px;
    color: var(--vp-c-text-2);
}

.fields {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.field {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 4px 10px;
    border-radius: 4px;
    background: var(--vp-c-bg);
    font-size: 14px;
}

.field code {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--vp-c-text-3);
    background: none;
    padding: 0;
}

.repeat {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 4px;
    padding: 8px;
    border: 1px dashed var(--vp-c-border);
    border-radius: 6px;
}

figcaption {
    font-size: 13px;
    color: var(--vp-c-text-2);
    text-align: center;
}
</style>
