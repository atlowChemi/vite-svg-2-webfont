import type { CollectedOutputs, WebfontGenerator, WebfontResult } from '@atlowchemi/webfont-browser-generator';
import { onBeforeUnmount, shallowRef, watch, type Ref } from 'vue';
import { loadEngineCache, saveEngineCache } from './engineCache';
import { toGenerateOptions, type PlaygroundIcon, type PlaygroundSettings } from './options';

export type WebfontBuild = {
    outputs: CollectedOutputs;
    css: string;
    html: string;
    ts: string;
    /** Font family registered for the preview, or undefined when no browser-loadable format was generated. */
    previewFamily?: string;
    /** Codepoints this build's font contains; icons added since show a placeholder until the next build. */
    codepoints: ReadonlySet<number>;
    milliseconds: number;
};

const DEBOUNCE_MS = 200;

/** Formats a browser can load through the FontFace API, in order of preference. */
const PREVIEW_FORMATS = ['woff2', 'woff', 'ttf'] as const;

/**
 * Start the generator's worker, which loads and compiles the WASM once for every build. The module and WASM are
 * hashed assets, cached as immutable, so later visits load them from the HTTP cache.
 */
async function createGenerator(): Promise<WebfontGenerator> {
    const { createWebfontGenerator } = await import('@atlowchemi/webfont-browser-generator');
    return createWebfontGenerator();
}

/** Run once the page is idle, so loading the page itself never waits for the generator. */
function whenIdle(): Promise<void> {
    const { promise, resolve } = Promise.withResolvers<void>();
    if ('requestIdleCallback' in window) requestIdleCallback(() => resolve(), { timeout: 2000 });
    else setTimeout(resolve, 200);
    return promise;
}

async function renderText(render: () => Promise<string>): Promise<string> {
    try {
        return await render();
    } catch (error) {
        return `/* ${(error as Error).message} */`;
    }
}

/**
 * Rebuild the font whenever the icons or settings change. Option changes need a fresh result, and every icon carries
 * an explicit codepoint option, so every change builds a new result; the last successful build stays visible while a
 * newer one fails. All builds share one generator, so the WASM loads once, while the page is idle. Each build hands
 * its engine cache to the next, so unchanged icons and formats are reused, and the latest cache is kept in IndexedDB
 * for the next visit.
 */
export function useWebfontBuild(icons: Ref<PlaygroundIcon[]>, settings: Ref<PlaygroundSettings>) {
    const build = shallowRef<WebfontBuild>();
    const error = shallowRef<string>();
    const building = shallowRef(false);
    let generation = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let face: FontFace | undefined;
    let result: WebfontResult | undefined;
    let cache: Promise<Uint8Array> = loadEngineCache();
    let generator: Promise<WebfontGenerator> | undefined = whenIdle().then(createGenerator);
    let unmounted = false;

    /** The shared generator, started now if the idle callback hasn't fired yet. */
    function currentGenerator(): Promise<WebfontGenerator> {
        return (generator ??= createGenerator());
    }

    /** A failed build may come from a crashed worker; start a new one, so the next build doesn't fail the same way. */
    function restartGenerator(failed: Promise<WebfontGenerator>) {
        if (generator !== failed) return;
        generator = undefined;
        void failed.then(
            stale => stale.dispose(),
            () => {},
        );
    }

    function replaceFace(next: FontFace | undefined) {
        if (face) document.fonts.delete(face);
        face = next;
        if (face) document.fonts.add(face);
    }

    /** With no icons there is nothing to build: release the previous result and show the empty state at once. */
    function clear() {
        result?.dispose();
        result = undefined;
        replaceFace(undefined);
        build.value = undefined;
        error.value = undefined;
        building.value = false;
    }

    /** `exportCache()` resolves undefined when nothing changed since the bytes were loaded, so the old ones stay. */
    async function keepCache(from: WebfontResult) {
        try {
            const updated = await from.exportCache();
            if (!updated) return;
            cache = Promise.resolve(updated);
            await saveEngineCache(updated);
        } catch {
            // A disposed result or failed export only costs the next build its warm start.
        }
    }

    async function run(current: number) {
        const started = performance.now();
        let next: WebfontResult | undefined;
        const used = currentGenerator();
        try {
            const options = toGenerateOptions(icons.value, settings.value);
            const [{ generateWebfonts }, bytes] = await Promise.all([used, cache]);
            next = await generateWebfonts({ ...options, cache: bytes });
            const [outputs, css, html, ts] = await Promise.all([next.collectOutputs(), renderText(next.generateCss), renderText(next.generateHtml), renderText(next.generateTs)]);
            const previewFormat = PREVIEW_FORMATS.find(format => options.types?.includes(format));
            const fontBytes = previewFormat && (await next.font(previewFormat));
            const family = `webfont-playground-${current}`;
            const nextFace = fontBytes ? await new FontFace(family, fontBytes).load() : undefined;
            if (current !== generation) {
                next.dispose();
                return;
            }
            result?.dispose();
            result = next;
            replaceFace(nextFace);
            build.value = {
                outputs,
                css,
                html,
                ts,
                previewFamily: nextFace && family,
                codepoints: new Set(Object.values(options.codepoints ?? {})),
                milliseconds: Math.round(performance.now() - started),
            };
            error.value = undefined;
            void keepCache(next);
        } catch (cause) {
            next?.dispose();
            if (!unmounted) restartGenerator(used);
            if (current === generation) error.value = cause instanceof Error ? cause.message : String(cause);
        } finally {
            if (current === generation) building.value = false;
        }
    }

    watch(
        [icons, settings],
        () => {
            // Bumping the generation also discards any build still in flight.
            const current = ++generation;
            clearTimeout(timer);
            if (!icons.value.length) {
                clear();
                return;
            }
            building.value = true;
            timer = setTimeout(() => run(current), DEBOUNCE_MS);
        },
        { deep: true, immediate: true },
    );

    onBeforeUnmount(() => {
        unmounted = true;
        generation++;
        clearTimeout(timer);
        result?.dispose();
        replaceFace(undefined);
        void generator?.then(
            created => created.dispose(),
            () => {},
        );
    });

    return { build, error, building };
}
