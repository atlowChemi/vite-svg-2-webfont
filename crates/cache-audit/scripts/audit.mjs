import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, openSync, closeSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const require = createRequire(new URL('../../../package.json', import.meta.url));
const { parse } = require('yaml');
const { tasks } = parse(readFileSync(join(root, 'pnpm-workspace.yaml'), 'utf8'));
const manifest = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
if (process.argv[2] === '--all') {
    const results = [];
    for (const policy of ['native', 'build']) {
        const definitions = parse(readFileSync(join(root, `crates/cache-audit/policy.${policy}.yaml`), 'utf8'));
        for (const key of Object.keys(definitions.tasks)) {
            const task = key.split('#')[1];
            const result = spawnSync(process.execPath, [fileURLToPath(import.meta.url), task], { cwd: root, stdio: 'inherit' });
            if (result.error) throw result.error;
            results.push({ task, exitCode: result.status });
        }
    }
    const directory = process.env.CACHE_AUDIT_REPORTS || join(root, 'artifacts/cache-audit');
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, 'task-audits.json'), JSON.stringify(results, null, 2));
    process.exit(results.every(result => result.exitCode === 0) ? 0 : 1);
}
const identity = JSON.parse(execFileSync(process.execPath, [join(root, 'scripts/next-pipeline.mjs'), '--fingerprint'], { cwd: root, encoding: 'utf8' }));
const environment = { ...process.env, ...identity.fileInputs, CACHE_TOOLCHAIN_FINGERPRINT: identity.fingerprint };
const task = process.argv[2];
if (!tasks[task] || !manifest.scripts[task]) throw new Error('Expected an explicit root task name');
const policy = resolve(process.argv[3] || join(root, `crates/cache-audit/policy.${task === 'next:native' ? 'native' : 'build'}.yaml`));
const binary = process.env.CACHE_AUDIT_BIN || join(root, 'crates/cache-audit/target/debug/cache-audit');
const evidence = resolve(process.env.CACHE_AUDIT_REPORTS || join(root, 'artifacts/cache-audit'));
mkdirSync(evidence, { recursive: true });
const done = new Set();
const visiting = new Set();
function prepare(name) {
    if (done.has(name)) return;
    if (visiting.has(name) || !tasks[name]) throw new Error(`Unsupported/cyclic prerequisite ${name}`);
    visiting.add(name);
    for (const dependency of tasks[name].dependsOn || []) prepare(dependency);
    visiting.delete(name);
    if (name !== task) {
        const result = spawnSync('pnpm', ['run', name], { cwd: root, stdio: 'inherit', env: environment });
        if (result.error) throw result.error;
        if (result.status !== 0) throw new Error(`Prerequisite ${name} failed (${result.status})`);
    }
    done.add(name);
}
prepare(task);
if (task === 'next:wasm') {
    const { setup } = await import(pathToFileURL(join(root, 'packages/webfont-browser/scripts/setup.mjs')).href);
    setup();
}
const target = mkdtempSync(join(tmpdir(), 'cache-audit-cargo-'));
const name = task.replaceAll(':', '-');
const stdout = openSync(join(evidence, `${name}.json`), 'w');
const stderr = openSync(join(evidence, `${name}.log`), 'w');
const started = Date.now();
let result;
try {
    result = spawnSync(binary, ['--workspace', root, '--project', '.', '--task', task, '--policy', policy, '--', 'pnpm', 'run', task], {
        cwd: root,
        stdio: ['inherit', stdout, stderr],
        env: { ...environment, CARGO_TARGET_DIR: target },
    });
    if (result.error) throw result.error;
} finally {
    closeSync(stdout);
    closeSync(stderr);
    rmSync(target, { recursive: true, force: true });
}
writeFileSync(join(evidence, `${name}.status.json`), JSON.stringify({ task, exitCode: result.status, seconds: (Date.now() - started) / 1000 }, null, 2));
if (task === 'next:native' && [0, 2].includes(result.status)) {
    const report = JSON.parse(readFileSync(join(evidence, `${name}.json`), 'utf8'));
    if (report.exit_code === 0) {
        const addon = spawnSync(
            process.execPath,
            ['--input-type=module', '-e', `await import(${JSON.stringify(pathToFileURL(join(root, 'packages/webfont-generator/binding.js')).href)})`],
            {
                cwd: root,
                encoding: 'utf8',
            },
        );
        writeFileSync(join(evidence, `${name}.addon.log`), `${addon.stdout ?? ''}${addon.stderr ?? ''}`);
        if (addon.error) throw addon.error;
        if (addon.status !== 0) throw new Error(`Generated addon failed to load (${addon.status})`);
    }
}
console.log(`${task}: exit ${result.status}; evidence ${evidence}`);
process.exit(result.status ?? 1);
