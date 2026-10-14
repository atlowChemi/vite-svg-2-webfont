import type { RequestPayload, Response, Result } from './protocol';
import type { CollectedOutput, FontType, GenerateWebfontsOptions, SourceChange, SourceMembership, WebfontResult } from './types';
export type { CollectedOutput, FontType, GenerateWebfontsOptions, JsonValue, SourceChange, SourceFontVariant, SourceMembership, SvgSource, WebfontResult } from './types';

function json(value: unknown): string {
    return JSON.stringify(value, (_key, item: unknown) => {
        if (typeof item === 'number' && !Number.isFinite(item)) throw new TypeError('Options must contain finite numbers');
        return item;
    });
}

/** Generate from SVG text in a dedicated module worker. Dispose the result when finished. */
export async function generateWebfonts(options: GenerateWebfontsOptions): Promise<WebfontResult> {
    const encoded = json(options);
    const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module', name: 'webfont-browser' });
    const pending = new Map<number, { resolve: (value: Result) => void; reject: (error: Error) => void }>();
    let nextId = 0;
    let failure: Error | undefined;

    function close(error: Error): void {
        if (failure) return;
        failure = error;
        worker.terminate();
        for (const call of pending.values()) call.reject(error);
        pending.clear();
    }
    worker.addEventListener('error', event => close(new Error(event.message || 'Webfont worker failed')));
    worker.addEventListener('messageerror', () => close(new Error('Unable to decode webfont worker response')));
    worker.addEventListener('message', ({ data }: MessageEvent<Response>) => {
        if (!data.ok && data.fatal) {
            close(new Error(data.error));
            return;
        }
        const call = pending.get(data.id);
        if (!call) return;
        pending.delete(data.id);
        if (data.ok) call.resolve(data.value);
        else call.reject(new Error(data.error));
    });

    function request<T extends Result>(message: RequestPayload): Promise<T> {
        if (failure) return Promise.reject(failure);
        const id = nextId++;
        return new Promise<T>((resolve, reject) => {
            pending.set(id, { resolve: value => resolve(value as T), reject });
            try {
                worker.postMessage({ ...message, id }, []);
            } catch (error) {
                pending.delete(id);
                reject(error);
            }
        });
    }

    try {
        await request({ method: 'generate', options: encoded });
    } catch (error) {
        close(error instanceof Error ? error : new Error(String(error)));
        throw error;
    }
    const dispose = (): void => close(new Error('Webfont result has been disposed'));
    return {
        font: (format: FontType) => request<Uint8Array<ArrayBuffer> | undefined>({ method: 'font', format }),
        generateCss: () => request<string>({ method: 'css' }),
        generateHtml: () => request<string>({ method: 'html' }),
        collectOutputs: () => request<CollectedOutput[]>({ method: 'outputs' }),
        regenerate: async (membership: SourceMembership, changes: SourceChange[]): Promise<void> => {
            await request({ method: 'regenerate', membership: json(membership), changes: json(changes) });
        },
        dispose,
        [Symbol.dispose]: dispose,
    };
}
