/// <reference lib="webworker" />
import init, { FontSession } from '../generated/webfont_browser.js';
import type { Request, Response, Result } from './types';

const scope = self as unknown as DedicatedWorkerGlobalScope;
// Start loading the WASM with the worker, so a generator is warm before its first generation.
const ready = init();
ready.catch(() => {});
const sessions = new Map<number, FontSession>();
// Initialization and requests share one queue, including the error path.
let queue: Promise<void> = Promise.resolve();
let failed = false;

function currentSession(id: number): FontSession {
    const session = sessions.get(id);
    if (!session) throw new Error('Session is not initialized');
    return session;
}

async function handleRequest(data: Request): Promise<void> {
    if (failed) return;
    let loaded = true;
    try {
        let value: Result;
        switch (data.method) {
            case 'generate':
                if (sessions.has(data.session)) throw new Error('Session is already initialized');
                loaded = false;
                await ready;
                loaded = true;
                sessions.set(data.session, new FontSession(data.options, data.cache));
                break;
            case 'dispose':
                sessions.get(data.session)?.free();
                sessions.delete(data.session);
                break;
            case 'regenerate':
                currentSession(data.session).regenerate(data.membership, data.changes);
                break;
            case 'add':
                currentSession(data.session).add(data.source, data.variant);
                break;
            case 'update':
                currentSession(data.session).update(data.sourceId, data.svg, data.name);
                break;
            case 'remove':
                currentSession(data.session).remove(data.sourceId);
                break;
            case 'font':
                value = currentSession(data.session).font(data.format) as Uint8Array<ArrayBuffer> | undefined;
                break;
            case 'css':
                value = currentSession(data.session).css();
                break;
            case 'html':
                value = currentSession(data.session).html();
                break;
            case 'outputs':
                value = currentSession(data.session).outputs();
                break;
            case 'ts':
                value = currentSession(data.session).ts();
                break;
            case 'cache':
                value = currentSession(data.session).cacheBytes() as Uint8Array<ArrayBuffer> | undefined;
                break;
        }
        let transfers: ArrayBuffer[] = [];
        if (value instanceof Uint8Array) transfers = [value.buffer];
        else if (value && typeof value === 'object') transfers = Object.values(value).map(bytes => bytes.buffer);
        scope.postMessage({ id: data.id, ok: true, value } satisfies Response, transfers);
    } catch (error) {
        // A WASM that failed to load or trapped leaves every session unusable; engine errors, including invalid
        // options, only fail their own request.
        failed = !loaded || error instanceof WebAssembly.RuntimeError;
        scope.postMessage({ id: data.id, ok: false, error: error instanceof Error ? error : new Error(String(error)), fatal: failed } satisfies Response, []);
    }
}

scope.addEventListener('message', ({ data }: MessageEvent<Request>) => {
    queue = queue.then(() => handleRequest(data));
});
