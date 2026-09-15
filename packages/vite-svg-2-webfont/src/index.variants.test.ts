import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build, createServer } from 'vite-plus';
import { afterEach, expect, it, vi, type MockInstance } from 'vite-plus/test';
import { viteSvgToWebfont, templates, type IconPluginVariantOptions } from './index';
import { parseOptions } from './optionParser';
import type { GenerateWebfontsResult } from '@atlowchemi/webfont-generator';

const { watcher, generate, writeCompanion } = vi.hoisted(() => ({
    writeCompanion: vi.fn<typeof import('./utils').ensureDirExistsAndWriteFile>(),
    watcher: vi.fn<typeof import('./utils').setupWatcher>(),
    generate: vi.fn<typeof import('@atlowchemi/webfont-generator').generateWebfonts>(),
}));
vi.mock('./utils', async importOriginal => {
    const actual = await importOriginal<typeof import('./utils')>();
    writeCompanion.mockImplementation(actual.ensureDirExistsAndWriteFile);
    return { ...actual, setupWatcher: watcher, ensureDirExistsAndWriteFile: writeCompanion };
});
vi.mock('@atlowchemi/webfont-generator', async importOriginal => {
    const actual = await importOriginal<typeof import('@atlowchemi/webfont-generator')>();
    generate.mockImplementation(actual.generateWebfonts);
    return { ...actual, generateWebfonts: generate };
});
const native = () => vi.importActual<typeof import('@atlowchemi/webfont-generator')>('@atlowchemi/webfont-generator');

afterEach(async () => {
    vi.restoreAllMocks();
    watcher.mockReset();
    writeCompanion.mockClear();
    generate.mockImplementation((await native()).generateWebfonts);
    generate.mockClear();
});

const svg = (width: number) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H${width}V24H0Z"/></svg>`;

async function fixture(overrides: Partial<IconPluginVariantOptions> = {}) {
    await using cleanup = new AsyncDisposableStack();
    const root = await mkdtemp(join(tmpdir(), 'plugin-family-'));
    cleanup.defer(() => rm(root, { recursive: true, force: true }));
    await Promise.all(
        ['light', 'bold'].map(async (name, index) => {
            await mkdir(join(root, name));
            await writeFile(join(root, name, 'add.svg'), svg(10 + index * 8));
        }),
    );
    await writeFile(
        join(root, 'index.html'),
        '<html><head><script type="module" src="/entry.js"></script></head><body><span class="icon icon-add icon--bold"></span></body></html>',
    );
    await writeFile(join(root, 'entry.js'), "import 'virtual:vite-svg-2-webfont.css';");

    const options: IconPluginVariantOptions = {
        context: root,
        dest: join(root, 'output'),
        fontName: 'icons',
        variants: [
            { name: 'light', context: 'light', default: true, weight: 300 },
            { name: 'bold', context: 'bold', weight: 700 },
        ],
        formatOptions: { ttf: { ts: 1_700_000_000 }, woff2: { compressionQuality: 10 } },
        ...overrides,
    };
    const ownedCleanup = cleanup.move();
    return { root, options, [Symbol.asyncDispose]: () => ownedCleanup.disposeAsync() };
}

async function serve(options: IconPluginVariantOptions) {
    await using cleanup = new AsyncDisposableStack();
    let reload!: MockInstance<Awaited<ReturnType<typeof createServer>>['reloadModule']>;
    let handler!: Parameters<typeof watcher>[2];
    watcher.mockImplementationOnce(async (_roots, _signal, callback) => {
        handler = callback;
    });

    const server = await createServer({
        configFile: false,
        root: options.context,
        logLevel: 'silent',
        plugins: [
            {
                name: 'observe-font-reloads',
                enforce: 'pre',
                configureServer(created) {
                    reload = vi.spyOn(created, 'reloadModule');
                },
            },
            viteSvgToWebfont(options),
        ],
        server: { port: 0, host: '127.0.0.1' },
    });
    cleanup.defer(() => server.close());

    await server.listen();
    const addressInfo = server.httpServer!.address();
    const port = typeof addressInfo === 'string' ? addressInfo : addressInfo?.port;
    const url = `http://127.0.0.1:${port}`;

    const css = () => fetch(`${url}/@id/__x00__virtual:vite-svg-2-webfont.css`).then(response => response.text());
    await css();

    const font = async () => new Uint8Array(await (await fetch(`${url}/icons.woff2`)).arrayBuffer());

    const ownedCleanup = cleanup.move();
    return { server, handler, reload, css, font, [Symbol.asyncDispose]: () => ownedCleanup.disposeAsync() };
}

