import { promisify } from 'util';
import _webfontGenerator from '@vusion/webfonts-generator';
import { setupWatcher, MIME_TYPES, guid } from './utils';
import { type IconPluginOptions, parseOptions, parseFiles } from './optionParser';
import type { Plugin, ModuleGraph, ModuleNode } from 'vite';
import type { GeneratedFontTypes, WebfontsGeneratorResult } from '@vusion/webfonts-generator';

const ac = new AbortController();
const webfontGenerator = promisify(_webfontGenerator);
const VIRTUAL_MODULE_ID = 'virtual:vite-svg-2-webfont.css';
const RESOLVED_VIRTUAL_MODULE_ID = `\0${VIRTUAL_MODULE_ID}`;

export function viteSvgToWebfont<T extends GeneratedFontTypes = GeneratedFontTypes>(options: IconPluginOptions<T>): Plugin {
    const processedOptions = parseOptions(options);
    let isBuild: boolean;
    let fileRefs: { [Ref in T]: string } | undefined;
    let _moduleGraph: ModuleGraph;
    let _reloadModule: undefined | ((module: ModuleNode) => void);
    let generatedFonts: undefined | Pick<WebfontsGeneratorResult, 'generateCss' | 'generateHtml' | T>;

    const generate = async (updateFiles?: boolean) => {
        if (updateFiles) {
            processedOptions.files = parseFiles(options);
        }
        generatedFonts = await webfontGenerator(processedOptions);
        if (updateFiles) {
            const module = _moduleGraph?.getModuleById(RESOLVED_VIRTUAL_MODULE_ID);
            if (module && _reloadModule) {
                _reloadModule(module);
            }
        }
    };
    return {
        name: 'vite-svg-2-webfont',
        enforce: 'pre',
        configResolved(_config) {
            isBuild = _config.command === 'build';
        },
        resolveId(id) {
            if (id !== VIRTUAL_MODULE_ID) {
                return undefined;
            }
            return RESOLVED_VIRTUAL_MODULE_ID;
        },
        transform(_code, id) {
            if (id !== RESOLVED_VIRTUAL_MODULE_ID) {
                return undefined;
            }
            return generatedFonts?.generateCss?.(fileRefs) || '';
        },
        load(id) {
            if (id !== RESOLVED_VIRTUAL_MODULE_ID) {
                return undefined;
            }
            return RESOLVED_VIRTUAL_MODULE_ID;
        },
        async buildStart() {
            if (!isBuild) {
                setupWatcher(options.context, ac.signal, () => generate(true));
            }
            await generate();
            if (isBuild) {
                const emitted = processedOptions.types.map<[T, string]>(type => [
                    type,
                    `/${this.getFileName(this.emitFile({ type: 'asset', fileName: `assets/${processedOptions.fontName}-${guid()}.${type}`, source: generatedFonts?.[type] }))}`,
                ]);
                fileRefs = Object.fromEntries(emitted) as { [Ref in T]: string };
            }
        },
        configureServer({ middlewares, reloadModule, moduleGraph }) {
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
        },
    };
}
export default viteSvgToWebfont;
