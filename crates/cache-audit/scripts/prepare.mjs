import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const upstream = join(root, '.pnpm/cache-audit-fspy');
const revision = '8db455ef9c2b1c7b33cf70d8ba370c1f2c49d4c9';
const patch = join(root, 'crates/cache-audit/patches/fspy-mutations.patch');
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
if (!applied.equals(readFileSync(patch))) throw new Error('Patched checkout differs from the checked-in patch');
