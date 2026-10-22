import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import { clearCache, generateWebfonts, type FontType } from '../index.js';

const icons = ['plus.svg', 'minus.svg', 'close.svg'].map(file =>
    join(import.meta.dirname, '..', '..', '..', 'crates', 'webfont-generator', 'src', 'svg', 'fixtures', 'icons', 'cleanicons', file),
);
const types: FontType[] = ['svg', 'ttf', 'eot', 'woff', 'woff2'];
const cleanupDirs = new Set<string>();

afterEach(async () => {
    await Promise.all([...cleanupDirs].map(path => rm(path, { force: true, recursive: true })));
    cleanupDirs.clear();
});

async function tempDir() {
    const path = await mkdtemp(join(tmpdir(), 'webfont-cache-'));
    cleanupDirs.add(path);
    return path;
}

async function fonts(cacheLocation?: string) {
    const result = await generateWebfonts({ dest: tmpdir(), files: icons, types, css: false, writeFiles: false, cacheLocation });
    return types.map(type => Buffer.from(result[type]!));
}

describe('cacheLocation', () => {
    it('produces the same fonts cold and warm', async () => {
        const cacheLocation = await tempDir();
        const uncached = await fonts();

        expect(await fonts(cacheLocation)).toEqual(uncached);
        expect((await readdir(cacheLocation)).filter(name => name.endsWith('.wfcache'))).toHaveLength(1);
        expect(await fonts(cacheLocation)).toEqual(uncached);
    });

    describe('unusable cache files fall back to a correct uncached run and are replaced', () => {
        const corruptions: [string, (valid: Buffer) => Buffer][] = [
            ['an invalid header', () => Buffer.from('not a webfont cache file')],
            ['an empty file', () => Buffer.alloc(0)],
            ['an unknown format version', valid => Buffer.concat([valid.subarray(0, 8), Buffer.from([0xff]), valid.subarray(9)])],
            ['a valid header with a truncated body', valid => valid.subarray(0, valid.length >> 1)],
            [
                'a valid header with a flipped byte inside a stored font',
                valid => {
                    const copy = Buffer.from(valid);
                    copy[copy.length - 10] ^= 0xff;
                    return copy;
                },
            ],
            ['a valid file with trailing bytes', valid => Buffer.concat([valid, Buffer.from('extra')])],
        ];
        it.each(corruptions)('%s', async (_, corrupt) => {
            const cacheLocation = await tempDir();
            const uncached = await fonts();
            await fonts(cacheLocation);
            const name = (await readdir(cacheLocation)).find(file => file.endsWith('.wfcache'));
            const file = join(cacheLocation, name as string);
            const valid = await readFile(file);

            await writeFile(file, corrupt(valid));
            expect(await fonts(cacheLocation)).toEqual(uncached);
            expect(await readFile(file)).toEqual(valid);
        });

        it('a directory where the cache file belongs', async () => {
            const cacheLocation = await tempDir();
            const uncached = await fonts();
            await fonts(cacheLocation);
            const [name] = await readdir(cacheLocation);
            await rm(join(cacheLocation, name));
            await mkdir(join(cacheLocation, name));

            expect(await fonts(cacheLocation)).toEqual(uncached);
        });

        it('a cache location that cannot be created', async () => {
            const parent = await tempDir();
            await writeFile(join(parent, 'file'), 'not a directory');
            expect(await fonts(join(parent, 'file', 'cache'))).toEqual(await fonts());
        });
    });

    it('rejects an empty location', async () => {
        await expect(generateWebfonts({ dest: tmpdir(), files: icons, writeFiles: false, cacheLocation: '' })).rejects.toThrow('"options.cacheLocation" must not be empty.');
    });
});

describe('clearCache', () => {
    it('removes only cache files and reports what it removed', async () => {
        const cacheLocation = await tempDir();
        await fonts(cacheLocation);
        await writeFile(join(cacheLocation, 'notes.txt'), 'keep');

        const report = await clearCache(cacheLocation);
        expect(report.filesRemoved).toBe(1);
        expect(report.bytesRemoved).toBeGreaterThan(0);
        expect(await readdir(cacheLocation)).toEqual(['notes.txt']);
    });

    it('rejects when the location is not a directory', async () => {
        const file = join(await tempDir(), 'file.txt');
        await writeFile(file, 'not a directory');
        await expect(clearCache(file)).rejects.toThrow(/os error/);
    });

    it('resolves with zero counts for a missing directory', async () => {
        await expect(clearCache(join(await tempDir(), 'missing'))).resolves.toEqual({ filesRemoved: 0, bytesRemoved: 0 });
    });
});
