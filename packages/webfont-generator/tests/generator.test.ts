import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdtemp, rm, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { afterEach, beforeAll, describe, expect, it } from 'vite-plus/test';
import { type FontType, generateWebfonts, type GenerateWebfontsInputOptions } from '../index.js';

const fixturesRoot = join(import.meta.dirname, '..', 'src', 'svg', 'fixtures');
const webfontFixtures = join(import.meta.dirname, '..', '..', 'vite-svg-2-webfont', 'src', 'fixtures', 'webfont-test', 'svg');

const cleanupDirs = new Set<string>();

afterEach(async () => {
    await Promise.all([...cleanupDirs].map(path => rm(path, { force: true, recursive: true })));
    cleanupDirs.clear();
});

async function createTempDir(prefix: string) {
    const path = await mkdtemp(join(tmpdir(), prefix));
    cleanupDirs.add(path);
    return path;
}

describe('generateWebfonts', () => {
    it('generates a ttf font and writes it to disk when ttf is requested', async () => {
        const dest = await createTempDir('vite-ttf-native-');
        const result = await generateWebfonts({
            codepoints: {
                add: 0xf201,
            },
            css: false,
            dest: `${dest}/`,
            files: [join(webfontFixtures, 'add.svg')],
            fontHeight: 1000,
            fontName: 'iconfont',
            order: ['ttf'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['ttf'],
            writeFiles: true,
        });

        expect(result.ttf).toBeInstanceOf(Uint8Array);
        expect(Buffer.from(result.ttf).subarray(0, 4)).toEqual(Buffer.from([0x00, 0x01, 0x00, 0x00]));
        expect(result.generateCss({ ttf: '/assets/iconfont.ttf' })).toContain('format("truetype")');
        await expect(readFile(`${dest}/iconfont.ttf`)).resolves.toEqual(expect.any(Buffer));
    });

    it('generates a deterministic svg font and writes requested svg/html/css outputs', async () => {
        const dest = await createTempDir('vite-svg-native-');
        const result = await generateWebfonts({
            codepoints: {
                add: 0xf201,
            },
            css: true,
            cssDest: `${dest}/iconfont.css`,
            dest: `${dest}/`,
            files: [join(webfontFixtures, 'add.svg')],
            fontHeight: 1000,
            fontName: 'iconfont',
            html: true,
            htmlDest: `${dest}/iconfont.html`,
            order: ['svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
            writeFiles: true,
        });

        expect(Buffer.from(result.svg).toString('utf8')).toContain('glyph-name="add"');
        expect(Buffer.from(result.svg).toString('utf8')).toContain('&#xF201;');
        expect(result.generateCss({ svg: '/assets/iconfont.svg' })).toContain('/assets/iconfont.svg');
        expect(result.generateHtml()).toContain('icon-add');
        await expect(readFile(`${dest}/iconfont.svg`, 'utf8')).resolves.toContain('glyph-name="add"');
        await expect(readFile(`${dest}/iconfont.css`, 'utf8')).resolves.toContain('@font-face');
        await expect(readFile(`${dest}/iconfont.html`, 'utf8')).resolves.toContain('<!DOCTYPE html>');
    });

    it('cssContext adds fields, preserves existing context, and can override fields', async () => {
        const dest = await createTempDir('vite-svg-css-context-');
        const templatePath = join(dest, 'css-context.hbs');
        await writeFile(templatePath, '{{fontName}}|{{baseSelector}}|{{custom}}|{{classPrefix}}');
        const result = await generateWebfonts({
            css: true,
            cssContext(context: Record<string, unknown>) {
                context.custom = 'added';
                context.classPrefix = 'overridden-';
            },
            cssTemplate: templatePath,
            dest: `${dest}/`,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
            writeFiles: false,
        } as never);

        expect(result.generateCss()).toBe('iconfont|.icon|added|overridden-');
    });

    it('htmlContext adds fields, preserves existing context, and can override fields', async () => {
        const dest = await createTempDir('vite-svg-html-context-');
        const templatePath = join(dest, 'html-context.hbs');
        await writeFile(templatePath, '{{fontName}}|{{baseSelector}}|{{custom}}|{{classPrefix}}');
        const result = await generateWebfonts({
            css: true,
            cssTemplate: join(import.meta.dirname, '..', 'templates', 'css.hbs'),
            dest: `${dest}/`,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            html: true,
            htmlContext(context: Record<string, unknown>) {
                context.custom = 'added';
                context.classPrefix = 'overridden-';
            },
            htmlTemplate: templatePath,
            order: ['svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
            writeFiles: false,
        } as never);

        expect(result.generateHtml()).toBe('iconfont|.icon|added|overridden-');
    });

    it('applies svgicons2svgfont-style svg options such as ligatures, fixed width, metadata, and font-face attributes', async () => {
        const result = await generateWebfonts({
            ascent: 900,
            codepoints: {
                add: 0xf201,
            } as never,
            context: '' as never,
            css: false,
            dest: `${await createTempDir('vite-svg-native-options-')}/`,
            files: [join(webfontFixtures, 'add.svg')],
            fixedWidth: true,
            fontHeight: 1000,
            fontName: 'iconfont',
            fontStyle: 'italic',
            fontWeight: '700',
            formatOptions: {
                svg: {
                    fontId: 'custom-font-id',
                    metadata: 'native-metadata',
                },
            },
            ligature: true,
            normalize: true,
            order: ['svg'],
            round: 100,
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
        } as never);

        const svg = Buffer.from(result.svg).toString('utf8');

        expect(svg).toContain('<metadata>native-metadata</metadata>');
        expect(svg).toContain('font id="custom-font-id"');
        expect(svg).toContain('font-weight="700"');
        expect(svg).toContain('font-style="italic"');
        expect(svg).toContain('ascent="900"');
        expect(svg).toContain('glyph-name="add-1"');
        expect(svg).toContain('unicode="&#x61;&#x64;&#x64;"');
    });

    it('can opt into post-processing the generated svg font output with svgtidy', async () => {
        const dest = `${await createTempDir('vite-svg-native-optimize-')}/`;
        const files = [join(webfontFixtures, 'add.svg')];
        const baseResult = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files,
            fontHeight: 1000,
            fontName: 'iconfont',
            order: ['svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
        } as never);
        const optimizedResult = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files,
            fontHeight: 1000,
            fontName: 'iconfont',
            formatOptions: {
                svg: {
                    optimizeOutput: true,
                },
            },
            order: ['svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
        } as never);

        const baseSvg = Buffer.from(baseResult.svg).toString('utf8');
        const optimizedSvg = Buffer.from(optimizedResult.svg).toString('utf8');

        expect(optimizedSvg).toContain('<glyph');
        expect(optimizedSvg.length).toBeLessThanOrEqual(baseSvg.length);
    });

    it('can generate svg and ttf together from the native path', async () => {
        const dest = `${await createTempDir('vite-svg-ttf-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['svg', 'ttf'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg', 'ttf'],
        } as never);

        expect(Buffer.from(result.svg).toString('utf8')).toContain('glyph-name="add"');
        expect(Buffer.from(result.ttf).subarray(0, 4)).toEqual(Buffer.from([0x00, 0x01, 0x00, 0x00]));
        expect(result.generateCss({ svg: '/assets/iconfont.svg', ttf: '/assets/iconfont.ttf' } as never)).toContain('format("svg")');
        expect(result.generateCss({ svg: '/assets/iconfont.svg', ttf: '/assets/iconfont.ttf' } as never)).toContain('format("truetype")');
    });

    it('can generate ttf and eot together from the native path', async () => {
        const dest = `${await createTempDir('vite-ttf-eot-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['eot', 'ttf'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['ttf', 'eot'],
        } as never);

        expect(Buffer.from(result.ttf).subarray(0, 4)).toEqual(Buffer.from([0x00, 0x01, 0x00, 0x00]));
        expect(Buffer.from(result.eot).subarray(34, 36).toString('ascii')).toBe('LP');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', ttf: '/assets/iconfont.ttf' } as never)).toContain('format("embedded-opentype")');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', ttf: '/assets/iconfont.ttf' } as never)).toContain('/assets/iconfont.eot?#iefix');
    });

    it('can generate ttf and woff together from the native path', async () => {
        const dest = `${await createTempDir('vite-ttf-woff-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['woff', 'ttf'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['ttf', 'woff'],
        } as never);

        expect(Buffer.from(result.ttf).subarray(0, 4)).toEqual(Buffer.from([0x00, 0x01, 0x00, 0x00]));
        expect(Buffer.from(result.woff).subarray(0, 4).toString('ascii')).toBe('wOFF');
        expect(result.generateCss({ ttf: '/assets/iconfont.ttf', woff: '/assets/iconfont.woff' } as never)).toContain('format("woff")');
    });

    it('can generate ttf and woff2 together from the native path', async () => {
        const dest = `${await createTempDir('vite-ttf-woff2-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['woff2', 'ttf'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['ttf', 'woff2'],
        } as never);

        expect(Buffer.from(result.ttf).subarray(0, 4)).toEqual(Buffer.from([0x00, 0x01, 0x00, 0x00]));
        expect(Buffer.from(result.woff2).subarray(0, 4).toString('ascii')).toBe('wOF2');
        expect(result.generateCss({ ttf: '/assets/iconfont.ttf', woff2: '/assets/iconfont.woff2' } as never)).toContain('format("woff2")');
    });

    it('can generate svg and eot together from the native path', async () => {
        const dest = `${await createTempDir('vite-svg-eot-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['eot', 'svg'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg', 'eot'],
        } as never);

        expect(Buffer.from(result.svg).toString('utf8')).toContain('glyph-name="add"');
        expect(Buffer.from(result.eot).subarray(34, 36).toString('ascii')).toBe('LP');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', svg: '/assets/iconfont.svg' } as never)).toContain('format("embedded-opentype")');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', svg: '/assets/iconfont.svg' } as never)).toContain('format("svg")');
    });

    it('can generate svg and woff together from the native path', async () => {
        const dest = `${await createTempDir('vite-svg-woff-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            formatOptions: {
                woff: {
                    metadata: '<metadata><uniqueid id="iconfont" /></metadata>',
                },
            },
            order: ['svg', 'woff'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg', 'woff'],
        } as never);

        expect(Buffer.from(result.svg).toString('utf8')).toContain('glyph-name="add"');
        expect(Buffer.from(result.woff).subarray(0, 4).toString('ascii')).toBe('wOFF');
        expect(result.generateCss({ svg: '/assets/iconfont.svg', woff: '/assets/iconfont.woff' } as never)).toContain('format("woff")');
    });

    it('can generate svg and woff2 together from the native path', async () => {
        const dest = `${await createTempDir('vite-svg-woff2-native-')}/`;
        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files: [join(webfontFixtures, 'add.svg')],
            fontName: 'iconfont',
            order: ['svg', 'woff2'],
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg', 'woff2'],
        } as never);

        expect(Buffer.from(result.svg).toString('utf8')).toContain('glyph-name="add"');
        expect(Buffer.from(result.woff2).subarray(0, 4).toString('ascii')).toBe('wOF2');
        expect(result.generateCss({ svg: '/assets/iconfont.svg', woff2: '/assets/iconfont.woff2' } as never)).toContain('format("woff2")');
    });

    it('passes preserveAspectRatio through to the native svg generator', async () => {
        const dest = `${await createTempDir('vite-svg-native-preserve-aspect-ratio-')}/`;
        const files = [join(fixturesRoot, 'icons/preserveaspectratio/square.svg')];

        const result = await generateWebfonts({
            context: '' as never,
            css: false,
            dest,
            files,
            fontName: 'preserveaspectratio',
            ligature: false,
            formatOptions: {
                svg: {
                    preserveAspectRatio: true,
                },
            },
            order: ['svg'],
            startCodepoint: 0xe001,
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
        } as never);

        const svg = Buffer.from(result.svg).toString('utf8');
        const expected = await readFile(join(fixturesRoot, 'expected/preserveaspectratio-preserved.svg'), 'utf8');

        expect(svg).toBe(expected);
    });

    it('rejects when an svg file path does not exist', async () => {
        await expect(
            generateWebfonts({
                context: '' as never,
                css: false,
                dest: `${await createTempDir('vite-svg-native-missing-file-')}/`,
                files: [join(webfontFixtures, 'does-not-exist.svg')],
                fontName: 'iconfont',
                order: ['svg'],
                templateOptions: {
                    baseSelector: '.icon',
                    classPrefix: 'icon-',
                },
                types: ['svg'],
            } as never),
        ).rejects.toThrow(expect.objectContaining({ message: expect.stringContaining('Failed to read source SVG file') }));
    });
});

