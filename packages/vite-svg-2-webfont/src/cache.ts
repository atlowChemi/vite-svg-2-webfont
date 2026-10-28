import { hash, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { access, readFile, rename, rm } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { Readable } from 'node:stream';
import { css as defaultCssTemplate } from '@atlowchemi/webfont-generator/templates';
import { ensureDirExistsAndWriteFile } from './utils';

/** Stands in for the per-run font directory in cached build CSS. */
export const CACHE_PLACEHOLDER = '__VSW_CACHE__';
/** Bump when the manifest shape or the cached CSS conventions change. */
const FORMAT_VERSION = 1;

export type CacheMode = 'dev' | 'build';
export type CacheEntry = {
    /** `null` when the inputs can't be fingerprinted; the entry then only tracks files for pruning. */
    key: string | null;
    fonts: Record<string, string>;
    /** Missing only when rendering failed; such entries are stored with a `null` key. */
    css?: string;
    moduleCss?: string;
    html?: string;
    ts?: string;
};
type Manifest = { version: number } & Partial<Record<CacheMode, CacheEntry>>;

let engineVersion: string | undefined;
function getEngineVersion(): string {
    return (engineVersion ??= JSON.parse(readFileSync(join(dirname(defaultCssTemplate), '..', 'package.json'), 'utf8')).version);
}

/**
 * Fingerprint generation inputs. Paths stay absolute: Vite's `config.root` can differ in form from the
 * plugin's paths (Windows 8.3 short names, macOS `/tmp` vs `/private/tmp`), so relativizing them is unreliable.
 * Throws when an input file can't be read; callers treat that as uncacheable.
 */
export async function computeCacheKey(settings: object, files: string[], extra: unknown[], signal?: AbortSignal): Promise<string> {
    // Bounded concurrency: ties sync reads on Node 22/24 and halves the time on Node 26, without risking EMFILE.
    const contents = await Readable.from(files)
        .map(async (file: string, options) => hash('sha1', await readFile(file, options)), { concurrency: 100, signal })
        .toArray();
    return hash('sha1', JSON.stringify([FORMAT_VERSION, getEngineVersion(), settings, contents, extra]));
}

export function fontFileName(fontName: string, type: string, hashHex: string): string {
    return `${fontName}-${hashHex}.${type}`;
}

const manifestPath = (folder: string, fontName: string) => join(folder, `${fontName}.json`);

async function readManifest(folder: string, fontName: string): Promise<Manifest> {
    const manifest: Manifest | undefined = await readFile(manifestPath(folder, fontName), 'utf8').then(JSON.parse, () => undefined);
    return manifest?.version === FORMAT_VERSION ? manifest : { version: FORMAT_VERSION };
}

async function writeAtomic(content: string | Uint8Array, path: string): Promise<void> {
    const tmp = `${path}.${randomUUID()}.tmp`;
    await ensureDirExistsAndWriteFile(content, tmp);
    await rename(tmp, path);
}

/** Any missing or unreadable piece is a miss. */
export async function readCacheEntry<T extends string>(
    folder: string,
    fontName: string,
    mode: CacheMode,
    key: string,
    types: readonly T[],
): Promise<{ entry: CacheEntry; fonts: Map<T, Buffer> } | undefined> {
    try {
        const entry = (await readManifest(folder, fontName))[mode];
        if (entry?.key !== key) return undefined;
        const fonts = await Promise.all(types.map(async type => [type, await readFile(join(folder, entry.fonts[type]!))] as const));
        return { entry, fonts: new Map(fonts) };
    } catch {
        return undefined;
    }
}

/** Write fonts before the manifest so it never points at missing files, then prune this mode's stale fonts. */
export async function writeCacheEntry(folder: string, fontName: string, mode: CacheMode, entry: CacheEntry, buffers: Map<string, Uint8Array>): Promise<void> {
    await Promise.all(
        Object.entries(entry.fonts).map(async ([type, name]) => {
            const path = join(folder, name);
            // Content-addressed: an existing file already holds these bytes.
            if (
                !(await access(path).then(
                    () => true,
                    () => false,
                ))
            )
                await writeAtomic(buffers.get(type)!, path);
        }),
    );
    const manifest = await readManifest(folder, fontName);
    const previous = Object.values(manifest[mode]?.fonts ?? {});
    manifest[mode] = entry;
    await writeAtomic(JSON.stringify(manifest), manifestPath(folder, fontName));
    // ponytail: read-modify-write without a lock; a concurrent writer can lose an entry or a file, which only causes a later miss.
    const referenced = new Set([...Object.values(manifest.dev?.fonts ?? {}), ...Object.values(manifest.build?.fonts ?? {})]);
    await Promise.all(previous.filter(name => !referenced.has(name)).map(name => rm(join(folder, name), { force: true })));
}
