import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const manifest = join(root, 'Cargo.toml');
const revision = '8db455ef9c2b1c7b33cf70d8ba370c1f2c49d4c9';
const patch = join(root, 'patches/fspy-mutations.patch');
const patchBytes = readFileSync(patch);
const hash = createHash('sha256').update('bootstrap-v2').update(patchBytes).digest('hex');
const upstream = join(root, 'target', `fspy-${revision}-${hash}`);
const git = args => execFileSync('git', args, { cwd: upstream, stdio: 'inherit' });
mkdirSync(upstream, { recursive: true });
if (!existsSync(join(upstream, '.git'))) {
    git(['init']);
    git(['remote', 'add', 'origin', 'https://github.com/voidzero-dev/vite-task']);
}
if (!existsSync(join(upstream, 'Cargo.toml'))) {
    git(['fetch', '--depth=1', 'origin', revision]);
    git(['checkout', '--detach', revision]);
}
const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: upstream, encoding: 'utf8' }).trim();
if (head !== revision) throw new Error(`Unexpected fspy checkout: ${head}`);
if (spawnSync('git', ['apply', '--reverse', '--check', patch], { cwd: upstream, stdio: 'ignore' }).status !== 0) {
    git(['apply', '--check', patch]);
    git(['apply', '--index', patch]);
}
git(['diff', '--exit-code']);
const applied = execFileSync('git', ['diff', '--cached', '--binary', '--full-index'], { cwd: upstream });
if (!applied.equals(patchBytes)) throw new Error('Patched checkout differs from the checked-in patch');
const [task, ...args] = process.argv.slice(2);
if (!['build', 'test', 'clippy', 'fmt'].includes(task)) throw new Error('Expected build, test, clippy, or fmt');
const overrides = ['fspy', 'fspy_preload_unix', 'fspy_seccomp_unotify'].flatMap(name => [
    '--config',
    `patch."https://github.com/voidzero-dev/vite-task".${name}.path=${JSON.stringify(join(upstream, 'crates', name))}`,
]);
// Cargo searches config from cwd, not the manifest. Avoid parent pnpm vendoring.
const result = spawnSync('cargo', ['+nightly-2026-08-02', task, ...(task === 'fmt' ? [] : ['-Z', 'bindeps', ...overrides]), '--manifest-path', manifest, ...args], {
    cwd: '/',
    stdio: 'inherit',
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