// musl's libm diverges from glibc/macOS/Windows at the ULP level, so the float geometry in
// TTF generation (kurbo's cubic->quadratic conversion) rounds a few coordinates differently
// and the exact byte sizes shift by a handful of bytes (smaller TTF, slightly larger woff2).
// The output is reproducible within a libc family but not across musl, so the exact-byte
// snapshots below can't run there. Detected via the glibc marker in process.report, which is
// present on glibc Node and absent on musl (Alpine); falls back to running the test if the
// report is unavailable rather than skipping silently.
function isMuslLinux(): boolean {
    if (process.platform !== 'linux') return false;
    try {
        const report = process.report?.getReport?.() as { header?: { glibcVersionRuntime?: string } } | undefined;
        return report?.header !== undefined && report.header.glibcVersionRuntime === undefined;
    } catch {
        return false;
    }
}

describe('output size (deterministic)', () => {
    // Build one font from the first 300 real icons of @iconify-json/simple-icons and generate
    // every variant once. Output bytes are a pure function of the inputs + options, so these are
    // exact regression guards.
    const ICON_COUNT = 300;
    const woff2ByQuality = {} as Record<`q${9 | 10 | 11}`, number>;
    const perFormat = {} as Record<FontType, number>;

    beforeAll(async () => {
        const iconSet = createRequire(import.meta.url)('@iconify-json/simple-icons/icons.json') as {
            width?: number;
            height?: number;
            icons: Record<string, { body: string; width?: number; height?: number }>;
        };
        const dir = await createTempDir('vite-size-');
        const slugs = Object.keys(iconSet.icons).slice(0, ICON_COUNT);
        const files = slugs.map((_, i) => join(dir, `i${String(i).padStart(3, '0')}.svg`));
        await Promise.all(
            slugs.map((slug, i) => {
                const icon = iconSet.icons[slug];
                const w = icon.width ?? iconSet.width ?? 24;
                const h = icon.height ?? iconSet.height ?? 24;
                return writeFile(files[i], `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}">${icon.body}</svg>`);
            }),
        );

        // `fontHeight` is pinned so the em square (and thus byte sizes) is stable.
        const base: GenerateWebfontsInputOptions = {
            files,
            dest: `${dir}/`,
            fontName: 'size',
            fontHeight: 24,
            css: false,
            writeFiles: false,
            optimizeOutput: true,
            types: ['woff2'],
        };

        const [nine, ten, eleven, all] = await Promise.all([
            generateWebfonts({ ...base, formatOptions: { woff2: { compressionQuality: 9 } } }),
            generateWebfonts({ ...base, formatOptions: { woff2: { compressionQuality: 10 } } }),
            generateWebfonts({ ...base, formatOptions: { woff2: { compressionQuality: 11 } } }),
            generateWebfonts({ ...base, types: ['svg', 'ttf', 'eot', 'woff', 'woff2'] }),
        ]);
        Object.assign(woff2ByQuality, { q9: nine.woff2.length, q10: ten.woff2.length, q11: eleven.woff2.length });
        Object.assign(perFormat, { svg: all.svg.length, ttf: all.ttf.length, eot: all.eot.length, woff: all.woff.length, woff2: all.woff2.length });
    });

    it('woff2 compression quality defaults to 11', () => {
        expect(perFormat.woff2).toBe(woff2ByQuality.q11);
    });

    it('woff2 size by brotli compression quality', { skip: isMuslLinux() }, () => {
        expect(woff2ByQuality).toMatchInlineSnapshot(`
          {
            "q10": 22860,
            "q11": 22320,
            "q9": 24900,
          }
        `);
    });

    it('per-format output sizes', { skip: isMuslLinux() }, () => {
        expect(perFormat).toMatchInlineSnapshot(`
          {
            "eot": 60944,
            "svg": 817318,
            "ttf": 60792,
            "woff": 29980,
            "woff2": 22320,
          }
        `);
    });
});

