import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdir, mkdtemp, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { clearCache, generateWebfonts, type FontType, type MultiVariantFontType } from '../index.js';

const fixtures = join(import.meta.dirname, '..', '..', '..', 'crates', 'webfont-generator', 'src', 'svg', 'fixtures', 'icons');
const icons = ['plus.svg', 'minus.svg', 'close.svg'].map(file => join(fixtures, 'cleanicons', file));
const types: FontType[] = ['svg', 'ttf', 'eot', 'woff', 'woff2'];
const variantTypes: MultiVariantFontType[] = ['ttf', 'woff', 'woff2'];
const variants = [
    { name: 'light', weight: 300, default: true, files: ['basket.svg', 'plus.svg'].map(file => join(fixtures, 'cleanicons', file)) },
    { name: 'bold', weight: 700, files: [join(fixtures, 'multipathicons', 'basket.svg'), join(fixtures, 'cleanicons', 'plus.svg')] },
];
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

/** A two-layer color icon whose first layer uses `color`. */
function coloredStar(color: string) {
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="${color}" d="M2 2H18V20H2Z"/><path fill="#0066ff" d="M6 6H12V12H6Z"/></svg>`;
}

async function fonts(cacheLocation?: string) {
    const result = await generateWebfonts({ dest: tmpdir(), files: icons, types, css: false, writeFiles: false, cacheLocation });
    return types.map(type => Buffer.from(result[type]!));
}

async function variantFonts(cacheLocation?: string) {
    const result = await generateWebfonts({ dest: tmpdir(), variants, types: variantTypes, css: false, writeFiles: false, cacheLocation });
    return variantTypes.map(type => Buffer.from(result[type]!));
}

describe('cacheLocation', () => {
    it('automatically persists incremental updates and restores TypeScript output on cache hits', { timeout: 15_000 }, async () => {
        const dir = await tempDir();
        const icon = join(dir, 'star.svg');
        await writeFile(icon, coloredStar('#ff9900'));
        const options = {
            dest: dir,
            files: [icon],
            types,
            css: false,
            incremental: true as const,
            cacheLocation: join(dir, 'cache'),
            ts: true,
            tsDest: join(dir, 'custom.ts'),
            formatOptions: { ttf: { ts: 0 } },
        };
        const original = await generateWebfonts(options);
        const cache = join(options.cacheLocation, (await readdir(options.cacheLocation))[0]);
        const before = await readFile(cache);
        await writeFile(icon, coloredStar('#00cc66').replace('H18', 'H16'));
        const replacement = await original.regenerateAsync({ files: [icon] });
        const extra = join(dir, 'extra.svg');
        await writeFile(extra, coloredStar('#000000'));
        const latest = await replacement.addAsync(extra);
        await vi.waitFor(async () => expect(await readFile(cache)).not.toEqual(before), { timeout: 10_000 });
        await original.flushCache(); // An old snapshot waits for the lineage; it cannot save stale fonts.
        expect(await readFile(cache)).not.toEqual(before);
        const identity = await stat(cache);
        await rm(options.tsDest);
        const warm = await generateWebfonts({ ...options, files: [icon, extra] });
        expect((await stat(cache)).ino).toBe(identity.ino);
        expect(await readFile(options.tsDest, 'utf8')).toBe(latest.generateTs());
        expect(warm.generateTs()).toContain('"extra"');
        const uncached = await generateWebfonts({ ...options, files: [icon, extra], cacheLocation: undefined });
        for (const type of types) expect(warm[type]).toEqual(uncached[type]);
        expect(warm.generateTs()).toBe(uncached.generateTs());
        await expect(uncached.flushCache()).resolves.toBeUndefined();
    });

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

    it('produces the same variant family fonts cold, warm, and from a corrupt file', async () => {
        const cacheLocation = await tempDir();
        const uncached = await variantFonts();

        expect(await variantFonts(cacheLocation)).toEqual(uncached);
        const name = (await readdir(cacheLocation)).find(file => file.endsWith('.wfcache'));
        const file = join(cacheLocation, name as string);
        const valid = await readFile(file);
        expect(await variantFonts(cacheLocation)).toEqual(uncached);

        const corrupt = Buffer.from(valid);
        corrupt[corrupt.length - 10] ^= 0xff;
        await writeFile(file, corrupt);
        expect(await variantFonts(cacheLocation)).toEqual(uncached);
        expect(await readFile(file)).toEqual(valid);
    });

    it('produces the same color glyph fonts cold, warm, and after a format option or icon change', async () => {
        const dir = await tempDir();
        const star = join(dir, 'star.svg');
        await writeFile(star, coloredStar('#ff9900'));
        const colorTypes: FontType[] = ['ttf', 'woff2'];
        const colorFonts = async (cacheLocation?: string, compressionQuality?: number) => {
            const result = await generateWebfonts({
                dest: tmpdir(),
                files: [...icons, star],
                types: colorTypes,
                colorGlyphs: ['star'],
                formatOptions: compressionQuality === undefined ? undefined : { woff2: { compressionQuality } },
                css: false,
                writeFiles: false,
                cacheLocation,
            });
            return colorTypes.map(type => Buffer.from(result[type]!));
        };
        const cacheLocation = join(dir, 'cache');
        expect(await colorFonts(cacheLocation)).toEqual(await colorFonts());
        expect(await colorFonts(cacheLocation)).toEqual(await colorFonts());
        expect(await colorFonts(cacheLocation, 9)).toEqual(await colorFonts(undefined, 9));
        await writeFile(star, coloredStar('#00cc66'));
        expect(await colorFonts(cacheLocation, 9)).toEqual(await colorFonts(undefined, 9));
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
