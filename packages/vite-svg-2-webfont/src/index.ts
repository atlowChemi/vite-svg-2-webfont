import { join as pathJoin } from 'node:path';
import type { ModuleGraph, ModuleNode, Plugin } from 'vite-plus';
import { setupWatcher, MIME_TYPES, ensureDirExistsAndWriteFile, getTmpDir, getBufferHash, rmDir } from './utils';
import { parseOptions, resolveSources, parsePreloadFormatsOption } from './optionParser';
import type { ParsedOptions } from './optionParser';
import { createGenerationQueue, regenerationFiles, reconcileChanges, watchRoots } from './regeneration';
import { CACHE_PLACEHOLDER, computeCacheKey, fontFileName, readCacheEntry, writeCacheEntry } from './cache';
import type { CacheEntry } from './cache';
import * as templatesImport from '@atlowchemi/webfont-generator/templates';
import type { WatchedChangeBatch } from './utils';
import type { FontType, GenerateWebfontsResult, MultiVariantFontType } from '@atlowchemi/webfont-generator';
import type { IconPluginOptions, IconPluginFileOptions, IconPluginVariantOptions } from './optionParser';
import type { GeneratedWebfont } from './types/generatedWebfont';
import type { PublicApi } from './types/publicApi';

type GenerateBundle = Extract<NonNullable<Plugin['generateBundle']>, Function>;
type OutputBundle = Parameters<GenerateBundle>[1];

const DEFAULT_MODULE_ID = 'vite-svg-2-webfont.css';

function getVirtualModuleId<T extends string>(moduleId: T): `virtual:${T}` {
    return `virtual:${moduleId}`;
}

function getResolvedVirtualModuleId<T extends string>(virtualModuleId: T): `\0${T}` {
    return `\0${virtualModuleId}`;
}

/** Ensure vp doesn't crash locally and in CI if the native binding doesn't exist yet.  */
let _generateWebfonts: (typeof import('@atlowchemi/webfont-generator'))['generateWebfonts'] | undefined;
async function getGenerateWebfonts() {
    return (_generateWebfonts ??= (await import('@atlowchemi/webfont-generator')).generateWebfonts);
}

/**
 * A Vite plugin that generates a webfont from your SVG icons.
 *
 * The plugin uses the {@link https://www.npmjs.com/package/@atlowchemi/webfont-generator @atlowchemi/webfont-generator} native engine to create fonts in any format.
 * It also generates CSS files that allow using the icons directly in your HTML output, using CSS classes per-icon.
 */