const REGEN_PATHS: Record<string, string> = {
    a: 'M2 2 L22 2 L22 22 Z',
    b: 'M2 2 L22 2 L12 22 Z',
    c: 'M4 4 L20 4 L20 20 L4 20 Z',
    changed: 'M0 0 L24 0 L24 24 Z',
};
const regenIcon = (d: string) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="${d}"/></svg>`;

async function writeRegenIcon(dir: string, name: string, key: string) {
    const path = join(dir, `${name}.svg`);
    await writeFile(path, regenIcon(REGEN_PATHS[key]));
    return path;
}

const regenBaseOpts = (dir: string, files: string[]): GenerateWebfontsInputOptions => ({
    files,
    dest: `${dir}/`,
    fontName: 'rc',
    fontHeight: 24,
    css: false,
    writeFiles: false,
    types: ['svg', 'ttf', 'eot', 'woff', 'woff2'],
});

// Normalize to a plain Uint8Array so a Node Buffer from `readFile` compares equal to a font getter.
const toBytes = (value: Uint8Array) => Uint8Array.from(value);
const isFontByteEqual = (a: Uint8Array, b: Uint8Array) => toBytes(a).toString() === toBytes(b).toString();

expect.extend({
    toEqualFont(received: Awaited<ReturnType<typeof generateWebfonts>>, expected: Awaited<ReturnType<typeof generateWebfonts>>) {
        const isSvgEqual = received.svg === expected.svg;
        const isTtfEqual = isFontByteEqual(received.ttf, expected.ttf);
        const isEotEqual = isFontByteEqual(received.eot, expected.eot);
        const isWoffEqual = isFontByteEqual(received.woff, expected.woff);
        const isWoff2Equal = isFontByteEqual(received.woff2, expected.woff2);
        const pass = isSvgEqual && isTtfEqual && isEotEqual && isWoffEqual && isWoff2Equal;
        return {
            pass,
            message: () => `expected ${pass ? 'not ' : ''}to equal font bytes`,
        };
    },
    toEqualCss(received: Awaited<ReturnType<typeof generateWebfonts>>, expected: Awaited<ReturnType<typeof generateWebfonts>>) {
        const pass = received.generateCss() === expected.generateCss();
        return {
            pass,
            message: () => `expected ${pass ? 'not ' : ''}to equal generated CSS`,
        };
    },
});

declare module 'vite-plus/test' {
    interface Matchers<T = any> {
        toEqualFont(expected: Awaited<ReturnType<typeof generateWebfonts>>): ReturnType<typeof expect.extend>;
        toEqualCss(expected: Awaited<ReturnType<typeof generateWebfonts>>): ReturnType<typeof expect.extend>;
    }
}

describe('regenerate (incremental)', () => {
    it('matches a fresh build after a content change', async () => {
        const dir = await createTempDir('regen-change-');
        const [a, b, c] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b'), writeRegenIcon(dir, 'c', 'c')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b, c]), incremental: true });

        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        result.regenerate([a, b, c], [{ path: b, changeType: 'changed' }]);

        expect(result).toEqualFont(await generateWebfonts(regenBaseOpts(dir, [a, b, c])));
    });

    it('matches a fresh build after adding a file', async () => {
        const dir = await createTempDir('regen-add-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b]), incremental: true });

        const c = await writeRegenIcon(dir, 'c', 'c');
        result.regenerate([a, b, c], [{ path: c, changeType: 'added' }]);

        const fresh = await generateWebfonts(regenBaseOpts(dir, [a, b, c]));
        expect(result).toEqualFont(fresh);
        expect(result).toEqualCss(fresh);
    });

    it('matches a fresh build after adding a file that sorts before existing glyphs', async () => {
        const dir = await createTempDir('regen-add-mid-');
        const [b, c] = await Promise.all([writeRegenIcon(dir, 'b', 'b'), writeRegenIcon(dir, 'c', 'c')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [b, c]), incremental: true });

        const a = await writeRegenIcon(dir, 'a', 'a');
        // The fresh-build order is [a, b, c]; passing it ensures the addition lands first, not at the tail.
        result.regenerate([a, b, c], [{ path: a, changeType: 'added' }]);

        expect(result).toEqualFont(await generateWebfonts(regenBaseOpts(dir, [a, b, c])));
    });

    it('matches a fresh build after removing a file', async () => {
        const dir = await createTempDir('regen-remove-');
        const [a, b, c] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b'), writeRegenIcon(dir, 'c', 'c')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b, c]), incremental: true });

        result.regenerate([a, c], [{ path: b, changeType: 'removed' }]);

        const fresh = await generateWebfonts(regenBaseOpts(dir, [a, c]));
        expect(result).toEqualFont(fresh);
        expect(result).toEqualCss(fresh);
    });

    it('matches a fresh build after re-diffing omitted changes', async () => {
        const dir = await createTempDir('regen-rediff-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b]), incremental: true });

        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        const c = await writeRegenIcon(dir, 'c', 'c');
        result.regenerate([a, b, c]);

        const fresh = await generateWebfonts(regenBaseOpts(dir, [a, b, c]));
        expect(result).toEqualFont(fresh);
        expect(result).toEqualCss(fresh);
    });

    it('matches a fresh build after re-diffing null changes', async () => {
        const dir = await createTempDir('regen-rediff-null-');
        const [a, b, c] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b'), writeRegenIcon(dir, 'c', 'c')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b, c]), incremental: true });

        await writeFile(c, regenIcon(REGEN_PATHS.changed));
        result.regenerate([a, c], null);

        const fresh = await generateWebfonts(regenBaseOpts(dir, [a, c]));
        expect(result).toEqualFont(fresh);
        expect(result).toEqualCss(fresh);
    });

    it('reuses the CSS render on a content edit and re-renders on rename', async () => {
        const dir = await createTempDir('regen-css-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const urls = { woff2: '/static/icons.woff2' };
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b]), incremental: true });
        const before = result.generateCss(urls);

        // Content edit keeps names/codepoints → CSS reused verbatim and equal to a fresh build.
        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        result.regenerate([a, b], [{ path: b, changeType: 'changed' }]);
        expect(result.generateCss(urls)).toBe(before);
        const fresh = await generateWebfonts(regenBaseOpts(dir, [a, b]));
        expect(result.generateCss(urls)).toBe(fresh.generateCss(urls));

        // A rename changes a glyph name the template reads → CSS must re-render.
        result.regenerate([a, b], [{ path: b, changeType: 'changed', name: 'renamed' }]);
        expect(result.generateCss(urls)).not.toBe(before);
        expect(result.generateCss(urls)).toContain('renamed');
    });

    it('refreshes on-disk outputs when writeFiles is true, and skips unchanged ones', async () => {
        const dir = await createTempDir('regen-write-src-');
        const dest = await createTempDir('regen-write-out-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const opts: GenerateWebfontsInputOptions = { files: [a, b], dest, fontName: 'rc', fontHeight: 24, css: true, writeFiles: true, incremental: true, types: ['woff2'] };
        const result = await generateWebfonts(opts);

        const woff2Path = join(dest, 'rc.woff2');
        const cssPath = join(dest, 'rc.css');
        const [woff2Before, cssBefore] = await Promise.all([readFile(woff2Path), readFile(cssPath)]);

        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        result.regenerate([a, b], [{ path: b, changeType: 'changed' }]);

        const [woff2After, cssAfter] = await Promise.all([readFile(woff2Path), readFile(cssPath)]);
        expect(woff2After).not.toEqual(woff2Before);
        expect(cssAfter).not.toEqual(cssBefore);
        // Disk matches the rebuilt in-memory bytes, and a fresh build of the new set.
        expect(toBytes(woff2After)).toEqual(toBytes(result.woff2));
        const fresh = await generateWebfonts({ ...opts, writeFiles: false, incremental: false });
        expect(toBytes(woff2After)).toEqual(toBytes(fresh.woff2));

        // A no-op regenerate reproduces identical output, so the write is skipped: a deleted file
        // is not recreated.
        await rm(woff2Path);
        result.regenerate([a, b], [{ path: b, changeType: 'changed' }]);
        await expect(readFile(woff2Path)).rejects.toThrow(/ENOENT/);
    });

    it('throws when regenerate is called without incremental', async () => {
        const dir = await createTempDir('regen-noinc-');
        const a = await writeRegenIcon(dir, 'a', 'a');
        const result = await generateWebfonts(regenBaseOpts(dir, [a]));

        expect(() => result.regenerate([a], [{ path: a, changeType: 'changed' }])).toThrow(/incremental/);
    });
});
