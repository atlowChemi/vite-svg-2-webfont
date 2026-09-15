import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, expect, test } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsVariantOptions } from '../index.js';

const roots: string[] = [];
afterEach(async () => {
    await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});
const svg = (width: number) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H${width}V24H0Z"/></svg>`;

async function fixture() {
    const root = await mkdtemp(join(tmpdir(), 'variant-incremental-'));
    roots.push(root);
    const variants = await Promise.all(
        ['light', 'bold'].map(async (name, index) => {
            const dir = join(root, name);
            await mkdir(dir);
            const path = join(dir, 'add.svg');
            await writeFile(path, svg(10 + index));
            return { name, files: [path], default: index === 0 };
        }),
    );
    const options = {
        dest: root,
        variants,
        incremental: true,
        writeFiles: false,
        types: ['ttf', 'woff', 'woff2'] as ['ttf', 'woff', 'woff2'],
        formatOptions: { ttf: { ts: 1_700_000_000 } },
    } satisfies GenerateWebfontsVariantOptions;
    const updates = [{ path: variants[1].files[0], changeType: 'changed' as const }];
    const fileSets = { variants: variants.map(variant => ({ variant: variant.name, files: variant.files })) };
    return { options, variants, updates, fileSets };
}

test('async variant regeneration preserves receiver and format inference; overlapping calls reject', async () => {
    const { options, variants, updates, fileSets } = await fixture();
    const result = await generateWebfonts(options);
    const old = result.ttf;
    await writeFile(variants[1].files[0], svg(18));
    // Native tasks may acquire the regeneration state in either invocation order.
    const outcomes = await Promise.allSettled([result.regenerateAsync(fileSets, updates), result.regenerateAsync(fileSets, updates)]);
    const fulfilled = outcomes.filter(outcome => outcome.status === 'fulfilled');
    expect(fulfilled).toHaveLength(1);
    expect(outcomes.filter(outcome => outcome.status === 'rejected')).toEqual([
        expect.objectContaining({ reason: expect.objectContaining({ message: expect.stringMatching(/regenerating|replaced/) }) }),
    ]);
    const next = fulfilled[0].value;
    expect(result.ttf).toEqual(old);
    expect(next.ttf).not.toEqual(old);
    const fresh = await generateWebfonts({ ...options, incremental: false });
    expect(next.ttf).toEqual(fresh.ttf);
    expect(next.woff).toEqual(fresh.woff);
    expect(next.woff2).toEqual(fresh.woff2);
    const rediff = await next.regenerateAsync(fileSets);
    expect(rediff.woff2).toEqual(next.woff2);
    expect(() => rediff.regenerate({ files: variants[0].files })).toThrow(/Single file lists/);
});

test('failed async variant update can be retried, including via synchronous methods', async () => {
    const { options, variants, updates, fileSets } = await fixture();
    const result = await generateWebfonts(options);
    const old = result.ttf;
    await writeFile(variants[1].files[0], 'invalid SVG');
    await expect(result.regenerateAsync(fileSets, updates)).rejects.toThrow(/SVG|document/);
    expect(result.ttf).toEqual(old);
    await writeFile(variants[1].files[0], svg(20));
    result.regenerate(fileSets, updates);
    result.regenerate(fileSets);
    expect(result.ttf).toEqual((await generateWebfonts({ ...options, incremental: false })).ttf);
});

test('callback results reject sync and async variant regeneration', async () => {
    const { options, updates, fileSets } = await fixture();
    const result = await generateWebfonts({
        ...options,
        cssContext(context) {
            context.extra = true;
        },
    });
    expect(() => result.regenerate(fileSets, updates)).toThrow(/callbacks/);
    await expect(result.regenerateAsync(fileSets, updates)).rejects.toThrow(/callbacks/);
    await expect(result.regenerateAsync(fileSets, updates)).rejects.toThrow(/callbacks/);
});

test('async write failure preserves the receiver and retries the disk output', async () => {
    const { options, variants, updates, fileSets } = await fixture();
    const result = await generateWebfonts({ ...options, writeFiles: true, fontName: 'icons' });
    const old = result.ttf;
    const output = join(options.dest, 'icons.ttf');
    await rm(output);
    await mkdir(output);
    await writeFile(variants[1].files[0], svg(21));
    await expect(result.regenerateAsync(fileSets, updates)).rejects.toThrow(/directory|os error/i);
    expect(result.ttf).toEqual(old);
    await rm(output, { recursive: true });
    const next = await result.regenerateAsync(fileSets, updates);
    expect(next.ttf).not.toEqual(old);
    expect(new Uint8Array(await readFile(output))).toEqual(next.ttf);
});

