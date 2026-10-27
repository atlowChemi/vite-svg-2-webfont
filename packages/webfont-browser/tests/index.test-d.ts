import { expectTypeOf, it } from 'vite-plus/test';
import {
    generateWebfonts,
    type CollectedOutputs,
    type FontType,
    type GenerateWebfontsOptions,
    type JsonValue,
    type SourceChange,
    type SourceFontVariant,
    type SourceMembership,
    type SvgSource,
    type WebfontResult,
} from '../src/index.js';

const source: SvgSource = { id: 'square', name: 'square', svg: '<svg/>' };

it('accepts SVG text and variants with serializable options and an optional worker URL', () => {
    expectTypeOf(generateWebfonts({ sources: [source], incremental: true }, { workerUrl: '/worker.js' })).toEqualTypeOf<Promise<WebfontResult>>();
    const variants: SourceFontVariant[] = [
        { name: 'light', sources: [source], weight: 300, default: true },
        { name: 'bold', sources: [source], weight: 700 },
    ];
    const templateOptions: Record<string, JsonValue> = { nested: { values: [null, true, 42, 'text'] } };
    expectTypeOf(
        generateWebfonts(
            {
                variants,
                templateOptions,
                types: ['ttf', 'woff2'],
                colorGlyphs: ['square'],
                missingGlyphs: { behavior: 'fallback', variant: 'light' },
                formatOptions: { ttf: { ts: 0 }, woff2: { compressionQuality: 5 } },
            },
            { workerUrl: new URL('https://example.com/worker.js') },
        ),
    ).toEqualTypeOf<Promise<WebfontResult>>();
    expectTypeOf<GenerateWebfontsOptions['colorGlyphs']>().toEqualTypeOf<true | string[] | undefined>();
    expectTypeOf<GenerateWebfontsOptions['cache']>().toEqualTypeOf<Uint8Array | undefined>();
    expectTypeOf<WebfontResult['exportCache']>().toEqualTypeOf<() => Promise<Uint8Array<ArrayBuffer> | undefined>>();
    // @ts-expect-error Browser inputs are SVG text, not filesystem paths.
    void generateWebfonts({ files: ['square.svg'] });
    // @ts-expect-error Templates are inline strings, not callbacks.
    void generateWebfonts({ sources: [source], cssTemplate: () => '' });
    // @ts-expect-error Template values must be JSON serializable.
    void generateWebfonts({ sources: [source], templateOptions: { callback: () => '' } });
    // @ts-expect-error SVG source content is required.
    void generateWebfonts({ sources: [{ id: 'square', name: 'square' }] });
});

it('returns optional owned bytes, async rendering and synchronous disposal', async () => {
    using result = await generateWebfonts({ sources: [source], types: ['ttf'] });
    expectTypeOf(result).toEqualTypeOf<WebfontResult>();
    expectTypeOf<FontType>().toEqualTypeOf<'svg' | 'ttf' | 'eot' | 'woff' | 'woff2'>();
    expectTypeOf(result.font('ttf')).toEqualTypeOf<Promise<Uint8Array<ArrayBuffer> | undefined>>();
    expectTypeOf(result.font('svg')).toEqualTypeOf<Promise<Uint8Array<ArrayBuffer> | undefined>>();
    expectTypeOf(result.generateCss()).toEqualTypeOf<Promise<string>>();
    expectTypeOf(result.generateHtml()).toEqualTypeOf<Promise<string>>();
    expectTypeOf(result.generateTs()).toEqualTypeOf<Promise<string>>();
    expectTypeOf(result.collectOutputs()).toEqualTypeOf<Promise<CollectedOutputs>>();
    expectTypeOf<CollectedOutputs>().toEqualTypeOf<Partial<Record<FontType | 'css' | 'html' | 'ts', Uint8Array<ArrayBuffer>>>>();
    expectTypeOf(result.dispose()).toBeVoid();
    expectTypeOf(result[Symbol.dispose]()).toBeVoid();
    // @ts-expect-error Unsupported font containers are rejected.
    void result.font('otf');
});

it('accepts complete memberships and discriminated source changes', async () => {
    const result = await generateWebfonts({ sources: [source], incremental: true });
    const membership: SourceMembership = { sources: [source.id] };
    const family: SourceMembership = { variants: [{ name: 'light', ids: [source.id] }] };
    const changes: SourceChange[] = [
        { type: 'added', ...source },
        { type: 'changed', id: source.id, svg: '<svg/>', name: 'renamed' },
        { type: 'removed', id: source.id },
    ];
    expectTypeOf(result.regenerate(membership, changes)).toEqualTypeOf<Promise<void>>();
    expectTypeOf(result.regenerate(family, [])).toEqualTypeOf<Promise<void>>();
    // @ts-expect-error Membership must choose sources or variants, not both.
    void result.regenerate({ sources: [], variants: [] }, []);
    // @ts-expect-error Complete membership is required.
    void result.regenerate({}, []);
    // @ts-expect-error Changed sources require replacement SVG content.
    void result.regenerate(membership, [{ type: 'changed', id: source.id }]);
    // @ts-expect-error Added sources require a glyph name.
    void result.regenerate(membership, [{ type: 'added', id: source.id, svg: '<svg/>' }]);
});

it('exposes async single actions without returning replacement results', async () => {
    const result = await generateWebfonts({ sources: [source], incremental: true });
    expectTypeOf(result.add(source)).toEqualTypeOf<Promise<void>>();
    expectTypeOf(result.add(source, 'light')).toEqualTypeOf<Promise<void>>();
    expectTypeOf(result.update(source.id, '<svg/>')).toEqualTypeOf<Promise<void>>();
    expectTypeOf(result.update(source.id, '<svg/>', 'renamed')).toEqualTypeOf<Promise<void>>();
    expectTypeOf(result.remove(source.id)).toEqualTypeOf<Promise<void>>();
    // @ts-expect-error Add accepts SVG source objects, not file paths.
    void result.add('square.svg');
    // @ts-expect-error Update requires replacement SVG content.
    void result.update(source.id);
    // @ts-expect-error Removal accepts a source ID, not a source object.
    void result.remove(source);
});
