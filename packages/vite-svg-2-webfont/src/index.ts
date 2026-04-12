import { join as pathJoin } from 'node:path';
import type { ModuleGraph, ModuleNode, Plugin } from 'vite';
import { setupWatcher, MIME_TYPES, ensureDirExistsAndWriteFile, getTmpDir, getBufferHash, rmDir } from './utils';
import { parseOptions, parseFiles, parsePreloadFormatsOption } from './optionParser';
import { generateWebfonts, templates, type FontType, type GenerateWebfontsResult } from '@atlowchemi/webfont-generator';
import type { IconPluginOptions } from './optionParser';
import type { GeneratedWebfont } from './types/generatedWebfont';
import type { PublicApi } from './types/publicApi';

type GenerateBundle = Extract<NonNullable<Plugin['generateBundle']>, Function>;
type OutputBundle = Parameters<GenerateBundle>[1];

const ac = new AbortController();
const DEFAULT_MODULE_ID = 'vite-svg-2-webfont.css';
const TMP_DIR = getTmpDir();

function getVirtualModuleId<T extends string>(moduleId: T): `virtual:${T}` {
    return `virtual:${moduleId}`;
}

function getResolvedVirtualModuleId<T extends string>(virtualModuleId: T): `\0${T}` {
    return `\0${virtualModuleId}`;
}

/**
 * A Vite plugin that generates a webfont from your SVG icons.
 *
 * The plugin uses {@link https://github.com/vusion/webfonts-generator/ webfonts-generator} package to create fonts in any format.
 * It also generates CSS files that allow using the icons directly in your HTML output, using CSS classes per-icon.
 */
export function viteSvgToWebfont<T extends FontType = FontType>(options: IconPluginOptions<T>): Plugin<PublicApi> {
    const processedOptions = parseOptions(options);
    const preloadFormats = parsePreloadFormatsOption<T>(options).filter((type): type is T => processedOptions.types.includes(type));
    let isBuild: boolean;
    let fileRefs: { [Ref in T]: string } | undefined;
    let _moduleGraph: ModuleGraph;
    let _reloadModule: undefined | ((module: ModuleNode) => Promise<void>);
    let generatedFonts: GenerateWebfontsResult<T> | undefined;
    const generatedWebfonts: GeneratedWebfont[] = [];
    const moduleId = options.moduleId ?? DEFAULT_MODULE_ID;
    const virtualModuleId = getVirtualModuleId(moduleId);
    const resolvedVirtualModuleId = getResolvedVirtualModuleId(virtualModuleId);
    const fontName = processedOptions.fontName || 'iconfont';

    const resolveGeneratedWebfonts = (bundle: OutputBundle) => {
        const resolvedWebfonts = new Map<T, string>();

        for (const chunk of Object.values(bundle)) {
            if (chunk.type !== 'asset') {
                continue;
            }
            const lastSlashIndex = chunk.fileName.lastIndexOf('/');
            const fileName = chunk.fileName.slice(lastSlashIndex + 1);
            if (!fileName.startsWith(fontName)) {
                continue;
            }

            const fontType = processedOptions.types.find(type => fileName.endsWith(`.${type}`));
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
            const font = Buffer.from(generatedFonts?.[type] || []);
            return `url("data:${MIME_TYPES[type]};charset=utf-8;base64,${font.toString('base64')}")`;
        }) as U;
    };

    const generate = async (updateFiles?: boolean) => {
        if (updateFiles) {
            processedOptions.files = parseFiles(options);
        }
        if (isBuild && !options.allowWriteFilesInBuild) {
            processedOptions.writeFiles = false;
        }
        generatedFonts = await generateWebfonts(processedOptions);
        const hasFilesToSave = !processedOptions.writeFiles && (processedOptions.css || processedOptions.html);
        if (!isBuild && hasFilesToSave) {
            await Promise.all([
                processedOptions.css && ensureDirExistsAndWriteFile(inline(generatedFonts.generateCss()), processedOptions.cssDest),
                processedOptions.html && ensureDirExistsAndWriteFile(generatedFonts.generateHtml(), processedOptions.htmlDest),
            ]);
        }
        if (updateFiles) {
            const module = _moduleGraph?.getModuleById(resolvedVirtualModuleId);
            if (module && _reloadModule) {
                _reloadModule(module).catch(() => null);
            }
        }
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
            return inline(generatedFonts?.generateCss?.(fileRefs)) || '';
        },
        load(id) {
            if (id !== resolvedVirtualModuleId) {
                return undefined;
            }
            return resolvedVirtualModuleId;
        },
        async buildStart() {
            if (!isBuild) {
                setupWatcher(options.context, ac.signal, () => generate(true)).catch(() => null);
            }
            await generate();
            if (isBuild && !options.inline) {
                const emitted = processedOptions.types.map<[T, string]>(type => {
                    if (!generatedFonts?.[type]) {
                        throw new Error(`Failed to generate font of type ${type}`);
                    }

                    const fileContents = Buffer.from(generatedFonts[type]);
                    const hash = getBufferHash(fileContents);
                    const filePath = pathJoin(TMP_DIR, `${processedOptions.fontName}-${hash}.${type}`);
                    ensureDirExistsAndWriteFile(fileContents, filePath).catch(() => null); // write font file to a temporary dir

                    return [type, filePath];
                });
                fileRefs = Object.fromEntries(emitted) as {
                    [Ref in T]: string;
                };
            }
        },
        configureServer(server) {
            if (options.inline) {
                return;
            }
            const { moduleGraph, middlewares } = server;
            for (const fontType of processedOptions.types) {
                const fileName = `${processedOptions.fontName}.${fontType}`;
                middlewares.use(`/${fileName}`, (_req, res) => {
                    _moduleGraph = moduleGraph;
                    _reloadModule = server.reloadModule.bind(server);
                    if (!generatedFonts) {
                        res.statusCode = 404;
                        return res.end();
                    }
                    const font = generatedFonts[fontType];
                    res.setHeader('content-type', MIME_TYPES[fontType]);
                    res.setHeader('content-length', font!.length);
                    res.statusCode = 200;
                    return res.end(font);
                });
            }
        },
        buildEnd() {
            ac.abort();
            rmDir(TMP_DIR);
        },
    };
}
export default viteSvgToWebfont;
export { type GeneratedWebfont, type PublicApi };
export { templates };
