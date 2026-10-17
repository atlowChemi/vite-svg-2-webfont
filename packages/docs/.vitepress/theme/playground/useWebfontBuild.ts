import type { CollectedOutputs, WebfontResult } from '@atlowchemi/webfont-browser-generator';
import { onBeforeUnmount, shallowRef, watch, type Ref } from 'vue';
import { toGenerateOptions, type PlaygroundIcon, type PlaygroundSettings } from './options';

export type WebfontBuild = {
    outputs: CollectedOutputs;
    css: string;
    html: string;
    /** Font family registered for the preview, or undefined when no browser-loadable format was generated. */
    previewFamily?: string;
    /** Codepoints this build's font contains; icons added since show a placeholder until the next build. */
    codepoints: ReadonlySet<number>;
    milliseconds: number;
};

const DEBOUNCE_MS = 200;

/** Formats a browser can load through the FontFace API, in order of preference. */
const PREVIEW_FORMATS = ['woff2', 'woff', 'ttf'] as const;

async function renderText(render: () => Promise<string>): Promise<string> {
    try {
        return await render();
    } catch (error) {
        return `/* ${(error as Error).message} */`;
    }
}

/**
 * Rebuild the font whenever the icons or settings change. Option changes need a fresh result, so every change
 * builds from scratch; the last successful build stays visible while a newer one fails.
 */
export function useWebfontBuild(icons: Ref<PlaygroundIcon[]>, settings: Ref<PlaygroundSettings>) {
    const build = shallowRef<WebfontBuild>();
    const error = shallowRef<string>();
    const building = shallowRef(false);
    let generation = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let face: FontFace | undefined;
    let result: WebfontResult | undefined;

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

    async function run(current: number) {
        const started = performance.now();
        let next: WebfontResult | undefined;
        try {
            const options = toGenerateOptions(icons.value, settings.value);
            const { generateWebfonts } = await import('@atlowchemi/webfont-browser-generator');
            next = await generateWebfonts(options);
            const [outputs, css, html] = await Promise.all([next.collectOutputs(), renderText(next.generateCss), renderText(next.generateHtml)]);
            const previewFormat = PREVIEW_FORMATS.find(format => options.types?.includes(format));
            const bytes = previewFormat && (await next.font(previewFormat));
            const family = `webfont-playground-${current}`;
            const nextFace = bytes ? await new FontFace(family, bytes).load() : undefined;
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
                previewFamily: nextFace && family,
                codepoints: new Set(Object.values(options.codepoints ?? {})),
                milliseconds: Math.round(performance.now() - started),
            };
            error.value = undefined;
        } catch (cause) {
            next?.dispose();
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
        generation++;
        clearTimeout(timer);
        result?.dispose();
        replaceFace(undefined);
    });

    return { build, error, building };
}
