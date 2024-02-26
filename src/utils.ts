import { constants } from 'fs';
import { resolve, dirname } from 'path';
import { watch, access, mkdir, writeFile } from 'fs/promises';
import type { FileChangeInfo } from 'fs/promises';
import type { GeneratedFontTypes } from '@vusion/webfonts-generator';

let watcher: ReturnType<typeof watch> | undefined;
export const MIME_TYPES: Record<GeneratedFontTypes, string> = {
    eot: 'application/vnd.ms-fontobject',
    svg: 'image/svg+xml',
    ttf: 'application/x-font-ttf',
    woff: 'application/font-woff',
    woff2: 'font/woff2',
};

export async function doesFileExist(folderPath: string, fileName: string) {
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
    onIconAdded: (e: FileChangeInfo<string>) => void,
    _doesFileExist = doesFileExist,
) {
    if (eventType !== 'rename' || !filename?.endsWith('.svg') || !(await _doesFileExist(folderPath, filename))) {
        return;
    }
    onIconAdded({ eventType, filename });
}

export async function setupWatcher(folderPath: string, signal: AbortSignal, handler: (e: FileChangeInfo<string>) => void) {
    try {
        watcher = watch(folderPath, { signal });
        for await (const event of watcher) {
            handleWatchEvent(folderPath, event, handler);
        }
    } catch (err) {
        if (err.name === 'AbortError') {
            return;
        }
        throw err;
    }
}

const alphabet = 'qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890';
export function guid(length = 8) {
    let result = '';
    for (let i = 0; i < length; i++) {
        const index = Math.floor(Math.random() * alphabet.length);
        result += alphabet[index];
    }
    return result;
}

export function hasFileExtension(fileName?: string | null | undefined) {
    const fileExtensionRegex = /(?:\.([^.]+))?$/;
    return Boolean(fileExtensionRegex.exec(fileName || '')?.[1]);
}

export async function ensureDirExistsAndWriteFile(content: string | Buffer, dest: string) {
    const options = { mode: 0o777, recursive: true };
    await mkdir(dirname(dest), options);
    await writeFile(dest, content);
}
