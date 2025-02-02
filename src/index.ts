import { promisify } from 'node:util';
import { join as pathJoin } from 'node:path';
import type { ModuleGraph, ModuleNode } from 'vite';
import _webfontGenerator from '@vusion/webfonts-generator';
import { setupWatcher, MIME_TYPES, ensureDirExistsAndWriteFile, getTmpDir, getBufferHash, rmDir } from './utils';
import { parseOptions, parseFiles } from './optionParser';
import type { GeneratedFontTypes, WebfontsGeneratorResult } from '@vusion/webfonts-generator';
import type { IconPluginOptions } from './optionParser';
import type { GeneratedWebfont } from './types/generatedWebfont';
import type { CompatiblePlugin, PublicApi } from './types/publicApi';

const ac = new AbortController();
const webfontGenerator = promisify(_webfontGenerator);
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
export function viteSvgToWebfont<T extends GeneratedFontTypes = GeneratedFontTypes>(options: IconPluginOptions<T>): CompatiblePlugin<PublicApi> {
    const processedOptions = parseOptions(options);
    let isBuild: boolean;
    let fileRefs: { [Ref in T]: string } | undefined;
    let _moduleGraph: ModuleGraph;
    let _reloadModule: undefined | ((module: ModuleNode) => Promise<void>);
    let generatedFonts: undefined | Pick<WebfontsGeneratorResult<T>, 'generateCss' | 'generateHtml' | T>;
    const generatedWebfonts: GeneratedWebfont[] = [];
    const tmpGeneratedWebfonts: GeneratedWebfont[] = [];
    const moduleId = options.moduleId ?? DEFAULT_MODULE_ID;
    const virtualModuleId = getVirtualModuleId(moduleId);
    const resolvedVirtualModuleId = getResolvedVirtualModuleId(virtualModuleId);

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
        generatedFonts = await webfontGenerator(processedOptions);
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
            for (const { type, href } of tmpGeneratedWebfonts) {
                for (const chunk of Object.values(bundle)) {
                    if (chunk.name && href.endsWith(chunk.name)) {
                        generatedWebfonts.push({ type, href: `/${chunk.fileName}` });
                    }
                }
            }
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

                emitted.forEach(([type, href]) => {
                    tmpGeneratedWebfonts.push({ type, href });
                });
                fileRefs = Object.fromEntries(emitted) as { [Ref in T]: string };
            }
        },
        configureServer({ middlewares, reloadModule, moduleGraph }) {
            if (options.inline) {
                return;
            }
            for (const fontType of processedOptions.types) {
                const fileName = `${processedOptions.fontName}.${fontType}`;
                middlewares.use(`/${fileName}`, (_req, res) => {
                    _moduleGraph = moduleGraph;
                    _reloadModule = reloadModule;
                    if (!generatedFonts) {
                        res.statusCode = 404;
                        return res.end();
                    }
                    const font = generatedFonts[fontType];
                    res.setHeader('content-type', MIME_TYPES[fontType]);
                    res.setHeader('content-length', font.length);
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

/**
 * Paths of default templates available for use.
 */
export const templates = _webfontGenerator.templates;
