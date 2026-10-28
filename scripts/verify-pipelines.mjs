import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, openSync, closeSync, writeFileSync, globSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: { workspace: { type: 'string' }, run: { type: 'boolean' } } });
const root = values.workspace ? pathToFileURL(`${resolve(values.workspace)}/`) : new URL('../', import.meta.url);
const manifest = path => JSON.parse(readFileSync(new URL(`${path}/package.json`, root), 'utf8'));
// Let installed pnpm resolve includes and setting ownership.
const workspace = Object.fromEntries(
    ['tasks', 'pipelines'].map(setting => [
        setting,
        JSON.parse(execFileSync('pnpm', ['config', 'get', setting, '--location', 'project', '--json'], { cwd: root, encoding: 'utf8' })),
    ]),
);
const rootPackage = manifest('.');
const mappings = new Map();
const rootTasks = {
    'next:test:js': 'test',
    'next:coverage': 'coverage',
    'next:bench:vitest': 'bench:vitest',
    'next:test:compat': 'test:compat',
    'next:coverage:scripts': 'coverage:scripts',
};
for (const [name, command] of Object.entries(rootPackage.scripts)) {
    const match = command.match(/^pnpm --dir (\S+) run next:(\S+)$/);
    if (name.startsWith('next:') && !['next:verify', 'next:fmt:rust', ...Object.keys(rootTasks)].includes(name)) {
        assert.ok(match, `Unrecognized dormant wrapper: ${name}`);
    }
    if (match) mappings.set(name, { directory: match[1], task: match[2] });
}
for (const [name, task] of Object.entries(rootTasks)) mappings.set(name, { directory: '.', task });
const configs = new Map();
const names = new Map();
await Promise.all(
    ['.', ...globSync(['packages/*/vite.config.ts', 'crates/*/vite.config.ts'], { cwd: root }).map(dirname)].map(async directory => {
        const config = (await import(pathToFileURL(resolve(fileURLToPath(root), directory, 'vite.config.ts')).href)).default;
        configs.set(directory, config.run.tasks);
        names.set(manifest(directory).name, directory);
    }),
);
const taskKey = (directory, task) => [...mappings].find(([, entry]) => entry.directory === directory && entry.task === task)?.[0];
for (const [directory, tasks] of configs) {
    for (const task of Object.keys(tasks)) assert.ok(taskKey(directory, task), `Missing dormant counterpart: ${directory}#${task}`);
}
const normalize = command => command.replace(/ && cargo fmt --all -- --check$/, '').replace(/ --run(?=\s|$)/g, '');
for (const [name, { directory, task }] of mappings) {
    const original = configs.get(directory)[task];
    assert.ok(original, `${name}: no matching Vite+ task`);
    const command = directory === '.' ? rootPackage.scripts[name] : manifest(directory).scripts[`next:${task}`];
    // Browser publishing moves its nested task invocation into a real graph edge.
    // Plugin release CI intentionally bypasses dependency builds; use raw scripts.
    const expectedCommand = original.command
        .replace(/^vp run pack:tgz && /, '')
        .replaceAll('vp run --ignore-depends-on pack:tgz', 'pnpm run next:pack:tgz')
        .replaceAll('vp run --ignore-depends-on pack', 'pnpm run next:pack');
    assert.equal(normalize(command), normalize(expectedCommand), `${name}: command differs from Vite+`);
    const dependencies = (original.dependsOn ?? []).map(dependency => {
        const [packageName, taskName] = dependency.includes('#') ? dependency.split('#') : [manifest(directory).name, dependency];
        const mapped = taskKey(names.get(packageName), taskName);
        assert.ok(mapped, `${name}: unmapped prerequisite ${dependency}`);
        return mapped;
    });
    if (name === 'next:browser:publish') dependencies.push('next:browser:tgz');
    assert.deepEqual(
        workspace.tasks[name].dependsOn.toSorted((a, b) => a.localeCompare(b)),
        dependencies.toSorted((a, b) => a.localeCompare(b)),
        `${name}: prerequisites differ from Vite+`,
    );
    for (const variable of original.cache?.env || []) {
        assert.ok(workspace.tasks[name].env?.includes(variable), `${name}: missing Vite+ cache environment input ${variable}`);
    }
}
assert.deepEqual(workspace.pipelines['next-test'], ['next:test:js'], 'Root test must preserve the Vitest scope and its build prerequisites');
for (const pipeline of ['next-check', 'next-rust-test']) {
    assert.ok(workspace.pipelines[pipeline].includes('next:fmt:rust'), `${pipeline}: missing workspace Rust formatting`);
}
for (const [pipeline, tasks] of Object.entries(workspace.pipelines)) {
    for (const task of tasks) assert.ok(rootPackage.scripts[task] && workspace.tasks[task], `${pipeline}: unknown task ${task}`);
}
const reached = new Set();
const visit = task => {
    if (reached.has(task)) return;
    reached.add(task);
    for (const dependency of workspace.tasks[task].dependsOn || []) visit(dependency);
};
for (const tasks of Object.values(workspace.pipelines)) for (const task of tasks) visit(task);
for (const task of mappings.keys()) assert.ok(reached.has(task), `${task}: no pipeline reaches this task`);
assert.equal(rootPackage.scripts['next:fmt:rust'], 'cargo fmt --all -- --check');
console.log(`Verified ${mappings.size} task commands/prerequisites against Vite+; --run and shared Rust formatting are intentional differences.`);
if (values.run) {
    const reports = new URL('artifacts/cache-audit/pipelines/', root);
    mkdirSync(reports, { recursive: true });
    const results = [];
    // Only finite, non-publishing validation entry points run automatically.
    // Dev/preview, release, benchmark, coverage-tool, and source-mutating commands
    // have dedicated pipelines invoked explicitly by their callers.
    const pipelines = [
        'next-native',
        'next-check',
        'next-test',
        'next-rust-test',
        'next-browser-test',
        'next-build',
        'next-docs',
        'next-example',
        'next-coverage',
        'next-assets',
        'next-plugin-test',
        'next-compat',
        'next-scripts-coverage',
        'next-browser-tarball',
        'next-plugin-tarball',
    ];
    const executions = pipelines.map(pipeline => ({
        name: pipeline,
        program: 'pnpm',
        args: ['pipeline', pipeline, '--full', '--no-cache'],
    }));
    for (const { name: pipeline, program, args } of executions) {
        const log = openSync(new URL(`${pipeline.replaceAll(':', '-')}.log`, reports), 'w');
        const started = Date.now();
        let result;
        try {
            result = spawnSync(program, args, {
                cwd: fileURLToPath(root),
                stdio: ['ignore', log, log],
            });
            if (result.error) throw result.error;
        } finally {
            closeSync(log);
        }
        results.push({ pipeline, exitCode: result.status, seconds: (Date.now() - started) / 1000 });
        writeFileSync(new URL('summary.json', reports), JSON.stringify(results, null, 2));
        console.log(`${pipeline}: exit ${result.status}`);
    }
    assert.ok(
        results.every(result => result.exitCode === 0),
        `Pipeline failures; see ${fileURLToPath(reports)}`,
    );
}
