import { resolve } from 'path';
import { constants } from 'fs';
import { watch, access, FileChangeInfo } from 'fs/promises';

let watcher: ReturnType<typeof watch> | undefined;
export const MIME_TYPES = {
    eot: 'application/vnd.ms-fontobject',
    svg: 'image/svg+xml',
    ttf: 'application/x-font-ttf',
    woff: 'application/font-woff',
    woff2: 'font/woff2',
} as const;

async function doesFileExist(folderPath: string, fileName: string) {
    const fileToFind = resolve(folderPath, fileName);
    try {
        await access(fileToFind, constants.R_OK);
        return true;
    } catch {
        return false;
    }
}

async function handleWatchEvent(folderPath: string, { eventType, filename }: FileChangeInfo<string>, onIconAdded: (e: FileChangeInfo<string>) => void) {
    if (eventType !== 'rename' || !filename.endsWith('.svg') || !(await doesFileExist(folderPath, filename))) {
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
