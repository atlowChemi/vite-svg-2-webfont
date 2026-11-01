import type { GenerateWebfontsOptions, WebfontResult } from '@atlowchemi/webfont-browser-generator';
import { nextTick, ref } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { defaultSettings, type PlaygroundIcon, type PlaygroundSettings } from './options';
import { useWebfontBuild } from './useWebfontBuild';

const hooks = vi.hoisted(() => ({ unmount: [] as (() => void)[] }));
vi.mock('vue', async importOriginal => ({ ...(await importOriginal<typeof import('vue')>()), onBeforeUnmount: (hook: () => void) => hooks.unmount.push(hook) }));

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
    hooks.unmount.length = 0;
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

function setup() {
    const icons = ref<PlaygroundIcon[]>([icon('add', 0xe001)]);
    const settings = ref<PlaygroundSettings>({ ...structuredClone(defaultSettings), fontName: 'first' });
    return { icons, settings, ...useWebfontBuild(icons, settings) };
}

/** Run the debounce, the idle fallback and every build step. */
async function settle() {
    await vi.runAllTimersAsync();
}

describe('useWebfontBuild', () => {
    it('creates the generator while idle, before any build is requested', async () => {
        vi.stubGlobal('window', { requestIdleCallback });
        vi.stubGlobal('requestIdleCallback', requestIdleCallback);
        const icons = ref<PlaygroundIcon[]>([]);
        useWebfontBuild(icons, ref(structuredClone(defaultSettings)));
        await settle();
        expect(generators).toHaveLength(1);
        expect(generators[0]!.builds).toEqual([]);
    });

    it('builds every change in one generator, passing the cache on and keeping the latest', async () => {
        const { icons, settings, build } = setup();
        await settle();
        expect(build.value).toMatchObject({ css: 'css first', previewFamily: expect.stringMatching(/^webfont-playground-/) });
        expect(generators[0]!.builds[0]).toMatchObject({ fontName: 'first', cache: cache.stored });

        settings.value.fontName = 'second';
        icons.value = [...icons.value, icon('remove', 0xe002)];
        await nextTick();
        await settle();
        expect(generators).toHaveLength(1);
        expect(generators[0]!.builds).toHaveLength(2);
        // The second build starts from the bytes the first one exported, and each export is stored.
        expect(generators[0]!.builds[1]).toMatchObject({ fontName: 'second', cache: new Uint8Array([1]), codepoints: { add: 0xe001, remove: 0xe002 } });
        expect(cache.saved).toEqual([new Uint8Array([1]), new Uint8Array([2])]);
        expect(build.value).toMatchObject({ css: 'css second', codepoints: new Set([0xe001, 0xe002]) });
        // The replaced result is released; the shown one stays live, and only its face is registered.
        expect(generators[0]!.results.map(({ disposed }) => disposed)).toEqual([true, false]);
        expect(fonts.size).toBe(1);
    });

    it('debounces bursts of changes into one build', async () => {
        const { settings } = setup();
        await settle();
        settings.value.fontName = 'a';
        await nextTick();
        settings.value.fontName = 'b';
        await nextTick();
        await settle();
        expect(generators[0]!.builds.map(({ fontName }) => fontName)).toEqual(['first', 'b']);
    });

    it('keeps the last build visible after a failure and builds the next change in a new generator', async () => {
        const { settings, build, error } = setup();
        await settle();
        generators[0]!.failNext = new Error('worker crashed');
        settings.value.fontName = 'broken';
        await nextTick();
        await settle();
        expect(error.value).toBe('worker crashed');
        expect(build.value).toMatchObject({ css: 'css first' });
        expect(generators[0]!.disposed).toBe(true);

        settings.value.fontName = 'fixed';
        await nextTick();
        await settle();
        expect(generators).toHaveLength(2);
        expect(error.value).toBeUndefined();
        expect(build.value).toMatchObject({ css: 'css fixed' });
    });

    it('clears the build without generating when the last icon is removed', async () => {
        const { icons, build } = setup();
        await settle();
        icons.value = [];
        await nextTick();
        expect(build.value).toBeUndefined();
        expect(generators[0]!.results[0]!.disposed).toBe(true);
        expect(fonts.size).toBe(0);
    });

    it('disposes the result, font face and generator on unmount', async () => {
        setup();
        await settle();
        for (const hook of hooks.unmount) hook();
        await settle();
        expect(generators[0]!.results[0]!.disposed).toBe(true);
        expect(generators[0]!.disposed).toBe(true);
        expect(fonts.size).toBe(0);
    });
});
