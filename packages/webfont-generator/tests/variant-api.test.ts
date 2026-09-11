import { expect, it } from 'vite-plus/test';
import { generateWebfonts as native } from '../binding.js';
import { generateWebfonts } from '../index.js';

const variants = [
    { name: 'light', files: ['light.svg'], weight: 300, default: true },
    { name: 'bold', files: ['bold.svg'], weight: 700 },
];

async function message(generate: () => Promise<unknown>) {
    try {
        await generate();
    } catch (error) {
        if (error instanceof Error) return error.message;
        throw error;
    }
    throw new Error('Expected generation to reject invalid options');
}

it.each([
    { name: 'mixed sources', patch: { files: ['icon.svg'] }, field: 'options.files' },
    { name: 'variant names', patch: { variants: [{ ...variants[0], name: 'bad name' }, variants[1]] }, field: 'options.variants[0].name' },
    { name: 'variant weights', patch: { variants: [{ ...variants[0], weight: 1001 }, variants[1]] }, field: 'options.variants[0].weight' },
    { name: 'missing default', patch: { variants: variants.map(variant => ({ ...variant, default: false })) }, field: 'options.variants' },
    { name: 'unsupported formats', patch: { types: ['eot'] }, field: 'options.types' },
    { name: 'incremental mode', patch: { incremental: true }, field: 'options.incremental' },
    { name: 'fallback reference', patch: { missingGlyphs: { behavior: 'fallback', variant: 'unknown' } }, field: 'options.missingGlyphs.variant' },
])('preserves Node/native error parity for $name', async ({ patch, field }) => {
    const options = { dest: 'artifacts', files: [], variants, writeFiles: false, ...patch };
    const publicMessage = await message(() => generateWebfonts(options as never));
    const nativeMessage = await message(() => native(options as never));
    expect(publicMessage).toContain(field);
    expect(publicMessage).toBe(nativeMessage);
});
