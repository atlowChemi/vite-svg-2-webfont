import { mkdtemp, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { expect, it } from 'vite-plus/test';
import { generateWebfonts } from '../index.js';

const svg = (width: number) => `<svg viewBox="0 0 24 24"><path d="M0 0H${width}V24H0Z"/></svg>`;

async function fixture() {
    const root = await mkdtemp(join(tmpdir(), 'single-actions-'));
    const paths = ['first', 'second', 'third'].map(name => join(root, `${name}.svg`));
    await Promise.all(paths.map((path, i) => writeFile(path, svg(10 + i))));
    return {
        paths,
        options: { files: [paths[0]], dest: root, incremental: true, writeFiles: false, types: ['ttf'] as const, formatOptions: { ttf: { ts: 0 } } },
        [Symbol.asyncDispose]: () => rm(root, { recursive: true, force: true }),
    };
}

it('single file helpers preserve order, recover after errors and interoperate with regenerate', async () => {
    await using f = await fixture();
    const [first, second, third] = f.paths as [string, string, string];
    const result = await generateWebfonts(f.options);
    const expectFresh = async (files: string[], renamed = false) => {
        const fresh = await generateWebfonts({ ...f.options, files, rename: path => (renamed && path === second ? 'renamed' : basename(path, '.svg')) });
        expect(result.ttf).toEqual(fresh.ttf);
        expect(result.generateCss()).toBe(fresh.generateCss());
    };
    result.add(third);
    await expectFresh([first, third]);
    result.add(second);
    await expectFresh([first, third, second]);
    const before = result.ttf;
    expect(() => result.add(first)).toThrow('Source ID/path already exists');
    expect(result.ttf).toEqual(before);
    expect(() => result.update('unknown')).toThrow('Unknown source ID/path');
    expect(result.ttf).toEqual(before);
    expect(() => result.remove('unknown')).toThrow('Unknown source ID/path');
    expect(result.ttf).toEqual(before);
    expect(() => result.add('')).toThrow('Source ID/path must not be empty');
    expect(result.ttf).toEqual(before);
    await writeFile(second, '<broken');
    expect(() => result.update(second)).toThrow('Failed to inspect SVG root element: the document does not have a root node');
    expect(result.ttf).toEqual(before);
    await writeFile(second, svg(21));
    result.update(second, 'renamed');
    await expectFresh([first, third, second], true);
    result.remove(third);
    await expectFresh([first, second], true);
    expect(await readFile(third, 'utf8')).toBe(svg(12));
    result.regenerate({ files: [second, first] });
    await expectFresh([second, first], true);
    result.remove(first);
    await expectFresh([second], true);
    const remaining = result.ttf;
    expect(() => result.remove(second)).toThrow('Each design must contain at least one source');
    expect(result.ttf).toEqual(remaining);
    const ordinary = await generateWebfonts({ ...f.options, incremental: false });
    const ordinaryBefore = ordinary.ttf;
    expect(() => ordinary.update(first)).toThrow('regenerate requires the font to be generated with `incremental` enabled.');
    expect(ordinary.ttf).toEqual(ordinaryBefore);
    expect(() => result.add(third, undefined, 'bold')).toThrow('Ordinary fonts do not accept a target variant');
    expect(result.ttf).toEqual(remaining);
});

it('async helpers return snapshots, reject overlaps and retain a usable receiver on failure', async () => {
    await using f = await fixture();
    const [first, second] = f.paths as [string, string];
    const original = await generateWebfonts(f.options);
    const saved = original.ttf;
    const added = await original.addAsync(second);
    expect(added.ttf).toEqual((await generateWebfonts({ ...f.options, files: [first, second] })).ttf);
    const addedBefore = added.ttf;
    expect(original.ttf).toEqual(saved);
    await expect(original.updateAsync(first)).rejects.toThrow('This result is already regenerating or has been replaced.');
    expect(original.ttf).toEqual(saved);
    await writeFile(second, '<broken');
    await expect(added.updateAsync(second)).rejects.toThrow('Failed to inspect SVG root element: the document does not have a root node');
    expect(added.ttf).toEqual(addedBefore);
    await writeFile(second, svg(23));
    const outcomes = await Promise.allSettled([added.updateAsync(second), added.updateAsync(second)]);
    expect(outcomes.filter(outcome => outcome.status === 'rejected')).toHaveLength(1);
    const rejected = outcomes.find(outcome => outcome.status === 'rejected')!;
    if (rejected.status !== 'rejected') throw new Error('Expected an overlap rejection');
    expect(rejected.reason).toHaveProperty('message', 'This result is already regenerating or has been replaced.');
    expect(added.ttf).toEqual(addedBefore);
    const next = outcomes.find(outcome => outcome.status === 'fulfilled')!;
    if (next.status !== 'fulfilled') throw new Error('Expected a successful update');
    expect(next.value.ttf).toEqual((await generateWebfonts({ ...f.options, files: [first, second] })).ttf);
    const result = await next.value.removeAsync(first);
    expect(result.ttf).toEqual((await generateWebfonts({ ...f.options, files: [second] })).ttf);
    const remaining = result.ttf;
    await expect(result.removeAsync(second)).rejects.toThrow('Each design must contain at least one source');
    expect(result.ttf).toEqual(remaining);
    await expect(result.addAsync(second)).rejects.toThrow('Source ID/path already exists');
    expect(result.ttf).toEqual(remaining);
    await expect(result.updateAsync('unknown')).rejects.toThrow('Unknown source ID/path');
    expect(result.ttf).toEqual(remaining);
});

it('variant helpers require a target for additions and update/remove shared paths across designs', async () => {
    await using f = await fixture();
    const [first, second, third] = f.paths as [string, string, string];
    const { files: _, ...base } = f.options;
    const variants = [
        { name: 'light', default: true, files: [first] },
        { name: 'bold', files: [first] },
    ];
    const options = { ...base, variants, missingGlyphs: { behavior: 'blank' as const } };
    const result = await generateWebfonts(options);
    const before = result.ttf;
    expect(() => result.add(second)).toThrow('Adding to a family requires a target variant');
    expect(result.ttf).toEqual(before);
    expect(() => result.add(second, undefined, 'unknown')).toThrow('Unknown target variant');
    expect(result.ttf).toEqual(before);
    result.add(second, undefined, 'light');
    expect(result.ttf).toEqual((await generateWebfonts({ ...options, variants: [{ ...variants[0], files: [first, second] }, variants[1]] })).ttf);
    const added = await result.addAsync(third, undefined, 'bold');
    expect(added.ttf).toEqual(
        (
            await generateWebfonts({
                ...options,
                variants: [
                    { ...variants[0], files: [first, second] },
                    { ...variants[1], files: [first, third] },
                ],
            })
        ).ttf,
    );
    await writeFile(first, svg(22));
    added.update(first);
    const fresh = await generateWebfonts({
        ...options,
        variants: [
            { ...variants[0], files: [first, second] },
            { ...variants[1], files: [first, third] },
        ],
    });
    expect(added.ttf).toEqual(fresh.ttf);
    const removed = await added.removeAsync(first);
    expect(removed.ttf).toEqual(
        (
            await generateWebfonts({
                ...options,
                variants: [
                    { ...variants[0], files: [second] },
                    { ...variants[1], files: [third] },
                ],
            })
        ).ttf,
    );
    const remaining = removed.ttf;
    expect(() => removed.remove(second)).toThrow('Each design must contain at least one source');
    expect(removed.ttf).toEqual(remaining);
});
