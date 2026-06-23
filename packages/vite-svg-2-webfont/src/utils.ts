import { createHash } from 'node:crypto';
import { tmpdir as osTmpdir } from 'node:os';
import { constants, rm as fsRm, mkdtempSync } from 'node:fs';
import { resolve, dirname, join as pathJoin } from 'node:path';
import { watch, access, mkdir, writeFile } from 'node:fs/promises';
import type { FontType } from '@atlowchemi/webfont-generator';

let watcher: ReturnType<typeof watch> | undefined;
export const MIME_TYPES: Record<FontType, string> = {
    eot: 'application/vnd.ms-fontobject',
    svg: 'image/svg+xml',
    ttf: 'application/x-font-ttf',
    woff: 'application/font-woff',
    woff2: 'font/woff2',
};

export async function doesFileExist(folderPath: string, fileName: string): Promise<boolean> {
    const fileToFind = resolve(folderPath, fileName);
    try {
        await access(fileToFind, constants.R_OK);
        return true;
    } catch {
        return false;
    }
}

/** A normalized SVG change surfaced by the watcher. `path` matches the `parseFiles` output. */
export type WatchedChange = { path: string; kind: 'added' | 'changed' | 'removed' };
export type WatchedChangeBatch = WatchedChange[];

type FileChangeInfoType = ReturnType<typeof watch> extends AsyncIterable<infer T> ? T : never;

export async function handleWatchEvent(
    folderPath: string,
    { eventType, filename }: FileChangeInfoType,
    onChange: (change: WatchedChange) => void | Promise<void>,
    _doesFileExist: typeof doesFileExist = doesFileExist,
): Promise<void> {
    if (typeof filename !== 'string' || !filename.endsWith('.svg')) {
        return;
    }
    // Match the path shape `parseFiles` produces (`join(context, file)`).
    const path = pathJoin(folderPath, filename);
    const exists = await _doesFileExist(folderPath, filename);
    if (eventType === 'change') {
        // Content edit. A delete can also surface as 'change' on some platforms, so guard on existence.
        await onChange({ path, kind: exists ? 'changed' : 'removed' });
        return;
    }
    // 'rename' covers both create and delete; existence disambiguates.
    await onChange({ path, kind: exists ? 'added' : 'removed' });
}

const WATCH_BATCH_DELAY_MS = 25;

function coalesceChange(previous: WatchedChange | undefined, next: WatchedChange): WatchedChange {
    if (!previous) {
        return next;
    }
    if (next.kind === 'removed') {
        return next;
    }
    if (previous.kind === 'added') {
        return { ...next, kind: 'added' };
    }
    if (previous.kind === 'removed' && next.kind === 'added') {
        return { ...next, kind: 'changed' };
    }
    return next;
}

export async function setupWatcher(
    folderPath: string,
    signal: AbortSignal,
    onChange: (changes: WatchedChangeBatch) => void | Promise<void>,
    _handleWatchEvent: typeof handleWatchEvent = handleWatchEvent,
): Promise<void> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let flushChain = Promise.resolve();
    const pending = new Map<string, WatchedChange>();

    const flush = async () => {
        if (timer) {
            clearTimeout(timer);
            timer = undefined;
        }
        if (pending.size === 0) {
            return;
        }
        const changes = [...pending.values()];
        pending.clear();
        await onChange(changes);
    };

    const scheduleFlush = () => {
        flushChain = flushChain.then(flush).catch(() => undefined);
    };

    const drain = async () => {
        await flushChain;
        await flush().catch(() => undefined);
    };

    const queueChange = (change: WatchedChange) => {
        pending.set(change.path, coalesceChange(pending.get(change.path), change));
        if (timer) {
            clearTimeout(timer);
        }
        timer = setTimeout(() => {
            scheduleFlush();
        }, WATCH_BATCH_DELAY_MS);
    };

    try {
        watcher = watch(folderPath, { signal });
        for await (const event of watcher) {
            // A single failed event classification (e.g. an unreadable file) must not tear down the watcher.
            await _handleWatchEvent(folderPath, event, queueChange).catch(() => undefined);
        }
        await drain();
    } catch (err: unknown) {
        await drain();
        if (err && typeof err === 'object' && 'name' in err && err.name === 'AbortError') {
            return;
        }
        throw err;
    }
}

export function getBufferHash(buf: Buffer): string {
    return createHash('sha256').update(buf).digest('hex');
}

export function hasFileExtension(fileName?: string | null): boolean {
    const fileExtensionRegex = /(?:\.([^.]+))?$/;
    return Boolean(fileExtensionRegex.exec(fileName || '')?.[1]);
}

export async function ensureDirExistsAndWriteFile(content: string | Uint8Array, dest: string): Promise<void> {
    const options = { mode: 0o777, recursive: true };
    await mkdir(dirname(dest), options);
    await writeFile(dest, content);
}

export function getTmpDir(): string {
    return mkdtempSync(pathJoin(osTmpdir(), '__vite-svg-2-webfont-'));
}

export function rmDir(path: string): void {
    fsRm(path, { force: true, recursive: true }, /* v8 ignore next -- best-effort temp cleanup callback has no observable behavior */ () => {});
}

export function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binaryString = Buffer.from(base64, 'base64').toString('binary');
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
}
