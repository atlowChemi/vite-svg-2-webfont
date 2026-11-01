const DATABASE = 'vite-svg-2-webfont:playground';
const STORE = 'engine-cache';
const KEY = 'bytes';

function open(): Promise<IDBDatabase> {
    const { promise, resolve, reject } = Promise.withResolvers<IDBDatabase>();
    const request = indexedDB.open(DATABASE, 1);
    request.addEventListener('upgradeneeded', () => request.result.createObjectStore(STORE));
    request.addEventListener('success', () => resolve(request.result));
    request.addEventListener('error', () => reject(request.error));
    return promise;
}

async function transact<T>(mode: IDBTransactionMode, run: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
    const database = await open();
    try {
        const { promise, resolve, reject } = Promise.withResolvers<T>();
        const request = run(database.transaction(STORE, mode).objectStore(STORE));
        request.addEventListener('success', () => resolve(request.result));
        request.addEventListener('error', () => reject(request.error));
        return await promise;
    } finally {
        database.close();
    }
}

/**
 * The engine's persistent cache from an earlier visit, or an empty array to start cold. The engine ignores bytes from
 * another version or other settings, and recomputes from damaged ones.
 */
export async function loadEngineCache(): Promise<Uint8Array> {
    try {
        const bytes = await transact<unknown>('readonly', store => store.get(KEY));
        return bytes instanceof Uint8Array ? bytes : new Uint8Array();
    } catch {
        // IndexedDB can be unavailable (private mode, blocked site data); builds then start cold.
        return new Uint8Array();
    }
}

/**
 * Keep the latest cache for the next visit. It holds outlines and fonts made from the viewer's icons, and like the
 * settings, it stays in their browser. Failures only cost a warm start.
 */
export async function saveEngineCache(bytes: Uint8Array): Promise<void> {
    try {
        await transact('readwrite', store => store.put(bytes, KEY));
    } catch {
        // Quota or availability errors leave the previous cache in place.
    }
}
