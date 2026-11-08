import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { deserialize, serialize } from 'node:v8';

// Results of slow rendering steps that only depend on their inputs (highlighted code, Mermaid diagrams), kept across
// builds and dev restarts in the docs' git-ignored `.cache`, which CI restores. This module runs from `src/build` in dev
// and from `.cache/server` once built: both two folders below the docs.
const docsRoot = join(import.meta.dirname, '..', '..');
const cacheRoot = join(docsRoot, '.cache', 'render');

const hash = (...parts: string[]) => {
    const h = createHash('sha256');
    for (const part of parts) h.update(part).update('\0');
    return h.digest('hex');
};

interface Stored {
    version: string;
    entries: Map<string, string>;
}

// The newest cache of each name, written once more as the process exits, in case its debounced write is still waiting.
// (A dev server reloads this module when its code changes, making new caches; only the newest is current.)
const current = new Map<string, DiskCache>();
process.once('exit', () => {
    for (const cache of current.values()) cache.flush();
});

/**
 * One kind of result, kept in memory and stored as a single file: read once, on first use, and written whole after
 * changes, debounced, or when a build is done (`prune`). V8's serialization reads several times faster than JSON.
 * Reads are synchronous, since markdown-it renders synchronously.
 */
export class DiskCache {
    readonly #file: string;
    readonly #version: string;
    #entries: Map<string, string> | undefined;
    readonly #used = new Set<string>();
    #dirty = false;
    #timer: ReturnType<typeof setTimeout> | undefined;

    /**
     * @param name The file's name under `.cache/render`.
     * @param sources The docs' own files whose code decides the output, relative to the docs; a change to any of them
     * starts the cache over.
     * @param also Anything else that decides the output: package versions, class names, theme colors.
     */
    constructor(name: string, sources: string[], also: unknown) {
        this.#file = join(cacheRoot, `${name}.bin`);
        // The file format follows V8's, so a different Node.js starts over too.
        this.#version = hash(process.versions.v8, ...sources.map(file => readFileSync(join(docsRoot, file), 'utf8')), JSON.stringify(also));
        current.set(name, this);
    }

    /** The entries, read from disk on first use; a missing, unreadable or outdated file leaves the cache empty. */
    get #map(): Map<string, string> {
        if (!this.#entries) {
            try {
                const stored = deserialize(readFileSync(this.#file)) as Stored;
                this.#entries = stored.version === this.#version ? stored.entries : new Map();
            } catch {
                this.#entries = new Map();
            }
        }
        return this.#entries;
    }

    #key(parts: string[]): string {
        const key = hash(...parts);
        this.#used.add(key);
        return key;
    }

    /** The entry for these inputs, computed and stored by `make` when there is none. */
    getOrCreate(key: string[], make: () => string): string {
        const id = this.#key(key);
        let value = this.#map.get(id);
        if (value === undefined) {
            value = make();
            this.#map.set(id, value);
            this.#changed();
        }
        return value;
    }

    /** The entry for these inputs, if one is stored. */
    get(key: string[]): string | undefined {
        return this.#map.get(this.#key(key));
    }

    set(key: string[], value: string): void {
        this.#map.set(this.#key(key), value);
        this.#changed();
    }

    // A dev server renders a page at a time, so changes are written a second after the last one, not on every entry.
    #changed(): void {
        this.#dirty = true;
        clearTimeout(this.#timer);
        this.#timer = setTimeout(() => this.flush(), 1000).unref();
    }

    /** Writes the entries if they changed, through a temporary file, so a reader never sees half of it. */
    flush(): void {
        clearTimeout(this.#timer);
        if (!this.#dirty || !this.#entries) return;
        this.#dirty = false;
        mkdirSync(cacheRoot, { recursive: true });
        const temporary = `${this.#file}.${process.pid}.tmp`;
        writeFileSync(temporary, serialize({ version: this.#version, entries: this.#entries } satisfies Stored));
        renameSync(temporary, this.#file);
    }

    /** After a full build: drops every entry it didn't use, which belong to old content or old code, and writes the rest. */
    prune(): void {
        for (const key of this.#map.keys()) {
            if (this.#used.has(key)) continue;
            this.#map.delete(key);
            this.#dirty = true;
        }
        this.flush();
    }
}

/** A package's installed version, as the path it resolves to: pnpm puts the version in it. */
export const installed = (name: string) => import.meta.resolve(name).replace(/.*\/node_modules\/\.pnpm\/([^/]+)\/.*/, '$1');
