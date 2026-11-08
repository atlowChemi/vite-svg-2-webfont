import { existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { DiskCache } from './diskCache.ts';

// A file of its own under `.cache/render`, removed after each test.
const name = `test-${process.pid}`;
const file = join(import.meta.dirname, '..', '..', '.cache', 'render', `${name}.bin`);
const sources = ['src/build/diskCache.ts'];
const open = (also: unknown = {}) => new DiskCache(name, sources, also);
afterEach(() => rmSync(file, { force: true }));

describe('DiskCache', () => {
    it('computes an entry once, then reads it back, in this process or, once written, the next', () => {
        const make = vi.fn(() => '<pre>a</pre>');
        const cache = open();
        expect(cache.getOrCreate(['ts', 'a'], make)).toBe('<pre>a</pre>');
        expect(cache.getOrCreate(['ts', 'a'], make)).toBe('<pre>a</pre>');
        cache.flush();
        expect(open().getOrCreate(['ts', 'a'], make)).toBe('<pre>a</pre>');
        expect(make).toHaveBeenCalledOnce();
    });

    it('writes nothing until flushed, and nothing when nothing changed', () => {
        const cache = open();
        cache.set(['a'], 'x');
        expect(existsSync(file)).toBe(false);
        cache.flush();
        expect(existsSync(file)).toBe(true);
        rmSync(file);
        cache.flush();
        expect(existsSync(file)).toBe(false);
    });

    it('writes a second after the last change on its own', () => {
        vi.useFakeTimers();
        try {
            const cache = open();
            cache.set(['a'], 'x');
            vi.advanceTimersByTime(900);
            cache.set(['b'], 'y');
            vi.advanceTimersByTime(900);
            expect(existsSync(file)).toBe(false);
            vi.advanceTimersByTime(100);
            expect(existsSync(file)).toBe(true);
        } finally {
            vi.useRealTimers();
        }
    });

    it('starts over when anything else that decides the output changes', () => {
        const cache = open({ v: 1 });
        cache.set(['a'], 'old');
        cache.flush();
        expect(open({ v: 2 }).get(['a'])).toBeUndefined();
    });

    it('tells keys apart however their parts split', () => {
        const cache = open();
        cache.set(['ab', 'c'], 'first');
        expect(cache.get(['a', 'bc'])).toBeUndefined();
    });

    it('prunes the entries this process left unused, and writes the rest', () => {
        const old = open();
        old.set(['stale'], 'x');
        old.set(['kept'], 'y');
        old.flush();
        const cache = open();
        expect(cache.get(['kept'])).toBe('y');
        cache.prune();
        const next = open();
        expect(next.get(['kept'])).toBe('y');
        expect(next.get(['stale'])).toBeUndefined();
    });
});
