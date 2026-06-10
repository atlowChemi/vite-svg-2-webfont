import { constants } from 'node:fs';
import { access, readFile, rm, writeFile } from 'node:fs/promises';
import { setTimeout } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import type { IndexHtmlTransformContext, InlineConfig, PreviewServer, ViteDevServer } from 'vite';
import { build, createServer, normalizePath, preview } from 'vite';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vite-plus/test';
import { viteSvgToWebfont } from './index';
import { base64ToArrayBuffer } from './utils';
import type { IconPluginOptions } from './optionParser';

const { generateWebfontsMock } = vi.hoisted(() => ({
    generateWebfontsMock: vi.fn<typeof import('@atlowchemi/webfont-generator').generateWebfonts>(),
}));
const { ensureDirExistsAndWriteFileMock, setupWatcherMock } = vi.hoisted(() => ({
    setupWatcherMock: vi.fn<typeof import('./utils').setupWatcher>(),
    ensureDirExistsAndWriteFileMock: vi.fn<typeof import('./utils').ensureDirExistsAndWriteFile>(),
}));

vi.mock('@atlowchemi/webfont-generator', async importOriginal => {
    const actual = await importOriginal<typeof import('@atlowchemi/webfont-generator')>();
    generateWebfontsMock.mockImplementation(actual.generateWebfonts);
    return {
        ...actual,
        generateWebfonts: generateWebfontsMock,
    };
});

vi.mock('./utils', async importOriginal => {
    const actual = await importOriginal<typeof import('./utils')>();
    setupWatcherMock.mockImplementation(actual.setupWatcher);
    ensureDirExistsAndWriteFileMock.mockImplementation(actual.ensureDirExistsAndWriteFile);
    return { ...actual, setupWatcher: setupWatcherMock, ensureDirExistsAndWriteFile: ensureDirExistsAndWriteFileMock };
});

type ViteBuildResult = Awaited<ReturnType<typeof build>>;
type RolldownOutput = Extract<ViteBuildResult, { output: unknown }>;
type OutputAsset = Extract<RolldownOutput['output'][1], { type: 'asset' }>;
type TransformIndexHtmlHook = Extract<Exclude<ReturnType<typeof viteSvgToWebfont>['transformIndexHtml'], undefined>, { handler: unknown }>;

// #region test utils
const root = new URL('./fixtures/', import.meta.url);
const types = ['svg', 'eot', 'woff', 'woff2', 'ttf'] as const;

const normalizeLineBreak = (input: string) => input.replaceAll('\r\n', '\n');
const fileURLToNormalizedPath = (url: URL) => normalizePath(fileURLToPath(url));

const webfontFolder = fileURLToNormalizedPath(new URL('webfont-test/svg', root));
const outputFolder = fileURLToNormalizedPath(new URL('webfont-test/artifacts', root));

const enum ConfigType {
    Basic,
    NoInline,
    AllowWriteFilesInBuild,
    Preload,
    PreloadInline,
}

const getConfig = (configType: ConfigType, overrides?: Partial<IconPluginOptions>): InlineConfig => {
    const base: InlineConfig = {
        logLevel: 'silent',
        root: fileURLToNormalizedPath(root),
        configFile: false,
    };
    switch (configType) {
        case ConfigType.Basic:
            return { ...base, plugins: [viteSvgToWebfont({ context: webfontFolder, ...overrides })] };
        case ConfigType.NoInline:
            return {
                ...base,
                build: { assetsInlineLimit: 0 },
                plugins: [viteSvgToWebfont({ context: webfontFolder, ...overrides })],
            };
        case ConfigType.AllowWriteFilesInBuild:
            return {
                ...base,
                build: { assetsInlineLimit: 0 },
                plugins: [
                    viteSvgToWebfont({
                        dest: outputFolder,
                        generateFiles: true,
                        context: webfontFolder,
                        allowWriteFilesInBuild: true,
                        fontName: 'allowWriteFilesInBuild-test',
                        ...overrides,
                    }),
                ],
            };
        case ConfigType.Preload:
            return {
                ...base,
                build: { assetsInlineLimit: 0 },
                plugins: [
                    viteSvgToWebfont({
                        context: webfontFolder,
                        types: ['woff2', 'ttf'],
                        // @ts-ignore -- 'woff' is intentionally not in `types` — exercises the runtime filter that drops mismatched preload formats.
                        preloadFormats: ['woff2', 'woff'],
                        ...overrides,
                    }),
                ],
            };
        case ConfigType.PreloadInline:
            return {
                ...base,
                build: { assetsInlineLimit: 0 },
                plugins: [viteSvgToWebfont({ context: webfontFolder, inline: true, preloadFormats: ['woff2'], types: ['woff2'], ...overrides })],
            };
        default:
            configType satisfies never;
            throw new Error('Invalid config type');
    }
};

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
    const buildConfig = getConfig(ConfigType.Basic, { formatOptions: { woff2: { compressionQuality: 11 } } });

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

