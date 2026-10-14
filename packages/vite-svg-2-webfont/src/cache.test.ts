import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build, createServer, type InlineConfig } from 'vite-plus';
import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { computeCacheKey } from './cache';
import { viteSvgToWebfont, type IconPluginFileOptions } from './index';

const { generate } = vi.hoisted(() => ({ generate: vi.fn<typeof import('@atlowchemi/webfont-generator').generateWebfonts>() }));
vi.mock('@atlowchemi/webfont-generator', async importOriginal => {
    const actual = await importOriginal<typeof import('@atlowchemi/webfont-generator')>();
    generate.mockImplementation(actual.generateWebfonts);
    return { ...actual, generateWebfonts: generate };
});

afterEach(() => {
    generate.mockClear();
});

const svg = (width: number) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0H${width}V24H0Z"/></svg>`;

async function project({ importCss = true } = {}) {
    const root = await mkdtemp(join(tmpdir(), 'plugin-cache-'));
    const icons = join(root, 'icons');
    await Promise.all([
        writeFile(join(root, 'index.html'), '<html><head><script type="module" src="/entry.js"></script></head></html>'),
        writeFile(join(root, 'entry.js'), importCss ? "import 'virtual:vite-svg-2-webfont.css';" : ''),
    ]);
    await mkdir(icons);
    await Promise.all([writeFile(join(icons, 'add.svg'), svg(10)), writeFile(join(icons, 'remove.svg'), svg(16))]);
    const cacheFolder = join(root, '.cache', 'vite-svg-2-webfont');
    const config = (overrides: Partial<IconPluginFileOptions> = {}): InlineConfig => ({
        configFile: false,
        logLevel: 'silent',
        root,
        cacheDir: join(root, '.cache'),
        build: { write: false, assetsInlineLimit: 0 },
        plugins: [viteSvgToWebfont({ context: icons, types: ['woff2', 'ttf'], formatOptions: { ttf: { ts: 0 } }, ...overrides })],
    });
    const run = async (overrides?: Partial<IconPluginFileOptions>) => {
        const result = await build(config(overrides));
        const { output } = Array.isArray(result) ? result[0]! : (result as Extract<typeof result, { output: unknown }>);
        return output
            .filter(chunk => chunk.type === 'asset')
            .map(({ fileName, source }) => [fileName, Buffer.from(source).toString('base64')])
            .toSorted(([a], [b]) => a!.localeCompare(b!));
    };
    return { root, icons, cacheFolder, config, run, [Symbol.asyncDispose]: () => rm(root, { recursive: true, force: true }) };
}

describe('output cache', () => {
    it('reuses outputs on an unchanged rebuild without calling the engine', async () => {
        await using p = await project();
        const first = await p.run();
        expect(generate).toHaveBeenCalledOnce();

        const second = await p.run();
        expect(generate).toHaveBeenCalledOnce();
        expect(second).toEqual(first);
    });

    it('regenerates when an icon changes and prunes the stale fonts', async () => {
        await using p = await project();
        await p.run();
        const before = await readdir(p.cacheFolder);

        await writeFile(join(p.icons, 'add.svg'), svg(12));
        await p.run();

        expect(generate).toHaveBeenCalledTimes(2);
        const after = await readdir(p.cacheFolder);
        expect(after).toHaveLength(before.length);
        expect(after.filter(name => before.includes(name))).toEqual(['iconfont.json']);
    });

    it('regenerates when an option changes', async () => {
        await using p = await project();
        await p.run();
        await p.run({ classPrefix: 'i-' });
        expect(generate).toHaveBeenCalledTimes(2);
    });

    it('keeps dev and build entries side by side, sharing identical fonts', async () => {
        await using p = await project();
        const serveOnce = async () => {
            const server = await createServer({ ...p.config(), server: { port: 0 } });
            await server.listen();
            const { port } = server.httpServer!.address() as { port: number };
            const font = Buffer.from(await (await fetch(`http://localhost:${port}/iconfont.woff2`)).arrayBuffer());
            await server.close();
            return font;
        };

        const devFont = await serveOnce();
        await p.run();
        expect(generate).toHaveBeenCalledTimes(2);

        expect(await serveOnce()).toEqual(devFont);
        await p.run();
        expect(generate).toHaveBeenCalledTimes(2);

        // ttf is identical across modes; woff2 differs by the dev compression default.
        const fonts = (await readdir(p.cacheFolder)).filter(name => !name.endsWith('.json'));
        expect(fonts.filter(name => name.endsWith('.ttf'))).toHaveLength(1);
        expect(fonts.filter(name => name.endsWith('.woff2'))).toHaveLength(2);
    });

    it('bypasses the cache when cssContext is set', async () => {
        await using p = await project();
        await p.run({ cssContext: () => undefined });
        await p.run({ cssContext: () => undefined });
        expect(generate).toHaveBeenCalledTimes(2);
    });

    it('treats a corrupt manifest or a missing font as a miss', async () => {
        await using p = await project();
        const first = await p.run();

        await writeFile(join(p.cacheFolder, 'iconfont.json'), '{not json');
        expect(await p.run()).toEqual(first);

        const font = (await readdir(p.cacheFolder)).find(name => name.endsWith('.woff2'))!;
        await rm(join(p.cacheFolder, font));
        expect(await p.run()).toEqual(first);
        expect(generate).toHaveBeenCalledTimes(3);
    });

    it('neither fails nor caches when an unused CSS template is broken', async () => {
        await using p = await project({ importCss: false });
        const overrides = { generateFiles: 'fonts', dest: join(p.root, 'out'), allowWriteFilesInBuild: true, cssTemplate: join(p.root, 'missing.hbs') } as const;

        await p.run(overrides);
        await p.run(overrides);
        expect(generate).toHaveBeenCalledTimes(2);
    });

    it('writes the engine output files on a hit', async () => {
        await using p = await project();
        const dest = join(p.root, 'out');
        const overrides = { dest, generateFiles: true, allowWriteFilesInBuild: true } as const;
        const read = async () => Promise.all(['iconfont.woff2', 'iconfont.ttf', 'iconfont.css', 'iconfont.html'].map(name => readFile(join(dest, name))));

        await p.run(overrides);
        const written = await read();
        await rm(dest, { recursive: true });

        await p.run(overrides);
        expect(generate).toHaveBeenCalledOnce();
        expect(await read()).toEqual(written);
    });
});

describe('computeCacheKey', () => {
    it('ignores the project location but not file contents', async () => {
        await using a = await project();
        await using b = await project();
        const key = (p: typeof a) => computeCacheKey({ files: [join(p.icons, 'add.svg')], dest: join(p.root, 'out') }, [join(p.icons, 'add.svg')], p.root, []);

        expect(key(a)).toBe(key(b));
        await writeFile(join(b.icons, 'add.svg'), svg(20));
        expect(key(a)).not.toBe(key(b));
    });

    it('keeps cssFontsUrl root-specific because it is rendered into the CSS', async () => {
        await using a = await project();
        await using b = await project();
        const key = (p: typeof a) => computeCacheKey({ cssFontsUrl: join(p.root, 'fonts') }, [], p.root, []);

        expect(key(a)).not.toBe(key(b));
    });
});
