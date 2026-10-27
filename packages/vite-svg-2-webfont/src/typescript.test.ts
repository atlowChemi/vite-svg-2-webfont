import { hash, randomUUID } from 'node:crypto';
import { mkdtemp, readdir, readFile, rename, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { build, createServer } from 'vite-plus';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { generateWebfonts } from '@atlowchemi/webfont-generator';
import { viteSvgToWebfont } from './index';
import { parseGenerateFilesOption, resolveFileDest } from './optionParser';
import { moduleDeclaration, moduleSource, publishDeclaration } from './virtualTypes';

const { readdirMock, renameMock } = vi.hoisted(() => ({ readdirMock: vi.fn(), renameMock: vi.fn() }));
const generateMock = vi.hoisted(() => vi.fn());
vi.mock(import('@atlowchemi/webfont-generator'), async importOriginal => {
    const original = await importOriginal();
    generateMock.mockImplementation(original.generateWebfonts);
    return { ...original, generateWebfonts: Object.assign(generateMock, original.generateWebfonts) };
});
const publication = vi.hoisted(() => ({ directory: '' }));
vi.mock(import('./virtualTypes'), async importOriginal => {
    const original = await importOriginal();
    return {
        ...original,
        publishDeclaration: (id: string, source: string, directory = publication.directory) => original.publishDeclaration(id, source, directory),
    };
});
vi.mock(import('node:fs/promises'), async importOriginal => {
    const original = await importOriginal();
    readdirMock.mockImplementation(original.readdir);
    renameMock.mockImplementation(original.rename);
    return { ...original, readdir: readdirMock, rename: renameMock };
});

const folders: string[] = [];
async function temporary() {
    const folder = await mkdtemp(join(tmpdir(), 'webfont-ts-'));
    folders.push(folder);
    return folder;
}
beforeEach(async () => {
    generateMock.mockClear();
    publication.directory = await temporary();
});
afterEach(async () => {
    await Promise.all(folders.splice(0).map(folder => rm(folder, { recursive: true, force: true })));
});
const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h20v20H2z"/></svg>';

it.each<[Parameters<typeof parseGenerateFilesOption>[0], boolean]>([
    [{}, false],
    [{ generateFiles: false }, false],
    [{ generateFiles: true }, false],
    [{ generateFiles: 'css' }, false],
    [{ generateFiles: ['fonts'] }, false],
    [{ generateFiles: 'ts', tsDest: 'icons.ts' }, true],
    [{ generateFiles: ['ts'], tsDest: 'icons.ts' }, true],
    [{ generateFiles: true, tsDest: 'icons.ts' }, true],
    [{ generateFiles: 'ts' }, true],
    [{ generateFiles: false, tsDest: 'icons.ts' }, false],
])('selects TS output for %j: %s', (options, expected) => {
    expect(parseGenerateFilesOption(options).ts).toBe(expected);
});

it('defaults the TS destination to the normalized font name', () => {
    expect(resolveFileDest('/fonts', undefined, 'Icons', 'ts')).toBe(resolve('/fonts', 'icons.ts'));
});

it('exposes real NAPI TS output, context callbacks with CSS disabled, writing and regeneration', async () => {
    const root = await temporary();
    const first = join(root, 'add.svg');
    const next = join(root, 'remove.svg');
    await Promise.all([writeFile(first, svg), writeFile(next, svg)]);
    const result = await generateWebfonts({
        files: [first],
        dest: root,
        types: ['ttf'],
        css: false,
        html: false,
        ts: true,
        tsDest: join(root, 'nested/icons.ts'),
        incremental: true,
        cssContext(context) {
            context.baseSelector = '.custom';
            context.classPrefix = 'custom-';
        },
    });
    expect(result.generateTs()).toContain('".custom" as const');
    expect(result.generateTs()).toContain('[\n  "add"\n] as const');
    expect(await readFile(join(root, 'nested/icons.ts'), 'utf8')).toBe(result.generateTs());
    const incremental = await generateWebfonts({ files: [first], dest: root, types: ['ttf'], writeFiles: false, incremental: true });
    const changed = await incremental.regenerateAsync({ files: [next] });
    expect(changed.generateTs()).toContain('[\n  "remove"\n] as const');
    expect(changed.generateTs()).not.toContain('"add"');
});

it('publishes escaped exact declarations, preserves other IDs and skips unchanged files', async () => {
    const root = await temporary();
    const source =
        'export const iconBaseSelector = ".icon" as const;\nexport const iconClassPrefix = "icon-" as const;\nexport const icons = ["add","remove"] as const;\nexport type IconName = (typeof icons)[number];\n';
    const ids = ['virtual:one', 'virtual:two/../"\\\n'];
    await Promise.all(ids.map(id => publishDeclaration(id, source, root)));
    const index = join(root, 'index.d.ts');
    expect((await readFile(index, 'utf8')).match(/reference path/g)).toHaveLength(2);
    const path = join(root, `${hash('sha256', ids[0]!)}.d.ts`);
    const before = await stat(path);
    const beforeIndex = await stat(index);
    await publishDeclaration(ids[0]!, source, root);
    expect((await stat(path)).mtimeMs).toBe(before.mtimeMs);
    expect((await stat(index)).mtimeMs).toBe(beforeIndex.mtimeMs);
    expect(moduleDeclaration(ids[1]!, source)).toContain(`declare module ${JSON.stringify(ids[1])}`);
    await publishDeclaration(ids[0]!, source.replace('"add","remove"', '"renamed"'), root);
    expect(await readFile(path, 'utf8')).toContain('readonly ["renamed"]');
    expect((await readFile(index, 'utf8')).match(/reference path/g)).toHaveLength(2);
});

it.each([
    { generateFiles: undefined, tsDest: undefined, write: false },
    { generateFiles: false, tsDest: 'metadata.ts', write: false },
    { generateFiles: true, tsDest: undefined, write: false },
    { generateFiles: true, tsDest: 'metadata.ts', write: true },
    { generateFiles: 'ts' as const, tsDest: undefined, write: true },
    { generateFiles: ['css', 'ts'] as ('css' | 'ts')[], tsDest: 'metadata.ts', write: true },
])('serves runtime and cached startup with file settings $generateFiles / $tsDest', async ({ generateFiles, tsDest, write }) => {
    const root = await temporary();
    await writeFile(join(root, 'add.svg'), svg);
    const id = `test-${randomUUID()}`;
    const options = { context: root, dest: join(root, 'out'), types: ['ttf'] as const, tsModuleId: id, generateFiles, tsDest };
    // The second startup must reuse the first startup's persisted cache.
    /* oxlint-disable no-await-in-loop */
    for (let run = 0; run < 2; run++) {
        const server = await createServer({
            root,
            configFile: false,
            logLevel: 'silent',
            plugins: [viteSvgToWebfont({ ...options, types: [...options.types] })],
            server: { watch: null },
        });
        try {
            await server.listen();
            expect(generateMock).toHaveBeenCalledOnce();
            const runtime = await server.ssrLoadModule(`virtual:${id}`);
            expect(runtime.icons).toEqual(['add']);
            expect(runtime.iconClassPrefix).toBe('icon-');
            const transformed = await server.transformRequest(`virtual:${id}`);
            expect(transformed?.code).not.toContain('as const');
            const declaration = await readFile(join(publication.directory, `${hash('sha256', `virtual:${id}`)}.d.ts`), 'utf8');
            expect(declaration).toContain('readonly [\n  "add"\n]');
            expect((await readFile(join(root, 'out', tsDest ?? 'iconfont.ts'), 'utf8').catch(() => '')).includes('[\n  "add"\n] as const')).toBe(write);
        } finally {
            await server.close();
        }
    }
});

it('reports failed publication', async () => {
    const root = await temporary();
    const blocked = join(root, 'file');
    await writeFile(blocked, 'not a directory');
    await expect(publishDeclaration('virtual:blocked', '', blocked)).rejects.toThrow(/EEXIST|ENOTDIR/);
});

it.each(['listing', 'writing'] as const)('releases the publication lock after a %s failure', async operation => {
    const root = await temporary();
    const failure = operation === 'listing' ? readdirMock : renameMock;
    failure.mockRejectedValueOnce(new Error('publication failed'));
    await expect(publishDeclaration('virtual:failure', '', root)).rejects.toThrow('publication failed');
    await expect(stat(join(root, '.lock'))).rejects.toMatchObject({ code: 'ENOENT' });
    expect((await readdir(root)).some(name => name.endsWith('.tmp'))).toBe(false);
    await publishDeclaration('virtual:retry', '', root);
    expect(await readFile(join(root, 'index.d.ts'), 'utf8')).toContain(`${hash('sha256', 'virtual:retry')}.d.ts`);
});

it.each(['\u2028', '\u2029'])('preserves selector strings containing Unicode separator %j in runtime and declarations', async separator => {
    const prefix = ` as const;${separator}\nexport type IconName = (typeof icons)[number];\n"\\`;
    const source = `export const iconBaseSelector = ".icon" as const;\nexport const iconClassPrefix = ${JSON.stringify(prefix)} as const;\nexport const icons = [\n  "add"\n] as const;\n\nexport type IconName = (typeof icons)[number];\n`;
    const runtime = await import(`data:text/javascript,${encodeURIComponent(moduleSource(source))}`);
    expect(runtime.iconClassPrefix).toBe(prefix);
    expect(runtime.icons).toEqual(['add']);
    expect(moduleDeclaration('virtual:escaped', source)).toContain(`export const iconClassPrefix: ${JSON.stringify(prefix)};\n`);
});

it.each([false, true])('builds virtual runtime and honors allowWriteFilesInBuild (%s)', async allowWriteFilesInBuild => {
    const root = await temporary();
    await writeFile(join(root, 'add.svg'), svg);
    await writeFile(join(root, 'entry.js'), 'export { icons, iconBaseSelector } from "virtual:build-icons";');
    const output = await build({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: [viteSvgToWebfont({ context: root, types: 'ttf', dest: join(root, 'source'), generateFiles: 'ts', tsModuleId: 'build-icons', allowWriteFilesInBuild })],
        build: { write: false, lib: { entry: join(root, 'entry.js'), formats: ['es'] } },
    });
    expect(JSON.stringify(output)).toContain('add');
    expect((await readFile(join(root, 'source/iconfont.ts'), 'utf8').catch(() => '')).includes('[\n  "add"\n] as const')).toBe(allowWriteFilesInBuild);
});

it.each([false, true])('reloads TS metadata only when it changes (context callback: %s)', { timeout: 15_000 }, async withContext => {
    const root = await temporary();
    const icon = join(root, 'add.svg');
    const tsDest = join(root, 'icons.ts');
    await writeFile(icon, svg);
    const server = await createServer({
        root,
        configFile: false,
        logLevel: 'silent',
        plugins: [
            viteSvgToWebfont({
                context: root,
                types: 'ttf',
                generateFiles: 'ts',
                tsDest,
                cssContext: withContext
                    ? context => {
                          context.classPrefix = 'icon-';
                      }
                    : undefined,
            }),
        ],
    });
    try {
        await server.listen();
        await server.transformRequest('virtual:vite-svg-2-webfont.css');
        await server.transformRequest('virtual:vite-svg-2-webfont.ts');
        const css = await server.moduleGraph.getModuleByUrl('virtual:vite-svg-2-webfont.css');
        const ts = await server.moduleGraph.getModuleByUrl('virtual:vite-svg-2-webfont.ts');
        const invalidate = vi.spyOn(server.moduleGraph, 'invalidateModule');
        const before = await stat(tsDest);
        await writeFile(icon, svg.replace('M2 2h20v20H2z', 'M4 4h16v16H4z'));
        await expect.poll(() => invalidate.mock.calls.some(([module]) => module === css), { timeout: 5_000 }).toBe(true);
        expect(invalidate.mock.calls.some(([module]) => module === ts)).toBe(false);
        expect((await stat(tsDest)).mtimeMs).toBe(before.mtimeMs);
        await writeFile(join(root, 'remove.svg'), svg);
        await expect.poll(() => invalidate.mock.calls.some(([module]) => module === ts), { timeout: 5_000 }).toBe(true);
        expect((await server.ssrLoadModule('virtual:vite-svg-2-webfont.ts')).icons).toEqual(['add', 'remove']);
    } finally {
        await server.close();
    }
});

it('refreshes virtual runtime and precise declarations after additions, renames and removals', { timeout: 30_000 }, async () => {
    const root = await temporary();
    await writeFile(join(root, 'add.svg'), svg);
    const id = `watch-${randomUUID()}`;
    const server = await createServer({ root, configFile: false, logLevel: 'silent', plugins: [viteSvgToWebfont({ context: root, types: 'ttf', tsModuleId: id })] });
    try {
        await server.listen();
        await server.transformRequest(`virtual:${id}`);
        const invalidate = vi.spyOn(server.moduleGraph, 'invalidateModule');
        const declaration = join(publication.directory, `${hash('sha256', `virtual:${id}`)}.d.ts`);
        await writeFile(join(root, 'remove.svg'), svg);
        await expect.poll(() => readFile(declaration, 'utf8'), { timeout: 5_000 }).toContain('readonly [\n  "add",\n  "remove"\n]');
        await expect.poll(async () => (await server.transformRequest(`virtual:${id}`))?.code, { timeout: 5_000 }).toContain('remove');
        await rename(join(root, 'remove.svg'), join(root, 'renamed.svg'));
        await expect.poll(() => readFile(declaration, 'utf8'), { timeout: 5_000 }).toContain('readonly [\n  "add",\n  "renamed"\n]');
        await expect.poll(async () => (await server.transformRequest(`virtual:${id}`))?.code, { timeout: 5_000 }).toContain('renamed');
        await rm(join(root, 'add.svg'));
        await expect.poll(() => readFile(declaration, 'utf8'), { timeout: 5_000 }).toContain('readonly [\n  "renamed"\n]');
        expect(invalidate).toHaveBeenCalled();
        await expect.poll(async () => (await server.transformRequest(`virtual:${id}`))?.code, { timeout: 5_000 }).not.toContain('"add"');
    } finally {
        await server.close();
    }
});
