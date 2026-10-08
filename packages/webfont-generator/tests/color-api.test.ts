import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, expect, it } from 'vite-plus/test';
import { FontType, generateWebfonts as native } from '../binding.js';
import { generateWebfonts } from '../index.js';

const roots: string[] = [];
afterEach(async () => {
    await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});
const svg = (color: string) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path fill="${color}" d="M10 10H90V90H10Z"/></svg>`;

async function fixture() {
    const dest = await mkdtemp(join(tmpdir(), 'color-api-'));
    roots.push(dest);
    const path = join(dest, 'icon.svg');
    await writeFile(path, svg('red'));
    return { dest, files: [path], types: ['ttf', 'woff', 'woff2'] as const, writeFiles: false };
}

it('selects names after rename through the wrapper and raw binding', async () => {
    const input = await fixture();
    const options = { ...input, types: [...input.types], colorGlyphs: ['renamed'] };
    const result = await generateWebfonts({ ...options, rename: () => 'renamed' });
    expect(Buffer.from(result.ttf!).includes(Buffer.from('COLR'))).toBe(true);
    const rawOptions = { ...options, types: [FontType.Ttf] };
    const raw = await native(rawOptions, paths => paths.map(() => 'renamed'));
    expect(Buffer.from(raw.ttf!).includes(Buffer.from('COLR'))).toBe(true);
    await Promise.all(
        [
            () => generateWebfonts({ ...options, colorGlyphs: ['icon'], rename: () => 'renamed' }),
            () => native({ ...rawOptions, colorGlyphs: ['icon'] }, paths => paths.map(() => 'renamed')),
        ].map(async generate => {
            await expect(generate()).rejects.toThrow('unknown glyph names: icon');
        }),
    );
});

it.each([null, false, 'icon', ['icon', 1], {}, 1].map(colorGlyphs => ({ colorGlyphs })))(
    'rejects invalid color selection $colorGlyphs through both APIs',
    async ({ colorGlyphs }) => {
        const input = await fixture();
        await Promise.all(
            [generateWebfonts, native].map(async generate => {
                await expect(async () => generate({ ...input, colorGlyphs } as never)).rejects.toThrow(/colorGlyphs/);
            }),
        );
    },
);

it('ordinary paint-only regeneration changes fonts and CSS hashes and preserves rollback', async () => {
    const input = await fixture();
    const options = { ...input, types: [...input.types], colorGlyphs: true as const, incremental: true };
    const result = await generateWebfonts(options);
    const before = result.ttf;
    const css = result.generateCss();
    await writeFile(input.files[0], svg('blue'));
    result.regenerate({ files: input.files }, [{ path: input.files[0], changeType: 'changed' }]);
    const fresh = await generateWebfonts(options);
    expect(result.ttf).not.toEqual(before);
    expect(result.generateCss()).not.toEqual(css);
    expect(result.ttf).toEqual(fresh.ttf);
    expect(result.woff).toEqual(fresh.woff);
    expect(result.woff2).toEqual(fresh.woff2);
    await writeFile(input.files[0], 'invalid SVG');
    await expect(result.regenerateAsync({ files: input.files })).rejects.toThrow(/SVG|document/);
    expect(result.ttf).toEqual(fresh.ttf);
    await writeFile(input.files[0], svg('green'));
    const next = await result.regenerateAsync({ files: input.files });
    expect(next.ttf).toEqual((await generateWebfonts(options)).ttf);
});

it('empty selection preserves bytes and active selection rejects ordinary default EOT', async () => {
    const input = await fixture();
    const options = { ...input, types: [...input.types] };
    const mono = await generateWebfonts(options);
    const empty = await generateWebfonts({ ...options, colorGlyphs: [] });
    expect(empty.ttf).toEqual(mono.ttf);
    await Promise.all(
        [generateWebfonts, native].map(async generate => {
            await expect(generate({ ...options, types: undefined, colorGlyphs: true })).rejects.toThrow('options.colorGlyphs: incompatible output formats: eot');
        }),
    );
});

it('rejects incompatible formats before file reads or rename callbacks in both APIs', async () => {
    const input = await fixture();
    let renameCalls = 0;
    const rename = () => {
        renameCalls++;
        return 'icon';
    };
    const options = { ...input, types: undefined, colorGlyphs: true as const };
    await Promise.all(
        [
            () => generateWebfonts({ ...options, rename }),
            () => native(options, paths => paths.map(rename)),
            () => generateWebfonts({ ...options, files: [join(input.dest, 'missing.svg')], rename }),
            () => native({ ...options, files: [join(input.dest, 'missing.svg')] }, paths => paths.map(rename)),
        ].map(async generate => {
            await expect(generate()).rejects.toThrow('options.colorGlyphs: incompatible output formats: eot');
        }),
    );
    expect(renameCalls).toBe(0);
});

it('raw NAPI selects a post-rename name found only in a non-default variant', async () => {
    const input = await fixture();
    const other = join(input.dest, 'other.svg');
    await writeFile(other, svg('red'));
    const options = {
        dest: input.dest,
        files: [],
        types: [FontType.Ttf],
        writeFiles: false,
        formatOptions: { ttf: { ts: 0 } },
        colorGlyphs: ['selected'],
        variants: [
            { name: 'light', files: [other], weight: 300, default: true },
            { name: 'bold', files: input.files, weight: 700 },
        ],
    };
    const rename = (paths: string[]) => paths.map(path => (path === other ? 'other' : 'selected'));
    const initial = await native(options, rename);
    expect(Buffer.from(initial.ttf!).includes(Buffer.from('COLR'))).toBe(true);
    // An unselected glyph's paint does not affect font bytes.
    await writeFile(other, svg('green'));
    expect((await native(options, rename)).ttf).toEqual(initial.ttf);
    await writeFile(input.files[0], svg('blue'));
    expect((await native(options, rename)).ttf).not.toEqual(initial.ttf);
    await expect(native({ ...options, colorGlyphs: ['z', 'a', 'z'] }, rename)).rejects.toThrow('unknown glyph names: z, a');
});

it('Node variant regeneration propagates fixed paint to fallback consumers', async () => {
    const input = await fixture();
    const other = join(input.dest, 'other.svg');
    await writeFile(other, svg('green'));
    const options = {
        dest: input.dest,
        types: ['ttf', 'woff', 'woff2'] as ['ttf', 'woff', 'woff2'],
        writeFiles: false,
        incremental: true,
        colorGlyphs: ['icon'],
        missingGlyphs: { behavior: 'fallback' as const, variant: 'light' },
        variants: [
            { name: 'light', files: [...input.files, other], weight: 300, default: true },
            { name: 'bold', files: [other], weight: 700 },
        ],
    };
    const files = { variants: options.variants.map(v => ({ variant: v.name, files: v.files })) };
    const initial = await generateWebfonts(options);
    const oldFont = initial.ttf;
    const oldCss = initial.generateCss();
    await writeFile(input.files[0], svg('blue'));
    const next = await initial.regenerateAsync(files, [{ path: input.files[0], changeType: 'changed' }]);
    const fresh = await generateWebfonts(options);
    expect(initial.ttf).toEqual(oldFont);
    expect(next.ttf).not.toEqual(oldFont);
    expect(next.generateCss()).not.toEqual(oldCss);
    expect(next.ttf).toEqual(fresh.ttf);
    expect(next.woff).toEqual(fresh.woff);
    expect(next.woff2).toEqual(fresh.woff2);
    // A fallback consumer must match an explicit copy of the updated source.
    const explicit = await generateWebfonts({
        ...options,
        variants: options.variants.map(v => ({ name: v.name, weight: v.weight, default: v.default, files: [...input.files, other] })),
    });
    expect(next.ttf).toEqual(explicit.ttf);
    expect((await next.regenerateAsync(files)).ttf).toEqual(next.ttf);
});
