import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vite-plus/test';
import { FontType, generateWebfonts } from '../binding.js';

it('keeps the browser proof fixture reproducible through the NAPI bridge', async () => {
    const dest = await mkdtemp(join(tmpdir(), 'browser-proof-'));
    try {
        const files = [join(dest, 'light.svg'), join(dest, 'bold.svg')];
        // SVG's Y axis is inverted when constructing the font outline.
        await Promise.all(
            [100, 700].map((x, index) =>
                writeFile(files[index], `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000"><path d="M${x} 900H${x + 200}V100H${x}Z"/></svg>`),
            ),
        );
        // Use the binding directly: the JS compatibility wrapper forces its legacy timestamp.
        const result = await generateWebfonts(
            {
                dest,
                files: [],
                fontName: 'Discrete rvrn proof',
                fontHeight: 1000,
                descent: 0,
                normalize: false,
                ligature: true,
                types: [FontType.Woff2],
                writeFiles: false,
                codepoints: { ab: 0xe001 },
                formatOptions: { ttf: { ts: 0 }, woff2: { compressionQuality: 11 } },
                variants: [
                    { name: 'Light', files: [files[0]], weight: 300, default: true },
                    { name: 'Bold', files: [files[1]], weight: 700 },
                ],
            },
            paths => paths.map(() => 'ab'),
        );
        const path = join(import.meta.dirname, 'browser/fixtures/discrete-rvrn.woff2');
        const font = result.woff2;
        if (!font) throw new Error('Expected a generated WOFF2 proof font');
        const actual = Buffer.from(font);
        if (process.env.UPDATE_VARIABLE_PROOF_FIXTURE && process.env.UPDATE_VARIABLE_PROOF_FIXTURE !== '0') await writeFile(path, actual);
        expect(actual.equals(await readFile(path)), 'Inspect changes before accepting with UPDATE_VARIABLE_PROOF_FIXTURE=1').toBe(true);
    } finally {
        await rm(dest, { recursive: true, force: true });
    }
});
