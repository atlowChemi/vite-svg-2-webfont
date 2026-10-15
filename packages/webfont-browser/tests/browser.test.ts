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
    expect(Object.keys(result).toSorted()).toEqual(['collectOutputs', 'dispose', 'font', 'generateCss', 'generateHtml', 'regenerate']);
    result.dispose();
    expect(Object.values(outputs).every(bytes => bytes.byteLength > 0)).toBe(true);
    await expect(result.font('ttf')).rejects.toThrow('disposed');
    result.dispose();
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
