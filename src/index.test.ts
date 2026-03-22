import { constants } from 'node:fs';
import { access, readFile, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import type { IndexHtmlTransformContext, InlineConfig, PreviewServer, ViteDevServer } from 'vite';
import { build, createServer, normalizePath, preview } from 'vite';
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test';
import { viteSvgToWebfont } from './index';
import { base64ToArrayBuffer } from './utils';

type ViteBuildResult = Awaited<ReturnType<typeof build>>;
type RolldownOutput = Extract<ViteBuildResult, { output: unknown }>;
type OutputAsset = Extract<RolldownOutput['output'][1], { type: 'asset' }>;
type TransformIndexHtmlHook = Extract<Exclude<ReturnType<typeof viteSvgToWebfont>['transformIndexHtml'], undefined>, { handler: unknown }>;

// #region test utils
const root = new URL('./fixtures/', import.meta.url);
const types = ['svg', 'eot', 'woff', 'woff2', 'ttf'] as const;

const normalizeLineBreak = (input: string) => input.replaceAll('\r\n', '\n');
const fileURLToNormalizedPath = (url: URL) => normalizePath(fileURLToPath(url));

const enum ConfigType {
    Basic = './vite.basic.config.ts',
    NoInline = './vite.no-inline.config.ts',
    AllowWriteFilesInBuild = './vite.allowWriteFilesInBuild.config.ts',
    Preload = './vite.preload.config.ts',
    PreloadInline = './vite.preload-inline.config.ts',
}
const getConfig = (configType: ConfigType): InlineConfig => ({
    logLevel: 'silent',
    root: fileURLToNormalizedPath(root),
    configFile: fileURLToNormalizedPath(new URL(configType, root)),
});

const getServerPort = (server: ViteDevServer | PreviewServer) => {
    const address = server.httpServer?.address();
    if (!address) {
        throw new Error('Address not found');
    }
    if (typeof address === 'string') {
        const [, port] = address.split(':', 2);
        return parseInt(port || '80', 10);
    }
    return address.port;
};

const fetchFromServer = async (server: ViteDevServer | PreviewServer, path: string) => {
    const port = getServerPort(server);
    const url = `http://localhost:${port}${path}`;
    return await fetch(url);
};

const fetchTextContent = async (server: ViteDevServer | PreviewServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    const content = await res.text();
    return normalizeLineBreak(content || '');
};

const fetchBufferContent = async (server: ViteDevServer | PreviewServer, path: string) => {
    const res = await fetchFromServer(server, path);
    if (!res.ok || res.status !== 200) {
        return undefined;
    }
    return await res.arrayBuffer();
};

const loadFileContent = async (path: string, encoding: BufferEncoding | 'buffer' = 'utf8'): Promise<string | ArrayBufferLike> => {
    const absolutePath = new URL(path, root);
    const content = await readFile(absolutePath, encoding === 'buffer' ? null : encoding);

    if (typeof content !== 'string') {
        return content.buffer;
    }
    return normalizeLineBreak(content);
};
// #endregion

describe('serve - handles virtual import and has all font types available', () => {
    const buildConfig = getConfig(ConfigType.Basic);

    let server: ViteDevServer;

    beforeAll(async () => {
        const createdServer = await createServer(buildConfig);
        server = await createdServer.listen();
    });

    afterAll(async () => {
        await server.close();
    });

    it.concurrent('handles virtual import', async () => {
        const res = await fetchTextContent(server, `/main.ts`);
        expect(res).toMatch(/^import "\/@id\/__x00__virtual:vite-svg-2-webfont\.css";/);
    });

    it.concurrent.each(types)(`has font of type %s available`, async type => {
        const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/iconfont.${type}`)]);
        expect(res).toStrictEqual(expected);
    });
});

describe('build', () => {
    const buildConfig = getConfig(ConfigType.Basic);

    let output: RolldownOutput['output'];
    let server: PreviewServer;
    let cssContent: string | undefined;

    const typeToMimeMap: Record<string, string> = {
        svg: 'image/svg+xml',
        eot: 'application/vnd.ms-fontobject',
        woff: 'font/woff',
        woff2: 'font/woff2',
        ttf: 'font/ttf',
    };

    beforeAll(async () => {
        let buildResult = await build(buildConfig);
        if (Array.isArray(buildResult)) {
            buildResult = buildResult[0]!;
        }
        if (!('output' in buildResult)) {
            throw new Error('Unexpected build result');
        }

        output = buildResult.output;
        server = await preview(buildConfig);
        server.printUrls();

        const cssFileName = output.find((out): out is OutputAsset => out.type === 'asset' && out.names.includes('index.css'))!.fileName;
        cssContent = await fetchTextContent(server, `/${cssFileName}`);
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects fonts css to page', () => {
        expect(cssContent).toMatch(/^@font-face{font-family:iconfont;/);
    });

    it.concurrent.each(types)('has font of type %s available', async type => {
        const res = await loadFileContent(`fonts/iconfont.${type}`, 'buffer');
        let expected: ArrayBuffer | string | undefined;

        const iconAsset = output.find((out): out is OutputAsset => out.type === 'asset' && out.fileName.startsWith('assets/iconfont-') && out.fileName.endsWith(type));
        if (iconAsset) {
            const iconAssetName = iconAsset.fileName;
            expected = await fetchBufferContent(server, `/${iconAssetName}`);
        } else if (cssContent) {
            // File asset not found in output, check if it's inlined in CSS

            const typeMime = typeToMimeMap[type];
            const regex = new RegExp(`url\\(data:(?<mime>${typeMime});base64,(?<data>.*?)\\)\\s?format\\("(?<format>.+?)"\\)`);

            const match = cssContent.match(regex);
            if (match?.groups && 'mime' in match.groups && 'data' in match.groups && match.groups.mime === typeMime) {
                expected = base64ToArrayBuffer(match.groups.data);
            }
        }

        expect(res).not.toEqual(undefined);
        expect(res).toStrictEqual(expected);
    });
});

describe('build:no-inline', () => {
    const buildConfig = getConfig(ConfigType.NoInline);

    let output: RolldownOutput['output'];
    let server: PreviewServer;
    beforeAll(async () => {
        let buildResult = await build(buildConfig);
        if (Array.isArray(buildResult)) {
            buildResult = buildResult[0]!;
        }
        if (!('output' in buildResult)) {
            throw new Error('Unexpected build result');
        }
        output = buildResult.output;
        server = await preview(buildConfig);
        server.printUrls();
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects fonts css to page', async () => {
        const cssFileName = output.find(({ type, name }) => type === 'asset' && name === 'index.css')!.fileName;
        const res = await fetchTextContent(server, `/${cssFileName}`);
        expect(res).toMatch(/^@font-face{font-family:iconfont;/);
    });

    it.concurrent.each(types)('has font of type %s available', async type => {
        const iconAssetName = output.find(({ fileName }) => fileName.startsWith('assets/iconfont-') && fileName.endsWith(type))!.fileName;
        const [expected, res] = await Promise.all([loadFileContent(`fonts/iconfont.${type}`, 'buffer'), fetchBufferContent(server, `/${iconAssetName}`)]);
        expect(res).toStrictEqual(expected);
    });
});

describe('build:preloadFormats', () => {
    const buildConfig = getConfig(ConfigType.Preload);

    let output: RolldownOutput['output'];
    let server: PreviewServer;
    let htmlContent: string | undefined;

    beforeAll(async () => {
        let buildResult = await build(buildConfig);
        if (Array.isArray(buildResult)) {
            buildResult = buildResult[0]!;
        }
        if (!('output' in buildResult)) {
            throw new Error('Unexpected build result');
        }

        output = buildResult.output;
        server = await preview(buildConfig);
        server.printUrls();
        htmlContent = await fetchTextContent(server, '/');
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('injects preload links into build html', () => {
        expect(htmlContent).toContain('<link rel="preload"');
    });

    it.concurrent('preloads only requested generated font formats', () => {
        const woff2AssetName = output.find(({ fileName }) => fileName.startsWith('assets/iconfont-') && fileName.endsWith('woff2'))!.fileName;

        expect(htmlContent).toContain(`href="/${woff2AssetName}"`);
        expect(htmlContent).toContain('as="font"');
        expect(htmlContent).toContain('type="font/woff2"');
        expect(htmlContent).toContain('crossorigin');
        expect(htmlContent).not.toContain('.ttf');
    });

    it.concurrent('ignores preload formats that are not being generated', () => {
        expect(htmlContent).not.toContain('type="application/font-woff"');
    });
});

describe('build:preloadFormats:inline', () => {
    const buildConfig = getConfig(ConfigType.PreloadInline);

    let server: PreviewServer;
    let htmlContent: string | undefined;

    beforeAll(async () => {
        await build(buildConfig);
        server = await preview(buildConfig);
        server.printUrls();
        htmlContent = await fetchTextContent(server, '/');
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('does not inject preload links when plugin fonts are inlined', () => {
        expect(htmlContent).not.toContain('<link rel="preload"');
    });
});

describe('build without preloadFormats', () => {
    const buildConfig = getConfig(ConfigType.Basic);

    let server: PreviewServer;
    let htmlContent: string | undefined;

    beforeAll(async () => {
        await build(buildConfig);
        server = await preview(buildConfig);
        server.printUrls();
        htmlContent = await fetchTextContent(server, '/');
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it.concurrent('does not inject preload links when preloadFormats is omitted', () => {
        expect(htmlContent).not.toContain('<link rel="preload"');
    });
});

describe('transformIndexHtml shouldProcessHtml', () => {
    const contextPath = fileURLToNormalizedPath(new URL('./webfont-test/svg', root));
    const buildContext = {
        bundle: {
            'assets/iconfont-test.woff2': {
                type: 'asset',
                fileName: 'assets/iconfont-test.woff2',
                names: ['iconfont-test.woff2'],
                originalFileNames: [],
                source: '',
            },
        },
        filename: '/virtual/index.html',
        path: '/index.html',
    } as unknown as IndexHtmlTransformContext;

    it.concurrent('skips preload tags when shouldProcessHtml returns false', async () => {
        const plugin = viteSvgToWebfont({
            context: contextPath,
            fontName: 'iconfont-test',
            preloadFormats: ['woff2'],
            shouldProcessHtml: () => false,
            types: ['woff2'],
        });
        const transformIndexHtml = plugin.transformIndexHtml as TransformIndexHtmlHook;
        const configResolved = plugin.configResolved as (config: { command: 'build' | 'serve' }) => void;

        configResolved({ command: 'build' });
        const result = await transformIndexHtml.handler.call({} as never, '', buildContext);

        expect(result).toBe(undefined);
    });

    it.concurrent('injects preload tags when shouldProcessHtml returns true', async () => {
        const plugin = viteSvgToWebfont({
            context: contextPath,
            fontName: 'iconfont-test',
            preloadFormats: ['woff2'],
            shouldProcessHtml: () => true,
            types: ['woff2'],
        });
        const transformIndexHtml = plugin.transformIndexHtml as TransformIndexHtmlHook;
        const configResolved = plugin.configResolved as (config: { command: 'build' | 'serve' }) => void;

        configResolved({ command: 'build' });
        const result = await transformIndexHtml.handler.call({} as never, '', buildContext);

        expect(result).toEqual([
            {
                attrs: {
                    as: 'font',
                    crossorigin: true,
                    href: '/assets/iconfont-test.woff2',
                    rel: 'preload',
                    type: 'font/woff2',
                },
                injectTo: 'head',
                tag: 'link',
            },
        ]);
    });
});

describe('build allowWriteFilesInBuild', () => {
    const buildConfig = getConfig(ConfigType.AllowWriteFilesInBuild);

    beforeAll(async () => {
        await build(buildConfig);
    });

    afterAll(async () => {
        await rm(new URL('webfont-test/artifacts', root), { recursive: true });
    });

    it.concurrent.each([...types, 'html', 'css'])('has generated font of type %s', async type => {
        const fileName = `webfont-test/artifacts/allowWriteFilesInBuild-test.${type}`;
        const fileNameCasing = types.includes(type as unknown as (typeof types)[number]) ? fileName : fileName.toLowerCase();
        const filePath = new URL(fileNameCasing, root);

        await expect(access(filePath, constants.F_OK)).resolves.not.toThrow();
    });
});
