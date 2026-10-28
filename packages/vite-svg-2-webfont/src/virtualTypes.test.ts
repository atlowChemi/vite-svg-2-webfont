import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { API } from 'typescript/unstable/async';
import { createServer } from 'vite-plus';
import { expect, it, vi } from 'vite-plus/test';
import { viteSvgToWebfont } from './index';

const { publication } = vi.hoisted(() => ({ publication: { directory: '' } }));
vi.mock('./virtualTypes', async importOriginal => {
    const actual = await importOriginal<typeof import('./virtualTypes')>();
    return {
        ...actual,
        // Keep declaration generation real, but redirect its writes into the test application.
        publishDeclaration: (id: string, source: string) => actual.publishDeclaration(id, source, publication.directory),
    };
});

const packageRoot = resolve(import.meta.dirname, '..');

it('exposes precise virtual types to an application through either type opt-in', async () => {
    const root = await mkdtemp(join(tmpdir(), 'webfont-virtual-types-'));
    const fixtureNodeModules = join(root, 'node_modules');
    const installed = join(fixtureNodeModules, 'vite-svg-2-webfont');
    publication.directory = join(installed, 'dist/generated');
    const typecheck = async () => {
        const api = new API({ cwd: root });
        try {
            const snapshot = await api.updateSnapshot({ openProjects: [join(root, 'tsconfig.json')] });
            const { program } = snapshot.getProject(join(root, 'tsconfig.json'))!;
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
    };
    try {
        await cp(join(import.meta.dirname, 'fixtures/virtual-types'), root, { recursive: true });
        await mkdir(join(installed, 'dist'), { recursive: true });
        await cp(join(packageRoot, 'package.json'), join(installed, 'package.json'));
        await writeFile(join(installed, 'dist/virtuals.d.ts'), '/// <reference path="./generated/index.d.ts" />\n');

        const server = await createServer({
            root,
            configFile: false,
            logLevel: 'silent',
            server: { port: 0, watch: null },
            plugins: [viteSvgToWebfont({ context: root, types: 'ttf', tsModuleId: 'test-icons' })],
        });
        try {
            await server.listen();
            expect((await server.ssrLoadModule('virtual:test-icons')).icons).toEqual(['add']);
        } finally {
            await server.close();
        }

        // tsconfig opt-in verifies literal values, readonly tuples and invalid-name rejection.
        expect(await typecheck()).toEqual([]);
        const app = await readFile(join(root, 'app.ts'), 'utf8');
        await writeFile(join(root, 'app.ts'), app.replace('// @ts-expect-error invalid icon names must be rejected', ''));
        expect(await typecheck()).toEqual(expect.arrayContaining([expect.objectContaining({ code: 2322 })]));

        // Also support the documented triple-slash alternative, without a separate fixture file.
        const config = JSON.parse(await readFile(join(root, 'tsconfig.json'), 'utf8'));
        config.compilerOptions.types = [];
        await writeFile(join(root, 'tsconfig.json'), JSON.stringify(config));
        await writeFile(join(root, 'app.ts'), `/// <reference types="vite-svg-2-webfont/virtuals" />\n${app}`);
        expect(await typecheck()).toEqual([]);
    } finally {
        await rm(root, { recursive: true, force: true });
    }
}, 30_000);
