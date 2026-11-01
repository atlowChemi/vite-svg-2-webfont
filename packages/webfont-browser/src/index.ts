import type {
    CollectedOutputs,
    FontType,
    GenerateWebfontsOptions,
    RequestPayload,
    Response,
    Result,
    SourceChange,
    SourceMembership,
    SvgSource,
    WebfontGenerator,
    WebfontResult,
    WorkerRuntime,
} from './types';
export type {
    CollectedOutputs,
    FontType,
    GenerateWebfontsOptions,
    JsonValue,
    SourceChange,
    SourceFontVariant,
    SourceMembership,
    SvgSource,
    WebfontGenerator,
    WebfontResult,
    WorkerRuntime,
} from './types';

function json(value: unknown): string {
    return JSON.stringify(value, (_key, item: unknown) => {
        if (typeof item === 'number' && !Number.isFinite(item)) throw new TypeError('Options must contain finite numbers');
        return item;
    });
}

type Call = Omit<PromiseWithResolvers<Result>, 'promise'> & { session: number };

/** Start a module worker. A crash or `close` rejects the pending calls of every session it hosts. */
function connect(runtime: WorkerRuntime) {
    const worker = runtime.workerUrl
        ? new Worker(runtime.workerUrl, { type: 'module', name: 'webfont-browser' })
        : new Worker(new URL('./worker.js', import.meta.url), { type: 'module', name: 'webfont-browser' });
    const pending = new Map<number, Call>();
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
            close(data.error);
            return;
        }
        const call = pending.get(data.id);
        if (!call) return;
        pending.delete(data.id);
        if (data.ok) call.resolve(data.value);
        else call.reject(data.error);
    });

    function request<T extends Result>(session: number, message: RequestPayload): Promise<T> {
        if (failure) return Promise.reject(failure);
        const id = nextId++;
        const { promise, resolve, reject } = Promise.withResolvers<Result>();
        pending.set(id, { resolve, reject, session });
        try {
            worker.postMessage({ ...message, id, session }, []);
        } catch (error) {
            pending.delete(id);
            reject(error);
        }
        return promise as Promise<T>;
    }

    /** Reject one session's pending calls and free it in the worker, keeping the worker for other sessions. */
    function release(session: number, error: Error): void {
        for (const [id, call] of pending) {
            if (call.session !== session) continue;
            pending.delete(id);
            call.reject(error);
        }
        request(session, { method: 'dispose' }).catch(() => {});
    }

    return { request, release, close };
}
type Connection = ReturnType<typeof connect>;

/** Create one session in the connected worker. `release` frees what the result holds once it is disposed. */
async function open(connection: Connection, session: number, options: GenerateWebfontsOptions, release: (error: Error) => void): Promise<WebfontResult> {
    // Cache bytes travel beside the JSON settings rather than as a number array inside them.
    const { cache, ...settings } = options;
    await connection.request(session, { method: 'generate', options: json(settings), cache });
    let disposed: Error | undefined;
    const request = <T extends Result>(message: RequestPayload): Promise<T> => (disposed ? Promise.reject(disposed) : connection.request<T>(session, message));
    const dispose = (): void => {
        if (disposed) return;
        disposed = new Error('Webfont result has been disposed');
        release(disposed);
    };
    return {
        font: (format: FontType) => request<Uint8Array<ArrayBuffer> | undefined>({ method: 'font', format }),
        generateCss: () => request<string>({ method: 'css' }),
        generateHtml: () => request<string>({ method: 'html' }),
        generateTs: () => request<string>({ method: 'ts' }),
        collectOutputs: () => request<CollectedOutputs>({ method: 'outputs' }),
        exportCache: () => request<Uint8Array<ArrayBuffer> | undefined>({ method: 'cache' }),
        regenerate: async (membership: SourceMembership, changes: SourceChange[]): Promise<void> => {
            await request({ method: 'regenerate', membership: json(membership), changes: json(changes) });
        },
        add: async (source: SvgSource, variant?: string): Promise<void> => {
            await request({ method: 'add', source: json(source), variant });
        },
        update: async (id: string, svg: string, name?: string): Promise<void> => {
            await request({ method: 'update', sourceId: id, svg, name });
        },
        remove: async (id: string): Promise<void> => {
            await request({ method: 'remove', sourceId: id });
        },
        dispose,
        [Symbol.dispose]: dispose,
    };
}

/**
 * Generate from SVG text in a dedicated module worker. Dispose the result when finished.
 * `runtime.workerUrl` selects a separately hosted worker; keep its WASM file beside it.
 */
export async function generateWebfonts(options: GenerateWebfontsOptions, runtime: WorkerRuntime = {}): Promise<WebfontResult> {
    const connection = connect(runtime);
    try {
        return await open(connection, 0, options, error => connection.close(error));
    } catch (error) {
        connection.close(error instanceof Error ? error : new Error(String(error)));
        throw error;
    }
}

/**
 * Start one module worker that can host many results, so every generation after the first skips loading and
 * compiling the WASM. The worker starts loading the WASM right away. Dispose the generator to terminate the worker.
 */
export function createWebfontGenerator(runtime: WorkerRuntime = {}): WebfontGenerator {
    const connection = connect(runtime);
    let nextSession = 0;
    const dispose = (): void => connection.close(new Error('Webfont generator has been disposed'));
    return {
        generateWebfonts: (options: GenerateWebfontsOptions) => {
            const session = nextSession++;
            return open(connection, session, options, error => connection.release(session, error));
        },
        dispose,
        [Symbol.dispose]: dispose,
    };
}