test('unified inputs reject old arrays and ambiguous modes without consuming the result', async () => {
    const { options, fileSets } = await fixture();
    const result = await generateWebfonts(options);
    const original = result.ttf;
    // @ts-expect-error Regeneration now requires a source object.
    expect(() => result.regenerate([])).toThrow(/exactly one/);
    // @ts-expect-error A source object cannot select both modes.
    await expect(result.regenerateAsync({ files: [], variants: fileSets.variants })).rejects.toThrow(/exactly one/);
    // @ts-expect-error A source object must select a mode.
    expect(() => result.regenerate({})).toThrow(/exactly one/);
    await expect(result.regenerateAsync({ variants: fileSets.variants.slice(0, 1) })).rejects.toThrow(/every configured variant/);
    expect(result.ttf).toEqual(original);
    const replacement = await result.regenerateAsync(fileSets);
    expect(replacement.ttf).toEqual(original);
    expect('regenerateVariants' in replacement).toBe(false);
});

test('failed async membership update restores old sources and reconciles partial writes on a no-op retry', async () => {
    const { options, fileSets } = await fixture();
    const result = await generateWebfonts({ ...options, writeFiles: true, fontName: 'icons', missingGlyphs: { behavior: 'blank' } });
    const old = result.ttf;
    const added = join(options.dest, 'extra.svg');
    await writeFile(added, svg(16));
    const changedSets = { variants: fileSets.variants.map(set => ({ ...set, files: set.variant === 'bold' ? [...set.files, added] : set.files })) };
    const blocked = join(options.dest, 'icons.woff2');
    await rm(blocked);
    await mkdir(blocked);
    await expect(result.regenerateAsync(changedSets)).rejects.toThrow(/directory|os error/i);
    expect(result.ttf).toEqual(old);
    await rm(blocked, { recursive: true });
    const restored = await result.regenerateAsync(fileSets, []);
    expect(restored.ttf).toEqual(old);
    expect(new Uint8Array(await readFile(join(options.dest, 'icons.ttf')))).toEqual(old);
    const next = await restored.regenerateAsync(changedSets, [{ path: added, changeType: 'added' }]);
    const fresh = await generateWebfonts({
        ...options,
        fontName: 'icons',
        missingGlyphs: { behavior: 'blank' },
        variants: options.variants.map(v => ({ name: v.name, default: v.default, files: changedSets.variants.find(set => set.variant === v.name)!.files })),
    });
    expect(next.ttf).toEqual(fresh.ttf);
    expect(next.woff2).toEqual(fresh.woff2);
});

test('duplicate glyph names within a design reject without consuming the result', async () => {
    const { options, fileSets } = await fixture();
    const result = await generateWebfonts({ ...options, missingGlyphs: { behavior: 'blank' } });
    const original = result.ttf;
    const added = join(options.dest, 'extra.svg');
    await writeFile(added, svg(16));
    const changedSets = {
        variants: fileSets.variants.map(set => ({ variant: set.variant, files: set.variant === 'bold' ? [...set.files, added] : set.files })),
    };
    const duplicate = [{ path: added, changeType: 'added' as const, name: 'add' }];
    expect(() => result.regenerate(changedSets, duplicate)).toThrow('The glyph name "add" must be unique.');
    await expect(result.regenerateAsync(changedSets, duplicate)).rejects.toThrow('The glyph name "add" must be unique.');
    const sameStem = join(options.dest, 'add.svg');
    await writeFile(sameStem, svg(17));
    const inferredDuplicate = {
        variants: fileSets.variants.map(set => ({ variant: set.variant, files: set.variant === 'bold' ? [...set.files, sameStem] : set.files })),
    };
    expect(() => result.regenerate(inferredDuplicate)).toThrow('The glyph name "add" must be unique.');
    await expect(result.regenerateAsync(inferredDuplicate)).rejects.toThrow('The glyph name "add" must be unique.');
    expect(result.ttf).toEqual(original);
    const next = await result.regenerateAsync(changedSets, [{ path: added, changeType: 'added' }]);
    const fresh = await generateWebfonts({
        ...options,
        missingGlyphs: { behavior: 'blank' },
        variants: changedSets.variants.map(set => ({ name: set.variant, files: set.files, default: set.variant === 'light' })),
    });
    expect(next.ttf).toEqual(fresh.ttf);
    expect(next.woff2).toEqual(fresh.woff2);
});
