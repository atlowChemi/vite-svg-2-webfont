import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, openSync, closeSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parse } from 'yaml';

const root = new URL('../', import.meta.url);
const manifest = path => JSON.parse(readFileSync(new URL(`${path}/package.json`, root), 'utf8'));
const workspace = parse(readFileSync(new URL('pnpm-workspace.yaml', root), 'utf8'));
const rootPackage = manifest('.');
const mappings = new Map();
for (const [name, command] of Object.entries(rootPackage.scripts)) {
    const match = command.match(/^(?:node scripts\/next-pipeline\.mjs --require-fingerprint && )?pnpm --dir (\S+) run next:(\S+)$/);
    if (name.startsWith('next:') && !['next:pipeline', 'next:verify', 'next:fmt:rust', 'next:test:js', 'next:coverage'].includes(name)) {
        assert.ok(match, `Unrecognized dormant wrapper: ${name}`);
    }
    if (match) mappings.set(name, { directory: match[1], task: match[2] });
}
mappings.set('next:test:js', { directory: '.', task: 'test' });
mappings.set('next:coverage', { directory: '.', task: 'coverage' });
const configs = new Map();
const names = new Map();
await Promise.all(
    [...new Set([...mappings.values()].map(({ directory }) => directory))].map(async directory => {
        const config = (await import(pathToFileURL(resolve(fileURLToPath(root), directory, 'vite.config.ts')).href)).default;
        configs.set(directory, config.run.tasks);
        names.set(manifest(directory).name, directory);
    }),
);
const taskKey = (directory, task) => [...mappings].find(([, entry]) => entry.directory === directory && entry.task === task)?.[0];
const normalize = command => command.replace(/ && cargo fmt --all -- --check$/, '').replace(/ --run(?=\s|$)/g, '');
for (const [name, { directory, task }] of mappings) {
    const original = configs.get(directory)[task];
    const command = directory === '.' ? rootPackage.scripts[name] : manifest(directory).scripts[`next:${task}`];
    assert.equal(normalize(command), normalize(original.command), `${name}: command differs from Vite+`);
    const dependencies = (original.dependsOn ?? []).map(dependency => {
        const [packageName, taskName] = dependency.includes('#') ? dependency.split('#') : [manifest(directory).name, dependency];
        const mapped = taskKey(names.get(packageName), taskName);
        assert.ok(mapped, `${name}: unmapped prerequisite ${dependency}`);
        return mapped;
    });
    assert.deepEqual(
        workspace.tasks[name].dependsOn.toSorted((a, b) => a.localeCompare(b)),
        dependencies.toSorted((a, b) => a.localeCompare(b)),
        `${name}: prerequisites differ from Vite+`,
    );
    for (const variable of original.cache?.env || []) {
        assert.ok(workspace.tasks[name].env?.includes(variable), `${name}: missing Vite+ cache environment input ${variable}`);
    }
}
for (const pipeline of ['next-check', 'next-test']) {
    assert.ok(workspace.pipelines[pipeline].includes('next:fmt:rust'), `${pipeline}: missing workspace Rust formatting`);
}
for (const [pipeline, tasks] of Object.entries(workspace.pipelines)) {
    for (const task of tasks) assert.ok(rootPackage.scripts[task] && workspace.tasks[task], `${pipeline}: unknown task ${task}`);
}
assert.equal(rootPackage.scripts['next:fmt:rust'], 'cargo fmt --all -- --check');
console.log(`Verified ${mappings.size} task commands/prerequisites against Vite+; --run and shared Rust formatting are intentional differences.`);
if (process.argv.includes('--run')) {
    const reports = new URL('../artifacts/cache-audit/pipelines/', import.meta.url);
    mkdirSync(reports, { recursive: true });
    const results = [];
    const reached = new Set();
    const visit = task => {
        if (reached.has(task)) return;
        reached.add(task);
        for (const dependency of workspace.tasks[task].dependsOn || []) visit(dependency);
    };
    for (const tasks of Object.values(workspace.pipelines)) for (const task of tasks) visit(task);
    const executions = Object.keys(workspace.pipelines).map(pipeline => ({
        name: pipeline,
        program: process.execPath,
        args: [fileURLToPath(new URL('./next-pipeline.mjs', import.meta.url)), pipeline, '--no-cache'],
    }));
    for (const name of mappings.keys()) {
        if (reached.has(name)) continue;
        assert.equal(workspace.tasks[name].cache, false, `${name}: cached tasks need a named pipeline`);
        assert.ok(
            workspace.tasks[name].dependsOn.every(dependency => reached.has(dependency)),
            `${name}: unprepared prerequisite`,
        );
        executions.push({ name, program: 'pnpm', args: ['run', name] });
    }
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
