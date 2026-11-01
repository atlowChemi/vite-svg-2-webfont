import 'fake-indexeddb/auto';
import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { loadEngineCache, saveEngineCache } from './engineCache';

afterEach(async () => {
    vi.restoreAllMocks();
    const { promise, resolve } = Promise.withResolvers<void>();
    indexedDB.deleteDatabase('vite-svg-2-webfont:playground').addEventListener('success', () => resolve());
    await promise;
});

describe('engine cache', () => {
    it('starts cold with an empty array', async () => {
        await expect(loadEngineCache()).resolves.toEqual(new Uint8Array());
    });

    it('returns the latest saved bytes on the next load', async () => {
        await saveEngineCache(new Uint8Array([1, 2]));
        await saveEngineCache(new Uint8Array([3, 4, 5]));
        await expect(loadEngineCache()).resolves.toEqual(new Uint8Array([3, 4, 5]));
    });

    it('starts cold and never throws when IndexedDB is unavailable', async () => {
        vi.spyOn(indexedDB, 'open').mockImplementation(() => {
            throw new DOMException('Blocked', 'SecurityError');
        });
        await expect(saveEngineCache(new Uint8Array([1]))).resolves.toBeUndefined();
        await expect(loadEngineCache()).resolves.toEqual(new Uint8Array());
    });
});
