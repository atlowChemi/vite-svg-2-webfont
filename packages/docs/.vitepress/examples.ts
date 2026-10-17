import type { Plugin, PluginOption, UserConfig } from 'vite';
import basic from '../examples/basic/vite.config';
import customNaming from '../examples/custom-naming/vite.config';
import ligatures from '../examples/ligatures/vite.config';
import typeSafeComponent from '../examples/type-safe-component/vite.config';
import customCssTemplate from '../examples/custom-css-template/vite.config';
import multiWeight from '../examples/multi-weight/vite.config';
import colorGlyphs from '../examples/color-glyphs/vite.config';
import pinnedCodepoints from '../examples/pinned-codepoints/vite.config';
import inlineFonts from '../examples/inline-fonts/vite.config';
import preload from '../examples/preload/vite.config';
import writeFiles from '../examples/write-files/vite.config';

/** Example slugs and their real configs. Each config must give the plugin a unique `fontName`. */
const examples: Record<string, UserConfig> = {
    basic: basic,
    'custom-naming': customNaming,
    ligatures: ligatures,
    'type-safe-component': typeSafeComponent,
    'custom-css-template': customCssTemplate,
    'multi-weight': multiWeight,
    'color-glyphs': colorGlyphs,
    'pinned-codepoints': pinnedCodepoints,
    'inline-fonts': inlineFonts,
    preload: preload,
    'write-files': writeFiles,
};

const PLUGIN_NAME = 'vite-svg-2-webfont';
const PLUGIN_MODULE = 'virtual:vite-svg-2-webfont.css';
const ALL_EXAMPLES_MODULE = 'virtual:docs-examples';

type Hook = ((this: unknown, ...args: never[]) => unknown) | { handler: (this: unknown, ...args: never[]) => unknown } | undefined;

function callHook<T>(hook: Hook, context: unknown, ...args: unknown[]): Promise<T> {
    const handler = typeof hook === 'function' ? hook : hook?.handler;
    return Promise.resolve(handler?.call(context, ...(args as never[])) as T);
}

/**
 * Keep the generated CSS working inside one container: `@font-face` and `@keyframes` stay global, and every
 * other rule is nested under the container class, so examples on one page can share class names.
 */
export function scopeCss(css: string, scope: string): string {
    const hoisted: string[] = [];
    const nested: string[] = [];
    const source = css.replaceAll(/\/\*[\s\S]*?\*\//g, '');
    let depth = 0;
    let start = 0;
    for (let index = 0; index < source.length; index++) {
        if (source[index] === '{') depth++;
        else if (source[index] === '}' && --depth === 0) {
            const block = source.slice(start, index + 1).trim();
            (/^@(?:font-face|keyframes)\b/.test(block) ? hoisted : nested).push(block);
            start = index + 1;
        }
    }
    return `${hoisted.join('\n')}\n.${scope} {\n${nested.join('\n')}\n}\n`;
}

/** Serve one example's plugin instance under its own virtual module, with its CSS scoped to the example. */
function isolate(plugin: Plugin, slug: string): Plugin {
    const publicId = `virtual:docs-example-${slug}.css`;
    const resolvedId = `\0${publicId}`;
    let pluginId: string | undefined;
    const ownId = async (context: unknown) => (pluginId ??= await callHook<string>(plugin.resolveId as Hook, context, PLUGIN_MODULE, undefined, {}));
    return {
        ...plugin,
        name: `${PLUGIN_NAME}:${slug}`,
        resolveId: id => (id === publicId ? resolvedId : undefined),
        async load(id) {
            if (id !== resolvedId) return undefined;
            return callHook(plugin.load as Hook, this, await ownId(this));
        },
        async transform(code, id) {
            if (id !== resolvedId) return undefined;
            const css = await callHook<string>(plugin.transform as Hook, this, code, await ownId(this));
            return scopeCss(css, `vp-example-${slug}`);
        },
    };
}

function findPlugin(options: PluginOption[] | undefined, slug: string): Plugin {
    const pending: unknown[] = [...(options ?? [])];
    while (pending.length) {
        const option = pending.shift();
        if (Array.isArray(option)) pending.push(...option);
        else if (typeof option === 'object' && option !== null && 'name' in option && option.name === PLUGIN_NAME) return option as Plugin;
    }
    throw new Error(`Example "${slug}" does not register ${PLUGIN_NAME}`);
}

/** One isolated plugin instance per example, plus a module that imports every example's CSS. */
export function examplePlugins(): Plugin[] {
    const isolated = Object.entries(examples).map(([slug, config]) => isolate(findPlugin(config.plugins, slug), slug));
    const imports = Object.keys(examples)
        .map(slug => `import 'virtual:docs-example-${slug}.css';`)
        .join('\n');
    const allExamples: Plugin = {
        name: 'docs-examples',
        resolveId: id => (id === ALL_EXAMPLES_MODULE ? `\0${ALL_EXAMPLES_MODULE}` : undefined),
        load: id => (id === `\0${ALL_EXAMPLES_MODULE}` ? imports : undefined),
    };
    return [...isolated, allExamples];
}
