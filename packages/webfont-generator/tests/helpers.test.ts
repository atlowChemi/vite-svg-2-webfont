import { mkdtemp, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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
    result.add(third);
    result.add(second);
    expect(result.ttf).toEqual((await generateWebfonts({ ...f.options, files: [first, third, second] })).ttf);
    const before = result.ttf;
    expect(() => result.add(first)).toThrow('already exists');
    expect(() => result.update('unknown')).toThrow('Unknown');
    expect(() => result.remove('unknown')).toThrow('Unknown');
    expect(() => result.add('')).toThrow('empty');
    await writeFile(second, '<broken');
    expect(() => result.update(second)).toThrow(/SVG|document/);
    expect(result.ttf).toEqual(before);
    await writeFile(second, svg(21));
    result.update(second, 'renamed');
    expect(result.generateCss()).toContain('renamed');
    result.remove(third);
    expect(await readFile(third, 'utf8')).toBe(svg(12));
    result.regenerate({ files: [second, first] });
    result.remove(first);
    expect(() => result.remove(second)).toThrow('at least one');
    expect(result.generateCss()).toContain('renamed');
    const ordinary = await generateWebfonts({ ...f.options, incremental: false });
    expect(() => ordinary.update(first)).toThrow('incremental');
    expect(() => result.add(third, undefined, 'bold')).toThrow('Ordinary');
});

it('async helpers return snapshots, reject overlaps and retain a usable receiver on failure', async () => {
    await using f = await fixture();
    const [first, second] = f.paths as [string, string];
    const original = await generateWebfonts(f.options);
    const saved = original.ttf;
    const added = await original.addAsync(second);
    expect(original.ttf).toEqual(saved);
    await expect(original.updateAsync(first)).rejects.toThrow(/regenerating|replaced/);
    await writeFile(second, '<broken');
    await expect(added.updateAsync(second)).rejects.toThrow(/SVG|document/);
    await writeFile(second, svg(23));
    const outcomes = await Promise.allSettled([added.updateAsync(second), added.updateAsync(second)]);
    expect(outcomes.filter(outcome => outcome.status === 'rejected')).toHaveLength(1);
    const next = outcomes.find(outcome => outcome.status === 'fulfilled')!;
    if (next.status !== 'fulfilled') throw new Error('Expected a successful update');
    const result = await next.value.removeAsync(first);
    expect(result.ttf).toEqual((await generateWebfonts({ ...f.options, files: [second] })).ttf);
    await expect(result.removeAsync(second)).rejects.toThrow('at least one');
    await expect(result.addAsync(second)).rejects.toThrow('already exists');
    await expect(result.updateAsync('unknown')).rejects.toThrow('Unknown');
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
    expect(() => result.add(second)).toThrow('target variant');
    expect(() => result.add(second, undefined, 'unknown')).toThrow('Unknown target');
    result.add(second, undefined, 'light');
    const added = await result.addAsync(third, undefined, 'bold');
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
    expect(() => removed.remove(second)).toThrow('at least one');
});
