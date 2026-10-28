import { afterEach, expect, it } from 'vite-plus/test';
import { generateWebfonts, type GenerateWebfontsOptions, type WebfontResult } from '../src';

const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#f00" d="M2 2H22V22H2Z"/></svg>';
const changed = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#00f" d="M1 1L23 23H1Z"/></svg>';
const source = { id: 'opaque:42', name: 'square', svg };
const results: WebfontResult[] = [];
async function generate(options: GenerateWebfontsOptions = {}): Promise<WebfontResult> {
    const result = await generateWebfonts({ sources: [source], types: ['ttf', 'woff', 'woff2'], formatOptions: { ttf: { ts: 0 }, woff2: { compressionQuality: 5 } }, ...options });
    results.push(result);
    return result;
}
afterEach(() => {
    for (const result of results.splice(0)) result.dispose();
});

function table(bytes: Uint8Array<ArrayBuffer>, tag: string): number {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    for (let i = 0; i < view.getUint16(4); i++) {
        const offset = 12 + i * 16;
        if (new TextDecoder().decode(bytes.subarray(offset, offset + 4)) === tag) return view.getUint32(offset + 8);
    }
    throw new Error(`Missing table ${tag}`);
}

it('generates copied fonts in a worker and browsers load all modern containers', async () => {
    const result = await generate({ types: ['svg', 'ttf', 'woff', 'woff2', 'eot'], html: true });
    const outputs = await result.collectOutputs();
    expect(Object.keys(outputs).toSorted()).toEqual(['css', 'eot', 'html', 'svg', 'ttf', 'woff', 'woff2']);
    expect(outputs.woff2).toEqual(await result.font('woff2'));
    expect(new TextDecoder().decode(outputs.css)).toBe(await result.generateCss());
    await Promise.all(
        (['ttf', 'woff', 'woff2'] as const).map(async format => {
            const bytes = await result.font(format);
            expect(bytes).toBeInstanceOf(Uint8Array);
            const face = new FontFace(`ordinary-${format}`, bytes!);
            await face.load();
            expect(face.status).toBe('loaded');
        }),
    );
    const original = await result.font('ttf');
    original!.fill(0);
    expect(await result.font('ttf')).not.toEqual(original);
    expect(await result.generateCss()).toContain('@font-face');
    expect(await result.generateHtml()).toContain('square');
    expect(Object.keys(result).toSorted()).toEqual([
        'add',
        'collectOutputs',
        'dispose',
        'exportCache',
        'font',
        'generateCss',
        'generateHtml',
        'generateTs',
        'regenerate',
        'remove',
        'update',
    ]);
    result.dispose();
    expect(Object.values(outputs).every(bytes => bytes.byteLength > 0)).toBe(true);
    await expect(result.font('ttf')).rejects.toThrow('disposed');
    result.dispose();
});

it('renders fixed TS independently of templates and refreshes metadata after every source action', async () => {
    const settings: GenerateWebfontsOptions = {
        types: ['ttf'],
        css: false,
        html: false,
        ts: true,
        incremental: true,
        cssTemplate: '{{#if',
        htmlTemplate: '{{#if',
        templateOptions: { baseSelector: '.custom', classPrefix: 'custom-' },
    };
    const result = await generate(settings);
    const initial = await result.generateTs();
    expect(initial).toContain('export const iconBaseSelector = ".custom" as const;');
    expect(initial).toContain('export const iconClassPrefix = "custom-" as const;');
    expect(initial).toContain('export const icons = [\n  "square"\n] as const;');
    expect(initial).toContain('export type IconName = (typeof icons)[number];');
    const collected = (await result.collectOutputs()).ts!;
    expect(new TextDecoder().decode(collected)).toBe(initial);
    const second = { id: 'second', name: 'second', svg };
    await result.add(second);
    expect(await result.generateTs()).toBe(await (await generate({ ...settings, sources: [source, second] })).generateTs());
    await result.update(second.id, changed, 'renamed');
    expect(await result.generateTs()).toContain('[\n  "square",\n  "renamed"\n]');
    await result.remove(source.id);
    expect(await result.generateTs()).toContain('[\n  "renamed"\n]');
    await result.regenerate({ sources: [second.id] }, [{ type: 'changed', id: second.id, name: 'batch', svg }]);
    expect(await result.generateTs()).toContain('[\n  "batch"\n]');
    expect(new TextDecoder().decode((await result.collectOutputs()).ts)).toBe(await result.generateTs());
    expect(new TextDecoder().decode(collected)).toBe(initial);
    const disabled = await generate({ ...settings, ts: false, templateOptions: { classPrefix: 'other-' } });
    expect((await disabled.collectOutputs()).ts).toBeUndefined();
    expect(await disabled.generateTs()).toContain('"other-"');
    const family = await generate({
        ...settings,
        sources: [],
        variants: [
            { name: 'light', default: true, sources: [source] },
            { name: 'bold', sources: [source] },
        ],
    });
    expect(await family.generateTs()).toBe(initial);
});

