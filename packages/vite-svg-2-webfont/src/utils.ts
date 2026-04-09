import { createHash } from 'node:crypto';
import { tmpdir as osTmpdir } from 'node:os';
import { constants, rm as fsRm, mkdtempSync } from 'node:fs';
import { resolve, dirname, join as pathJoin } from 'node:path';
import { watch, access, mkdir, writeFile } from 'node:fs/promises';
import type { FileChangeInfo } from 'node:fs/promises';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

let watcher: ReturnType<typeof watch> | undefined;
export const MIME_TYPES: Record<GeneratedFontTypes, string> = {
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

export async function handleWatchEvent(
    folderPath: string,
    { eventType, filename }: FileChangeInfo<string>,
    onIconAdded: (e: FileChangeInfo<string>) => void | Promise<void>,
    _doesFileExist: typeof doesFileExist = doesFileExist,
): Promise<void> {
    if (eventType !== 'rename' || !filename?.endsWith('.svg') || !(await _doesFileExist(folderPath, filename))) {
        return;
    }
    await onIconAdded({ eventType, filename });
}

export async function setupWatcher(folderPath: string, signal: AbortSignal, handler: (e: FileChangeInfo<string>) => void | Promise<void>): Promise<void> {
    try {
        watcher = watch(folderPath, { signal });
        for await (const event of watcher) {
            await handleWatchEvent(folderPath, event, handler);
        }
    } catch (err) {
        if (err.name === 'AbortError') {
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

export async function ensureDirExistsAndWriteFile(content: string | Buffer, dest: string): Promise<void> {
    const options = { mode: 0o777, recursive: true };
    await mkdir(dirname(dest), options);
    await writeFile(dest, content);
}

export function getTmpDir(): string {
    return mkdtempSync(pathJoin(osTmpdir(), '__vite-svg-2-webfont-'));
}

export function rmDir(path: string): void {
    fsRm(path, { force: true, recursive: true }, () => {});
}

export function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binaryString = Buffer.from(base64, 'base64').toString('binary');
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
}
