import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vitest';
import { parse, stringify } from 'yaml';

it('versions intents with pnpm, synchronizes Cargo, and publishes only reviewed version changes', () => {
    const root = mkdtempSync(join(tmpdir(), 'pnpm-release-'));
    const script = fileURLToPath(new URL('./release.mjs', import.meta.url));
    const config = parse(readFileSync(new URL('../pnpm-workspace.yaml', import.meta.url), 'utf8'));
    // Git hooks can inject repository-local Git variables into child processes.
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('GIT_')));
    const run = (command: string, ...args: string[]) => execFileSync(command, args, { cwd: root, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
    const write = (path: string, value: string) => writeFileSync(join(root, path), value);
    const read = (path: string) => readFileSync(join(root, path), 'utf8');
    const native = 'packages/webfont-generator';
    const engine = 'crates/webfont-generator';
    const plugin = 'packages/vite-svg-2-webfont';
    const browser = 'packages/webfont-browser';
    try {
        write('package.json', JSON.stringify({ name: 'fixture', private: true }));
        write('pnpm-workspace.yaml', stringify({ packages: ['packages/*', 'crates/*'], versioning: config.versioning }));
        for (const [dir, name] of [
            [native, '@atlowchemi/webfont-generator'],
            [engine, '@atlowchemi/webfont-engine'],
            [plugin, 'vite-svg-2-webfont'],
            [browser, '@atlowchemi/webfont-browser-generator'],
        ]) {
            mkdirSync(join(root, dir!), { recursive: true });
            write(
                `${dir}/package.json`,
                JSON.stringify({
                    name,
                    version: '1.0.0',
                    private: true,
                    ...(dir === plugin ? { dependencies: { '@atlowchemi/webfont-generator': 'workspace:*' } } : {}),
                }),
            );
            write(`${dir}/CHANGELOG.md`, '# Changelog\n\n## 1.0.0\n\nExisting history.\n');
        }
        write(`${engine}/Cargo.toml`, '[package]\nname = "webfont-generator"\nversion = "1.0.0"\n');
        write(
            `${native}/Cargo.toml`,
            '[package]\nname = "webfont-generator-napi"\nversion = "1.0.0"\n\n[dependencies]\nwebfont-generator = { path = "../../crates/webfont-generator", version = "1.0.0" }\n',
        );
        run('git', 'init');
        run('git', 'config', 'user.name', 'Release Test');
        run('git', 'config', 'user.email', 'release@example.test');
        run('git', 'config', 'core.hooksPath', '/dev/null');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'initial');
        const base = run('git', 'rev-parse', 'HEAD');
        expect(JSON.parse(run('node', script, 'plan', base))).toEqual([]);
        expect(() => run('node', script, 'intent', base)).toThrow(/Every PR must add a new intent/);
        run('pnpm', 'change', '@atlowchemi/webfont-generator', '--bump', 'none', '--summary', 'No release needed.');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'no release intent');
        run('node', script, 'intent', base);
        expect(run('pnpm', 'version', '-r', '--dry-run', '--json')).toBe('[]');
        const intentBase = run('git', 'rev-parse', 'HEAD');
        // Neither an existing intent nor a README added in a PR satisfies the gate.
        write('.changeset/README.md', 'Not an intent.');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'readme only');
        expect(() => run('node', script, 'intent', intentBase)).toThrow(/Every PR must add a new intent/);
        write('.changeset/invalid.md', '---\n{}\n---\nMissing a package decision.\n');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'invalid intent');
        expect(() => run('node', script, 'intent', intentBase)).toThrow(/Invalid intent bumps/);
        run('git', 'rm', '.changeset/invalid.md');
        run('git', 'commit', '-m', 'remove invalid intent');
        run('pnpm', 'change', '@atlowchemi/webfont-generator', '--bump', 'minor', '--summary', 'Fix engine output.');
        run('pnpm', 'change', 'check');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'intent');
        const releaseBase = run('git', 'rev-parse', 'HEAD');
        run('pnpm', 'version', '-r');
        expect(() => run('node', script, 'check')).toThrow(/Engine\/native versions must match/);
        run('node', script, 'sync');
        run('node', script, 'check');
        const notes = read(`${engine}/CHANGELOG.md`);
        expect(notes).toContain('## 1.1.0');
        expect(notes).toContain('Fix engine output.');
        expect(notes).toContain('Existing history.');
        run('node', script, 'sync');
        expect(read(`${engine}/CHANGELOG.md`)).toBe(notes);
        expect(read(`${native}/Cargo.toml`)).toContain('version = "1.1.0"');
        expect(JSON.parse(read(`${engine}/package.json`)).version).toBe('1.0.0');
        run('git', 'add', '.');
        run('node', script, 'release-intent');
        expect(read(`.changeset/release-${releaseBase}.md`)).toContain('none');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'release');
        run('node', script, 'intent', releaseBase);
        expect(run('pnpm', 'version', '-r', '--dry-run', '--json')).toBe('[]');
        const plan = JSON.parse(run('node', script, 'plan', base));
        expect(plan.map(({ tag }: { tag: string }) => tag)).toEqual(['webfont-generator-v1.1.0', 'webfont-engine-v1.1.0', 'vite-svg-2-webfont-v1.0.1']);
        expect(JSON.parse(run('node', script, 'plan', run('git', 'rev-parse', 'HEAD')))).toEqual([]);
        // A later merge regenerates the live proposal from main, not from its prior bumps.
        run('git', 'checkout', '--detach', releaseBase);
        run('pnpm', 'change', '@atlowchemi/webfont-browser-generator', '--bump', 'patch', '--summary', 'Fix browser output.');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'another merged intent');
        const refreshedBase = run('git', 'rev-parse', 'HEAD');
        run('pnpm', 'version', '-r');
        run('node', script, 'sync');
        run('git', 'add', '.');
        run('node', script, 'release-intent');
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'refreshed release');
        run('node', script, 'intent', refreshedBase);
        expect(JSON.parse(read(`${native}/package.json`)).version).toBe('1.1.0');
        expect(JSON.parse(read(`${browser}/package.json`)).version).toBe('1.0.1');
        expect(read(`${engine}/CHANGELOG.md`).match(/## 1\.1\.0/g)).toHaveLength(1);
        // A version edit alone must not initiate publication.
        const previous = run('git', 'rev-parse', 'HEAD');
        write(`${browser}/package.json`, JSON.stringify({ name: '@atlowchemi/webfont-browser-generator', version: '1.0.2', private: true }));
        run('git', 'add', '.');
        run('git', 'commit', '-m', 'unplanned version');
        expect(() => run('node', script, 'plan', previous)).toThrow(/ledger update/);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
}, 30_000);