it('captures the browser clock when a timestamp is omitted and retains it across updates', async () => {
    const before = Math.floor(Date.now() / 1000);
    const result = await generate({ formatOptions: { woff2: { compressionQuality: 5 } }, incremental: true });
    const bytes = (await result.font('ttf'))!;
    const head = table(bytes, 'head');
    const created = Number(new DataView(bytes.buffer).getBigUint64(head + 20)) - 2082844800;
    expect(created).toBeGreaterThanOrEqual(before);
    expect(created).toBeLessThanOrEqual(Math.floor(Date.now() / 1000));
    await result.regenerate({ sources: [source.id] }, [{ type: 'changed', id: source.id, svg: changed }]);
    const updated = (await result.font('ttf'))!;
    expect(new DataView(updated.buffer).getBigUint64(table(updated, 'head') + 20)).toBe(new DataView(bytes.buffer).getBigUint64(head + 20));
});

it('serializes single actions using retained membership and rolls back rejected actions', async () => {
    const result = await generate({ incremental: true });
    const second = { id: 'second', name: 'second', svg };
    const third = { id: 'third', name: 'third', svg };
    const renamed = { ...second, name: 'renamed', svg: changed };
    const [, afterAdd, , afterSecondAdd, , afterUpdate, , afterRemove] = await Promise.all([
        result.add(second),
        result.collectOutputs(),
        result.add(third),
        result.collectOutputs(),
        result.update(second.id, changed, 'renamed'),
        result.collectOutputs(),
        result.remove(source.id),
        result.collectOutputs(),
    ]);
    expect(afterAdd).toEqual(await (await generate({ sources: [source, second] })).collectOutputs());
    expect(afterSecondAdd).toEqual(await (await generate({ sources: [source, second, third] })).collectOutputs());
    expect(afterUpdate).toEqual(await (await generate({ sources: [source, renamed, third] })).collectOutputs());
    const fresh = await generate({ sources: [{ ...second, name: 'renamed', svg: changed }, third] });
    expect(afterRemove).toEqual(await fresh.collectOutputs());
    const saved = await result.collectOutputs();
    await expect(result.add(second)).rejects.toThrow('Source ID/path already exists');
    expect(await result.collectOutputs()).toEqual(saved);
    await expect(result.update('unknown', svg)).rejects.toThrow('Unknown source ID/path');
    expect(await result.collectOutputs()).toEqual(saved);
    await expect(result.remove('unknown')).rejects.toThrow('Unknown source ID/path');
    expect(await result.collectOutputs()).toEqual(saved);
    await expect(result.add({ id: 'broken', name: 'broken', svg: '<broken' })).rejects.toThrow(
        'SVG source "broken": Failed to inspect SVG root element: the document does not have a root node',
    );
    expect(await result.collectOutputs()).toEqual(saved);
    await expect(result.update(second.id, '<broken')).rejects.toThrow('SVG source "second": Failed to inspect SVG root element: the document does not have a root node');
    expect(await result.collectOutputs()).toEqual(saved);
    await result.add({ id: 'broken', name: 'fixed', svg });
    expect(await result.collectOutputs()).toEqual(await (await generate({ sources: [renamed, third, { id: 'broken', name: 'fixed', svg }] })).collectOutputs());
    await result.regenerate({ sources: ['third', 'second'] }, [{ type: 'removed', id: 'broken' }]);
    expect(await result.collectOutputs()).toEqual(await (await generate({ sources: [third, renamed] })).collectOutputs());
    await result.remove('third');
    const single = await (await generate({ sources: [renamed] })).collectOutputs();
    expect(await result.collectOutputs()).toEqual(single);
    await expect(result.remove('second')).rejects.toThrow('Each design must contain at least one source');
    expect(await result.collectOutputs()).toEqual(single);
    await result.update('second', svg);
    expect(await result.collectOutputs()).toEqual(await (await generate({ sources: [{ ...renamed, svg }] })).collectOutputs());
});

