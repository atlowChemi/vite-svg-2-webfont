import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import type { RequestPayload, Response } from '../src/types';

// Exercise the real worker and WASM in Node, replacing only the browser transport
// and URL fetch so V8 can instrument worker.ts in this test process.
vi.mock('../generated/webfont_browser.js', async importOriginal => {
    const binding = await importOriginal<typeof import('../generated/webfont_browser.js')>();
    return { ...binding, default: () => binding.default({ module_or_path: readFileSync(new URL('../generated/webfont_browser_bg.wasm', import.meta.url)) }) };
});

let scope: EventTarget;
let respond: (response: Response) => void;
let id = 0;
/** Load a fresh worker module on its own message target, as a new worker would be. */
async function startWorker(): Promise<void> {
    vi.resetModules();
    scope = Object.assign(new EventTarget(), { postMessage: (response: Response) => respond(structuredClone(response)) });
    vi.stubGlobal('self', scope);
    await import('../src/worker');
}
beforeEach(startWorker);
afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
});

function request(payload: RequestPayload): Promise<Response> {
    return new Promise(resolve => {
        respond = resolve;
        scope.dispatchEvent(new MessageEvent('message', { data: { ...payload, id: id++ } }));
    });
}

const options = JSON.stringify({
    sources: [{ id: 'icon', name: 'icon', svg: '<svg viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>' }],
    types: ['ttf'],
    incremental: true,
    formatOptions: { ttf: { ts: 0 } },
});

it('routes worker requests through real WASM and recovers after invalid updates', async () => {
    expect(await request({ method: 'font', format: 'ttf' })).toMatchObject({ ok: false, fatal: false });
    expect(await request({ method: 'generate', options })).toMatchObject({ ok: true });
    const before = await request({ method: 'font', format: 'ttf' });
    expect(before).toMatchObject({ ok: true, value: expect.any(Uint8Array) });
    expect(await request({ method: 'regenerate', membership: '{"sources":["unknown"]}', changes: '[]' })).toMatchObject({ ok: false, fatal: false });
    expect(await request({ method: 'regenerate', membership: '{"sources":["icon"]}', changes: '[]' })).toMatchObject({ ok: true });
    expect(await request({ method: 'font', format: 'ttf' })).toMatchObject({ ok: true, value: before.ok ? before.value : undefined });
    expect(await request({ method: 'css' })).toMatchObject({ ok: true, value: expect.stringContaining('@font-face') });
    expect(await request({ method: 'html' })).toMatchObject({ ok: true, value: expect.stringContaining('icon') });
    expect(await request({ method: 'ts' })).toMatchObject({ ok: true, value: expect.stringContaining('export type IconName = (typeof icons)[number];') });
    expect(await request({ method: 'outputs' })).toMatchObject({ ok: true, value: { ttf: expect.any(Uint8Array), css: expect.any(Uint8Array) } });
    expect(await request({ method: 'add', source: JSON.stringify({ ...JSON.parse(options).sources[0], id: 'second', name: 'second' }) })).toMatchObject({ ok: true });
    expect(await request({ method: 'update', sourceId: 'second', svg: JSON.parse(options).sources[0].svg, name: 'renamed' })).toMatchObject({ ok: true });
    expect(await request({ method: 'remove', sourceId: 'second' })).toMatchObject({ ok: true });
    expect(await request({ method: 'generate', options })).toMatchObject({ ok: false, fatal: true, error: new Error('Session is already initialized') });
});

it('marks initialization failures as fatal', async () => {
    expect(await request({ method: 'generate', options: '{"files":[]}' })).toMatchObject({
        ok: false,
        fatal: true,
        error: expect.objectContaining({ message: expect.stringContaining('unknown field') }),
    });
});

it('normalizes non-Error throws and recovers for subsequent requests', async () => {
    const { FontSession } = await import('../generated/webfont_browser.js');
    await request({ method: 'generate', options });
    vi.spyOn(FontSession.prototype, 'css').mockImplementationOnce(() => {
        throw 'render failed';
    });
    expect(await request({ method: 'css' })).toMatchObject({ ok: false, fatal: false, error: new Error('render failed') });
    expect(await request({ method: 'css' })).toMatchObject({ ok: true });
});

