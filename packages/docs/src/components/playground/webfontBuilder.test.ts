import type { GenerateWebfontsOptions, WebfontResult } from '@atlowchemi/webfont-browser-generator';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultSettings, type PlaygroundIcon, type PlaygroundSettings } from './options.ts';
import { createWebfontBuilder, type WebfontBuildState } from './webfontBuilder.ts';

const cache = vi.hoisted(() => ({ stored: new Uint8Array([9]), saved: [] as Uint8Array[] }));
vi.mock('./engineCache', () => ({
    loadEngineCache: async () => cache.stored,
    saveEngineCache: async (bytes: Uint8Array) => void cache.saved.push(bytes),
}));

type FakeGenerator = { builds: GenerateWebfontsOptions[]; results: (WebfontResult & { disposed: boolean })[]; disposed: boolean; failNext?: Error };
const generators = vi.hoisted(() => [] as FakeGenerator[]);
vi.mock('@atlowchemi/webfont-browser-generator', () => ({
    createWebfontGenerator: () => {
        const generator: FakeGenerator = { builds: [], results: [], disposed: false };
        generators.push(generator);
        return {
            generateWebfonts: async (options: GenerateWebfontsOptions) => {
                generator.builds.push(options);
                if (generator.failNext) throw generator.failNext;
                const result: Record<string, unknown> = {
                    disposed: false,
                    font: async () => new Uint8Array([1]),
                    generateCss: async () => `css ${options.fontName}`,
                    generateHtml: async () => 'html',
                    generateTs: async () => 'ts',
                    collectOutputs: async () => ({ woff2: new Uint8Array([1]) }),
                    exportCache: async () => new Uint8Array([generator.builds.length]),
                    dispose: () => {
                        result.disposed = true;
                    },
                };
                generator.results.push(result as WebfontResult & { disposed: boolean });
                return result;
            },
            dispose: () => {
                generator.disposed = true;
            },
        };
    },
}));

const requestIdleCallback = (callback: () => void) => setTimeout(callback, 50);
const fonts = new Set<unknown>();
beforeEach(() => {
    vi.useFakeTimers();
    generators.length = 0;
    cache.saved.length = 0;
    fonts.clear();
    vi.stubGlobal('window', {});
    vi.stubGlobal('document', { fonts: { add: (face: unknown) => fonts.add(face), delete: (face: unknown) => fonts.delete(face) } });
    vi.stubGlobal(
        'FontFace',
        class {
            constructor(readonly family: string) {}
            async load() {
                return this;
            }
        },
    );
});
afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
});

const icon = (name: string, codepoint: number): PlaygroundIcon => ({ id: name, name, svg: '<svg/>', codepoint });

/** A builder for one icon, its first build requested; `change` requests the next build like an edit would. */
function setup() {
    let icons: PlaygroundIcon[] = [icon('add', 0xe001)];
    let settings: PlaygroundSettings = { ...structuredClone(defaultSettings), fontName: 'first' };
    const builder = createWebfontBuilder(() => {});
    builder.update(icons, settings);
    function change(next: { icons?: PlaygroundIcon[]; settings?: Partial<PlaygroundSettings> }) {
        icons = next.icons ?? icons;
        settings = { ...settings, ...next.settings };
        builder.update(icons, settings);
    }
    return { builder, change, icons: () => icons };
}

/** Run the debounce, the idle fallback and every build step. */
async function settle() {
    await vi.runAllTimersAsync();
}

describe('createWebfontBuilder', () => {
    it('creates the generator while idle, before any build is requested', async () => {
        vi.stubGlobal('window', { requestIdleCallback });
        vi.stubGlobal('requestIdleCallback', requestIdleCallback);
        createWebfontBuilder(() => {}).update([], structuredClone(defaultSettings));
        await settle();
        expect(generators).toHaveLength(1);
        expect(generators[0]!.builds).toEqual([]);
    });

    it('builds every change in one generator, passing the cache on and keeping the latest', async () => {
        const { builder, change, icons } = setup();
        await settle();
        expect(builder.state.build).toMatchObject({ css: 'css first', previewFamily: expect.stringMatching(/^webfont-playground-/) });
        expect(generators[0]!.builds[0]).toMatchObject({ fontName: 'first', cache: cache.stored });

        change({ settings: { fontName: 'second' } });
        change({ icons: [...icons(), icon('remove', 0xe002)] });
        await settle();
        expect(generators).toHaveLength(1);
        expect(generators[0]!.builds).toHaveLength(2);
        // The second build starts from the bytes the first one exported, and each export is stored.
        expect(generators[0]!.builds[1]).toMatchObject({ fontName: 'second', cache: new Uint8Array([1]), codepoints: { add: 0xe001, remove: 0xe002 } });
        expect(cache.saved).toEqual([new Uint8Array([1]), new Uint8Array([2])]);
        expect(builder.state.build).toMatchObject({ css: 'css second', codepoints: new Set([0xe001, 0xe002]) });
        // The replaced result is released; the shown one stays live, and only its face is registered.
        expect(generators[0]!.results.map(({ disposed }) => disposed)).toEqual([true, false]);
        expect(fonts.size).toBe(1);
    });

    it('reports building at once, then the finished build, to the listener', async () => {
        const states: WebfontBuildState[] = [];
        const builder = createWebfontBuilder(state => states.push(state));
        builder.update([icon('add', 0xe001)], structuredClone(defaultSettings));
        expect(states).toEqual([{ building: true }]);
        await settle();
        expect(states.at(-1)).toMatchObject({ building: false, error: undefined, build: { css: 'css iconfont' } });
        expect(builder.state).toBe(states.at(-1));
        builder.dispose();
        builder.update([], structuredClone(defaultSettings));
        expect(states).toHaveLength(2);
    });

    it('reports invalid settings as a build error', async () => {
        const { builder, change } = setup();
        await settle();
        change({ settings: { templateData: '[]' } });
        await settle();
        expect(builder.state).toMatchObject({ error: 'Template data must be a JSON object', build: { css: 'css first' }, building: false });
    });

    it('debounces bursts of changes into one build', async () => {
        const { change } = setup();
        await settle();
        change({ settings: { fontName: 'a' } });
        change({ settings: { fontName: 'b' } });
        await settle();
        expect(generators[0]!.builds.map(({ fontName }) => fontName)).toEqual(['first', 'b']);
    });

    it('keeps the last build visible after a failure and builds the next change in a new generator', async () => {
        const { builder, change } = setup();
        await settle();
        generators[0]!.failNext = new Error('worker crashed');
        change({ settings: { fontName: 'broken' } });
        await settle();
        expect(builder.state).toMatchObject({ error: 'worker crashed', building: false, build: { css: 'css first' } });
        expect(generators[0]!.disposed).toBe(true);

        change({ settings: { fontName: 'fixed' } });
        await settle();
        expect(generators).toHaveLength(2);
        expect(builder.state.error).toBeUndefined();
        expect(builder.state.build).toMatchObject({ css: 'css fixed' });
    });

    it('clears the build without generating when the last icon is removed', async () => {
        const { builder, change } = setup();
        await settle();
        change({ icons: [] });
        expect(builder.state).toEqual({ build: undefined, error: undefined, building: false });
        expect(generators[0]!.results[0]!.disposed).toBe(true);
        expect(fonts.size).toBe(0);
    });

    it('disposes the result, font face and generator', async () => {
        const { builder } = setup();
        await settle();
        builder.dispose();
        await settle();
        expect(generators[0]!.results[0]!.disposed).toBe(true);
        expect(generators[0]!.disposed).toBe(true);
        expect(fonts.size).toBe(0);
    });
});