it('targets variant additions and applies shared source actions to every consumer', async () => {
    const variants = [
        { name: 'light', default: true, sources: [source] },
        { name: 'bold', sources: [source] },
    ];
    const result = await generate({ sources: [], variants, incremental: true, missingGlyphs: { behavior: 'blank' } });
    const extra = { id: 'extra', name: 'extra', svg };
    const before = await result.collectOutputs();
    await expect(result.add(extra)).rejects.toThrow('Adding to a family requires a target variant');
    expect(await result.collectOutputs()).toEqual(before);
    await expect(result.add(extra, 'unknown')).rejects.toThrow('Unknown target variant');
    expect(await result.collectOutputs()).toEqual(before);
    await result.add(extra, 'light');
    const added = await generate({ sources: [], variants: [{ ...variants[0]!, sources: [source, extra] }, variants[1]!], missingGlyphs: { behavior: 'blank' } });
    expect(await result.collectOutputs()).toEqual(await added.collectOutputs());
    await result.update(source.id, changed);
    const fresh = await generate({
        sources: [],
        variants: [
            { ...variants[0]!, sources: [{ ...source, svg: changed }, extra] },
            { ...variants[1]!, sources: [{ ...source, svg: changed }] },
        ],
        missingGlyphs: { behavior: 'blank' },
    });
    expect(await result.collectOutputs()).toEqual(await fresh.collectOutputs());
    await expect(result.remove(source.id)).rejects.toThrow('Each design must contain at least one source');
    expect(await result.collectOutputs()).toEqual(await fresh.collectOutputs());
    await result.remove(extra.id);
    const removed = await generate({
        sources: [],
        variants: [
            { ...variants[0]!, sources: [{ ...source, svg: changed }] },
            { ...variants[1]!, sources: [{ ...source, svg: changed }] },
        ],
        missingGlyphs: { behavior: 'blank' },
    });
    expect(await result.collectOutputs()).toEqual(await removed.collectOutputs());
});

it('serializes recoverable updates, supports additions/removals/renames and preserves old copies', async () => {
    const result = await generate({ incremental: true });
    const before = await result.collectOutputs();
    const saved = structuredClone(before);
    const invalid = result.regenerate({ sources: [source.id] }, [{ type: 'changed', id: source.id, svg: '<broken' }]);
    const readAfterFailure = result.collectOutputs();
    await expect(invalid).rejects.toThrow('SVG source');
    expect(await readAfterFailure).toEqual(before);

    const second = { id: 'other:id', name: 'second', svg };
    await result.regenerate({ sources: [second.id, source.id] }, [
        { type: 'added', ...second },
        { type: 'changed', id: source.id, svg: changed, name: 'renamed' },
    ]);
    const fresh = await generate({ sources: [second, { ...source, name: 'renamed', svg: changed }] });
    expect(await result.collectOutputs()).toEqual(await fresh.collectOutputs());
    await result.regenerate({ sources: [source.id] }, [{ type: 'removed', id: second.id }]);
    const single = await generate({ sources: [{ ...source, name: 'renamed', svg: changed }] });
    expect(await result.collectOutputs()).toEqual(await single.collectOutputs());
    expect(before).toEqual(saved);
});

it('keeps templates inline and lazy, independently of requested font bytes', async () => {
    const result = await generate({ cssTemplate: '/* {{fontName}} */', htmlTemplate: '{{#if', html: true });
    expect(await result.generateCss()).toBe('/* iconfont */');
    await expect(result.generateHtml()).rejects.toThrow('compile HTML');
    expect((await result.font('ttf'))!.byteLength).toBeGreaterThan(0);
    await expect(result.collectOutputs()).rejects.toThrow('compile HTML');
    const inline = await generate({ css: false, html: false, cssTemplate: '', htmlTemplate: '<main>{{{styles}}}</main>', types: ['svg'] });
    expect(await inline.generateCss()).toBe('');
    expect(await inline.generateHtml()).toBe('<main></main>');
    const outputs = await inline.collectOutputs();
    expect(Object.keys(outputs)).toEqual(['svg']);
    expect(outputs.woff2).toBeUndefined();
    expect(outputs.css).toBeUndefined();
    expect(await inline.font('woff')).toBeUndefined();
});