it('ignores queued requests after a WASM trap', async () => {
    const { FontSession } = await import('../generated/webfont_browser.js');
    await request({ method: 'generate', options });
    vi.spyOn(FontSession.prototype, 'css').mockImplementationOnce(() => {
        throw new WebAssembly.RuntimeError('unreachable');
    });
    expect(await request({ method: 'css' })).toMatchObject({ ok: false, fatal: true });
    const font = vi.spyOn(FontSession.prototype, 'font');
    const reply = vi.fn();
    respond = reply;
    scope.dispatchEvent(new MessageEvent('message', { data: { id: id++, method: 'font', format: 'ttf' } }));
    // Drain the worker's promise queue before asserting that the request was ignored.
    await new Promise(resolve => setTimeout(resolve, 0));
    expect(font).not.toHaveBeenCalled();
    expect(reply).not.toHaveBeenCalled();
});

it('preserves standard error details across the message boundary and continues after rejection', async () => {
    const { FontSession } = await import('../generated/webfont_browser.js');
    expect(await request({ method: 'generate', options })).toMatchObject({ ok: true });
    const error = new TypeError('render failed', { cause: new Error('underlying error') });
    vi.spyOn(FontSession.prototype, 'css').mockImplementationOnce(() => {
        throw error;
    });
    const response = await request({ method: 'css' });
    expect(response).toMatchObject({
        ok: false,
        fatal: false,
        error: {
            name: 'TypeError',
            message: error.message,
            stack: error.stack,
            cause: { message: 'underlying error' },
        },
    });
    expect(await request({ method: 'css' })).toMatchObject({ ok: true, value: expect.stringContaining('@font-face') });
});

it('passes cache bytes to the session and exports updated bytes', async () => {
    expect(await request({ method: 'generate', options, cache: new Uint8Array() })).toMatchObject({ ok: true });
    const exported = await request({ method: 'cache' });
    expect(exported).toMatchObject({ ok: true, value: expect.any(Uint8Array) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: undefined });
});

const allFormats = JSON.stringify({ ...JSON.parse(options), types: ['svg', 'ttf', 'eot', 'woff', 'woff2'] });

/** Uncached outputs (every format) and a valid exported cache for `allFormats`, from two fresh workers. */
async function baseline(): Promise<{ outputs: Response; cache: Uint8Array }> {
    expect(await request({ method: 'generate', options: allFormats })).toMatchObject({ ok: true });
    const outputs = await request({ method: 'outputs' });
    await startWorker();
    expect(await request({ method: 'generate', options: allFormats, cache: new Uint8Array() })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    const exported = await request({ method: 'cache' });
    if (!exported.ok || !(exported.value instanceof Uint8Array)) throw new Error('expected exported cache bytes');
    await startWorker();
    return { outputs, cache: exported.value };
}

it('produces identical outputs from a warm cache', async () => {
    const { outputs, cache } = await baseline();
    expect(await request({ method: 'generate', options: allFormats, cache })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: undefined });
});

/** The font outputs of an `outputs` response, without CSS and HTML. */
function fonts(response: Response): Record<string, Uint8Array> {
    if (!response.ok) throw new Error('expected outputs');
    const { css: _, html: __, ...rest } = response.value as Record<string, Uint8Array>;
    return rest;
}

it('produces identical color glyph outputs from a warm cache', async () => {
    const color = JSON.stringify({
        ...JSON.parse(allFormats),
        sources: [{ id: 'logo', name: 'logo', svg: '<svg viewBox="0 0 24 24"><path fill="#ff9900" d="M2 2H22V22H2Z"/><path fill="#0066ff" d="M6 6H12V12H6Z"/></svg>' }],
        types: ['ttf', 'woff', 'woff2'],
        colorGlyphs: true,
    });
    expect(await request({ method: 'generate', options: color })).toMatchObject({ ok: true });
    const outputs = await request({ method: 'outputs' });
    await startWorker();
    expect(await request({ method: 'generate', options: color, cache: new Uint8Array() })).toMatchObject({ ok: true });
    const exported = await request({ method: 'cache' });
    if (!exported.ok || !(exported.value instanceof Uint8Array)) throw new Error('expected exported cache bytes');

    await startWorker();
    expect(await request({ method: 'generate', options: color, cache: exported.value })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: undefined });
});

