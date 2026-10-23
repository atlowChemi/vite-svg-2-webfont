/// <reference lib="webworker" />
import init, { FontSession } from '../generated/webfont_browser.js';
import type { Request, Response, Result } from './types';

const scope = self as unknown as DedicatedWorkerGlobalScope;
let session: FontSession | undefined;
// Initialization and requests share one queue, including the error path.
let queue: Promise<void> = Promise.resolve();
let failed = false;

function currentSession(): FontSession {
    if (!session) throw new Error('Session is not initialized');
    return session;
}

async function handleRequest(data: Request): Promise<void> {
    if (failed) return;
    try {
        let value: Result;
        switch (data.method) {
            case 'generate':
                if (session) throw new Error('Session is already initialized');
                await init();
                session = new FontSession(data.options, data.cache);
                break;
            case 'regenerate':
                currentSession().regenerate(data.membership, data.changes);
                break;
            case 'add':
                currentSession().add(data.source, data.variant);
                break;
            case 'update':
                currentSession().update(data.sourceId, data.svg, data.name);
                break;
            case 'remove':
                currentSession().remove(data.sourceId);
                break;
            case 'font':
                value = currentSession().font(data.format) as Uint8Array<ArrayBuffer> | undefined;
                break;
            case 'css':
                value = currentSession().css();
                break;
            case 'html':
                value = currentSession().html();
                break;
            case 'outputs':
                value = currentSession().outputs();
                break;
            case 'cache':
                value = currentSession().cacheBytes() as Uint8Array<ArrayBuffer> | undefined;
                break;
        }
        let transfers: ArrayBuffer[] = [];
        if (value instanceof Uint8Array) transfers = [value.buffer];
        else if (value && typeof value === 'object') transfers = Object.values(value).map(bytes => bytes.buffer);
        scope.postMessage({ id: data.id, ok: true, value } satisfies Response, transfers);
    } catch (error) {
        // A WASM trap can leave Rust state unusable; ordinary engine errors are recoverable.
        failed = data.method === 'generate' || error instanceof WebAssembly.RuntimeError;
        scope.postMessage({ id: data.id, ok: false, error: error instanceof Error ? error : new Error(String(error)), fatal: failed } satisfies Response, []);
    }
}

scope.addEventListener('message', ({ data }: MessageEvent<Request>) => {
    queue = queue.then(() => handleRequest(data));
});
