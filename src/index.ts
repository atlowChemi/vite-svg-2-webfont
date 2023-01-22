import { promisify } from 'util';
import _webfontGenerator from '@vusion/webfonts-generator';
import { setupWatcher, MIME_TYPES } from './utils';
import { type IconPluginOptions, parseOptions } from './optionParser';
import type { Plugin } from 'vite';
import type { IncomingMessage, ServerResponse } from 'http';

const ac = new AbortController();
const webfontGenerator = promisify(_webfontGenerator);

export function iconPlugin(options: IconPluginOptions): Plugin {
    const processedOptions = parseOptions(options);
    let isBuild: boolean;
    let cssAssetName = `${processedOptions.fontName}.css`;
    let generatedFonts: undefined | Awaited<ReturnType<typeof webfontGenerator>>;

    const generate = async () => {
        generatedFonts = await webfontGenerator(processedOptions);
    };
    return {
        name: 'vite-svg-2-webfont',
        configResolved(_config) {
            isBuild = _config.command === 'build';
        },
        transformIndexHtml: {
            handler(html) {
                return {
                    html,
                    tags: [{ tag: 'link', injectTo: 'head', attrs: { rel: 'stylesheet', href: `/${cssAssetName}` } }],
                };
            },
        },
        async buildStart() {
            if (!isBuild) {
                setupWatcher(options.context, ac.signal, generate);
            }
            await generate();
        },
        configureServer({ middlewares }) {
            const middlewareHandler = (fontKey: Exclude<keyof NonNullable<typeof generatedFonts>, 'generateHtml'>, res: ServerResponse<IncomingMessage>) => {
                if (!generatedFonts) {
                    res.statusCode = 404;
                    return res.end();
                }
                const value = fontKey === 'generateCss' ? generatedFonts.generateCss() : generatedFonts[fontKey];
                if (fontKey !== 'generateCss') {
                    res.setHeader('content-type', MIME_TYPES[fontKey]);
                }
                res.setHeader('content-length', value.length);
                res.statusCode = 200;
                return res.end(value);
            };
            middlewares.use(`/${cssAssetName}`, (_req, res) => middlewareHandler('generateCss', res));
            for (const fontType of processedOptions.types) {
                const fileName = `${processedOptions.fontName}.${fontType}`;
                middlewares.use(`/${fileName}`, (_req, res) => middlewareHandler(fontType, res));
            }
        },
        renderStart() {
            if (!isBuild) {
                return;
            }
            const emitted = processedOptions.types.map(type => [
                type,
                this.getFileName(this.emitFile({ type: 'asset', name: `${processedOptions.fontName}.${type}`, source: generatedFonts?.[type] })).replace('assets', '.'),
            ]);
            cssAssetName = this.getFileName(this.emitFile({ type: 'asset', name: `${processedOptions.fontName}.css`, source: generatedFonts?.generateCss(Object.fromEntries(emitted)) }));
        },
        buildEnd() {
            ac.abort();
        },
    };
}
export default iconPlugin;