it('serves one shared font, reloads on edits, and matches a fresh family build', async () => {
    await using files = await fixture();
    const { root, options } = files;
    await using dev = await serve(options);

    const before = await dev.font();
    const css = await dev.css();
    expect(css).toContain('icon--bold');
    expect(css).toContain('font-synthesis');

    await writeFile(join(root, 'bold/add.svg'), svg(23));
    await dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
    expect(dev.reload).toHaveBeenCalled();

    const after = await dev.font();
    expect(after).not.toEqual(before);

    const fresh = await (await native()).generateWebfonts(parseOptions(options));
    expect(after).toEqual(fresh.woff2);
    expect(watcher).toHaveBeenCalledExactlyOnceWith([join(root, 'light'), join(root, 'bold')], expect.any(AbortSignal), dev.handler);
});

it('reloads inline CSS without requesting a font endpoint', async () => {
    await using files = await fixture({ inline: true });
    const { root, options } = files;
    await using dev = await serve(options);

    const before = await dev.css();
    expect(before).toContain('data:font/woff2');

    await writeFile(join(root, 'light/add.svg'), svg(5));
    await dev.handler([{ path: join(root, 'light/add.svg'), kind: 'changed' }]);
    expect(dev.reload).toHaveBeenCalledOnce();
    expect(await dev.css()).not.toBe(before);
});

it('reconciles nested additions, shared paths, and atomic-save hints', async () => {
    await using files = await fixture({ missingGlyphs: { behavior: 'blank' } });
    const { root, options } = files;
    options.variants = [
        { name: 'light', default: true, files: ['light/**/*.svg'] },
        { name: 'bold', files: ['light/**/*.svg', 'bold/*.svg'] },
    ];

    // The same path may be consumed by both designs; avoid duplicate logical names in bold.
    await rm(join(root, 'bold/add.svg'));
    await using dev = await serve(options);
    await mkdir(join(root, 'light/new'));
    await writeFile(join(root, 'light/new/extra.svg'), svg(8));
    await dev.handler([{ path: root, kind: 'changed' }]);
    await writeFile(join(root, 'light/add.svg'), svg(7));
    await dev.handler([{ path: join(root, 'light/add.svg'), kind: 'added' }]);
    await rm(join(root, 'light/new/extra.svg'));
    await dev.handler([{ path: join(root, 'light/new/extra.svg'), kind: 'removed' }]);

    expect(await dev.font()).toEqual((await (await native()).generateWebfonts(parseOptions(options))).woff2);
    expect(watcher.mock.calls[0]![0]).toEqual([root]);
    expect(watcher).toHaveBeenCalledExactlyOnceWith([root], expect.any(AbortSignal), dev.handler);
});

it('retains served output on failed edits and recovers on the next valid batch', async () => {
    await using files = await fixture();
    const { root, options } = files;
    await using dev = await serve(options);

    const before = await dev.font();
    await writeFile(join(root, 'bold/add.svg'), 'invalid svg');
    await expect(dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }])).rejects.toThrow(/SVG|document/i);
    expect(await dev.font()).toEqual(before);
    expect(dev.reload).not.toHaveBeenCalled();

    await writeFile(join(root, 'bold/add.svg'), svg(21));
    await dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
    expect(await dev.font()).not.toEqual(before);
    expect(dev.reload).toHaveBeenCalledOnce();
});

it('uses full generation for callbacks and writes requested dev companions', async () => {
    const context = vi.fn();
    await using files = await fixture({ cssContext: context, generateFiles: ['css', 'html'] });
    const { root, options } = files;
    await using dev = await serve(options);

    expect(context).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ variants: [expect.objectContaining({ name: 'light' }), expect.objectContaining({ name: 'bold' })] }));

    await writeFile(join(root, 'bold/add.svg'), svg(21));
    await dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
    expect(context).toHaveBeenCalledTimes(2);
    expect(generate).toHaveBeenCalledTimes(2);
    expect(await readFile(join(root, 'output/icons.css'), 'utf8')).toContain('icon--bold');
    expect(await readFile(join(root, 'output/icons.html'), 'utf8')).toContain('icon--bold');
});

