/// <reference lib="webworker" />
import init, { FontSession } from '../generated/webfont_browser.js';
import type { CollectedOutput } from './types';
import type { Request, Response, Result } from './protocol';

const scope = self as unknown as DedicatedWorkerGlobalScope;
let session: FontSession | undefined;
// Initialization and requests share one queue, including the error path.
let queue: Promise<void> = Promise.resolve();
let failed = false;

scope.addEventListener('message', ({ data }: MessageEvent<Request>) => {
    queue = queue.then(async () => {
        if (failed) return undefined;
        try {
            let value: Result;
            if (data.method === 'generate') {
                if (session) throw new Error('Session is already initialized');
                await init();
                session = new FontSession(data.options);
            } else {
                if (!session) throw new Error('Session is not initialized');
                switch (data.method) {
                    case 'regenerate':
                        session.regenerate(data.membership, data.changes);
                        break;
                    case 'font':
                        value = session.font(data.format) as Uint8Array<ArrayBuffer> | undefined;
                        break;
                    case 'css':
                        value = session.css();
                        break;
                    case 'html':
                        value = session.html();
                        break;
                    case 'outputs':
                        value = session.outputs() as CollectedOutput[];
                        break;
                }
            }
            const transfers = value instanceof Uint8Array ? [value.buffer] : Array.isArray(value) ? value.map(output => output.bytes.buffer) : [];
            scope.postMessage({ id: data.id, ok: true, value } satisfies Response, transfers);
        } catch (error) {
            // A WASM trap can leave Rust state unusable; ordinary engine errors are recoverable.
            failed = data.method === 'generate' || error instanceof WebAssembly.RuntimeError;
            scope.postMessage({ id: data.id, ok: false, error: error instanceof Error ? error.message : String(error), fatal: failed } satisfies Response, []);
        }
        return undefined;
    });
});
