import { cp, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { API } from 'typescript/unstable/async';
import { createServer } from 'vite-plus';
import { expect, it, vi } from 'vite-plus/test';
import { viteSvgToWebfont } from './index';

// Model an immutable plugin installation without changing permissions on the checkout.
vi.mock(import('node:fs/promises'), async importOriginal => {
    const actual = await importOriginal();
    return {
        ...actual,
        mkdir: ((...args: Parameters<typeof actual.mkdir>) => {
            if (String(args[0]).startsWith(packageRoot)) throw new Error('EACCES: read-only plugin installation');
            return actual.mkdir(...args);
        }) as typeof actual.mkdir,
    };
});

const packageRoot = resolve(import.meta.dirname, '..');

async function typecheck(project: string) {
    const api = new API({ cwd: project });
    try {
        const snapshot = await api.updateSnapshot({ openProjects: [join(project, 'tsconfig.json')] });
        const { program } = snapshot.getProject(join(project, 'tsconfig.json'))!;
        return (
            await Promise.all([
                program.getConfigFileParsingDiagnostics(),
                program.getProgramDiagnostics(),
                program.getGlobalDiagnostics(),
                program.getSyntacticDiagnostics(),
                program.getBindDiagnostics(),
                program.getSemanticDiagnostics(),
            ])
        ).flat();
    } finally {
        await api.close();
    }
}

it('isolates precise types for consumers of a read-only installation through either type opt-in', async () => {
    const root = await mkdtemp(join(tmpdir(), 'webfont-virtual-types-'));
    const first = join(root, 'first');
    const second = join(root, 'second');
    try {
        await Promise.all(
            [first, second].map(async project => {
                await cp(join(import.meta.dirname, 'fixtures/virtual-types'), project, { recursive: true });
                if (project === second) {
                    await rename(join(project, 'add.svg'), join(project, 'remove.svg'));
                    const app = await readFile(join(project, 'app.ts'), 'utf8');
                    await writeFile(join(project, 'app.ts'), app.replaceAll('add', 'remove'));
                }
                const server = await createServer({
                    root: project,
                    cacheDir: join(root, 'shared-cache'),
                    configFile: false,
                    logLevel: 'silent',
                    server: { port: 0, watch: null },
                    plugins: [viteSvgToWebfont({ context: project, types: 'ttf', tsModuleId: 'test-icons' })],
                });
                try {
                    await server.listen();
                    expect((await server.ssrLoadModule('virtual:test-icons')).icons).toEqual([project === first ? 'add' : 'remove']);
                } finally {
                    await server.close();
                }
            }),
        );

        // tsconfig opt-in verifies literal values, readonly tuples and invalid-name rejection.
        expect(await typecheck(first)).toEqual([]);
        expect(await typecheck(second)).toEqual([]);
        const app = await readFile(join(first, 'app.ts'), 'utf8');
        await writeFile(join(first, 'app.ts'), app.replace('// @ts-expect-error invalid icon names must be rejected', ''));
        expect(await typecheck(first)).toEqual(expect.arrayContaining([expect.objectContaining({ code: 2322 })]));

        // Also support the documented triple-slash alternative, without a separate fixture file.
        const config = JSON.parse(await readFile(join(first, 'tsconfig.json'), 'utf8'));
        config.files = ['app.ts'];
        await writeFile(join(first, 'tsconfig.json'), JSON.stringify(config));
        await writeFile(join(first, 'app.ts'), `/// <reference path="./.generated/vite-svg-2-webfont/index.d.ts" />\n${app}`);
        expect(await typecheck(first)).toEqual([]);
    } finally {
        await rm(root, { recursive: true, force: true });
    }
}, 30_000);