it('supports colored multi-variant families, missing glyphs and retained variant updates', async () => {
    const variants = [
        { name: 'light', weight: 300, default: true, sources: [source] },
        { name: 'bold', weight: 700, sources: [{ ...source, id: 'bold', svg: changed }] },
    ];
    const result = await generate({ sources: [], variants, colorGlyphs: true, incremental: true });
    const ttf = (await result.font('ttf'))!;
    expect(table(ttf, 'COLR')).toBeGreaterThan(0);
    expect(table(ttf, 'fvar')).toBeGreaterThan(0);
    await Promise.all(
        (['ttf', 'woff', 'woff2'] as const).map(async format => {
            const face = new FontFace(`variant-${format}`, (await result.font(format))!, { weight: '300 700' });
            await face.load();
            expect(face.status).toBe('loaded');
        }),
    );
    const membership = {
        variants: [
            { name: 'light', ids: [source.id] },
            { name: 'bold', ids: ['bold'] },
        ],
    };
    await expect(result.regenerate({ variants: membership.variants.toReversed() }, [])).rejects.toThrow('original order');
    await result.regenerate(membership, [{ type: 'changed', id: source.id, svg: changed }]);
    const fresh = await generate({ sources: [], variants: [{ ...variants[0]!, sources: [{ ...source, svg: changed }] }, variants[1]!], colorGlyphs: true });
    expect(await result.collectOutputs()).toEqual(await fresh.collectOutputs());
    const sparse = await generate({
        sources: [],
        variants: [variants[0]!, { ...variants[1]!, sources: [{ id: 'another', name: 'triangle', svg: changed }] }],
        missingGlyphs: { behavior: 'blank' },
    });
    expect((await sparse.font('woff2'))!.byteLength).toBeGreaterThan(0);
    await expect(generate({ sources: [], variants, types: ['eot'] })).rejects.toThrow('eot');
});

it('rejects file options, unknown nested options, unsafe numbers and invalid color selection', async () => {
    await expect(generate({ files: ['icon.svg'] } as GenerateWebfontsOptions)).rejects.toThrow('unknown field');
    await expect(generate({ formatOptions: { ttf: { unknown: true } } } as GenerateWebfontsOptions)).rejects.toThrow('unknown field');
    await expect(generate({ ascent: Number.NaN })).rejects.toThrow('finite numbers');
    await expect(generate({ colorGlyphs: null } as unknown as GenerateWebfontsOptions)).rejects.toThrow('untagged enum');
    const result = await generate();
    await expect(result.regenerate({ sources: [source.id] }, [])).rejects.toThrow('incremental');
});

it('rejects pending calls on disposal and isolates independent results', async () => {
    const [first, second] = await Promise.all([generate(), generate({ fontName: 'separate' })]);
    const pending = first.collectOutputs();
    first.dispose();
    await expect(pending).rejects.toThrow('disposed');
    expect(await second.generateCss()).toContain('separate');
});

it.each([10, 11])('generates WOFF2 quality %s without native threads', async quality => {
    const result = await generate({ types: ['woff2'], formatOptions: { woff2: { compressionQuality: quality } } });
    const face = new FontFace(`quality-${quality}`, (await result.font('woff2'))!);
    await face.load();
    expect(face.status).toBe('loaded');
});

it('allows repeated manual and explicit resource disposal of an already disposed result', async () => {
    const result = await generate();
    expect(() => {
        result.dispose();
        result.dispose();
        result[Symbol.dispose]();
        result[Symbol.dispose]();
    }).not.toThrow();
    expect(() => {
        using scoped = result;
        scoped.dispose();
    }).not.toThrow();
    await expect(result.font('ttf')).rejects.toThrow('disposed');
});

