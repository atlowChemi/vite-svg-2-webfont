import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { mkdtemp, rm, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { compileString } from 'sass';
import * as templates from '../templates.js';
import { afterEach, beforeAll, describe, expect, it } from 'vite-plus/test';
import { generateWebfonts as generateNativeBinding } from '../binding.js';
import { type FontType, generateWebfonts, type GenerateWebfontsFileOptions, type GenerateWebfontsVariantOptions } from '../index.js';

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
    const renameFiles = ['plus.svg', 'minus.svg', 'close.svg'].map(file => join(fixturesRoot, 'icons', 'cleanicons', file));
    const renameOptions = {
        css: false,
        dest: tmpdir(),
        files: renameFiles,
        fontName: 'renamed',
        html: false,
        types: ['svg'] as FontType[],
        writeFiles: false,
    };
    const variantOptions = {
        dest: tmpdir(),
        types: ['woff2'],
        variants: [
            { default: true, files: [renameFiles[0]], name: 'small', weight: 300 },
            { files: [renameFiles[0]], name: 'large', weight: 700 },
        ],
        writeFiles: false,
    } satisfies GenerateWebfontsVariantOptions;

    it('generates shared modern formats through existing getters', async () => {
        const result = await generateWebfonts({ ...variantOptions, fontName: 'variant-test', variantClassPrefix: 'weight--' });

        expect(result.woff2).toBeInstanceOf(Uint8Array);
        expect(result.eot).toBeNull();
        expect(result.svg).toBeNull();
    });

    it('writes multi-face companions and keeps URL render caches independent', async () => {
        const dest = await createTempDir('variant-companions-');
        const result = await generateWebfonts({ ...variantOptions, dest, html: true, writeFiles: true });
        const css = result.generateCss();
        expect(await readFile(join(dest, 'iconfont.css'), 'utf8')).toBe(css);
        expect(await readFile(join(dest, 'iconfont.html'), 'utf8')).toBe(result.generateHtml());
        expect(css.match(/@font-face/g)).toHaveLength(2);
        const templated = await generateWebfonts({ ...variantOptions, dest, html: true, cssTemplate: templates.css, htmlTemplate: templates.html });
        const urls = { woff2: '/shared.woff2' };
        expect(templated.generateCss(urls)).toBe(result.generateCss(urls));
        expect(templated.generateHtml(urls)).toBe(result.generateHtml(urls));
        expect(result.generateHtml().match(/class="icon icon-plus"/g)).toHaveLength(1);
        for (const url of ['/first.woff2', '/second.woff2', '/first.woff2']) {
            expect(result.generateCss({ woff2: url }).split(url)).toHaveLength(3);
            expect(result.generateHtml({ woff2: url }).split(url)).toHaveLength(3);
        }
        expect(result.generateCss()).toBe(css);
    });

    it('keeps variant rendering consistent when callbacks remove every face', async () => {
        const options = {
            ...variantOptions,
            cssContext(context: Record<string, unknown>) {
                context.variants = [];
            },
        };
        const builtin = await generateWebfonts(options);
        const templated = await generateWebfonts({ ...options, cssTemplate: templates.css });
        expect(builtin.generateCss()).toBe(templated.generateCss());
        expect(builtin.generateCss()).not.toContain('@font-face');
        expect(builtin.generateCss()).toContain('font-synthesis: none');
    });

    it('reports variant companion write failures', async () => {
        const dest = await createTempDir('variant-write-error-');
        const blocker = join(dest, 'blocker');
        await writeFile(blocker, 'not a directory');
        await expect(generateWebfonts({ ...variantOptions, dest, cssDest: join(blocker, 'font.css'), writeFiles: true })).rejects.toThrow(/directory|exists/i);
    });

    it('compiles SCSS with ordinary and multiple variant families in either import order', async () => {
        const variant = await generateWebfonts({ ...variantOptions, cssTemplate: templates.scss });
        const second = await generateWebfonts({
            ...variantOptions,
            cssTemplate: templates.scss,
            fontName: 'second',
            variants: variantOptions.variants.map((item, index) => ({ ...item, weight: index ? 800 : 200 })),
            rename: () => 'other',
        });
        const ordinary = await generateWebfonts({ ...renameOptions, cssTemplate: templates.scss, css: true });
        for (const sources of [
            [variant, second],
            [second, variant],
        ]) {
            const { css } = compileString(
                sources.map(result => result.generateCss()).join('\n') + '\n.first { @include webfont-icon("plus"); }\n.second { @include webfont-icon("other"); }',
                { logger: { warn() {}, debug() {} } },
            );
            expect(css.match(/\.first\.icon--large:before \{([^}]+)\}/)?.[1]).toContain('font-weight: 700 !important');
            expect(css.match(/\.second\.icon--large:before \{([^}]+)\}/)?.[1]).toContain('font-weight: 800 !important');
            expect(css).not.toMatch(/^\.icon--/m);
        }
        for (const [index, sources] of [
            [ordinary, variant, second],
            [second, variant, ordinary],
        ].entries()) {
            const css = compileString(
                sources.map(result => result.generateCss()).join('\n') + '\n.default { @include webfont-icon("plus"); }\n.other { @include webfont-icon("other"); }',
                { logger: { warn() {}, debug() {} } },
            ).css;
            // The last family defining plus owns that map entry; other always belongs to second.
            const defaultRule = css.match(/\.default:before \{([^}]+)\}/)?.[1];
            const otherRule = css.match(/\.other:before \{([^}]+)\}/)?.[1];
            expect(defaultRule).toContain(`font-weight: ${index ? 'normal !important' : '300'};`);
            expect(otherRule).toContain('font-family: "second" !important');
            expect(otherRule).toContain('font-weight: 200;');
            expect(otherRule).toContain('font-synthesis: none');
            expect(otherRule).toContain('content: "\\f101"');
            expect(css.match(/\.other\.icon--large:before \{([^}]+)\}/)?.[1]).toContain('font-weight: 800 !important');
            expect(css.match(/\.other\.icon--small:before \{([^}]+)\}/)?.[1]).toContain('font-weight: 200 !important');
            expect(css).not.toMatch(/^\.icon--/m);
            expect(css).toContain('font-synthesis: none');
            expect(css).toContain('content:');
        }
    });

    it.each([undefined, templates.css, templates.scss])('keeps arbitrary ordinary template variants from enabling multi-face rendering (%s)', async cssTemplate => {
        const options = { ...renameOptions, css: true, cssTemplate };
        const baseline = await generateWebfonts(options);
        const result = await generateWebfonts({ ...options, templateOptions: { variants: [{ name: 'unrelated' }], __webfontVariantMode: true } });
        expect(result.generateCss({ svg: '/ordinary.svg' })).toBe(baseline.generateCss({ svg: '/ordinary.svg' }));
    });

    it.each([undefined, templates.css])('preserves finalized CSS callback styles in HTML with destination-relative URLs (%s)', async cssTemplate => {
        const dest = await createTempDir('variant-callback-styles-');
        const result = await generateWebfonts({
            ...variantOptions,
            dest,
            cssDest: join(dest, 'styles', 'icons.css'),
            htmlDest: join(dest, 'preview', 'nested', 'icons.html'),
            cssTemplate,
            html: true,
            writeFiles: true,
            cssContext(context) {
                context.fontStyle = 'italic';
                context.baseSelector = '.preview';
                context.classPrefix = 'preview-';
                context.defaultWeight = 350;
                (context.variants as Array<{ weight: number }>)[0].weight = 350;
            },
        });
        const css = result.generateCss();
        const html = result.generateHtml();
        for (const output of [css, html, result.generateHtml({ woff2: '/override.woff2' })]) {
            expect(output).toContain('font-style: italic');
            expect(output).toContain('font-weight: 350');
            expect(output).not.toContain('font-weight: 300');
        }
        expect(css).toContain('url("iconfont.woff2?');
        expect(html).toContain('../../iconfont.woff2');
        expect(html).toContain('class="preview preview-plus"');
        expect(html).toContain('.preview-plus:before');
        expect(html).not.toContain('class="icon icon-plus"');
        expect(result.generateHtml({ woff2: '/override.woff2' })).toContain('class="preview preview-plus"');
        expect(result.generateHtml({ woff2: '/override.woff2' })).toContain('/override.woff2');
        expect(result.generateHtml()).toBe(html);
        expect(await readFile(join(dest, 'preview', 'nested', 'icons.html'), 'utf8')).toBe(html);
    });

    it('applies variant rename callbacks in flattened source order', async () => {
        const paths = [renameFiles.slice(0, 2), [renameFiles[2], renameFiles[0]]];
        const calls: string[] = [];

        const result = await generateWebfonts({
            ...variantOptions,
            rename(path) {
                calls.push(path);
                return `glyph-${(calls.length - 1) % 2}`;
            },
            variants: [
                { default: true, files: paths[0], name: 'small' },
                { files: paths[1], name: 'large' },
            ],
        });
        expect(calls).toEqual(paths.flat());
        expect(result.woff2).toBeInstanceOf(Uint8Array);
    });

    it('passes variant paths to one native rename batch', async () => {
        const batches: string[][] = [];

        const result = await generateNativeBinding({ ...variantOptions, files: [], types: undefined }, paths => {
            batches.push(paths);
            return paths.map((_, index) => `glyph-${index}`);
        });
        expect(batches).toEqual([[renameFiles[0], renameFiles[0]]]);
        expect(result.woff2).toBeInstanceOf(Uint8Array);
    });

    it('passes ordered variant data to context callbacks', async () => {
        let names: string[] = [];
        let htmlNames: string[] = [];

        const result = await generateWebfonts({
            ...variantOptions,
            cssContext(context) {
                names = (context.variants as Array<{ name: string }>).map(variant => variant.name);
                expect(context.defaultWeight).toBe(300);
                expect(context.fontStyle).toBe('normal');
                expect(context.variants).toEqual([
                    { name: 'small', weight: 300, default: true, className: 'icon--small', selector: 'icon--small' },
                    { name: 'large', weight: 700, default: false, className: 'icon--large', selector: 'icon--large' },
                ]);
            },
            htmlContext(context) {
                htmlNames = context.names;
                expect(context.defaultWeight).toBe(300);
                expect(context.fontStyle).toBe('normal');
            },
        });

        expect(names).toEqual(['small', 'large']);
        expect(htmlNames).toEqual(['plus']);
        expect(() => result.regenerate([], [])).toThrow('Multi-variant regeneration is not yet available.');
        await expect(result.regenerateAsync([], [])).rejects.toThrow('Multi-variant regeneration is not yet available.');
    });

    it('renders complete variant URL overrides and validates them', async () => {
        const dest = await createTempDir('vite-variant-urls-');
        const cssTemplate = join(dest, 'variants.hbs');
        await writeFile(cssTemplate, '{{{src}}}');
        const result = await generateWebfonts({ ...variantOptions, cssTemplate, types: ['woff', 'woff2'] });
        const urls = { woff2: '/assets/family.woff2' };
        expect(result.generateCss(urls)).toContain('/assets/family.woff2');
        expect(result.generateCss(urls)).toContain('url("") format("woff")');
        expect(result.generateHtml(urls)).toContain('/assets/family.woff2');
        expect(result.generateCss({})).not.toContain('/assets/family.woff2');
        expect(() => result.generateCss({ eot: '/bad.eot' })).toThrow(/SVG and EOT URLs/);
        expect(() => result.generateHtml({ svg: '/bad.svg' })).toThrow(/SVG and EOT URLs/);
    });

    it('rejects sync and async regeneration for variant results', async () => {
        const result = await generateWebfonts(variantOptions);

        expect(() => result.regenerate([], [])).toThrow(/Multi-variant regeneration/);
        expect(() => result.regenerate([])).toThrow(/Multi-variant regeneration/);
        await expect(result.regenerateAsync([], [])).rejects.toThrow(/Multi-variant regeneration/);
        await expect(result.regenerateAsync([])).rejects.toThrow(/Multi-variant regeneration/);
    });

    it('rejects an invalid variant rename batch length', async () => {
        await expect(generateNativeBinding({ ...variantOptions, files: [], types: undefined }, paths => paths.slice(1))).rejects.toThrow(
            'rename callback returned an unexpected number of glyph names',
        );
    });

    it('calls every variant rename before rejecting a local duplicate', async () => {
        const files = renameFiles.slice(0, 2);
        const calls: string[] = [];

        await expect(
            generateWebfonts({
                ...variantOptions,
                rename(path) {
                    calls.push(path);
                    return 'duplicate';
                },
                variants: [
                    { default: true, files, name: 'small' },
                    { files: [renameFiles[2]], name: 'large' },
                ],
            }),
        ).rejects.toThrow('The glyph name "duplicate" must be unique.');
        expect(calls).toEqual([...files, renameFiles[2]]);
    });

    it('applies rename callbacks in file order', async () => {
        const calls: string[] = [];
        const result = await generateWebfonts({
            ...renameOptions,
            rename(path) {
                calls.push(path);
                return `renamed-${calls.length}`;
            },
        });

        expect(calls).toEqual(renameFiles);
        expect(
            Buffer.from(result.svg)
                .toString('utf8')
                .match(/glyph-name="renamed-\d"/g),
        ).toEqual(['glyph-name="renamed-1"', 'glyph-name="renamed-2"', 'glyph-name="renamed-3"']);
    });

    it('stops rename callbacks at the first exception', async () => {
        const calls: string[] = [];
        await expect(
            generateWebfonts({
                ...renameOptions,
                rename(path) {
                    calls.push(path);
                    if (calls.length === 2) throw new Error('rename failed');
                    return `renamed-${calls.length}`;
                },
            }),
        ).rejects.toThrow('rename failed');
        expect(calls).toEqual(renameFiles.slice(0, 2));
    });

    it('stops rename callbacks at the first invalid return', async () => {
        const calls: string[] = [];
        await expect(
            generateWebfonts({
                ...renameOptions,
                rename(path) {
                    calls.push(path);
                    return undefined as never;
                },
            }),
        ).rejects.toThrow('rename callback must return a string');
        expect(calls).toEqual(renameFiles.slice(0, 1));
    });

    it('calls every rename callback before rejecting duplicate names', async () => {
        const calls: string[] = [];
        await expect(
            generateWebfonts({
                ...renameOptions,
                rename(path) {
                    calls.push(path);
                    return 'duplicate';
                },
            }),
        ).rejects.toThrow('The glyph name "duplicate" must be unique.');
        expect(calls).toEqual(renameFiles);
    });

    it('rejects an invalid raw rename batch length', async () => {
        await expect(generateNativeBinding({ ...renameOptions, types: undefined }, paths => paths.slice(1))).rejects.toThrow(
            'rename callback returned an unexpected number of glyph names',
        );
    });

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
        expect(result.generateCss({ svg: '/assets/iconfont.svg', ttf: '/assets/iconfont.ttf' })).toContain('format("svg")');
        expect(result.generateCss({ svg: '/assets/iconfont.svg', ttf: '/assets/iconfont.ttf' })).toContain('format("truetype")');
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
        expect(result.generateCss({ eot: '/assets/iconfont.eot', ttf: '/assets/iconfont.ttf' })).toContain('format("embedded-opentype")');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', ttf: '/assets/iconfont.ttf' })).toContain('/assets/iconfont.eot?#iefix');
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
        expect(result.generateCss({ ttf: '/assets/iconfont.ttf', woff: '/assets/iconfont.woff' })).toContain('format("woff")');
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
        expect(result.generateCss({ ttf: '/assets/iconfont.ttf', woff2: '/assets/iconfont.woff2' })).toContain('format("woff2")');
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
        expect(result.generateCss({ eot: '/assets/iconfont.eot', svg: '/assets/iconfont.svg' })).toContain('format("embedded-opentype")');
        expect(result.generateCss({ eot: '/assets/iconfont.eot', svg: '/assets/iconfont.svg' })).toContain('format("svg")');
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
        expect(result.generateCss({ svg: '/assets/iconfont.svg', woff: '/assets/iconfont.woff' })).toContain('format("woff")');
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
        expect(result.generateCss({ svg: '/assets/iconfont.svg', woff2: '/assets/iconfont.woff2' })).toContain('format("woff2")');
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
        const base: GenerateWebfontsFileOptions = {
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
            "q10": 22884,
            "q11": 22480,
            "q9": 24828,
          }
        `);
    });

    it('per-format output sizes', { skip: isMuslLinux() }, () => {
        expect(perFormat).toMatchInlineSnapshot(`
          {
            "eot": 60660,
            "svg": 807668,
            "ttf": 60508,
            "woff": 29956,
            "woff2": 22480,
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

const regenBaseOpts = (dir: string, files: string[]): GenerateWebfontsFileOptions => ({
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
        const opts: GenerateWebfontsFileOptions = { files: [a, b], dest, fontName: 'rc', fontHeight: 24, css: true, writeFiles: true, incremental: true, types: ['woff2'] };
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

describe('regenerateAsync (incremental)', () => {
    it('is byte-identical to synchronous regeneration', async () => {
        const dir = await createTempDir('regen-async-parity-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const options = { ...regenBaseOpts(dir, [a, b]), incremental: true };
        const [syncResult, asyncSource] = await Promise.all([generateWebfonts(options), generateWebfonts(options)]);

        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        const changes = [{ path: b, changeType: 'changed' as const }];
        syncResult.regenerate([a, b], changes);
        const asyncResult = await asyncSource.regenerateAsync([a, b], changes);

        expect(asyncResult).toEqualFont(syncResult);
        expect(asyncResult).toEqualCss(syncResult);
        expect(asyncResult.generateHtml()).toBe(syncResult.generateHtml());
    });

    it('returns a replacement while leaving the original unchanged', async () => {
        const dir = await createTempDir('regen-async-');
        const [a, b] = await Promise.all([writeRegenIcon(dir, 'a', 'a'), writeRegenIcon(dir, 'b', 'b')]);
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a, b]), incremental: true });
        const before = result.svg;

        await writeFile(b, regenIcon(REGEN_PATHS.changed));
        const replacement = await result.regenerateAsync([a, b], [{ path: b, changeType: 'changed' }]);

        expect(result.svg).toBe(before);
        expect(replacement.svg).not.toBe(before);
        expect(replacement).toEqualFont(await generateWebfonts(regenBaseOpts(dir, [a, b])));
        await expect(result.regenerateAsync([a, b])).rejects.toThrow(/replaced/);
    });

    it('keeps the original readable after a failed rebuild', async () => {
        const dir = await createTempDir('regen-async-fail-');
        const a = await writeRegenIcon(dir, 'a', 'a');
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a]), incremental: true });
        const before = result.svg;

        await rm(a);
        await expect(result.regenerateAsync([a], [{ path: a, changeType: 'changed' }])).rejects.toThrow(/No such file|ENOENT|cannot find the file/i);
        expect(result.svg).toBe(before);

        await writeRegenIcon(dir, 'a', 'a');
        const replacement = await result.regenerateAsync([a], [{ path: a, changeType: 'changed' }]);
        expect(replacement).toEqualFont(await generateWebfonts(regenBaseOpts(dir, [a])));
    });

    it('rejects overlapping rebuilds on the same result lineage', async () => {
        const dir = await createTempDir('regen-async-overlap-');
        const a = await writeRegenIcon(dir, 'a', 'a');
        const result = await generateWebfonts({ ...regenBaseOpts(dir, [a]), incremental: true });
        const replacement = await result.regenerateAsync([a]);

        const outcomes = await Promise.allSettled([replacement.regenerateAsync([a]), replacement.regenerateAsync([a])]);

        expect(outcomes.filter(outcome => outcome.status === 'fulfilled')).toHaveLength(1);
        expect(outcomes.filter(outcome => outcome.status === 'rejected')).toEqual([
            expect.objectContaining({ reason: expect.objectContaining({ message: expect.stringMatching(/regenerating|replaced/) }) }),
        ]);
    });

    it('returns a pending promise while rebuilding', async () => {
        const dir = await createTempDir('regen-async-responsive-');
        const files = await Promise.all(Array.from({ length: 64 }, (_, index) => writeRegenIcon(dir, `icon-${index}`, 'a')));
        const result = await generateWebfonts({ ...regenBaseOpts(dir, files), incremental: true });

        await writeFile(files[0], regenIcon(REGEN_PATHS.changed));
        const regeneration = result.regenerateAsync(files, [{ path: files[0], changeType: 'changed' }]);
        const published = {
            svg: result.svg,
            ttf: result.ttf,
            eot: result.eot,
            woff: result.woff,
            woff2: result.woff2,
            css: result.generateCss(),
            html: result.generateHtml(),
        };
        expect(result.svg).toBe(published.svg);
        expect(result.ttf).toEqual(published.ttf);
        expect(result.eot).toEqual(published.eot);
        expect(result.woff).toEqual(published.woff);
        expect(result.woff2).toEqual(published.woff2);
        expect(result.generateCss()).toBe(published.css);
        expect(result.generateHtml()).toBe(published.html);
        const firstSettled = await Promise.race([regeneration.then(() => 'regeneration'), Promise.resolve('microtask')]);

        expect(firstSettled).toBe('microtask');
        await regeneration;
    });
});
