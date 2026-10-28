import { unzipSync, strFromU8 } from 'fflate';
import { describe, expect, it } from 'vite-plus/test';
import { codeThemes } from '../../codeThemes';
import { codepointsSnippet, DEFAULT_START_CODEPOINT, formatCodepoint, isPrivateUse, nextCodepoint, parseCodepoint } from './codepoints';
import { highlight, loadHighlighter } from './highlight';
import { iconName, isValidName, uniqueName } from './names';
import { defaultSettings, parseTemplateData, selectedTypes, toGenerateOptions, unavailableTypes, type PlaygroundIcon } from './options';
import { outputFileName, zipOutputs } from './zip';

describe('names', () => {
    it.each([
        ['add.svg', 'add'],
        ['Arrow Left.SVG', 'Arrow-Left'],
        ['arrow.left.svg', 'arrow-left'],
        ['icon@2x.svg', 'icon-2x'],
        ['ünïcode_ok.svg', 'ünïcode_ok'],
        ['...svg', 'icon'],
    ])('derives %s as %s', (file, name) => {
        expect(iconName(file)).toBe(name);
        expect(isValidName(name)).toBe(true);
    });

    it('rejects names the engine would refuse', () => {
        expect(isValidName('my icon')).toBe(false);
        expect(isValidName('')).toBe(false);
    });

    it('suffixes taken names', () => {
        expect(uniqueName('add', new Set())).toBe('add');
        expect(uniqueName('add', new Set(['add', 'add-2']))).toBe('add-3');
    });
});

describe('codepoints', () => {
    it('skips used codepoints like the engine', () => {
        expect(nextCodepoint(DEFAULT_START_CODEPOINT, new Set([0xf101, 0xf102, 0xf104]))).toBe(0xf103);
    });

    it.each([
        ['f101', 0xf101],
        ['0xF101', 0xf101],
        ['U+e001', 0xe001],
        ['\\f102', 0xf102],
        ['10ffff', 0x10ffff],
    ])('parses %s', (input, codepoint) => {
        expect(parseCodepoint(input)).toBe(codepoint);
    });

    it.each(['', 'zz', 'd800', '110000', '1234567'])('rejects %j', input => {
        expect(parseCodepoint(input)).toBeUndefined();
    });

    it('formats and classifies codepoints', () => {
        expect(formatCodepoint(0x41)).toBe('U+0041');
        expect(isPrivateUse(0xf101)).toBe(true);
        expect(isPrivateUse(0x41)).toBe(false);
    });

    it('writes a pasteable codepoints option', () => {
        expect(
            codepointsSnippet([
                { name: 'add', codepoint: 0xf101 },
                { name: 'arrow-right', codepoint: 0xe001 },
            ]),
        ).toBe("codepoints: {\n    add: 0xf101,\n    'arrow-right': 0xe001,\n},");
    });
});

describe('options', () => {
    const icons: PlaygroundIcon[] = [
        { id: 'a', name: 'add', svg: '<svg/>', codepoint: 0xf101 },
        { id: 'b', name: 'star', svg: '<svg/>', codepoint: 0xe001 },
    ];

    it('passes every codepoint explicitly and omits empty settings', () => {
        const options = toGenerateOptions(icons, defaultSettings);
        expect(options.sources).toEqual([
            { id: 'a', name: 'add', svg: '<svg/>' },
            { id: 'b', name: 'star', svg: '<svg/>' },
        ]);
        expect(options.codepoints).toEqual({ add: 0xf101, star: 0xe001 });
        expect(options.types).toEqual(['woff2', 'woff']);
        expect(options.fontStyle).toBeUndefined();
        expect(options.cssTemplate).toBeUndefined();
        expect(options.ts).toBe(false);
        expect(toGenerateOptions(icons, { ...defaultSettings, ts: true, css: false }).ts).toBe(true);
        expect(options.formatOptions).toEqual({ woff2: { compressionQuality: 11 } });
        expect(options.templateOptions).toEqual({ classPrefix: 'icon-', baseSelector: '.icon' });
    });

    it('orders formats and drops the ones color glyphs cannot use', () => {
        const settings = { ...defaultSettings, types: ['svg', 'woff2', 'eot'] as const, colorMode: 'all' as const };
        expect(unavailableTypes(settings)).toEqual(['eot', 'svg']);
        expect(selectedTypes({ ...settings, types: [...settings.types] })).toEqual(['woff2']);
        expect(selectedTypes({ ...defaultSettings, types: ['svg', 'woff2', 'eot'] })).toEqual(['eot', 'woff2', 'svg']);
    });

    it('only enables selected color glyphs when names are chosen', () => {
        expect(toGenerateOptions(icons, { ...defaultSettings, colorMode: 'selected' }).colorGlyphs).toBeUndefined();
        expect(toGenerateOptions(icons, { ...defaultSettings, colorMode: 'selected', colorNames: ['star'] }).colorGlyphs).toEqual(['star']);
    });

    it('merges template data under the naming settings', () => {
        const options = toGenerateOptions(icons, { ...defaultSettings, templateData: '{ "classPrefix": "x-", "brand": "acme" }' });
        expect(options.templateOptions).toEqual({ brand: 'acme', classPrefix: 'icon-', baseSelector: '.icon' });
    });

    it('explains invalid template data', () => {
        expect(() => parseTemplateData('{')).toThrow(/not valid JSON/);
        expect(() => parseTemplateData('[]')).toThrow(/JSON object/);
    });
});

describe('zip', () => {
    it('names files after the font so extracted CSS still finds them', async () => {
        const zip = unzipSync(await zipOutputs('icons', { woff2: new Uint8Array([1, 2]), css: new TextEncoder().encode('a{}') }));
        expect(Object.keys(zip).toSorted()).toEqual(['icons.css', 'icons.woff2']);
        expect(strFromU8(zip['icons.css']!)).toBe('a{}');
        expect(outputFileName('icons', 'html')).toBe('icons.html');
        const ts = 'export const icons = ["square"] as const;';
        const withTs = unzipSync(await zipOutputs('icons', { ts: new TextEncoder().encode(ts) }));
        expect(strFromU8(withTs['icons.ts']!)).toBe(ts);
    });
});

describe('highlight', () => {
    it('loads the themes the VitePress config uses', async () => {
        expect((await loadHighlighter()).getLoadedThemes().toSorted()).toEqual(Object.values(codeThemes).toSorted());
    });

    it('highlights SVG fonts as XML once requested', async () => {
        expect(await highlight('<svg><glyph unicode="&#xf101;" d="M0 0h10v10z"/></svg>', 'xml')).toContain('--shiki-dark:');
        expect((await loadHighlighter()).getLoadedLanguages()).toContain('xml');
    });

    it('loads the HTML grammar on first use', async () => {
        expect(await highlight('<p class="a">b</p>', 'html')).toContain('--shiki-dark:');
        expect((await loadHighlighter()).getLoadedLanguages()).toContain('html');
    });

    it('highlights TypeScript metadata', async () => {
        expect(await highlight('export const icons = ["square"] as const;', 'ts')).toContain('--shiki-dark:');
    });

    it('emits light and dark colors like the site code blocks', async () => {
        const html = await highlight('.icon { color: red; }', 'css');
        expect(html).toContain('--shiki-light:');
        expect(html).toContain('--shiki-dark:');
    });
});
