import { mkdir, mkdtemp, readdir, readFile, rm, stat, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build, createServer, type InlineConfig } from 'vite-plus';
import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { computeCacheKey } from './cache';
import { viteSvgToWebfont, type IconPluginOptions, type IconPluginVariantOptions } from './index';

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
    const config = (overrides: Partial<IconPluginOptions> = {}): InlineConfig => ({
        configFile: false,
        logLevel: 'silent',
        root,
        cacheDir: join(root, '.cache'),
        build: { write: false, assetsInlineLimit: 0 },
        // Overrides may switch to variant options, so the merged object is asserted as either mode.
        plugins: [viteSvgToWebfont({ context: icons, types: ['woff2', 'ttf'], formatOptions: { ttf: { ts: 0 } }, ...overrides } as IconPluginOptions)],
    });
    const run = async (overrides?: Partial<IconPluginOptions>) => {
        const result = await build(config(overrides));
        const { output } = Array.isArray(result) ? result[0]! : (result as Extract<typeof result, { output: unknown }>);
        return output
            .filter(chunk => chunk.type === 'asset')
            .map(({ fileName, source }) => [fileName, Buffer.from(source).toString('base64')])
            .toSorted(([a], [b]) => a!.localeCompare(b!));
    };
    /** The plugin's own cache files, without the engine's cache folder beside them. */
    const pluginFiles = async () => (await readdir(cacheFolder)).filter(name => name !== 'engine');
    return { root, icons, cacheFolder, config, run, pluginFiles, [Symbol.asyncDispose]: () => rm(root, { recursive: true, force: true }) };
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

    it('reuses outputs when the same cache directory is accessed through another path', async () => {
        await using p = await project();
        await p.run();
        expect(generate).toHaveBeenCalledOnce();

        const alias = join(p.root, '.cache-alias');
        await symlink(join(p.root, '.cache'), alias, 'junction');
        await build({ ...p.config(), cacheDir: alias });
        expect(generate).toHaveBeenCalledOnce();
    });

    it('regenerates an older entry without TypeScript metadata', async () => {
        await using p = await project();
        const first = await p.run();
        const path = join(p.cacheFolder, 'iconfont.json');
        const manifest = JSON.parse(await readFile(path, 'utf8'));
        delete manifest.build.ts;
        await writeFile(path, JSON.stringify(manifest));
        expect(await p.run()).toEqual(first);
        expect(generate).toHaveBeenCalledTimes(2);
        expect(JSON.parse(await readFile(path, 'utf8')).build.ts).toContain('export const icons');
    });

    it('regenerates when an icon changes and prunes the stale fonts', async () => {
        await using p = await project();
        await p.run();
        const before = await p.pluginFiles();

        await writeFile(join(p.icons, 'add.svg'), svg(12));
        await p.run();

        expect(generate).toHaveBeenCalledTimes(2);
        const after = await p.pluginFiles();
        expect(after).toHaveLength(before.length);
        expect(after.filter(name => before.includes(name))).toEqual(['iconfont.json']);
        // Pruning only touches the plugin's own fonts, never the engine cache beside them.
        expect((await readdir(join(p.cacheFolder, 'engine'))).filter(name => name.endsWith('.wfcache'))).toHaveLength(1);
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
        const fonts = (await p.pluginFiles()).filter(name => !name.endsWith('.json'));
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

describe('cache option', () => {
    it('hands the engine a persistent cache next to the output cache by default', async () => {
        await using p = await project();
        const first = await p.run();
        const cacheLocation = join(p.cacheFolder, 'engine');
        expect(generate).toHaveBeenCalledWith(expect.objectContaining({ cacheLocation }));
        expect((await readdir(cacheLocation)).some(name => name.endsWith('.wfcache'))).toBe(true);

        // Without the plugin's own entry the engine runs again, warm, and produces the same output.
        // A cold engine would rewrite its cache file (a new inode); a warm full hit doesn't touch it.
        const file = (await readdir(cacheLocation)).find(name => name.endsWith('.wfcache'));
        const { ino } = await stat(join(cacheLocation, file!));
        await rm(join(p.cacheFolder, 'iconfont.json'));
        expect(await p.run()).toEqual(first);
        expect(generate).toHaveBeenCalledTimes(2);
        expect((await stat(join(cacheLocation, file!))).ino).toBe(ino);
    });

    it('generates from scratch and writes nothing to cacheDir when disabled', async () => {
        await using p = await project();
        const first = await p.run({ cache: false });
        expect(await p.run({ cache: false })).toEqual(first);
        expect(generate).toHaveBeenCalledTimes(2);
        expect(generate).not.toHaveBeenCalledWith(expect.objectContaining({ cacheLocation: expect.anything() }));
        await expect(readdir(p.cacheFolder)).rejects.toMatchObject({ code: 'ENOENT' });

        // A cache left by earlier runs is neither read nor updated.
        await p.run();
        const manifest = await readFile(join(p.cacheFolder, 'iconfont.json'));
        await writeFile(join(p.icons, 'add.svg'), svg(12));
        await p.run({ cache: false });
        await writeFile(join(p.icons, 'add.svg'), svg(10));
        expect(await p.run({ cache: false })).toEqual(first);
        expect(generate).toHaveBeenCalledTimes(5);
        expect(await readFile(join(p.cacheFolder, 'iconfont.json'))).toEqual(manifest);
    });
});

describe('cache parity', () => {
    it('builds the same output with the cache off, cold and warm', async () => {
        await using p = await project();
        const off = await p.run({ cache: false });
        expect(await p.run()).toEqual(off);
        expect(await p.run()).toEqual(off);
        await rm(join(p.cacheFolder, 'iconfont.json'));
        expect(await p.run()).toEqual(off);
    });

    it('builds the same variant family output with the cache off, cold and warm', async () => {
        await using p = await project();
        const family: Partial<IconPluginVariantOptions> = {
            types: ['woff2', 'ttf'],
            variants: [
                { name: 'light', default: true },
                { name: 'bold', weight: 700 },
            ],
        };
        const off = await p.run({ ...family, cache: false });
        expect(await p.run(family)).toEqual(off);
        await rm(join(p.cacheFolder, 'iconfont.json'));
        expect(await p.run(family)).toEqual(off);
        expect((await readdir(join(p.cacheFolder, 'engine'))).some(name => name.endsWith('.wfcache'))).toBe(true);
    });

    it('serves the same dev fonts after an edit when the engine starts warm', async () => {
        await using p = await project();
        const serve = async (overrides: Partial<IconPluginOptions> = {}) => {
            const server = await createServer({ ...p.config(overrides), server: { port: 0 } });
            await server.listen();
            const { port } = server.httpServer!.address() as { port: number };
            const fonts = await Promise.all(['woff2', 'ttf'].map(async type => Buffer.from(await (await fetch(`http://localhost:${port}/iconfont.${type}`)).arrayBuffer())));
            await server.close();
            return fonts;
        };
        await serve();
        // The edit misses the output cache; the engine, incremental and warm, reprocesses one icon.
        await writeFile(join(p.icons, 'add.svg'), svg(12));
        const warm = await serve();
        expect(generate).toHaveBeenLastCalledWith(expect.objectContaining({ cacheLocation: join(p.cacheFolder, 'engine'), incremental: true }));
        expect(warm).toEqual(await serve({ cache: false }));
    });
});

describe('computeCacheKey', () => {
    it('is stable for identical inputs and changes with file contents or paths', async () => {
        await using a = await project();
        await using b = await project();
        const key = (p: typeof a) => computeCacheKey({ files: [join(p.icons, 'add.svg')], dest: join(p.root, 'out') }, [join(p.icons, 'add.svg')], []);

        expect(await key(a)).toBe(await key(a));
        expect(await key(a)).not.toBe(await key(b));
        const before = await key(a);
        await writeFile(join(a.icons, 'add.svg'), svg(20));
        expect(await key(a)).not.toBe(before);
    });
});