it('serializes cross-root updates while a native replacement is pending', async () => {
    await using files = await fixture();
    const { root, options } = files;
    const actual = await native();
    let initial!: GenerateWebfontsResult;
    generate.mockImplementationOnce(async input => {
        const result = await actual.generateWebfonts(input);
        initial = result as GenerateWebfontsResult;
        return result;
    });
    await using dev = await serve(options);
    const entered = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const method = initial.regenerateAsync.bind(initial);
    const spy = vi.spyOn(initial, 'regenerateAsync').mockImplementationOnce(async (...args) => {
        entered.resolve();
        await gate.promise;
        return method(...args);
    });
    const first = dev.handler([{ path: join(root, 'light/add.svg'), kind: 'changed' }]);
    let second: void | Promise<void>;
    try {
        await entered.promise;
        second = dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
        expect(spy).toHaveBeenCalledOnce();
        expect(dev.reload).not.toHaveBeenCalled();
    } finally {
        gate.resolve();
        await first;
    }
    await second;
    expect(dev.reload).toHaveBeenCalledTimes(2);
    expect(generate).toHaveBeenCalledOnce();
});

it.each([false, true])('builds shared assets, public metadata and preloads (inline=%s)', async inline => {
    await using files = await fixture({ inline, preloadFormats: ['woff', 'woff2'] });
    const { root, options } = files;
    const plugin = viteSvgToWebfont(options);
    const result = await build({ configFile: false, root, logLevel: 'silent', plugins: [plugin], build: { write: false, assetsInlineLimit: 0 } });
    if (Array.isArray(result) || !('output' in result)) throw new Error('Expected a single bundle');
    const assets = result.output.filter(chunk => chunk.type === 'asset');
    const fonts = assets.filter(asset => /\.(woff2?|ttf)$/.test(asset.fileName));
    expect(fonts).toHaveLength(inline ? 0 : 2);
    expect(plugin.api!.getGeneratedWebfonts()).toHaveLength(fonts.length);
    const html = String(assets.find(asset => asset.fileName === 'index.html')!.source);
    expect(html.match(/rel="preload"/g) ?? []).toHaveLength(inline ? 0 : 2);
    const css = String(assets.find(asset => asset.fileName.endsWith('.css'))!.source);
    expect(css).toContain('icon--bold');
    expect(css).toContain('font-weight:700');
    expect(css.includes('data:')).toBe(inline);
});