export function viteSvgToWebfont<T extends MultiVariantFontType = MultiVariantFontType>(options: IconPluginVariantOptions<T>): Plugin<PublicApi>;
export function viteSvgToWebfont<T extends FontType = FontType>(options: IconPluginFileOptions<T>): Plugin<PublicApi>;
export function viteSvgToWebfont<T extends FontType = FontType>(options: IconPluginOptions<T>): Plugin<PublicApi>;
export function viteSvgToWebfont<T extends FontType = FontType>(options: IconPluginOptions<T>): Plugin<PublicApi> {
    let ac = new AbortController();
    let generationQueue = createGenerationQueue(ac.signal);
    let tmpDir = getTmpDir();
    const processedOptions = parseOptions(options);
    const preloadFormats = parsePreloadFormatsOption<T>(options).filter((type): type is T => processedOptions.types.includes(type as any));
    let isBuild: boolean;
    let cacheFolder: string;
    /** Rendered outputs for the current fonts; `generatedFonts` is unset after a cache hit. */
    let outputs: CacheEntry | undefined;
    let _moduleGraph: ModuleGraph | undefined;
    let _reloadModule: undefined | ((module: ModuleNode) => Promise<void>);
    let generatedFonts: GenerateWebfontsResult<T, never> | undefined;
    let publishedSources = regenerationFiles(processedOptions);
    // Recursive watchers also report SVGs outside the globs, so their events are reconciled against membership.
    const reconcileWatchEvents = Array.isArray(watchRoots(options));
    let watcherTask: Promise<void> | undefined;
    const generatedFontBuffers = new Map<T, Buffer>();
    const generatedWebfonts: GeneratedWebfont[] = [];
    const moduleId = options.moduleId ?? DEFAULT_MODULE_ID;
    const virtualModuleId = getVirtualModuleId(moduleId);
    const resolvedVirtualModuleId = getResolvedVirtualModuleId(virtualModuleId);

    const getGeneratedFont = (type: T) => {
        const cached = generatedFontBuffers.get(type);
        if (cached) {
            return cached;
        }
        const value = generatedFonts?.[type];
        if (!value) {
            return undefined;
        }
        const font = Buffer.from(value);
        generatedFontBuffers.set(type, font);
        return font;
    };

    const resolveGeneratedWebfonts = (bundle: OutputBundle) => {
        const resolvedWebfonts = new Map<T, string>();

        for (const chunk of Object.values(bundle)) {
            if (chunk.type !== 'asset') {
                continue;
            }
            const lastSlashIndex = chunk.fileName.lastIndexOf('/');
            const fileName = chunk.fileName.slice(lastSlashIndex + 1);
            if (!fileName.startsWith(processedOptions.fontName)) {
                continue;
            }

            const fontType = (processedOptions.types as T[]).find(type => fileName.endsWith(`.${type}`));
            /* v8 ignore next 3 -- guard against malformed bundle chunks; never reached by Vite in practice */
            if (!fontType || resolvedWebfonts.has(fontType)) {
                continue;
            }

            resolvedWebfonts.set(fontType, `/${chunk.fileName}`);
        }

        return Array.from(resolvedWebfonts, ([type, href]) => ({ type, href }));
    };

    const inline = <U extends string | undefined>(css: U) => {
        if (!options.inline) {
            return css;
        }
        return css?.replace(/url\(".*?\.([^?]+)\?[^"]+"\)/g, (_, type: T) => {
            const font = getGeneratedFont(type)!;
            return `url("data:${MIME_TYPES[type]};charset=utf-8;base64,${font.toString('base64')}")`;
        }) as U;
    };

    const getCss = () => outputs?.css ?? generatedFonts!.generateCss();
    const getHtml = () => outputs?.html ?? generatedFonts!.generateHtml();

    let lastCssOutput: string | undefined;
    let lastHtmlOutput: string | undefined;

    /** Write the companion CSS/HTML to disk in dev, skipping any file whose rendered output is unchanged. */
    const writeDevFiles = async () => {
        if (isBuild || processedOptions.writeFiles || !(processedOptions.css || processedOptions.html)) {
            return;
        }
        const tasks: Array<Promise<void>> = [];
        if (processedOptions.css) {
            const css = inline(getCss());
            if (css !== lastCssOutput) {
                tasks.push(
                    ensureDirExistsAndWriteFile(css, processedOptions.cssDest).then(() => {
                        lastCssOutput = css;
                        return undefined;
                    }),
                );
            }
        }
        if (processedOptions.html) {
            const html = getHtml();
            if (html !== lastHtmlOutput) {
                tasks.push(
                    ensureDirExistsAndWriteFile(html, processedOptions.htmlDest).then(() => {
                        lastHtmlOutput = html;
                        return undefined;
                    }),
                );
            }
        }
        await Promise.all(tasks);
    };

    const reloadVirtualModule = async () => {
        const module = _moduleGraph?.getModuleById(resolvedVirtualModuleId);
        if (module && _reloadModule) {
            await _reloadModule(module).catch(() => null);
        }
    };

    const getCacheKey = async (input: ParsedOptions) => {
        // cssContext is a JS callback that can change the output in ways a key can't capture.
        if (processedOptions.cssContext) return null;
        const files = [...(input.files ?? input.variants.flatMap(variant => variant.files)), ...[input.cssTemplate, input.htmlTemplate].filter(file => file !== undefined)];
        try {
            return await computeCacheKey(input, files, [isBuild, Boolean(options.inline)], ac.signal);
        } catch {
            return null;
        }
    };

    const renderModuleCss = (fonts: Record<string, string>) =>
        generatedFonts!.generateCss(Object.fromEntries(Object.entries(fonts).map(([type, name]) => [type, `${CACHE_PLACEHOLDER}/${name}`])));

    /** Render the outputs of a fresh engine result and persist them for the next run. */
    const storeOutputs = async (key: string | null) => {
        const buffers = new Map(
            (processedOptions.types as T[]).map(type => {
                const font = getGeneratedFont(type);
                if (!font) {
                    throw new Error(`Failed to generate font of type ${type}`);
                }
                return [type, font];
            }),
        );
        const fonts = Object.fromEntries([...buffers].map(([type, font]) => [type, fontFileName(processedOptions.fontName, type, getBufferHash(font))]));
        // Rendering can fail on templates the run may never use (e.g. CSS disabled and the module never imported).
        // Leave such outputs to the lazy getters, which surface the engine error only if they're needed, and never hit.
        const render = (fn: () => string) => {
            try {
                return fn();
            } catch {
                key = null;
                return undefined;
            }
        };
        const css = render(() => generatedFonts!.generateCss());
        const moduleCss = isBuild && !options.inline ? render(() => renderModuleCss(fonts)) : undefined;
        const html = processedOptions.html ? render(() => generatedFonts!.generateHtml()) : undefined;
        const entry: CacheEntry = { key, fonts, css, ...(moduleCss !== undefined && { moduleCss }), ...(html !== undefined && { html }) };
        // The cache is an optimization; an unwritable cacheDir must not fail generation.
        await writeCacheEntry(cacheFolder, processedOptions.fontName, isBuild ? 'build' : 'dev', entry, buffers).catch(() => undefined);
        return entry;
    };

    /** Mirror the files the engine writes itself when `writeFiles` is set. */
    const writeEngineFiles = async () => {
        await Promise.all([
            ...(processedOptions.types as T[]).map(type =>
                ensureDirExistsAndWriteFile(getGeneratedFont(type)!, pathJoin(processedOptions.dest, `${processedOptions.fontName}.${type}`)),
            ),
            processedOptions.css && ensureDirExistsAndWriteFile(outputs!.css!, processedOptions.cssDest),
            processedOptions.html && ensureDirExistsAndWriteFile(outputs!.html!, processedOptions.htmlDest),
        ]);
    };

    const generate = async (updateFiles?: boolean) => {
        const sources = updateFiles ? resolveSources(options) : undefined;
        if (isBuild && !options.allowWriteFilesInBuild) {
            processedOptions.writeFiles = false;
        }
        const input = { ...processedOptions, ...sources } as ParsedOptions;
        const key = await getCacheKey(input);
        // Watch-triggered rebuilds follow input changes, so only the initial run looks up the cache.
        const hit = key && !updateFiles ? await readCacheEntry(cacheFolder, processedOptions.fontName, isBuild ? 'build' : 'dev', key, processedOptions.types as T[]) : undefined;
        if (ac.signal.aborted) {
            return;
        }
        if (hit) {
            generatedFonts = undefined;
            outputs = hit.entry;
            generatedFontBuffers.clear();
            hit.fonts.forEach((font, type) => generatedFontBuffers.set(type, font));
            if (processedOptions.writeFiles) {
                await writeEngineFiles();
            }
        } else {
            const generateWebfonts = await getGenerateWebfonts();
            const replacement = await generateWebfonts(input as Parameters<typeof generateWebfonts>[0]);
            if (ac.signal.aborted) {
                return;
            }
            generatedFonts = replacement as GenerateWebfontsResult<T, never>;
            generatedFontBuffers.clear();
            outputs = await storeOutputs(key);
        }
        if (sources) publishedSources = regenerationFiles(sources);
        await writeDevFiles();
        if (updateFiles && !ac.signal.aborted) {
            await reloadVirtualModule();
        }
    };

    /** Rebuild on watch events, incrementally reusing cached glyphs, else a full rebuild. */
    const regenerateFromWatch = async (changes: WatchedChangeBatch) => {
        if (changes.length === 0) {
            return;
        }
        // Reuse cached glyphs, handing the engine the fresh glob order so additions land in their
        // correct position (byte-identical to a fresh build). When `writeFiles` is set, regeneration
        // refreshes the on-disk fonts itself; otherwise `writeDevFiles` handles the CSS/HTML below.
        if (!generatedFonts || !processedOptions.incremental) {
            await generate(true);
            return;
        }
        try {
            const sources = regenerationFiles(resolveSources(options));
            const hints = reconcileWatchEvents ? reconcileChanges(publishedSources, sources, changes) : changes.map(change => ({ path: change.path, changeType: change.kind }));
            if (hints?.length === 0 && JSON.stringify(sources) === JSON.stringify(publishedSources)) return;
            const replacement = await generatedFonts.regenerateAsync(sources, hints);
            if (ac.signal.aborted) {
                return;
            }
            generatedFonts = replacement;
            outputs = undefined;
            publishedSources = sources;
            generatedFontBuffers.clear();
        } catch {
            if (ac.signal.aborted) {
                return;
            }
            await generate(true);
            return;
        }
        await writeDevFiles();
        if (ac.signal.aborted) {
            return;
        }
        await reloadVirtualModule();
    };
    return {
        name: 'vite-svg-2-webfont',
        enforce: 'pre',
        api: {
            getGeneratedWebfonts(): GeneratedWebfont[] {
                return generatedWebfonts;
            },
        },
        configResolved(_config) {
            isBuild = _config.command === 'build';
            cacheFolder = `${_config.cacheDir}/vite-svg-2-webfont`;
            if (!isBuild) {
                // Retain parsed glyphs so watch rebuilds can reuse them via `result.regenerateAsync(...)`.
                // cssContext runs in JS during generation, so skip the incremental path when
                // callbacks would need to be replayed after a rebuild.
                if (!processedOptions.cssContext) {
                    processedOptions.incremental = true;
                }
                // In dev, default WOFF2 to a faster Brotli quality unless the user pinned one.
                // Rebuilds run on every icon change and the dev-served font is never shipped, so the
                // marginally larger output is irrelevant while the ~2x faster encode speeds up rebuilds.
                if (processedOptions.formatOptions.woff2?.compressionQuality === undefined) {
                    processedOptions.formatOptions.woff2 = { ...processedOptions.formatOptions.woff2, compressionQuality: 10 };
                }
            }
        },
        resolveId(id) {
            if (id !== virtualModuleId) {
                return undefined;
            }
            return resolvedVirtualModuleId;
        },
        generateBundle(_, bundle) {
            const resolvedGeneratedWebfonts = resolveGeneratedWebfonts(bundle);
            generatedWebfonts.push(...resolvedGeneratedWebfonts);
        },
        transformIndexHtml: {
            order: 'post',
            handler(_html, ctx) {
                if (!isBuild || options.inline || preloadFormats.length === 0 || !('bundle' in ctx) || !ctx.bundle) {
                    return undefined;
                }
                if (options.shouldProcessHtml && !options.shouldProcessHtml(ctx)) {
                    return undefined;
                }

                const preloadWebfonts = resolveGeneratedWebfonts(ctx.bundle).filter(({ type }) => preloadFormats.includes(type));

                if (preloadWebfonts.length === 0) {
                    return undefined;
                }

                return preloadWebfonts.map(({ type, href }) => ({
                    tag: 'link',
                    attrs: {
                        rel: 'preload',
                        href,
                        as: 'font',
                        type: MIME_TYPES[type],
                        crossorigin: true,
                    },
                    injectTo: 'head',
                }));
            },
        },
        transform(_code, id) {
            if (id !== resolvedVirtualModuleId) {
                return undefined;
            }
            if (isBuild && !options.inline) {
                return (outputs!.moduleCss ?? renderModuleCss(outputs!.fonts)).replaceAll(`${CACHE_PLACEHOLDER}/`, `${tmpDir.replaceAll('\\', '/')}/`);
            }
            return inline(getCss());
        },
        load(id) {
            if (id !== resolvedVirtualModuleId) {
                return undefined;
            }
            return resolvedVirtualModuleId;
        },
        async buildStart() {
            if (ac.signal.aborted) {
                // Hosts such as VitePress (client + SSR) or Vite's environment builder reuse one plugin
                // instance across builds; the previous buildEnd aborted its signal and removed its temp dir.
                ac = new AbortController();
                generationQueue = createGenerationQueue(ac.signal);
                tmpDir = getTmpDir();
            }
            const initial = generationQueue.run(() => generate());
            if (!isBuild) {
                watcherTask = setupWatcher(watchRoots(options), ac.signal, changes =>
                    generationQueue
                        .run(() => regenerateFromWatch(changes))
                        .catch(error => {
                            this.warn(`Font regeneration failed: ${String(error)}`);
                            throw error;
                        }),
                ).catch(error => {
                    this.warn(`Font watcher failed: ${String(error)}`);
                });
            }
            await initial;
            if (isBuild && !options.inline) {
                // Copy fonts to the per-run temp dir rather than referencing the shared cache folder,
                // so another process pruning the cache can't remove files this build still resolves.
                // Await the writes: Vite reads these files when it resolves the url() references in the
                // virtual module's CSS. If a write has not flushed by then the asset is silently dropped.
                await Promise.all(Object.entries(outputs?.fonts ?? {}).map(([type, name]) => ensureDirExistsAndWriteFile(getGeneratedFont(type as T)!, pathJoin(tmpDir, name))));
            }
        },
        configureServer(server) {
            const { moduleGraph } = server;
            _moduleGraph = moduleGraph;
            _reloadModule = server.reloadModule.bind(server);
            if (options.inline) {
                return;
            }
            const { middlewares } = server;
            for (const fontType of processedOptions.types) {
                const fileName = `${processedOptions.fontName}.${fontType}`;
                middlewares.use(`/${fileName}`, (_req, res) => {
                    const font = getGeneratedFont(fontType as T);
                    /* v8 ignore next 4 -- buildStart awaits generate() before the server listens, so fonts are always available by the time middleware fires */
                    if (!font) {
                        res.statusCode = 404;
                        return res.end();
                    }
                    res.setHeader('content-type', MIME_TYPES[fontType]);
                    res.setHeader('content-length', font.length);
                    res.statusCode = 200;
                    return res.end(font);
                });
            }
        },
        async buildEnd() {
            ac.abort();
            await watcherTask;
            await generationQueue.idle();
            watcherTask = undefined;
            rmDir(tmpDir);
            generatedFonts = undefined;
            generatedFontBuffers.clear();
            outputs = undefined;
            _moduleGraph = undefined;
            _reloadModule = undefined;
        },
    };
}
export default viteSvgToWebfont;
export { type GeneratedWebfont, type PublicApi };
export type { IconPluginOptions, IconPluginFileOptions, IconPluginVariantOptions, IconPluginVariant } from './optionParser';

/**
 * Paths of default templates available for use.
 */
export const templates: typeof templatesImport = templatesImport;