it('disposes at scope exit with explicit resource management, including exceptional exits', async () => {
    const result = await generate();
    await expect(
        (async () => {
            using scoped = result;
            expect((await scoped.font('ttf'))!.byteLength).toBeGreaterThan(0);
            throw new Error('scope failed');
        })(),
    ).rejects.toThrow('scope failed');
    await expect(result.font('ttf')).rejects.toThrow('disposed');
    expect(() => result.dispose()).not.toThrow();
});

function idb<T>(request: IDBRequest<T>): Promise<T> {
    const { promise, reject, resolve } = Promise.withResolvers<T>();
    request.addEventListener('success', () => resolve(request.result), { once: true });
    request.addEventListener('error', () => reject(request.error), { once: true });
    return promise;
}

it('round-trips cache bytes through IndexedDB into byte-identical warm generations', async () => {
    const options: GenerateWebfontsOptions = { types: ['svg', 'ttf', 'woff', 'woff2'], incremental: true };
    const expected = await (await generate(options)).collectOutputs();
    const cold = await generate({ ...options, cache: new Uint8Array() });
    expect(await cold.collectOutputs()).toEqual(expected);
    const bytes = await cold.exportCache();
    expect(bytes).toBeInstanceOf(Uint8Array);
    await expect(cold.exportCache()).resolves.toBeUndefined();

    const open = indexedDB.open('webfont-cache-test', 1);
    open.addEventListener('upgradeneeded', () => open.result.createObjectStore('cache'), { once: true });
    const db = await idb(open);
    await idb(db.transaction('cache', 'readwrite').objectStore('cache').put(bytes, 'iconfont'));
    const stored = await idb<Uint8Array>(db.transaction('cache').objectStore('cache').get('iconfont'));
    db.close();
    await idb(indexedDB.deleteDatabase('webfont-cache-test'));

    const warm = await generate({ ...options, cache: stored });
    expect(await warm.collectOutputs()).toEqual(expected);
    await expect(warm.exportCache()).resolves.toBeUndefined();
    await warm.regenerate({ sources: [source.id] }, [{ type: 'changed', id: source.id, svg: changed }]);
    expect(await warm.exportCache()).toBeInstanceOf(Uint8Array);
    // The caller keeps its buffer: cache bytes are copied to the worker, not transferred.
    expect(stored.byteLength).toBeGreaterThan(0);
});

it.each([
    ['an invalid header', () => new TextEncoder().encode('not a cache')],
    ['a truncated body', (valid: Uint8Array) => valid.subarray(0, valid.length >> 1)],
    [
        'a flipped byte inside a stored font',
        (valid: Uint8Array) => {
            const flipped = valid.slice();
            flipped[flipped.length - 10]! ^= 0xff;
            return flipped;
        },
    ],
])('falls back to byte-identical outputs for cache bytes with %s', async (_, corrupt) => {
    const options: GenerateWebfontsOptions = { types: ['svg', 'ttf', 'eot', 'woff', 'woff2'] };
    const expected = await (await generate(options)).collectOutputs();
    const valid = await (await generate({ ...options, cache: new Uint8Array() })).exportCache();
    if (!valid) throw new Error('expected exported cache bytes');

    const recovered = await generate({ ...options, cache: corrupt(valid) });
    expect(await recovered.collectOutputs()).toEqual(expected);
    expect(await recovered.exportCache()).toEqual(valid);
});

it('produces byte-identical variant family outputs with and without cache bytes, including corrupt ones', async () => {
    const options: GenerateWebfontsOptions = {
        sources: [],
        variants: [
            { name: 'light', weight: 300, default: true, sources: [source] },
            { name: 'bold', weight: 700, sources: [{ ...source, id: 'bold', svg: changed }] },
        ],
        incremental: true,
    };
    const expected = await (await generate(options)).collectOutputs();
    const cold = await generate({ ...options, cache: new Uint8Array() });
    expect(await cold.collectOutputs()).toEqual(expected);
    const valid = await cold.exportCache();
    if (!valid) throw new Error('expected exported cache bytes');

    const warm = await generate({ ...options, cache: valid });
    expect(await warm.collectOutputs()).toEqual(expected);
    await expect(warm.exportCache()).resolves.toBeUndefined();

    const flipped = valid.slice();
    flipped[flipped.length - 10]! ^= 0xff;
    const recovered = await generate({ ...options, cache: flipped });
    expect(await recovered.collectOutputs()).toEqual(expected);
    expect(await recovered.exportCache()).toEqual(valid);
});