it('reuses cached fonts across sessions with the default browser clock', async () => {
    const clockOptions = JSON.stringify({ ...JSON.parse(allFormats), formatOptions: undefined });
    vi.useFakeTimers({ now: 1_700_000_000_000, toFake: ['Date'] });
    expect(await request({ method: 'generate', options: clockOptions, cache: new Uint8Array() })).toMatchObject({ ok: true });
    const outputs = await request({ method: 'outputs' });
    const exported = await request({ method: 'cache' });
    if (!exported.ok || !(exported.value instanceof Uint8Array)) throw new Error('expected exported cache bytes');
    // The faked clock reached the engine: the fonts match an explicit timestamp of that time (CSS
    // differs only in its settings hash).
    await startWorker();
    const pinned = JSON.stringify({ ...JSON.parse(allFormats), formatOptions: { ttf: { ts: 1_700_000_000 } } });
    expect(await request({ method: 'generate', options: pinned })).toMatchObject({ ok: true });
    expect(fonts(await request({ method: 'outputs' }))).toEqual(fonts(outputs));

    // A later session has another clock, yet every font comes back unchanged and the stored bytes stay valid.
    vi.setSystemTime(1_800_000_000_000);
    await startWorker();
    expect(await request({ method: 'generate', options: clockOptions, cache: exported.value })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: undefined });
});

it.each([
    ['an invalid header', () => new TextEncoder().encode('not a cache')],
    ['an empty buffer', () => new Uint8Array()],
    [
        'an unknown format version',
        (valid: Uint8Array) => {
            const copy = valid.slice();
            copy[8]! ^= 0xff;
            return copy;
        },
    ],
    ['a truncated body', (valid: Uint8Array) => valid.subarray(0, valid.length >> 1)],
    [
        'a flipped byte inside a stored font',
        (valid: Uint8Array) => {
            const flipped = valid.slice();
            flipped[flipped.length - 10]! ^= 0xff;
            return flipped;
        },
    ],
    [
        'trailing bytes',
        (valid: Uint8Array) => {
            const longer = new Uint8Array(valid.length + 5);
            longer.set(valid);
            return longer;
        },
    ],
])('falls back to identical outputs for cache bytes with %s and exports a fresh cache', async (_, corrupt) => {
    const { outputs, cache } = await baseline();
    expect(await request({ method: 'generate', options: allFormats, cache: corrupt(cache) })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: cache });
});

it('produces identical variant family outputs cold, warm and from corrupt cache bytes', async () => {
    const icon = JSON.parse(options).sources[0];
    const family = JSON.stringify({
        variants: [
            { name: 'light', weight: 300, default: true, sources: [icon] },
            { name: 'bold', weight: 700, sources: [{ ...icon, id: 'bold', svg: '<svg viewBox="0 0 24 24"><path d="M1 1L23 23H1Z"/></svg>' }] },
        ],
        types: ['ttf', 'woff', 'woff2'],
        formatOptions: { ttf: { ts: 0 } },
    });
    expect(await request({ method: 'generate', options: family })).toMatchObject({ ok: true });
    const outputs = await request({ method: 'outputs' });
    await startWorker();
    expect(await request({ method: 'generate', options: family, cache: new Uint8Array() })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    const exported = await request({ method: 'cache' });
    if (!exported.ok || !(exported.value instanceof Uint8Array)) throw new Error('expected exported cache bytes');
    const cache = exported.value;

    await startWorker();
    expect(await request({ method: 'generate', options: family, cache })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: undefined });

    await startWorker();
    const flipped = cache.slice();
    flipped[flipped.length - 10]! ^= 0xff;
    expect(await request({ method: 'generate', options: family, cache: flipped })).toMatchObject({ ok: true });
    expect(await request({ method: 'outputs' })).toEqual({ ...outputs, id: expect.any(Number) });
    expect(await request({ method: 'cache' })).toEqual({ id: expect.any(Number), ok: true, value: cache });
});
