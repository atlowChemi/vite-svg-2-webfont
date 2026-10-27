import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vite-plus/test';
import { stringify } from 'yaml';

const config = (directory: string, tasks: object) => writeFileSync(join(directory, 'vite.config.ts'), `export default ${JSON.stringify({ run: { tasks } })}`);

it('rejects an unmapped Vite+ task, including tasks in newly discovered packages', () => {
    const root = mkdtempSync(join(tmpdir(), 'pipeline-parity-'));
    try {
        mkdirSync(join(root, 'scripts'));
        symlinkSync(fileURLToPath(new URL('../node_modules', import.meta.url)), join(root, 'node_modules'), 'junction');
        const script = join(root, 'scripts/verify-pipelines.mjs');
        copyFileSync(fileURLToPath(new URL('./verify-pipelines.mjs', import.meta.url)), script);
        const aliases = {
            'next:test:js': 'test',
            'next:coverage': 'coverage',
            'next:bench:vitest': 'bench:vitest',
            'next:test:compat': 'test:compat',
            'next:coverage:scripts': 'coverage:scripts',
        };
        const scripts = Object.fromEntries(Object.keys(aliases).map(name => [name, 'echo fixture']));
        scripts['next:fmt:rust'] = 'cargo fmt --all -- --check';
        const original = Object.fromEntries(Object.values(aliases).map(name => [name, { command: 'echo fixture' }]));
        writeFileSync(join(root, 'package.json'), JSON.stringify({ name: 'fixture', type: 'module', packageManager: 'pnpm@12.15.0', scripts }));
        writeFileSync(join(root, 'pnpm-workspace.yaml'), stringify({ include: { './tasks.yaml': ['tasks', 'pipelines'] } }));
        writeFileSync(
            join(root, 'tasks.yaml'),
            stringify({
                tasks: Object.fromEntries(Object.keys(scripts).map(name => [name, { cache: false, dependsOn: [] }])),
                pipelines: {
                    'next-check': ['next:fmt:rust'],
                    'next-rust-test': ['next:fmt:rust'],
                    'next-test': ['next:test:js'],
                    'fixture-other-tasks': Object.keys(scripts).filter(name => name !== 'next:test:js'),
                },
            }),
        );
        config(root, original);
        expect(execFileSync(process.execPath, [script], { encoding: 'utf8' })).toContain('Verified 5 task');
        config(root, { ...original, overlooked: { command: 'echo missing' } });
        let result = spawnSync(process.execPath, [script], { encoding: 'utf8' });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('Missing dormant counterpart: .#overlooked');
        config(root, original);
        const added = join(root, 'packages/new-package');
        mkdirSync(added, { recursive: true });
        writeFileSync(join(added, 'package.json'), JSON.stringify({ name: 'new-package', type: 'module' }));
        config(added, { dev: { command: 'echo new' } });
        result = spawnSync(process.execPath, [script], { encoding: 'utf8' });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('Missing dormant counterpart: packages/new-package#dev');
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});
