import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { expect, it } from 'vite-plus/test';
import { parse } from 'yaml';

type Task = { project: string; script: string; missingScript: boolean; dependsOn: { project: string; script: string }[] };
const graph = (pipeline: string): Task[] =>
    JSON.parse(execFileSync('pnpm', ['pipeline', pipeline, '--full', '--dry-run', '--json'], { encoding: 'utf8' })).tasks.filter((task: Task) => !task.missingScript);

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
