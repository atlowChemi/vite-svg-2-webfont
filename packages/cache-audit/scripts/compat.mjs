import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { stringify } from 'yaml';

// Exercise the installed pnpm, so routine upgrades test behavior rather than
// requiring another exact-version allowlist edit.
const root = mkdtempSync(join(tmpdir(), 'pnpm-cache-compat-'));
const manifest = JSON.parse(readFileSync(new URL('../../../package.json', import.meta.url), 'utf8'));
const put = (path, text) => writeFileSync(join(root, path), text);
const exec = (program, args, env = {}) => execFileSync(program, args, { cwd: root, encoding: 'utf8', env: { ...process.env, ...env } });
try {
    put(
        'package.json',
        JSON.stringify({
            name: 'cache-compat',
            private: true,
            packageManager: manifest.packageManager,
            scripts: {
                build: `node -e "const f=require('node:fs');f.appendFileSync('trace','build\\n');f.writeFileSync('result',f.readFileSync('source.txt','utf8')+f.readFileSync('dependency.out','utf8'))"`,
                dependency: `node -e "require('node:fs').copyFileSync('dependency.txt','dependency.out')"`,
            },
        }),
    );
    put('.gitignore', 'trace\nresult\ndependency.out\nignored\nnode_modules/\n');
    put('source.txt', 'source');
    put('dependency.txt', 'dependency');
    put('unrelated', 'before');
    exec('git', ['init', '--quiet']);
    exec('git', ['add', '.']);
    exec('git', [
        '-c',
        'user.name=Cache compatibility',
        '-c',
        'user.email=cache@example.invalid',
        '-c',
        'core.hooksPath=/dev/null',
        'commit',
        '--quiet',
        '-m',
        'test: initialize cache compatibility fixture',
    ]);
    const configure = inputs =>
        put(
            'pnpm-workspace.yaml',
            stringify({
                packages: ['.'],
                pipelines: { proof: ['build'] },
                tasks: {
                    build: { cache: true, inputs, outputs: ['result'], env: ['CACHE_COMPAT_VALUE'], dependsOn: ['dependency'] },
                    dependency: { cache: true, inputs: ['dependency.txt'], outputs: ['dependency.out'], dependsOn: [] },
                },
            }),
        );
    const runs = () => (existsSync(join(root, 'trace')) ? readFileSync(join(root, 'trace'), 'utf8').split('\n').length - 1 : 0);
    const run = (hit, value = 'a') => {
        const before = runs();
        const log = exec('pnpm', ['pipeline', 'proof', '--full'], { CACHE_COMPAT_VALUE: value });
        assert.equal(runs(), before + (hit ? 0 : 1), `Expected cache ${hit ? 'hit' : 'miss'}\n${log}`);
        assert.equal(readFileSync(join(root, 'result'), 'utf8'), readFileSync(join(root, 'source.txt'), 'utf8') + readFileSync(join(root, 'dependency.txt'), 'utf8'));
    };
    configure(['source.*']);
    run(false);
    run(true);
    rmSync(join(root, 'result'));
    run(true); // Declared outputs restore without executing the task.
    put('ignored', 'ignored');
    put('unrelated', 'replacement excludes this');
    run(true);
    put('source.txt', 'changed');
    run(false);
    put('source.new', 'untracked input');
    run(false);
    put('dependency.txt', 'changed dependency');
    run(false);
    run(false, 'b');
    run(true, 'b');
    for (const inputs of [[], ['+source.*']]) {
        configure(inputs);
        run(false);
        run(true);
        put('unrelated', JSON.stringify(inputs));
        run(false); // Empty and additive selections retain default inputs.
        put('ignored', JSON.stringify(inputs));
        run(true);
    }
    console.log(`pnpm ${exec('pnpm', ['--version']).trim()}: cache input/dependency/env invalidation and output restoration passed.`);
} finally {
    rmSync(root, { recursive: true, force: true });
}
