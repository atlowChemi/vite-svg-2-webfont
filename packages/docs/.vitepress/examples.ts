import { resolve } from 'node:path';
import type { Plugin, PluginOption, UserConfig } from 'vite';
import { createMarkdownRenderer, type MarkdownOptions } from 'vitepress';
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
const GENERATED_CSS_MODULE = 'virtual:docs-examples-css';

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

/**
 * Font URLs as a reader would recognize them: the font's file name, or a shortened data URL. The plugin points at
 * temporary files in builds; Vite rewrites those to the emitted asset paths.
 */
export function displayCss(css: string): string {
    return css.replaceAll(/url\("([^"]*)"\)/g, (_, url: string) => {
        if (url.startsWith('data:')) return `url("${url.slice(0, url.indexOf(',') + 1)}…")`;
        const file = url.split(/[?#]/)[0]!.split('/').at(-1)!;
        return `url("${file.replace(/-[0-9a-f]{8,}(\.\w+)$/, '$1')}")`;
    });
}

/** Render CSS as a fenced block through the site's own Markdown renderer, so it matches every other code block. */
function cssRenderer(markdown: MarkdownOptions): (code: string) => Promise<string> {
    let renderer: ReturnType<typeof createMarkdownRenderer> | undefined;
    return async code => {
        renderer ??= createMarkdownRenderer(resolve(import.meta.dirname, '..'), markdown);
        return (await renderer).renderAsync(`\`\`\`css\n${code.trimEnd()}\n\`\`\`\n`);
    };
}

type IsolatedExample = { plugin: Plugin; generatedCss: (context: unknown) => Promise<string> };

/** Serve one example's plugin instance under its own virtual module, with its CSS scoped to the example. */
function isolate(plugin: Plugin, slug: string): IsolatedExample {
    const publicId = `virtual:docs-example-${slug}.css`;
    const resolvedId = `\0${publicId}`;
    let pluginId: string | undefined;
    const ownId = async (context: unknown) => (pluginId ??= await callHook<string>(plugin.resolveId as Hook, context, PLUGIN_MODULE, undefined, {}));
    const generatedCss = async (context: unknown) => {
        const id = await ownId(context);
        return callHook<string>(plugin.transform as Hook, context, await callHook(plugin.load as Hook, context, id), id);
    };
    const isolated: Plugin = {
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
    return { plugin: isolated, generatedCss };
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

/**
 * One isolated plugin instance per example, a module importing every example's CSS, and a module with each
 * example's generated CSS highlighted for display.
 */
export function examplePlugins(markdown: MarkdownOptions): Plugin[] {
    const highlightCss = cssRenderer(markdown);
    const isolated = Object.entries(examples).map(([slug, config]) => [slug, isolate(findPlugin(config.plugins, slug), slug)] as const);
    const imports = isolated.map(([slug]) => `import 'virtual:docs-example-${slug}.css';`).join('\n');
    const virtualModules: Plugin = {
        name: 'docs-examples',
        resolveId: id => (id === ALL_EXAMPLES_MODULE || id === GENERATED_CSS_MODULE ? `\0${id}` : undefined),
        async load(id) {
            if (id === `\0${ALL_EXAMPLES_MODULE}`) return imports;
            if (id !== `\0${GENERATED_CSS_MODULE}`) return undefined;
            const entries = await Promise.all(isolated.map(async ([slug, { generatedCss }]) => [slug, await highlightCss(displayCss(await generatedCss(this)))]));
            return `export default ${JSON.stringify(Object.fromEntries(entries))};`;
        },
    };
    return [...isolated.map(([, { plugin }]) => plugin), virtualModules];
}
