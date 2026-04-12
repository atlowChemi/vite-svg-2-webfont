import { promisify } from 'node:util';
import { mkdtemp, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import opentype from 'opentype.js';
import { compile as compileScss } from 'sass';
import { generateWebfonts, templates as newCoreTemplates, type FontType, type GenerateWebfontsInputOptions, type GenerateWebfontsResult } from '@atlowchemi/webfont-generator';

type ImplementationTarget = {
    enabled: boolean;
    generator: (options: GenerateWebfontsInputOptions) => Promise<GenerateWebfontsResult>;
    name: string;
};

const require = createRequire(import.meta.url);
const upstreamCallback = require('@vusion/webfonts-generator') as ((
    options: GenerateWebfontsInputOptions,
    done: (error: unknown, result?: GenerateWebfontsResult) => void,
) => void) & {
    templates: Record<'css' | 'html' | 'scss', string>;
};
const upstreamDirect = promisify(upstreamCallback) as unknown as (options: GenerateWebfontsInputOptions) => Promise<GenerateWebfontsResult>;
const fixturesDir = new URL('./fixtures/', import.meta.url);
const iconsDir = new URL('./icons/', fixturesDir);
const generatedTypes: FontType[] = ['ttf', 'woff', 'woff2', 'eot', 'svg'];
const fontName = 'fontName';
const fixtureFileNames = await readdir(iconsDir);
fixtureFileNames.sort();
const fixtureFiles = fixtureFileNames.map((file: string) => fileURLToPath(new URL(`./${file}`, iconsDir)));

const targets: [ImplementationTarget, ImplementationTarget] = [
    {
        enabled: true,
        generator: upstreamDirect,
        name: 'upstream-direct',
    },
    {
        enabled: true,
        generator: generateWebfonts,
        name: 'new-core',
    },
];
const [upstreamTarget, newCoreTarget] = targets;

const baseOptions = (dest: string, overrides: Partial<GenerateWebfontsInputOptions> = {}): GenerateWebfontsInputOptions => ({
    dest,
    files: [...fixtureFiles],
    fontName,
    types: [...generatedTypes],
    ...overrides,
});

const cleanupDirs = new Set<string>();
const sassSilenceDeprecations = ['global-builtin', 'import', 'new-global'] as const;

afterEach(async () => {
    await Promise.all([...cleanupDirs].map(path => rm(path, { force: true, recursive: true })));
    cleanupDirs.clear();
});

async function createTempDir(prefix: string): Promise<string> {
    const path = await mkdtemp(join(tmpdir(), prefix));
    cleanupDirs.add(path);
    return path;
}

async function createInvalidSvgFixture() {
    const root = await createTempDir('__webfonts-compat-invalid-svg-');
    const file = join(root, 'bad.svg');
    await writeFile(file, '<svg><path d="M0 0"');
    return file;
}

async function createDuplicateNamedFixtures() {
    const root = await createTempDir('__webfonts-compat-duplicate-names-');
    const leftDir = join(root, 'left');
    const rightDir = join(root, 'right');
    await mkdir(leftDir, { recursive: true });
    await mkdir(rightDir, { recursive: true });

    const source = await readFile(fixtureFiles.find(file => basename(file) === 'back.svg') ?? fixtureFiles[0]!);
    const leftFile = join(leftDir, 'duplicate.svg');
    const rightFile = join(rightDir, 'duplicate.svg');
    await Promise.all([writeFile(leftFile, source), writeFile(rightFile, source)]);

    return [leftFile, rightFile];
}

async function createTemplateFixture(fileName: string, contents: string) {
    const root = await createTempDir('__webfonts-compat-template-');
    const file = join(root, fileName);
    await writeFile(file, contents);
    return file;
}

async function captureRejectionMessage(target: ImplementationTarget, options: GenerateWebfontsInputOptions) {
    try {
        await target.generator(options);
        return null;
    } catch (error) {
        return error instanceof Error ? error.message : String(error);
    }
}

function detectFontType(buffer: Buffer): Exclude<FontType, 'svg'> | 'unknown' {
    if (buffer.length >= 4 && buffer[0] === 0x00 && buffer[1] === 0x01 && buffer[2] === 0x00 && buffer[3] === 0x00) {
        return 'ttf';
    }
    if (buffer.subarray(0, 4).toString('ascii') === 'wOFF') {
        return 'woff';
    }
    if (buffer.subarray(0, 4).toString('ascii') === 'wOF2') {
        return 'woff2';
    }
    if (buffer.length >= 36 && buffer.subarray(34, 36).toString('ascii') === 'LP') {
        return 'eot';
    }
    return 'unknown';
}

function run(target: ImplementationTarget, options: GenerateWebfontsInputOptions): Promise<GenerateWebfontsResult> {
    return target.generator(options);
}

async function runSideBySide(options: GenerateWebfontsInputOptions) {
    const [upstream, newCore] = await Promise.all([run(upstreamTarget, options), run(newCoreTarget, options)]);

    return { newCore, upstream };
}

async function runSideBySideWithDifferentDest(options: GenerateWebfontsInputOptions) {
    const upstreamDest = options.dest ? `${options.dest}-upstream` : undefined;
    const newCoreDest = options.dest ? `${options.dest}-newcore` : undefined;
    const upstreamOptions = { ...options, dest: upstreamDest };
    const newCoreOptions = { ...options, dest: newCoreDest };

    const [upstream, newCore] = await Promise.all([
        run(upstreamTarget, upstreamOptions as GenerateWebfontsInputOptions),
        run(newCoreTarget, newCoreOptions as GenerateWebfontsInputOptions),
    ]);

    return { newCore, upstream, upstreamDest, newCoreDest };
}

function collectAttributeValues(source: string, tagName: string, attributeName: string): string[] {
    const values: string[] = [];
    const tagPattern = new RegExp(`<${tagName}\\b[\\s\\S]*?>`, 'g');
    const attributePattern = new RegExp(`${attributeName}="([^"]*)"`);

    for (const tag of source.match(tagPattern) ?? []) {
        const match = tag.match(attributePattern);
        if (match?.[1]) {
            values.push(match[1]);
        }
    }

    return values;
}

function parseSvgSemanticSummary(svg: string) {
    return {
        ascent: collectAttributeValues(svg, 'font-face', 'ascent')[0] ?? '',
        descent: collectAttributeValues(svg, 'font-face', 'descent')[0] ?? '',
        fontFamily: collectAttributeValues(svg, 'font-face', 'font-family')[0] ?? '',
        glyphAdvanceWidths: collectAttributeValues(svg, 'glyph', 'horiz-adv-x'),
        glyphNames: collectAttributeValues(svg, 'glyph', 'glyph-name'),
        glyphUnicodes: collectAttributeValues(svg, 'glyph', 'unicode'),
        unitsPerEm: collectAttributeValues(svg, 'font-face', 'units-per-em')[0] ?? '',
    };
}

function parseCssSemanticSummary(css: string, classPrefix = 'icon-') {
    const classPattern = new RegExp(`\\.${classPrefix}([a-zA-Z0-9_-]+):before\\s*\\{[^}]*content\\s*:\\s*["']\\\\([0-9a-fA-F]+)["']`, 'g');
    const glyphs = sortGlyphEntries([...css.matchAll(classPattern)].map((match): [string, string] => [match[1]!, match[2]!.toUpperCase()]));

    return {
        glyphs,
        hasFontFace: css.includes('@font-face'),
    };
}

function parseHtmlIconNames(html: string, classPrefix = 'icon-') {
    const pattern = new RegExp(`${classPrefix}([a-zA-Z0-9_-]+)`, 'g');
    const names = new Set<string>();

    for (const match of html.matchAll(pattern)) {
        if (match[1] && match[1] !== 'row') {
            names.add(match[1]);
        }
    }

    return sortStrings(names);
}

function sanitizeTemplateOutput(source: string) {
    source = source.replace(/(fontName\.(?:svg|ttf|woff|woff2|eot)\?)[\da-f]{32}/g, '$1HASH');
    return source;
}

function extractCssHashes(source: string) {
    const matches = [...source.matchAll(/\.(?:svg|ttf|woff|woff2|eot)\?([\da-f]{32})/g)];
    const hashes = new Set(matches.map(match => match[1]).filter((value): value is string => Boolean(value)));

    // oxlint-disable-next-line unicorn/no-array-sort -- this is a new array created from a Set, so sorting it does not cause side effects.
    return [...hashes].sort();
}

function toArrayBuffer(buffer: Buffer) {
    return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
}

function summarizeTtf(buffer: Buffer) {
    const font = opentype.parse(toArrayBuffer(buffer));
    const glyphs = Array.from({ length: font.glyphs.length }, (_, index) => {
        const glyph = font.glyphs.get(index);

        return {
            advanceWidth: glyph.advanceWidth ?? 0,
            unicodes: glyph.unicodes ?? [],
        };
    });

    return {
        ascender: font.ascender,
        descender: font.descender,
        fullName: font.names.fullName?.en ?? '',
        fontFamily: font.names.fontFamily?.en ?? '',
        fontSubfamily: font.names.fontSubfamily?.en ?? '',
        glyphCount: font.glyphs.length,
        glyphs,
        headCreated: font.tables.head!.created,
        headModified: font.tables.head!.modified,
        headVersion: font.tables.head!.fontRevision,
        postScriptName: font.names.postScriptName?.en ?? '',
        ttfCopyright: font.names.copyright?.en ?? '',
        ttfDescription: font.names.description?.en ?? '',
        ttfManufacturerUrl: font.names.manufacturerURL?.en ?? '',
        version: font.names.version?.en ?? '',
    };
}

function summarizeTtfLigatureResolution(buffer: Buffer, texts: string[]) {
    const font = opentype.parse(toArrayBuffer(buffer));

    return texts.map(text => ({
        glyphs: font.stringToGlyphs(text).map(glyph => ({
            advanceWidth: glyph.advanceWidth ?? 0,
            unicodes: glyph.unicodes ?? [],
        })),
        text,
    }));
}

function summarizeEot(buffer: Buffer) {
    const fontLength = buffer.readUInt32LE(4);
    const embeddedTtf = buffer.subarray(buffer.length - fontLength);

    return {
        charset: buffer.readUInt8(26),
        embeddedTtf: summarizeTtf(embeddedTtf),
        italic: buffer.readUInt8(27),
        magic: buffer.subarray(34, 36).toString('ascii'),
        version: buffer.readUInt32LE(8),
        weight: buffer.readUInt32LE(28),
    };
}

function normalizeWoffBuffer(buffer: Buffer) {
    const magic = Buffer.from('wOFF');
    const offset = buffer.subarray(0, 4).equals(magic) ? 0 : buffer.indexOf(magic);

    if (offset < 0 || offset + 12 > buffer.length) {
        return buffer;
    }

    const declaredSize = buffer.readUInt32BE(offset + 8);
    if (declaredSize > 0 && offset + declaredSize <= buffer.length) {
        return buffer.subarray(offset, offset + declaredSize);
    }

    return buffer.subarray(offset);
}

function summarizeWoff(buffer: Buffer) {
    const normalized = normalizeWoffBuffer(buffer);
    const metadataOffset = normalized.readUInt32BE(24);
    const metadataLength = normalized.readUInt32BE(28);
    const metadata = metadataOffset > 0 && metadataLength > 0 ? inflateSync(normalized.subarray(metadataOffset, metadataOffset + metadataLength)).toString('utf8') : '';

    return {
        flavor: normalized.readUInt32BE(4),
        metadata,
        parsedFont: summarizeTtf(normalized),
        versionMaj: normalized.readUInt16BE(20),
        versionMin: normalized.readUInt16BE(22),
    };
}

function summarizeWoff2(buffer: Buffer) {
    return {
        magic: buffer.subarray(0, 4).toString('ascii'),
    };
}

function sortGlyphEntries(entries: [string, string][]) {
    const sorted: [string, string][] = [];

    for (const entry of entries) {
        const index = sorted.findIndex(candidate => candidate[0].localeCompare(entry[0]) > 0);
        if (index === -1) {
            sorted.push(entry);
        } else {
            sorted.splice(index, 0, entry);
        }
    }

    return sorted;
}

function sortStrings(values: Iterable<string>) {
    const sorted: string[] = [];

    for (const value of values) {
        const index = sorted.findIndex(candidate => candidate.localeCompare(value) > 0);
        if (index === -1) {
            sorted.push(value);
        } else {
            sorted.splice(index, 0, value);
        }
    }

    return sorted;
}

async function readDestFile(dest: string, fileName: string): Promise<Buffer> {
    return await readFile(join(dest, fileName));
}

async function expectNonEmptyFile(dest: string, fileName: string): Promise<void> {
    const filePath = join(dest, fileName);
    const fileStat = await stat(filePath);
    expect(fileStat.size).toBeGreaterThan(0);
}

async function createScssWorkspace(): Promise<{
    cssDest: string;
    dest: string;
    multipleScssFile: string;
    singleScssFile: string;
}> {
    const root = await createTempDir('__webfonts-compat-scss-');
    const testsDir = join(root, 'tests');
    const scssDir = join(testsDir, 'scss');
    const destDir = join(testsDir, 'dest');

    await mkdir(scssDir, { recursive: true });
    await mkdir(destDir, { recursive: true });

    const singleScssSource = await readFile(new URL('./scss/singleFont.scss', fixturesDir), 'utf8');
    const multipleScssSource = await readFile(new URL('./scss/multipleFonts.scss', fixturesDir), 'utf8');
    const singleScssFile = join(scssDir, 'singleFont.scss');
    const multipleScssFile = join(scssDir, 'multipleFonts.scss');

    await writeFile(singleScssFile, singleScssSource);
    await writeFile(multipleScssFile, multipleScssSource);

    return {
        cssDest: join(destDir, `${fontName}.scss`),
        dest: destDir,
        multipleScssFile,
        singleScssFile,
    };
}

function compileCompatScss(filePath: string) {
    return compileScss(filePath, {
        silenceDeprecations: [...sassSilenceDeprecations],
    });
}

for (const target of targets) {
    describe(`compat:webfonts-generator:${target.name}`, { skip: !target.enabled }, () => {
        it('generates all fonts and css files', async () => {
            const dest = await createTempDir('__webfonts-compat-');
            await run(target, baseOptions(dest));

            const destFiles = await readdir(dest);

            await Promise.all(
                generatedTypes.map(async type => {
                    const fileName = `${fontName}.${type}`;
                    expect(destFiles).toContain(fileName);
                    await expectNonEmptyFile(dest, fileName);

                    if (type !== 'svg') {
                        const buffer = await readDestFile(dest, fileName);
                        expect(detectFontType(buffer)).toBe(type);
                    }
                }),
            );

            await expectNonEmptyFile(dest, `${fontName}.css`);
            await expect(stat(join(dest, `${fontName}.html`))).rejects.toThrow();
        });

        it('returns object with fonts and functions generateCss(), generateHtml()', async () => {
            const dest = await createTempDir('__webfonts-compat-');
            const result = await run(target, baseOptions(dest, { types: ['svg'] }));

            expect(result.svg).toBeTruthy();
            expect(result.generateCss).toBeTypeOf('function');
            expect(result.generateHtml).toBeTypeOf('function');
            expect(result.generateCss()).toBeTypeOf('string');
            expect(result.generateHtml()).toBeTypeOf('string');
        });

        it('function generateCss can change urls', async () => {
            const dest = await createTempDir('__webfonts-compat-');
            const result = await run(target, baseOptions(dest, { types: ['svg'] }));
            const css = result.generateCss({ svg: 'AAA' });

            expect(css).toContain('AAA');
        });

        it('gives error when "dest" is undefined', async () => {
            await expect(run(target, baseOptions(undefined!, target.name === 'new-core' ? { types: ['svg'] } : {}))).rejects.toThrow();
        });

        it('gives error when "files" is undefined', async () => {
            await expect(
                run(
                    target,
                    baseOptions(await createTempDir('__webfonts-compat-'), {
                        files: undefined,
                        ...(target.name === 'new-core' && { types: ['svg'] }),
                    }),
                ),
            ).rejects.toThrow();
        });

        it('gives error when "files" is empty', async () => {
            await expect(
                run(
                    target,
                    baseOptions(await createTempDir('__webfonts-compat-'), {
                        files: [],
                        ...(target.name === 'new-core' && { types: ['svg'] }),
                    }),
                ),
            ).rejects.toThrow('"options.files" is empty.');
        });

        it('uses codepoints and startCodepoint', async () => {
            const dest = await createTempDir('__webfonts-compat-');
            const startCodepoint = 0x40;
            const codepoints = { close: 0xff };

            await run(target, baseOptions(dest, { codepoints, startCodepoint, types: ['svg'] }));

            const svg = await readFile(join(dest, `${fontName}.svg`), 'utf8');
            expect(svg).toContain(startCodepoint.toString(16).toUpperCase());
            expect(svg).toContain((startCodepoint + 1).toString(16).toUpperCase());
            expect(svg).toContain(codepoints.close.toString(16).toUpperCase());
        });

        it('generates html file when options.html is true', async () => {
            const dest = await createTempDir('__webfonts-compat-');
            await run(target, baseOptions(dest, { html: true }));

            await expectNonEmptyFile(dest, `${fontName}.html`);
        });

        it('is deterministic across repeated runs for default outputs and helpers', async () => {
            const firstDest = await createTempDir('__webfonts-compat-determinism-defaults-first-');
            const secondDest = await createTempDir('__webfonts-compat-determinism-defaults-second-');
            const [firstResult, secondResult] = await Promise.all([run(target, baseOptions(firstDest, { html: true })), run(target, baseOptions(secondDest, { html: true }))]);

            expect(firstResult.svg).toEqual(secondResult.svg);
            expect(firstResult.ttf).toEqual(secondResult.ttf);
            expect(firstResult.woff).toEqual(secondResult.woff);
            expect(firstResult.woff2).toEqual(secondResult.woff2);
            expect(firstResult.eot).toEqual(secondResult.eot);
            expect(firstResult.generateCss()).toBe(secondResult.generateCss());
            expect(firstResult.generateHtml()).toBe(secondResult.generateHtml());

            await Promise.all(
                ['svg', 'ttf', 'woff', 'woff2', 'eot', 'css', 'html'].map(async extension => {
                    const [firstOutput, secondOutput] = await Promise.all([
                        readDestFile(firstDest, `${fontName}.${extension}`),
                        readDestFile(secondDest, `${fontName}.${extension}`),
                    ]);

                    expect(firstOutput).toEqual(secondOutput);
                }),
            );
        });

        it('is deterministic across repeated runs for explicit codepoints and multi-format output', async () => {
            const firstDest = await createTempDir('__webfonts-compat-determinism-codepoints-first-');
            const secondDest = await createTempDir('__webfonts-compat-determinism-codepoints-second-');
            const options = {
                codepoints: {
                    back: 0xe001,
                    close: 0xe101,
                },
                html: true,
                ligature: false,
                startCodepoint: 0xf101,
                types: ['eot', 'woff2', 'woff', 'ttf', 'svg'] satisfies FontType[],
            };
            const [firstResult, secondResult] = await Promise.all([run(target, baseOptions(firstDest, options)), run(target, baseOptions(secondDest, options))]);

            expect(firstResult.svg).toEqual(secondResult.svg);
            expect(firstResult.ttf).toEqual(secondResult.ttf);
            expect(firstResult.woff).toEqual(secondResult.woff);
            expect(firstResult.woff2).toEqual(secondResult.woff2);
            expect(firstResult.eot).toEqual(secondResult.eot);
            expect(firstResult.generateCss()).toBe(secondResult.generateCss());
            expect(firstResult.generateHtml()).toBe(secondResult.generateHtml());
        });

        it('is deterministic across repeated runs for custom templates', async () => {
            const template = await createTemplateFixture('deterministic-template.hbs', '{{fontName}}|{{{src}}}|{{#each codepoints}}{{@key}}={{this}};{{/each}}');
            const firstDest = await createTempDir('__webfonts-compat-determinism-template-first-');
            const secondDest = await createTempDir('__webfonts-compat-determinism-template-second-');
            const options = {
                cssTemplate: template,
                html: true,
                htmlTemplate: template,
                templateOptions: {
                    baseSelector: '.icon',
                    classPrefix: 'icon-',
                },
                types: ['svg'] satisfies FontType[],
            };
            const [firstResult, secondResult] = await Promise.all([run(target, baseOptions(firstDest, options)), run(target, baseOptions(secondDest, options))]);

            expect(firstResult.generateCss()).toBe(secondResult.generateCss());
            expect(firstResult.generateHtml()).toBe(secondResult.generateHtml());

            const [firstCss, secondCss, firstHtml, secondHtml] = await Promise.all([
                readDestFile(firstDest, `${fontName}.css`),
                readDestFile(secondDest, `${fontName}.css`),
                readDestFile(firstDest, `${fontName}.html`),
                readDestFile(secondDest, `${fontName}.html`),
            ]);

            expect(firstCss).toEqual(secondCss);
            expect(firstHtml).toEqual(secondHtml);
        });

        it('generates a stable hash for identical default options across repeated runs', async () => {
            const firstDest = await createTempDir('__webfonts-compat-hash-defaults-first-');
            const secondDest = await createTempDir('__webfonts-compat-hash-defaults-second-');
            const [firstResult, secondResult] = await Promise.all([run(target, baseOptions(firstDest)), run(target, baseOptions(secondDest))]);
            const firstHashes = extractCssHashes(firstResult.generateCss());
            const secondHashes = extractCssHashes(secondResult.generateCss());

            expect(firstHashes).toHaveLength(1);
            expect(secondHashes).toHaveLength(1);
            expect(firstHashes).toEqual(secondHashes);
        });

        it('generates the same hash even when different "dest", "htmlDest", and "cssDest" are passed as options', async () => {
            const dest = await createTempDir('__webfonts-compat-hash-ignored-paths-');
            const [firstResult, secondResult] = await Promise.all([
                run(
                    target,
                    baseOptions(dest, {
                        css: true,
                        cssDest: join(`${dest}-some`, 'nested', `${fontName}.css`),
                        dest: `${dest}-some`,
                        html: true,
                        htmlDest: join(`${dest}-some`, 'nested', `${fontName}.html`),
                    }),
                ),
                run(
                    target,
                    baseOptions(dest, {
                        css: true,
                        cssDest: join(`${dest}-other`, 'alt', `${fontName}.css`),
                        dest: `${dest}-other`,
                        html: true,
                        htmlDest: join(`${dest}-other`, 'alt', `${fontName}.html`),
                    }),
                ),
            ]);

            cleanupDirs.add(`${dest}-some`);
            cleanupDirs.add(`${dest}-other`);

            expect(extractCssHashes(firstResult.generateCss())).toEqual(extractCssHashes(secondResult.generateCss()));
        });

        it('changes the hash when selected generated types change', async () => {
            const firstDest = await createTempDir('__webfonts-compat-hash-types-first-');
            const secondDest = await createTempDir('__webfonts-compat-hash-types-second-');
            const [firstResult, secondResult] = await Promise.all([
                run(target, baseOptions(firstDest, { types: ['svg'] })),
                run(target, baseOptions(secondDest, { types: ['svg', 'woff2'] })),
            ]);

            expect(extractCssHashes(firstResult.generateCss())).not.toEqual(extractCssHashes(secondResult.generateCss()));
        });

        it('changes the hash when template options change', async () => {
            const firstDest = await createTempDir('__webfonts-compat-hash-template-options-first-');
            const secondDest = await createTempDir('__webfonts-compat-hash-template-options-second-');
            const [firstResult, secondResult] = await Promise.all([
                run(target, baseOptions(firstDest, { templateOptions: { baseSelector: '.icon' } })),
                run(target, baseOptions(secondDest, { templateOptions: { baseSelector: '.glyph' } })),
            ]);

            expect(extractCssHashes(firstResult.generateCss())).not.toEqual(extractCssHashes(secondResult.generateCss()));
        });

        it('changes the hash when the input file set changes', async () => {
            const firstDest = await createTempDir('__webfonts-compat-hash-files-first-');
            const secondDest = await createTempDir('__webfonts-compat-hash-files-second-');
            const [firstResult, secondResult] = await Promise.all([
                run(target, baseOptions(firstDest, { files: fixtureFiles.slice(0, 1), types: ['svg'] })),
                run(target, baseOptions(secondDest, { files: fixtureFiles.slice(0, 2), types: ['svg'] })),
            ]);

            expect(extractCssHashes(firstResult.generateCss())).not.toEqual(extractCssHashes(secondResult.generateCss()));
        });

        it('changes the hash when explicit codepoints change', async () => {
            const firstDest = await createTempDir('__webfonts-compat-hash-codepoints-first-');
            const secondDest = await createTempDir('__webfonts-compat-hash-codepoints-second-');
            const [firstResult, secondResult] = await Promise.all([
                run(target, baseOptions(firstDest, { codepoints: { back: 0xe001 }, types: ['svg'] })),
                run(target, baseOptions(secondDest, { codepoints: { back: 0xe100 }, types: ['svg'] })),
            ]);

            expect(extractCssHashes(firstResult.generateCss())).not.toEqual(extractCssHashes(secondResult.generateCss()));
        });

        describe('custom templates', () => {
            const customTemplate = fileURLToPath(new URL('./templates/customTemplate.hbs', fixturesDir));
            const renderedTemplate = 'custom template TEST\n';

            it('uses custom css template', async () => {
                const dest = await createTempDir('__webfonts-compat-');
                await run(target, baseOptions(dest, { cssTemplate: customTemplate, templateOptions: { option: 'TEST' } }));

                const css = await readFile(join(dest, `${fontName}.css`), 'utf8');
                expect(css).toBe(renderedTemplate);
            });

            it('uses custom html template', async () => {
                const dest = await createTempDir('__webfonts-compat-');
                await run(target, baseOptions(dest, { html: true, htmlTemplate: customTemplate, templateOptions: { option: 'TEST' } }));

                const html = await readFile(join(dest, `${fontName}.html`), 'utf8');
                expect(html).toBe(renderedTemplate);
            });
        });

        describe('custom context', () => {
            const customContextTemplate = fileURLToPath(new URL('./templates/customContextTemplate.hbs', fixturesDir));

            it('uses custom html context', async () => {
                const dest = await createTempDir('__webfonts-compat-');
                await run(
                    target,
                    baseOptions(dest, {
                        html: true,
                        htmlTemplate: customContextTemplate,
                        htmlContext: context => {
                            context.hello = 'world';
                        },
                        templateOptions: { option: 'TEST' },
                    }),
                );

                const html = await readFile(join(dest, `${fontName}.html`), 'utf8');
                expect(html).toBe('world');
            });

            it('uses custom css context', async () => {
                const dest = await createTempDir('__webfonts-compat-');
                await run(
                    target,
                    baseOptions(dest, {
                        cssContext: context => {
                            context.hello = 'world';
                        },
                        cssTemplate: customContextTemplate,
                        templateOptions: { option: 'TEST' },
                    }),
                );

                const css = await readFile(join(dest, `${fontName}.css`), 'utf8');
                expect(css).toBe('world');
            });

            it('applies cssContext even when css write is disabled', async () => {
                const dest = await createTempDir('__webfonts-compat-css-ctx-no-write-');
                const result = await run(
                    target,
                    baseOptions(dest, {
                        css: false,
                        html: false,
                        writeFiles: false,
                        cssContext: context => {
                            context.hello = 'world';
                        },
                        cssTemplate: customContextTemplate,
                        templateOptions: { option: 'TEST' },
                    }),
                );
                expect(result.generateCss()).toBe('world');
            });

            it('applies htmlContext even when html write is disabled', async () => {
                const dest = await createTempDir('__webfonts-compat-html-ctx-no-write-');
                const result = await run(
                    target,
                    baseOptions(dest, {
                        css: false,
                        html: false,
                        writeFiles: false,
                        htmlTemplate: customContextTemplate,
                        htmlContext: context => {
                            context.hello = 'world';
                        },
                        templateOptions: { option: 'TEST' },
                    }),
                );
                expect(result.generateHtml()).toBe('world');
            });
        });

        describe('scss template', () => {
            it('creates mixins that can be used to create icons styles', async () => {
                const { cssDest, dest, singleScssFile } = await createScssWorkspace();

                await run(
                    target,
                    baseOptions(dest, {
                        cssDest,
                        cssTemplate: newCoreTemplates.scss,
                    }),
                );

                const rendered = compileCompatScss(singleScssFile);
                expect(rendered.css).toContain(fontName);
            });

            it('multiple scss mixins can be used together', async () => {
                const { cssDest, dest, multipleScssFile } = await createScssWorkspace();
                const secondFontName = `${fontName}2`;
                const secondCssDest = join(dirname(cssDest), `${secondFontName}.scss`);

                await Promise.all([
                    run(
                        target,
                        baseOptions(dest, {
                            cssDest,
                            cssTemplate: newCoreTemplates.scss,
                            files: [fixtureFiles.find((file: string) => basename(file) === 'close.svg')!],
                        }),
                    ),
                    run(
                        target,
                        baseOptions(dest, {
                            cssDest: secondCssDest,
                            cssTemplate: newCoreTemplates.scss,
                            files: [fixtureFiles.find((file: string) => basename(file) === 'back.svg')!],
                            fontName: secondFontName,
                        }),
                    ),
                ]);

                const rendered = compileCompatScss(multipleScssFile);
                expect(rendered.css).toContain(fontName);
                expect(rendered.css).toContain(secondFontName);
            });
        });
    });
}

describe('compat:webfonts-generator:side-by-side', () => {
    it('matches svg font semantics for the native svg-only path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-svg-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches svg font normalize semantics for mixed-size glyphs', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-normalize-');
        const files = fixtureFiles.slice(0, 2); // two differently-sized fixtures

        const { newCore: normalized, upstream: upNormalized } = await runSideBySide(
            baseOptions(dest, {
                fontHeight: 1000,
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );
        const { newCore: nonNormalized, upstream: upNonNormalized } = await runSideBySide(
            baseOptions(dest, {
                files,
                fontHeight: 1000,
                ligature: false,
                normalize: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(normalized.svg)).toEqual(parseSvgSemanticSummary(upNormalized.svg));
        expect(parseSvgSemanticSummary(nonNormalized.svg)).toEqual(parseSvgSemanticSummary(upNonNormalized.svg));
    });

    it('matches svg font fixedWidth semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-fixed-width-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                fixedWidth: true,
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(newCore.svg)).toEqual(parseSvgSemanticSummary(upstream.svg));
    });

    it('matches svg font centerHorizontally semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-center-h-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                centerHorizontally: true,
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(newCore.svg)).toEqual(parseSvgSemanticSummary(upstream.svg));
    });

    it('matches svg font centerVertically semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-center-v-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                centerVertically: true,
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(newCore.svg)).toEqual(parseSvgSemanticSummary(upstream.svg));
    });

    it('matches svg font preserveAspectRatio semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-par-');
        const { newCore: withPar, upstream: upWithPar } = await runSideBySide(
            baseOptions(dest, {
                formatOptions: { svg: { preserveAspectRatio: true } },
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );
        const { newCore: withoutPar, upstream: upWithoutPar } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(withPar.svg)).toEqual(parseSvgSemanticSummary(upWithPar.svg));
        expect(parseSvgSemanticSummary(withoutPar.svg)).toEqual(parseSvgSemanticSummary(upWithoutPar.svg));
    });

    it('matches svg font custom round semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-round-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                round: 100,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        expect(parseSvgSemanticSummary(newCore.svg)).toEqual(parseSvgSemanticSummary(upstream.svg));
    });

    it('matches svg font metadata semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-svg-metadata-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                formatOptions: { svg: { metadata: '<test>compat-metadata</test>' } },
                ligature: false,
                startCodepoint: 0xe001,
                types: ['svg'],
            }),
        );

        const ncSvg = newCore.svg;
        const upSvg = upstream.svg;
        expect(parseSvgSemanticSummary(ncSvg)).toEqual(parseSvgSemanticSummary(upSvg));
        expect(ncSvg).toContain('<test>compat-metadata</test>');
        expect(upSvg).toContain('<test>compat-metadata</test>');
    });

    it('matches codepoint ordering semantics for explicit codepoints and startCodepoint', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-codepoints-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: { close: 0xff },
                ligature: false,
                startCodepoint: 0x40,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches parsed ttf metadata and glyph semantics for the native ttf path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-ttf-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                types: ['ttf'],
            }),
        );

        expect(summarizeTtf(Buffer.from(newCore.ttf))).toEqual(summarizeTtf(Buffer.from(upstream.ttf)));
    });

    it('matches parsed ttf codepoints and ligature semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-ttf-codepoints-ligatures-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: { close: 0xff },
                ligature: true,
                startCodepoint: 0x40,
                types: ['ttf'],
            }),
        );

        expect(summarizeTtf(Buffer.from(newCore.ttf))).toEqual(summarizeTtf(Buffer.from(upstream.ttf)));
        expect(summarizeTtfLigatureResolution(Buffer.from(newCore.ttf), ['back', 'close', 'string'])).toEqual(
            summarizeTtfLigatureResolution(Buffer.from(upstream.ttf), ['back', 'close', 'string']),
        );
    });

    it('matches svg2ttf metadata options for parsed ttf output', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-ttf-metadata-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                formatOptions: {
                    ttf: {
                        copyright: 'copyright text',
                        description: 'description text',
                        ts: 1_700_000_000,
                        url: 'https://example.com/font',
                        version: '2.5',
                    },
                },
                ligature: false,
                types: ['ttf'],
            }),
        );

        expect(summarizeTtf(Buffer.from(newCore.ttf))).toEqual(summarizeTtf(Buffer.from(upstream.ttf)));
    });

    it('deduplicates identical glyphs in ttf by mapping multiple codepoints to a single outline', async () => {
        const sourceIcon = fixtureFiles[0]!;
        const [fixtureDir, originalIcon] = await Promise.all([createTempDir('__webfonts-compat-side-by-side-ttf-dedup-'), readFile(sourceIcon)]);
        const copyA = join(fixtureDir, 'copy-a.svg');
        const copyB = join(fixtureDir, 'copy-b.svg');
        const copyC = join(fixtureDir, 'copy-c.svg');
        await Promise.all([writeFile(copyA, originalIcon), writeFile(copyB, originalIcon), writeFile(copyC, originalIcon)]);

        const dest = await createTempDir('__webfonts-compat-side-by-side-ttf-dedup-dest-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                files: [sourceIcon, copyA, copyB, copyC],
                ligature: false,
                types: ['ttf'],
            }),
        );

        const upSummary = summarizeTtf(Buffer.from(upstream.ttf));
        const ncSummary = summarizeTtf(Buffer.from(newCore.ttf));

        expect(ncSummary.glyphCount).toBe(upSummary.glyphCount);
        expect(ncSummary.glyphs).toEqual(upSummary.glyphs);
    });

    it('matches eot header and embedded ttf semantics for the native eot path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-eot-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                types: ['eot'],
            }),
        );

        expect(summarizeEot(Buffer.from(newCore.eot))).toEqual(summarizeEot(Buffer.from(upstream.eot)));
    });

    it('matches woff header and parsed font semantics for the native woff path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-woff-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                types: ['woff'],
            }),
        );

        expect(summarizeWoff(Buffer.from(newCore.woff))).toEqual(summarizeWoff(Buffer.from(upstream.woff)));
    });

    it('matches woff2 header presence and paired ttf semantics for the native woff2 path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-woff2-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                ligature: false,
                types: ['ttf', 'woff2'],
            }),
        );

        expect(summarizeWoff2(Buffer.from(newCore.woff2))).toEqual(summarizeWoff2(Buffer.from(upstream.woff2)));
        expect(summarizeTtf(Buffer.from(newCore.ttf))).toEqual(summarizeTtf(Buffer.from(upstream.ttf)));
    });

    it('matches ttf2woff metadata behavior for parsed woff output', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-woff-metadata-');
        const metadata = '<metadata><uniqueid id="compat-woff" /></metadata>';
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                formatOptions: {
                    woff: {
                        metadata,
                    },
                },
                ligature: false,
                types: ['woff'],
            }),
        );

        expect(summarizeWoff(Buffer.from(newCore.woff))).toEqual(summarizeWoff(Buffer.from(upstream.woff)));
    });

    it('matches duplicate explicit codepoint semantics', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-duplicate-codepoints-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: {
                    back: 0x41,
                    close: 0x41,
                },
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches runtime codepoint coercion semantics for non-numeric values', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-codepoint-coercion-string-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: {
                    back: 'wat' as never,
                },
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches runtime codepoint coercion semantics for negative values', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-codepoint-coercion-negative-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: {
                    back: -1 as never,
                },
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches runtime codepoint coercion semantics for very large values', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-codepoint-coercion-large-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                codepoints: {
                    back: 99999999 as never,
                },
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('matches css and html helper semantics for the native svg-only path', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-helpers-');
        const urls = { svg: '/assets/fontName.svg' } satisfies Partial<Record<FontType, string>>;
        const options = baseOptions(dest, {
            ligature: false,
            templateOptions: {
                baseSelector: '.icon',
                classPrefix: 'icon-',
            },
            types: ['svg'],
        });
        const { newCore, upstream } = await runSideBySide(options);
        const upstreamCss = upstream.generateCss(urls);
        const newCoreCss = newCore.generateCss(urls);
        const upstreamHtml = upstream.generateHtml(urls);
        const newCoreHtml = newCore.generateHtml(urls);

        expect(parseCssSemanticSummary(newCoreCss)).toEqual(parseCssSemanticSummary(upstreamCss));
        expect(parseHtmlIconNames(newCoreHtml)).toEqual(parseHtmlIconNames(upstreamHtml));
        expect(newCoreCss).toContain('/assets/fontName.svg');
        expect(newCoreHtml).toContain('icon-close');
    });

    describe.each(['html', 'css'] as const)('%s template parity', async templateType => {
        it('matches default template output', async () => {
            const dest = await createTempDir(`__webfonts-compat-side-by-side-default-${templateType}-template-`);
            const common = { css: templateType === 'css', html: templateType === 'html', types: ['svg' as const] };

            const { newCore, upstream, newCoreDest, upstreamDest } = await runSideBySideWithDifferentDest(baseOptions(dest, common));

            const upstreamOutput = (await readDestFile(upstreamDest!, `${fontName}.${templateType}`)).toString('utf-8');
            const newOutput = (await readDestFile(newCoreDest!, `${fontName}.${templateType}`)).toString('utf-8');

            const normalizedUpstreamOutput = sanitizeTemplateOutput(upstreamOutput);
            const normalizedNewOutput = sanitizeTemplateOutput(newOutput);

            if (templateType === 'css') {
                expect(parseCssSemanticSummary(normalizedNewOutput)).toEqual(parseCssSemanticSummary(normalizedUpstreamOutput));
                expect(sanitizeTemplateOutput(newCore.generateCss())).toBe(sanitizeTemplateOutput(upstream.generateCss()));
            } else {
                expect(normalizedUpstreamOutput).toEqual(normalizedNewOutput);
                expect(sanitizeTemplateOutput(newCore.generateHtml())).toBe(sanitizeTemplateOutput(upstream.generateHtml()));
            }
        });

        it('matches custom template output when using all available context values', async () => {
            const dest = await createTempDir('__webfonts-compat-side-by-side-custom-template-');

            const template =
                templateType === 'css'
                    ? '{{fontName}}|{{{src}}}|{{baseSelector}}|{{classPrefix}}|{{codepoints.back}}|{{option}}'
                    : '{{fontName}}|{{{styles}}}|{{baseSelector}}|{{classPrefix}}|{{codepoints.back}}|{{#each names}}{{this}}{{/each}}|{{option}}';
            const customTemplate = await createTemplateFixture('all-context.hbs', template);
            const options = baseOptions(dest, {
                css: templateType === 'css',
                html: templateType === 'html',
                cssTemplate: customTemplate,
                htmlTemplate: customTemplate,
                templateOptions: { option: 'TEST' },
                types: ['svg'],
            });
            const { newCore, upstream, newCoreDest, upstreamDest } = await runSideBySideWithDifferentDest(options);

            const upstreamOutput = (await readDestFile(upstreamDest!, `${fontName}.${templateType}`)).toString('utf-8');
            const newOutput = (await readDestFile(newCoreDest!, `${fontName}.${templateType}`)).toString('utf-8');

            expect(sanitizeTemplateOutput(newOutput)).toBe(sanitizeTemplateOutput(upstreamOutput));

            if (templateType === 'css') {
                expect(sanitizeTemplateOutput(newCore.generateCss())).toBe(sanitizeTemplateOutput(upstream.generateCss()));
            } else {
                expect(sanitizeTemplateOutput(newCore.generateHtml())).toBe(sanitizeTemplateOutput(upstream.generateHtml()));
            }
        });

        it('rejects invalid custom template syntax in both implementations', async () => {
            const dest = await createTempDir('__webfonts-compat-side-by-side-invalid-template-');
            const invalidTemplate = await createTemplateFixture('invalid.hbs', '{{#if}}');
            const options = baseOptions(dest, {
                css: templateType === 'css',
                html: templateType === 'html',
                cssTemplate: invalidTemplate,
                htmlTemplate: invalidTemplate,
                types: ['svg'],
            });

            await expect(run(upstreamTarget, options)).rejects.toThrow();
            await expect(run(newCoreTarget, options)).rejects.toThrow();
        });

        it.each([true, false])('rejects invalid custom template syntax in both implementations in generate function when writeFiles is %s', async writeFiles => {
            const dest = await createTempDir('__webfonts-compat-side-by-side-invalid-template-');
            const invalidTemplate = await createTemplateFixture('invalid.hbs', '{{#if}}');
            const options = baseOptions(dest, {
                writeFiles,
                css: false,
                html: false,
                cssTemplate: invalidTemplate,
                htmlTemplate: invalidTemplate,
                types: ['svg'],
            });

            const { newCore, upstream } = await runSideBySide(options);

            expect(() => upstream.generateCss()).toThrow();
            expect(() => newCore.generateCss()).toThrow();
            expect(() => upstream.generateHtml()).toThrow();
            expect(() => newCore.generateHtml()).toThrow();
        });
    });

    it.each([true, false])('rejects invalid CSS custom template syntax in both implementations in generateHtml function when writeFiles is %s', async writeFiles => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-invalid-template-');
        const invalidTemplate = await createTemplateFixture('invalid.hbs', '{{#if}}');
        const options = baseOptions(dest, {
            writeFiles,
            css: false,
            html: false,
            cssTemplate: invalidTemplate,
            types: ['svg'],
        });

        const { newCore, upstream } = await runSideBySide(options);

        expect(() => upstream.generateHtml()).toThrow();
        expect(() => newCore.generateHtml()).toThrow();
    });

    it('matches full multi-format CSS output side-by-side', async () => {
        const destUpstream = await createTempDir('__webfonts-compat-side-by-side-default-css-multiformat-upstream-');
        const destNewCore = await createTempDir('__webfonts-compat-side-by-side-default-css-multiformat-new-core-');
        const options = {
            css: true,
            html: false,
            order: ['eot', 'woff2', 'woff', 'ttf', 'svg'] satisfies FontType[],
            types: ['eot', 'woff2', 'woff', 'ttf', 'svg'] satisfies FontType[],
        };

        const [upstream, newCore] = await Promise.all([run(upstreamTarget, baseOptions(destUpstream, options)), run(newCoreTarget, baseOptions(destNewCore, options))]);

        const upstreamOutput = (await readDestFile(destUpstream, `${fontName}.css`)).toString('utf-8');
        const newOutput = (await readDestFile(destNewCore, `${fontName}.css`)).toString('utf-8');

        expect(sanitizeTemplateOutput(newOutput)).toBe(sanitizeTemplateOutput(upstreamOutput));
        expect(sanitizeTemplateOutput(newCore.generateCss())).toBe(sanitizeTemplateOutput(upstream.generateCss()));
    });

    describe.each([
        {
            label: 'relative paths',
            urls: {
                eot: '../fonts/custom-font.eot',
                svg: '../fonts/custom-font.svg',
                ttf: '../fonts/custom-font.ttf',
                woff: '../fonts/custom-font.woff',
                woff2: '../fonts/custom-font.woff2',
            } satisfies Partial<Record<FontType, string>>,
        },
        {
            label: 'absolute file-system-like paths',
            urls: {
                eot: '/var/www/assets/custom-font.eot',
                svg: '/var/www/assets/custom-font.svg',
                ttf: '/var/www/assets/custom-font.ttf',
                woff: '/var/www/assets/custom-font.woff',
                woff2: '/var/www/assets/custom-font.woff2',
            } satisfies Partial<Record<FontType, string>>,
        },
        {
            label: 'absolute URL paths',
            urls: {
                eot: 'https://cdn.example.com/fonts/custom-font.eot?v=1',
                svg: 'https://cdn.example.com/fonts/custom-font.svg?v=1',
                ttf: 'https://cdn.example.com/fonts/custom-font.ttf?v=1',
                woff: 'https://cdn.example.com/fonts/custom-font.woff?v=1',
                woff2: 'https://cdn.example.com/fonts/custom-font.woff2?v=1',
            } satisfies Partial<Record<FontType, string>>,
        },
        {
            label: 'non-existent-looking paths',
            urls: {
                eot: '__missing__/font.eot',
                svg: '__missing__/font.svg',
                ttf: '__missing__/font.ttf',
                woff: '__missing__/font.woff',
                woff2: '__missing__/font.woff2',
            } satisfies Partial<Record<FontType, string>>,
        },
        {
            label: 'partial overrides',
            urls: {
                svg: '/assets/override.svg',
                woff2: '/assets/override.woff2',
            } satisfies Partial<Record<FontType, string>>,
        },
        {
            label: 'unknown types are ignored',
            urls: {
                bogus: '/assets/ignored.woff',
                svg: '/assets/override.svg',
            } as Partial<Record<FontType, string>>,
        },
    ] as const)('helper urls parity: $label', ({ urls }) => {
        it('matches generateCss(urls) side-by-side', async () => {
            const dest = await createTempDir('__webfonts-compat-side-by-side-helper-css-urls-');
            const options = baseOptions(dest, {
                css: false,
                html: false,
                order: ['eot', 'woff2', 'woff', 'ttf', 'svg'],
                types: ['eot', 'woff2', 'woff', 'ttf', 'svg'],
            });
            const { newCore, upstream } = await runSideBySide(options);

            expect(sanitizeTemplateOutput(newCore.generateCss(urls))).toBe(sanitizeTemplateOutput(upstream.generateCss(urls)));
        });

        it('matches generateHtml(urls) side-by-side', async () => {
            const dest = await createTempDir('__webfonts-compat-side-by-side-helper-html-urls-');
            const options = baseOptions(dest, {
                css: false,
                html: false,
                order: ['eot', 'woff2', 'woff', 'ttf', 'svg'],
                types: ['eot', 'woff2', 'woff', 'ttf', 'svg'],
            });
            const { newCore, upstream } = await runSideBySide(options);

            expect(sanitizeTemplateOutput(newCore.generateHtml(urls))).toBe(sanitizeTemplateOutput(upstream.generateHtml(urls)));
        });
    });

    it('matches default scss template output', async () => {
        const destUpstream = await createTempDir(`__webfonts-compat-side-by-side-default-scss-template-upstream-`);
        const destNewCore = await createTempDir(`__webfonts-compat-side-by-side-default-scss-template-new-core-`);

        const [upstream, newCore] = await Promise.all([
            run(upstreamTarget, baseOptions(destUpstream, { types: ['svg'], cssTemplate: newCoreTemplates.scss, cssDest: join(destUpstream, `${fontName}.scss`) })),
            run(newCoreTarget, baseOptions(destNewCore, { types: ['svg'], cssTemplate: newCoreTemplates.scss, cssDest: join(destNewCore, `${fontName}.scss`) })),
        ]);

        const upstreamOutput = (await readDestFile(destUpstream, `${fontName}.scss`)).toString('utf-8');
        const newOutput = (await readDestFile(destNewCore, `${fontName}.scss`)).toString('utf-8');

        const normalizedUpstreamOutput = sanitizeTemplateOutput(upstreamOutput);
        const normalizedNewOutput = sanitizeTemplateOutput(newOutput);

        expect(parseCssSemanticSummary(normalizedNewOutput)).toEqual(parseCssSemanticSummary(normalizedUpstreamOutput));
        expect(sanitizeTemplateOutput(newCore.generateCss())).toBe(sanitizeTemplateOutput(upstream.generateCss()));
    });

    it('rejects missing template files in both implementations', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-missing-template-file-');
        const missingTemplate = join(dest, 'does-not-exist.hbs');

        await expect(
            run(
                upstreamTarget,
                baseOptions(dest, {
                    css: true,
                    cssTemplate: missingTemplate,
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();
        await expect(
            run(
                newCoreTarget,
                baseOptions(dest, {
                    css: true,
                    cssTemplate: missingTemplate,
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();

        await expect(
            run(
                upstreamTarget,
                baseOptions(dest, {
                    html: true,
                    htmlTemplate: missingTemplate,
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();
        await expect(
            run(
                newCoreTarget,
                baseOptions(dest, {
                    html: true,
                    htmlTemplate: missingTemplate,
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();
    });

    it('matches validation error messages for missing required options', async () => {
        await expect(run(upstreamTarget, baseOptions(undefined!, { types: ['svg'] }))).rejects.toThrow('"options.dest" is undefined.');
        await expect(run(newCoreTarget, baseOptions(undefined!, { types: ['svg'] }))).rejects.toThrow();

        const validateDest = await createTempDir('__webfonts-compat-side-by-side-validate-');
        await expect(run(upstreamTarget, baseOptions(validateDest, { files: undefined, types: ['svg'] }))).rejects.toThrow('"options.files" is undefined.');
        await expect(run(newCoreTarget, baseOptions(validateDest, { files: undefined, types: ['svg'] }))).rejects.toThrow();
    });

    it('matches validation error messages for empty files', async () => {
        const validateDest = await createTempDir('__webfonts-compat-side-by-side-empty-files-');
        await expect(run(upstreamTarget, baseOptions(validateDest, { files: [], types: ['svg'] }))).rejects.toThrow('"options.files" is empty.');
        await expect(run(newCoreTarget, baseOptions(validateDest, { files: [], types: ['svg'] }))).rejects.toThrow('"options.files" is empty.');
    });

    it('rejects malformed svg input in both implementations', async () => {
        const invalidSvg = await createInvalidSvgFixture();
        const dest = await createTempDir('__webfonts-compat-side-by-side-invalid-svg-');

        await expect(
            run(
                upstreamTarget,
                baseOptions(dest, {
                    files: [invalidSvg],
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();
        await expect(
            run(
                newCoreTarget,
                baseOptions(dest, {
                    files: [invalidSvg],
                    types: ['svg'],
                }),
            ),
        ).rejects.toThrow();
    });

    it('matches duplicate glyph-name rejection semantics for default rename behavior', async () => {
        const files = await createDuplicateNamedFixtures();
        const dest = await createTempDir('__webfonts-compat-side-by-side-duplicate-glyph-names-');
        const options = baseOptions(dest, {
            files,
            types: ['svg'],
        });

        const [upstreamError, newCoreError] = await Promise.all([captureRejectionMessage(upstreamTarget, options), captureRejectionMessage(newCoreTarget, options)]);

        expect(upstreamError).toBeTruthy();
        expect(newCoreError).toBe(upstreamError);
    });

    it('matches empty glyph-name semantics when rename returns an empty name', async () => {
        const dest = await createTempDir('__webfonts-compat-side-by-side-missing-glyph-name-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                files: [fixtureFiles.find(file => basename(file) === 'back.svg') ?? fixtureFiles[0]!],
                rename: () => '',
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('handles empty SVGs (no paths) identically to upstream', async () => {
        const fixtureDir = await createTempDir('__webfonts-compat-side-by-side-empty-svg-');
        const emptySvg = join(fixtureDir, 'empty.svg');
        await writeFile(emptySvg, '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n</svg>');
        const dest = await createTempDir('__webfonts-compat-side-by-side-empty-svg-dest-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                files: [emptySvg],
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });

    it('handles empty SVGs mixed with normal SVGs identically to upstream', async () => {
        const fixtureDir = await createTempDir('__webfonts-compat-side-by-side-empty-svg-mixed-');
        const emptySvg = join(fixtureDir, 'empty.svg');
        await writeFile(emptySvg, '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n</svg>');
        const normalSvg = fixtureFiles.find(file => basename(file) === 'back.svg') ?? fixtureFiles[0]!;
        const dest = await createTempDir('__webfonts-compat-side-by-side-empty-svg-mixed-dest-');
        const { newCore, upstream } = await runSideBySide(
            baseOptions(dest, {
                files: [normalSvg, emptySvg],
                ligature: false,
                types: ['svg'],
            }),
        );

        const upstreamSvg = typeof upstream.svg === 'string' ? upstream.svg : Buffer.from(upstream.svg).toString('utf8');
        const newCoreSvg = typeof newCore.svg === 'string' ? newCore.svg : Buffer.from(newCore.svg).toString('utf8');

        expect(parseSvgSemanticSummary(newCoreSvg)).toEqual(parseSvgSemanticSummary(upstreamSvg));
    });
});
