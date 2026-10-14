import { readFileSync } from 'node:fs';
import { beforeEach, expect, it, vi } from 'vite-plus/test';
import type { RequestPayload, Response } from '../src/protocol';

// Exercise the real worker and WASM in Node, replacing only the browser transport
// and URL fetch so V8 can instrument worker.ts in this test process.
vi.mock('../generated/webfont_browser.js', async importOriginal => {
    const binding = await importOriginal<typeof import('../generated/webfont_browser.js')>();
    return { ...binding, default: () => binding.default({ module_or_path: readFileSync(new URL('../generated/webfont_browser_bg.wasm', import.meta.url)) }) };
});

let scope: EventTarget;
let respond: (response: Response) => void;
let id = 0;
beforeEach(async () => {
    vi.resetModules();
    scope = Object.assign(new EventTarget(), { postMessage: (response: Response) => respond(response) });
    vi.stubGlobal('self', scope);
    await import('../src/worker');
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
    expect(await request({ method: 'outputs' })).toMatchObject({ ok: true, value: expect.any(Array) });
    expect(await request({ method: 'generate', options })).toMatchObject({ ok: false, fatal: true, error: 'Session is already initialized' });
});

it('marks initialization failures as fatal', async () => {
    expect(await request({ method: 'generate', options: '{"files":[]}' })).toMatchObject({ ok: false, fatal: true, error: expect.stringContaining('unknown field') });
});