describe('build cssFontsUrl root', () => {
    // Regression test for https://github.com/atlowChemi/vite-svg-2-webfont/issues/121:
    // cssFontsUrl: '/' must produce absolute-from-root URLs like '/iconfont.woff2',
    // not a bare filename. Captures the rendered src via cssContext to inspect the
    // plugin output directly, before Vite's asset pipeline rewrites the URLs.
    let capturedSrc: string | undefined;

    beforeAll(async () => {
        await build({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            build: { assetsInlineLimit: 0, write: false },
            plugins: [
                viteSvgToWebfont({
                    context: webfontFolder,
                    types: ['woff2', 'ttf'],
                    cssFontsUrl: '/',
                    cssContext: context => {
                        capturedSrc = context.src;
                    },
                }),
            ],
        });
    });

    it.each(['woff2', 'ttf'] as const)('emits leading-slash url for type %s', type => {
        expect(capturedSrc).toMatch(new RegExp(`url\\("/iconfont\\.${type}\\?[^"]+"\\)`));
        expect(capturedSrc).not.toMatch(new RegExp(`url\\("iconfont\\.${type}\\?`));
    });
});

describe('build api.getGeneratedWebfonts', () => {
    let plugin: ReturnType<typeof viteSvgToWebfont>;

    beforeAll(async () => {
        plugin = viteSvgToWebfont({ context: webfontFolder, types: ['woff2', 'ttf'] });
        await build({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            build: { assetsInlineLimit: 0, write: false },
            plugins: [plugin],
        });
    });

    it('exposes generated webfonts after build via the public api', () => {
        const fonts = plugin.api!.getGeneratedWebfonts();
        expect(fonts).toHaveLength(2);
        expect(fonts.map(({ type }) => type).toSorted()).toEqual(['ttf', 'woff2']);
        expect(fonts.map(({ href }) => href)).toEqual(
            expect.arrayContaining([expect.stringMatching(/^\/assets\/iconfont-[^/]+\.(ttf)$/), expect.stringMatching(/^\/assets\/iconfont-[^/]+\.(woff2)$/)]),
        );
    });
});

describe('build font write ordering', () => {
    // Regression test: in build mode the plugin writes each generated font to a temp dir and
    // references those paths from the virtual module's CSS. Vite reads the files when it resolves
    // the url() references into bundle assets, so the writes must be awaited before buildStart
    // resolves — otherwise a lagging write loses the race and that font's asset is silently
    // dropped from the bundle (and from getGeneratedWebfonts).
    it('awaits font temp-file writes before completing the build', { timeout: 15_000 }, async () => {
        const writeGate = Promise.withResolvers<void>();
        const realUtils = await vi.importActual<typeof import('./utils')>('./utils');
        // Perform the real write (so the build still succeeds once unblocked) but hold the
        // returned promise open: if buildStart awaits it, the build cannot finish until the
        // gate is released. A fire-and-forget write would let the build settle immediately.
        ensureDirExistsAndWriteFileMock.mockImplementationOnce(async (content, dest) => {
            await writeGate.promise;
            await realUtils.ensureDirExistsAndWriteFile(content, dest);
        });

        const plugin = viteSvgToWebfont({ context: webfontFolder, types: ['woff2', 'ttf'] });
        const buildPromise = build({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            build: { assetsInlineLimit: 0, write: false },
            plugins: [plugin],
        });

        const isSettledBeforeGate = await Promise.race([buildPromise.then(() => true), setTimeout(250, false)]);
        expect(isSettledBeforeGate).toBe(false);
        expect(plugin.api!.getGeneratedWebfonts()).toHaveLength(0);

        writeGate.resolve();
        await buildPromise;

        expect(
            plugin
                .api!.getGeneratedWebfonts()
                .map(({ type }) => type)
                .toSorted(),
        ).toEqual(['ttf', 'woff2']);
    });
});