it('queues source changes arriving during initial generation', async () => {
    await using files = await fixture();
    const { root, options } = files;
    const entered = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const actual = await native();
    const regenerated = vi.fn();
    generate.mockImplementationOnce(async input => {
        const result = await actual.generateWebfonts(input);
        const method = result.regenerateAsync.bind(result);
        vi.spyOn(result, 'regenerateAsync').mockImplementation(async (...args) => {
            regenerated();
            return method(...args);
        });
        entered.resolve();
        await gate.promise;
        return result;
    });
    const starting = serve(options);
    let updating: void | Promise<void>;
    try {
        await entered.promise;
        await writeFile(join(root, 'bold/add.svg'), svg(9));
        updating = watcher.mock.calls[0]![2]([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
        expect(regenerated).not.toHaveBeenCalled();
    } finally {
        gate.resolve();
    }
    await using dev = await starting;
    await updating;
    expect(regenerated).toHaveBeenCalledOnce();
    expect(generate).toHaveBeenCalledOnce();
    expect(await dev.font()).toEqual((await actual.generateWebfonts(parseOptions(options))).woff2);
});

it.each([false, true])('settles an in-flight replacement on shutdown without late reloads (rejects=%s)', async rejects => {
    await using files = await fixture();
    const { root, options } = files;
    const actual = await native();
    let initial!: GenerateWebfontsResult;
    generate.mockImplementationOnce(async input => {
        const result = await actual.generateWebfonts(input);
        initial = result as GenerateWebfontsResult;
        return result;
    });
    await using dev = await serve(options);
    const entered = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const aborted = Promise.withResolvers<void>();
    const method = initial.regenerateAsync.bind(initial);
    vi.spyOn(initial, 'regenerateAsync').mockImplementationOnce(async (...args) => {
        entered.resolve();
        await gate.promise;
        if (rejects) throw new Error('generation failed during shutdown');
        return method(...args);
    });
    watcher.mock.calls[0]![1].addEventListener('abort', () => aborted.resolve(), { once: true });
    const updating = dev.handler([{ path: join(root, 'bold/add.svg'), kind: 'changed' }]);
    let closing: Promise<void> | undefined;
    try {
        await entered.promise;
        closing = dev.server.close();
        await aborted.promise;
    } finally {
        gate.resolve();
        await Promise.all([updating, closing]);
    }
    expect(dev.reload).not.toHaveBeenCalled();
    expect(generate).toHaveBeenCalledOnce();
});

it('discards a full-generation result completed after shutdown without writing companions', async () => {
    await using files = await fixture({ cssContext: () => undefined, generateFiles: ['css'] });
    await using dev = await serve(files.options);
    const entered = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const aborted = Promise.withResolvers<void>();
    const actual = await native();
    generate.mockImplementationOnce(async input => {
        const result = await actual.generateWebfonts(input);
        entered.resolve();
        await gate.promise;
        return result;
    });
    const writesBefore = writeCompanion.mock.calls.length;
    watcher.mock.calls[0]![1].addEventListener('abort', () => aborted.resolve(), { once: true });
    const updating = dev.handler([{ path: join(files.root, 'bold/add.svg'), kind: 'changed' }]);
    let closing: Promise<void> | undefined;
    try {
        await entered.promise;
        closing = dev.server.close();
        await aborted.promise;
    } finally {
        gate.resolve();
        await Promise.all([updating, closing]);
    }
    expect(generate).toHaveBeenCalledTimes(2);
    expect(writeCompanion).toHaveBeenCalledTimes(writesBefore);
    expect(dev.reload).not.toHaveBeenCalled();
});

it('waits for companion writes during shutdown and suppresses the late CSS reload', async () => {
    await using files = await fixture({ generateFiles: ['css'], missingGlyphs: { behavior: 'blank' } });
    await using dev = await serve(files.options);
    const entered = Promise.withResolvers<void>();
    const gate = Promise.withResolvers<void>();
    const aborted = Promise.withResolvers<void>();
    const actual = await vi.importActual<typeof import('./utils')>('./utils');
    writeCompanion.mockImplementationOnce(async (...args) => {
        entered.resolve();
        await gate.promise;
        return actual.ensureDirExistsAndWriteFile(...args);
    });
    const added = join(files.root, 'bold/new.svg');
    await writeFile(added, svg(12));
    watcher.mock.calls[0]![1].addEventListener('abort', () => aborted.resolve(), { once: true });
    const updating = dev.handler([{ path: added, kind: 'added' }]);
    let closing: Promise<void> | undefined;
    const closed = vi.fn();
    try {
        await entered.promise;
        closing = dev.server.close().then(closed);
        await aborted.promise;
        expect(closed).not.toHaveBeenCalled();
    } finally {
        gate.resolve();
        await Promise.all([updating, closing]);
    }
    expect(closed).toHaveBeenCalledOnce();
    expect(await readFile(join(files.root, 'output/icons.css'), 'utf8')).toContain('icon-new');
    expect(dev.reload).not.toHaveBeenCalled();
});

it('skips unmatched SVG notifications without regenerating or reloading the family', async () => {
    await using files = await fixture();
    const actual = await native();
    let initial!: GenerateWebfontsResult;
    generate.mockImplementationOnce(async input => {
        const result = await actual.generateWebfonts(input);
        initial = result as GenerateWebfontsResult;
        return result;
    });
    await using dev = await serve(files.options);
    const regenerate = vi.spyOn(initial, 'regenerateAsync');
    const before = await dev.font();
    await dev.handler([{ path: join(files.root, 'unmatched.svg'), kind: 'added' }]);
    expect(regenerate).not.toHaveBeenCalled();
    expect(generate).toHaveBeenCalledOnce();
    expect(dev.reload).not.toHaveBeenCalled();
    expect(await dev.font()).toEqual(before);
});

it('honors custom selectors, URLs and build write controls', async () => {
    await using files = await fixture({
        types: ['ttf', 'woff2'],
        classPrefix: 'glyph-',
        baseSelector: '.glyph',
        variantClassPrefix: 'design-',
        cssFontsUrl: 'https://cdn.example/fonts/',
        generateFiles: true,
        allowWriteFilesInBuild: true,
    });
    const { root, options } = files;
    await build({ configFile: false, root, logLevel: 'silent', plugins: [viteSvgToWebfont(options)], build: { write: false, assetsInlineLimit: 0 } });
    const css = await readFile(join(root, 'output/icons.css'), 'utf8');
    expect(css).toContain('glyph-add');
    expect(css).toContain('design-bold');
    expect(css).toContain('https://cdn.example/fonts/');
    expect((await readFile(join(root, 'output/icons.woff2'))).length).toBeGreaterThan(0);
    expect((await readFile(join(root, 'output/icons.ttf'))).length).toBeGreaterThan(0);
});

it('supports the existing SCSS template with family metadata', async () => {
    await using files = await fixture({ cssTemplate: templates.scss });
    const { options } = files;
    const result = await (await native()).generateWebfonts(parseOptions(options));
    const sass = await import('sass');
    const css = sass.compileString(result.generateCss(), { logger: sass.Logger.silent }).css;
    expect(css).toContain('font-weight: 700');
});
