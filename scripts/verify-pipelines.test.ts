import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vite-plus/test';
import { stringify } from 'yaml';

const config = (directory: string, tasks: object) => writeFileSync(join(directory, 'vite.config.ts'), `export default ${JSON.stringify({ run: { tasks } })}`);

it('preserves the real workspace command, prerequisite and cache-environment contracts', () => {
    const script = fileURLToPath(new URL('./verify-pipelines.mjs', import.meta.url));
    expect(execFileSync(process.execPath, [script], { encoding: 'utf8' })).toContain('Verified 45 task commands/prerequisites');
});

it('rejects an unmapped Vite+ task, including tasks in newly discovered packages', () => {
    const root = mkdtempSync(join(tmpdir(), 'pipeline-parity-'));
    try {
        symlinkSync(fileURLToPath(new URL('../node_modules', import.meta.url)), join(root, 'node_modules'), 'junction');
        const script = fileURLToPath(new URL('./verify-pipelines.mjs', import.meta.url));
        const args = [script, '--workspace', root];
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
        expect(execFileSync(process.execPath, args, { encoding: 'utf8' })).toContain('Verified 5 task');
        config(root, { ...original, overlooked: { command: 'echo missing' } });
        let result = spawnSync(process.execPath, args, { encoding: 'utf8' });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('Missing dormant counterpart: .#overlooked');
        config(root, original);
        const added = join(root, 'packages/new-package');
        mkdirSync(added, { recursive: true });
        writeFileSync(join(added, 'package.json'), JSON.stringify({ name: 'new-package', type: 'module' }));
        config(added, { dev: { command: 'echo new' } });
        result = spawnSync(process.execPath, args, { encoding: 'utf8' });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('Missing dormant counterpart: packages/new-package#dev');
        rmSync(join(root, 'packages'), { recursive: true });

        // Keep native config resolution; replace only pipeline execution with a controlled runner.
        const pnpm = execFileSync('which', ['pnpm'], { encoding: 'utf8' }).trim();
        const bin = join(root, 'bin');
        mkdirSync(bin);
        const runner = join(bin, 'pnpm');
        writeFileSync(
            runner,
            `#!${process.execPath}\n` +
                `import {spawnSync} from 'node:child_process';\n` +
                `if(process.argv[2]==='pipeline'){console.log(process.argv.slice(2).join(' '));process.exit(process.argv[3]===process.env.FAIL_PIPELINE?1:0)}\n` +
                `const r=spawnSync(${JSON.stringify(pnpm)},process.argv.slice(2),{stdio:'inherit'});process.exit(r.status??1);\n`,
        );
        chmodSync(runner, 0o755);
        const env = { ...process.env, PATH: `${bin}:${process.env.PATH}` };
        const summary = join(root, 'artifacts/cache-audit/pipelines/summary.json');
        result = spawnSync(process.execPath, [...args, '--run'], { encoding: 'utf8', env });
        expect(result.status).toBe(0);
        const executions = JSON.parse(readFileSync(summary, 'utf8')) as { pipeline: string; exitCode: number }[];
        expect(executions).toHaveLength(15);
        expect(executions.every(entry => entry.exitCode === 0)).toBe(true);
        expect(readFileSync(join(root, 'artifacts/cache-audit/pipelines/next-docs.log'), 'utf8')).toContain('pipeline next-docs --full --no-cache');
        result = spawnSync(process.execPath, [...args, '--run'], { encoding: 'utf8', env: { ...env, FAIL_PIPELINE: 'next-docs' } });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('Pipeline failures; see');
        expect(JSON.parse(readFileSync(summary, 'utf8'))).toContainEqual(expect.objectContaining({ pipeline: 'next-docs', exitCode: 1 }));
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});