describe('serve - inline mode skips font middleware', () => {
    let server: ViteDevServer;

    beforeAll(async () => {
        const createdServer = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [viteSvgToWebfont({ context: webfontFolder, inline: true, types: ['woff2'] })],
        });
        server = await createdServer.listen();
    });

    afterAll(async () => {
        await server.close();
    });

    it('does not register a font-serving middleware when inline is enabled', async () => {
        const res = await fetchFromServer(server, '/iconfont.woff2');
        const contentType = res.headers.get('content-type') ?? '';
        expect(contentType.startsWith('font/')).toBe(false);
        expect(contentType.startsWith('application/font-')).toBe(false);
        expect(contentType).not.toBe('application/vnd.ms-fontobject');
        expect(contentType).toBe('text/html');
    });
});

describe('serve - generateFiles writes css and html to disk in dev', () => {
    const artifactsUrl = new URL('webfont-test/serve-artifacts/', root);
    const cssUrl = new URL('serve-test.css', artifactsUrl);
    const htmlUrl = new URL('serve-test.html', artifactsUrl);
    let server: ViteDevServer;

    beforeAll(async () => {
        const createdServer = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [
                viteSvgToWebfont({
                    context: webfontFolder,
                    dest: fileURLToNormalizedPath(artifactsUrl),
                    fontName: 'serve-test',
                    generateFiles: ['css', 'html'],
                }),
            ],
        });
        server = await createdServer.listen();
    });

    afterAll(async () => {
        await Promise.all([server.close(), rm(artifactsUrl, { recursive: true, force: true })]);
    });

    it.each([
        ['css', () => cssUrl],
        ['html', () => htmlUrl],
    ] as const)('writes the generated %s file to disk', async (_kind, urlOf) => {
        await expect(access(urlOf(), constants.F_OK)).resolves.not.toThrow();
    });
});

describe('build:preloadFormats inlined-asset short-circuit', () => {
    // With default assetsInlineLimit (4kb), small fonts get base64-inlined into
    // CSS and don't appear as separate bundle chunks. preloadFormats stays
    // non-empty after the types filter, but resolveGeneratedWebfonts produces
    // an empty list, so no preload tags are injected.
    let server: PreviewServer;
    let htmlContent: string | undefined;

    beforeAll(async () => {
        const buildConfig: InlineConfig = {
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [
                viteSvgToWebfont({
                    context: webfontFolder,
                    types: ['woff2'],
                    preloadFormats: ['woff2'],
                }),
            ],
        };
        await build(buildConfig);
        server = await preview(buildConfig);
        server.printUrls();
        htmlContent = await fetchTextContent(server, '/');
    });

    afterAll(() => {
        server.httpServer.close();
    });

    it('omits preload tags when all preloadable fonts were inlined into CSS', () => {
        expect(htmlContent).not.toContain('<link rel="preload"');
    });
});

describe('serve - regenerates css when a new svg is added', () => {
    const filename = '__watcher-test__.svg';
    const watcherSvgUrl = new URL(`webfont-test/svg/${filename}`, root);
    const reloadedIds: string[] = [];
    const { promise: waitForCssReload, resolve: markCssReloaded } = Promise.withResolvers<void>();
    let watcherHandler: Parameters<typeof setupWatcherMock>[2];
    let server: ViteDevServer;

    beforeAll(async () => {
        setupWatcherMock.mockImplementationOnce(async (_path, _signal, handler) => {
            watcherHandler = handler;
        });
        const created = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [viteSvgToWebfont({ context: webfontFolder })],
        });
        const originalReload = created.reloadModule.bind(created);
        created.reloadModule = async mod => {
            reloadedIds.push(mod.id ?? '');
            if (mod.id?.includes('vite-svg-2-webfont.css')) markCssReloaded();
            return originalReload(mod);
        };
        server = await created.listen();
        // Hit the font middleware so the plugin captures moduleGraph + reloadModule…
        await fetchBufferContent(server, '/iconfont.woff2');
        // …and load the virtual module so getModuleById finds it after the watch handler fires.
        await fetchTextContent(server, '/@id/__x00__virtual:vite-svg-2-webfont.css');
    });

    afterAll(async () => {
        await Promise.all([server.close(), rm(watcherSvgUrl, { force: true })]);
    });

    it('reloads the virtual css module after a new svg appears', async () => {
        await writeFile(watcherSvgUrl, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><path d="M0 0h1024v1024H0z"/></svg>');
        await watcherHandler({ eventType: 'rename', filename });
        await waitForCssReload;
        expect(reloadedIds.some(id => id.includes('vite-svg-2-webfont.css'))).toBe(true);
    });
});

