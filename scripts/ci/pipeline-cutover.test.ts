import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { parse } from 'yaml';

type Task = {
    project: string;
    script: string;
    missingScript: boolean;
    dependsOn: { project: string; script: string }[];
};

it('uses the rustup sysroot with CI-installed components inside pnpm', () => {
    const rustc = execFileSync('rustup', ['which', 'rustc'], { encoding: 'utf8' }).trim();
    const expected = execFileSync(rustc, ['--print', 'sysroot'], { encoding: 'utf8' }).trim();
    expect(execFileSync('pnpm', ['exec', 'rustc', '--print', 'sysroot'], { encoding: 'utf8' }).trim()).toBe(expected);
});

it('restores pipeline outputs using only the persisted pipeline directory', () => {
    const directory = mkdtempSync(join(tmpdir(), 'pipeline-cache-'));
    try {
        writeFileSync(join(directory, 'package.json'), JSON.stringify({ name: 'cache-probe', scripts: { build: 'node build.cjs' } }));
        writeFileSync(join(directory, 'build.cjs'), "require('node:fs').writeFileSync('output.txt', String(Date.now()))");
        writeFileSync(
            join(directory, 'pnpm-workspace.yaml'),
            'packages: [.]\npipelines:\n  build: [build]\ntasks:\n  build:\n    cache: true\n    inputs: [build.cjs]\n    outputs: [output.txt]\n',
        );
        const cache = join(directory, 'cache');
        const options = {
            cwd: directory,
            env: { ...process.env, PNPM_CONFIG_CACHE_DIR: cache },
            encoding: 'utf8' as const,
        };
        execFileSync('pnpm', ['install', '--ignore-scripts'], options);
        execFileSync('git', ['init', '--quiet'], options);
        execFileSync('git', ['add', 'package.json', 'pnpm-workspace.yaml', 'pnpm-lock.yaml', 'build.cjs'], options);
        execFileSync('pnpm', ['pipeline', 'build', '--full'], options);
        const output = readFileSync(join(directory, 'output.txt'), 'utf8');
        expect(existsSync(join(cache, 'pipeline'))).toBe(true);
        for (const entry of readdirSync(cache)) {
            if (entry !== 'pipeline') rmSync(join(cache, entry), { recursive: true, force: true });
        }
        rmSync(join(directory, 'output.txt'));
        expect(execFileSync('pnpm', ['pipeline', 'build', '--full'], options)).toContain('1 from cache');
        expect(readFileSync(join(directory, 'output.txt'), 'utf8')).toBe(output);
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
const graph = (pipeline: string): Task[] =>
    JSON.parse(
        execFileSync('pnpm', ['pipeline', pipeline, '--full', '--dry-run', '--json'], {
            encoding: 'utf8',
        }),
    ).tasks.filter((task: Task) => !task.missingScript);

it.each([
    ['docs', ['native', 'plugin:pack', 'wasm', 'browser:pack', 'docs:svg', 'docs:social', 'docs']],
    ['example', ['native', 'plugin:pack', 'docs:svg', 'example']],
    ['test', ['native', 'wasm', 'browser:pack', 'test:js']],
    ['rust-check', ['fmt:rust', 'engine:check', 'adapter:check']],
    ['native-release', ['fmt:rust', 'engine:check', 'engine:features', 'engine:test', 'adapter:check', 'adapter:test', 'native:release']],
    ['plugin-publish', ['plugin:publish']],
    ['browser-publish', ['wasm', 'browser:pack', 'browser:tgz', 'browser:publish']],
] as const)('resolves %s with its full prerequisites and release bypasses', (pipeline, expected) => {
    const tasks = graph(pipeline);
    expect(tasks.map(task => task.script).toSorted()).toEqual(expected.toSorted());
    expect(tasks.every(task => task.project === '.')).toBe(true);
    for (const task of tasks) {
        for (const dependency of task.dependsOn) {
            expect(tasks).toContainEqual(expect.objectContaining(dependency));
        }
    }
});

it('keeps supplied-artifact CI consumers on raw scripts and the pipeline sweep off PRs', () => {
    const main = parse(readFileSync('.github/workflows/main.yaml', 'utf8'));
    for (const name of ['test-host', 'test-docker', 'test-vite-compat']) {
        const pack = main.jobs[name].steps.find((step: { name?: string }) => step.name === 'Build plugin');
        expect(pack.run).toContain('pnpm --dir packages/vite-svg-2-webfont run pack');
    }
    const audit = parse(readFileSync('.github/workflows/cache-audit.yaml', 'utf8'));
    const plan = audit.jobs['select-work'].steps.find((step: { id?: string }) => step.id === 'work');
    expect(plan.env.AUDIT_ONLY).toBe("${{ github.event_name == 'pull_request' }}");
    expect(audit.jobs.pipelines.strategy['max-parallel']).toBe(2);
});
