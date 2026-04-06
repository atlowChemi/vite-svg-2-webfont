import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdtemp, rm, readFile, writeFile } from 'node:fs/promises';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import { generateWebfonts } from '../index.js';

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