describe('serve - swallows reloadModule rejection from the watcher', () => {
    const filename = '__reload-reject-test__.svg';
    const watcherSvgUrl = new URL(`webfont-test/svg/${filename}`, root);
    const { promise: waitForReloadCalled, resolve: markReloadCalled } = Promise.withResolvers<void>();
    const rejectingReload = vi.fn(async () => {
        markReloadCalled();
        throw new Error('intentional reload failure');
    });
    let watcherHandler: Parameters<typeof setupWatcherMock>[2];
    let server: ViteDevServer;

    beforeAll(async () => {
        setupWatcherMock.mockImplementationOnce(async (_path, _signal, handler) => {
            watcherHandler = handler;
        });
        const created = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [viteSvgToWebfont({ context: webfontFolder })],
        });
        created.reloadModule = rejectingReload;
        server = await created.listen();
        await fetchBufferContent(server, '/iconfont.woff2');
        await fetchTextContent(server, '/@id/__x00__virtual:vite-svg-2-webfont.css');
    });

    afterAll(async () => {
        await Promise.all([server.close(), rm(watcherSvgUrl, { force: true })]);
    });

    it('does not crash the watcher when reloadModule rejects', async () => {
        await writeFile(watcherSvgUrl, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><path d="M0 0h512v512H0z"/></svg>');
        await watcherHandler({ eventType: 'rename', filename });
        await waitForReloadCalled;
        expect(rejectingReload).toHaveBeenCalledOnce();
    });
});

describe('build - throws when generator omits a requested font type', () => {
    it('surfaces the missing-type error from buildStart', async () => {
        const { generateWebfonts: realGen } = await vi.importActual<typeof import('@atlowchemi/webfont-generator')>('@atlowchemi/webfont-generator');
        generateWebfontsMock.mockImplementationOnce(async options => {
            const real = await realGen(options);
            const { woff2: _omitted, ...rest } = real;
            return rest as typeof real;
        });

        await expect(
            build({
                logLevel: 'silent',
                root: fileURLToNormalizedPath(root),
                configFile: false,
                build: { write: false },
                plugins: [viteSvgToWebfont({ context: webfontFolder, types: ['woff2', 'ttf'] })],
            }),
        ).rejects.toThrow(/Failed to generate font of type woff2/);
    });
});

describe('WOFF2 compression quality by mode', () => {
    let realGen: typeof import('@atlowchemi/webfont-generator').generateWebfonts;

    beforeAll(async () => {
        ({ generateWebfonts: realGen } = await vi.importActual<typeof import('@atlowchemi/webfont-generator')>('@atlowchemi/webfont-generator'));
    });

    // Capture the options the plugin forwards to the generator on the next call, while still
    // running the real generator so the dev server / build behaves normally.
    const captureNextGenerateOptions = () => {
        const captured: { options?: Parameters<typeof generateWebfontsMock>[0] } = {};
        generateWebfontsMock.mockImplementationOnce(async options => {
            captured.options = options;
            return realGen(options);
        });
        return captured;
    };

    it('serve mode forwards the faster compressionQuality (10) to the generator', async () => {
        const captured = captureNextGenerateOptions();
        const createdServer = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [viteSvgToWebfont({ context: webfontFolder, types: ['woff2'] })],
        });
        const server = await createdServer.listen();
        await server.close();

        expect(captured.options?.formatOptions?.woff2?.compressionQuality).toBe(10);
    });

    it('serve mode does not override user-specified compressionQuality', async () => {
        const captured = captureNextGenerateOptions();
        const createdServer = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [viteSvgToWebfont({ context: webfontFolder, types: ['woff2'], formatOptions: { woff2: { compressionQuality: 5 } } })],
        });
        const server = await createdServer.listen();
        await server.close();

        expect(captured.options?.formatOptions?.woff2?.compressionQuality).toBe(5);
    });

    it('the top-level woff2CompressionQuality option overrides the dev default', async () => {
        const captured = captureNextGenerateOptions();
        const createdServer = await createServer({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            plugins: [
                viteSvgToWebfont({
                    context: webfontFolder,
                    types: ['woff2'],
                    woff2CompressionQuality: 7,
                }),
            ],
        });
        const server = await createdServer.listen();
        await server.close();

        expect(captured.options?.formatOptions?.woff2?.compressionQuality).toBe(7);
    });

    it('build mode leaves compressionQuality unset, so the engine default (11) applies', async () => {
        const captured = captureNextGenerateOptions();
        await build({
            logLevel: 'silent',
            root: fileURLToNormalizedPath(root),
            configFile: false,
            build: { write: false },
            plugins: [viteSvgToWebfont({ context: webfontFolder, types: ['woff2'] })],
        });

        expect(captured.options?.formatOptions?.woff2?.compressionQuality).toBeUndefined();
    });
});
