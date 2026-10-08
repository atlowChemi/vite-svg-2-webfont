import { describe, expect, it } from 'vite-plus/test';
import { validateOptions } from '../validations.js';

const variants = () => [
    { default: true, files: ['small.svg'], name: 'small', weight: 300 },
    { files: ['large.svg'], name: 'large', weight: 700 },
];

const options = (overrides = {}) => ({
    dest: 'artifacts',
    types: ['woff2'],
    variants: variants(),
    ...overrides,
});

describe('validateOptions', () => {
    it('accepts omitted, all, empty, and named color selection in both modes', () => {
        for (const colorGlyphs of [undefined, true, [], ['icon'], ['icon', 'icon']]) {
            expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], types: ['woff2'], colorGlyphs })).not.toThrow();
            expect(() => validateOptions(options({ colorGlyphs }))).not.toThrow();
        }
    });

    it('rejects invalid color values and sparse arrays before calling the native binding', () => {
        const sparse: string[] = [];
        sparse.length = 1;
        for (const colorGlyphs of [null, false, 'icon', 1, {}, ['icon', 1], ['icon', undefined], sparse]) {
            expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], colorGlyphs })).toThrow(/options\.colorGlyphs/);
            expect(() => validateOptions(options({ colorGlyphs }))).toThrow(/options\.colorGlyphs/);
        }
        expect(() => validateOptions(options({ colorGlyphs: ['icon', 1] }))).toThrow('options.colorGlyphs[1]');
    });

    it('accepts ordinary file options with default types', () => {
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'] })).not.toThrow();
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], order: ['eot'] })).not.toThrow();
    });

    it('validates active color formats and preserves disabled ordinary legacy output', () => {
        const ordinary = { dest: 'artifacts', files: ['icon.svg'] };
        for (const colorGlyphs of [true, ['icon']]) {
            expect(() => validateOptions({ ...ordinary, colorGlyphs })).toThrow('options.colorGlyphs: incompatible output formats: eot');
            expect(() => validateOptions({ ...ordinary, colorGlyphs, types: ['woff2', 'svg', 'eot'] })).toThrow('incompatible output formats: svg, eot');
            expect(() => validateOptions({ ...ordinary, colorGlyphs, types: ['ttf', 'woff', 'woff2'] })).not.toThrow();
            expect(() => validateOptions(options({ colorGlyphs, types: undefined }))).not.toThrow();
            expect(() => validateOptions(options({ colorGlyphs, types: ['eot'] }))).toThrow('options.types');
            expect(() => validateOptions(options({ colorGlyphs, types: ['svg'] }))).toThrow('options.types');
        }
        for (const colorGlyphs of [undefined, []]) {
            expect(() => validateOptions({ ...ordinary, colorGlyphs })).not.toThrow();
            expect(() => validateOptions({ ...ordinary, colorGlyphs, types: ['svg', 'eot'] })).not.toThrow();
        }
    });

    it('requires a destination', () => {
        expect(() => validateOptions({ files: ['icon.svg'] })).toThrow('options.dest');
    });

    it('rejects empty template paths', () => {
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], cssTemplate: '' })).toThrow('options.cssTemplate');
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], htmlTemplate: '' })).toThrow('options.htmlTemplate');
    });

    it('rejects invalid font order', () => {
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], order: ['svg'] })).toThrow("'svg' is not present in 'types'");
        expect(() => validateOptions(options({ order: ['eot'] }))).toThrow("'eot' is not present in 'types'");
    });

    it('requires either files or variants', () => {
        expect(() => validateOptions({ dest: 'artifacts' })).toThrow('Either "options.files" or "options.variants"');
    });

    it('rejects variant-only options without variants', () => {
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], missingGlyphs: { behavior: 'blank' } })).toThrow('options.missingGlyphs');
        expect(() => validateOptions({ dest: 'artifacts', files: ['icon.svg'], variantClassPrefix: 'icon--' })).toThrow('options.variantClassPrefix');
    });

    it('accepts valid variants and custom class options', () => {
        expect(() => validateOptions(options())).not.toThrow();
        expect(() => validateOptions(options({ templateOptions: {}, variantClassPrefix: 'weight--' }))).not.toThrow();
    });

    it('rejects files alongside variants', () => {
        expect(() => validateOptions(options({ files: ['icon.svg'] }))).toThrow('options.files');
    });

    it('requires at least two variants', () => {
        expect(() => validateOptions(options({ variants: variants().slice(0, 1) }))).toThrow('at least two variants');
    });

    it('rejects unsupported variant combinations', () => {
        expect(() => validateOptions(options({ types: ['svg'] }))).toThrow('options.types');
        expect(() => validateOptions(options({ types: ['eot'] }))).toThrow('options.types');
        expect(() => validateOptions(options({ incremental: true }))).not.toThrow();
        expect(() => validateOptions(options({ fontWeight: '400' }))).toThrow('options.fontWeight');
        expect(() => validateOptions(options({ templateOptions: { variantClassPrefix: 'weight--' } }))).toThrow('options.templateOptions.variantClassPrefix');
    });

    it('validates variant class prefixes', () => {
        expect(() => validateOptions(options({ variantClassPrefix: 1 }))).toThrow('options.variantClassPrefix');
        expect(() => validateOptions(options({ variantClassPrefix: '' }))).toThrow('options.variantClassPrefix');
        expect(() => validateOptions(options({ variantClassPrefix: 'icon prefix' }))).toThrow('options.variantClassPrefix');
        expect(() => validateOptions(options({ variantClassPrefix: 'icon\0' }))).toThrow('options.variantClassPrefix');
    });

    it('requires files in every variant', () => {
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], files: [] }, variants()[1]] }))).toThrow('options.variants[0].files');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], files: undefined }, variants()[1]] }))).toThrow('options.variants[0].files');
    });

    it('validates variant names', () => {
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], name: '' }, variants()[1]] }))).toThrow('options.variants[0].name');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], name: 'small icon' }, variants()[1]] }))).toThrow('whitespace');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], name: 'small\0' }, variants()[1]] }))).toThrow('NUL');
        expect(() => validateOptions(options({ variants: [variants()[0], { ...variants()[1], name: 'small' }] }))).toThrow('duplicates variant name');
    });

    it('validates explicit weights', () => {
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], weight: 1.5 }, variants()[1]] }))).toThrow('integer between 1 and 1000');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], weight: 0 }, variants()[1]] }))).toThrow('between 1 and 1000');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], weight: 1001 }, variants()[1]] }))).toThrow('between 1 and 1000');
        expect(() => validateOptions(options({ variants: [variants()[0], { ...variants()[1], weight: 200 }] }))).toThrow('preceding explicit weight');
    });

    it('accepts mixed automatic and explicit weights', () => {
        expect(() => validateOptions(options({ variants: [{ ...variants()[0], weight: undefined }, variants()[1]] }))).not.toThrow();
    });

    it('requires exactly one default variant', () => {
        expect(() =>
            validateOptions(
                options({
                    variants: [
                        { files: ['small.svg'], name: 'small' },
                        { files: ['large.svg'], name: 'large' },
                    ],
                }),
            ),
        ).toThrow('exactly one default variant');
        expect(() => validateOptions(options({ variants: [{ ...variants()[0] }, { ...variants()[1], default: true }] }))).toThrow('exactly one default variant');
    });

    it('validates missing-glyph fallbacks', () => {
        expect(() => validateOptions(options({ missingGlyphs: { behavior: 'fallback' } }))).toThrow('variant" is required');
        expect(() => validateOptions(options({ missingGlyphs: { behavior: 'fallback', variant: 'unknown' } }))).toThrow('does not name a configured variant');
        expect(() => validateOptions(options({ missingGlyphs: { behavior: 'fallback', variant: 'small' } }))).not.toThrow();
    });

    it('rejects fallback variants for non-fallback behavior', () => {
        expect(() => validateOptions(options({ missingGlyphs: { behavior: 'blank', variant: 'small' } }))).toThrow('only valid when behavior is "fallback"');
        expect(() => validateOptions(options({ missingGlyphs: { behavior: 'error' } }))).not.toThrow();
    });
});
