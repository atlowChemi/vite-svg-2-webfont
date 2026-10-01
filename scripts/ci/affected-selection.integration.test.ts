import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vite-plus/test';
import { packageNames, resolveSelection } from './affected-selection';

const repo = resolve(import.meta.dirname, '../..');
let fixture: string;
let base: string;
const run = (command: string, args: string[]) => execFileSync(command, args, { cwd: fixture, encoding: 'utf8', timeout: 25_000, stdio: ['pipe', 'pipe', 'pipe'] });
const git = (...args: string[]) => run('git', args).trim();
const commit = () => {
    git('add', '-A');
    git('commit', '-qm', 'synthetic selection scenario');
    return git('rev-parse', 'HEAD');
};
const change = (path: string) =>
    writeFileSync(join(fixture, path), `${readFileSync(join(fixture, path), 'utf8')}\n${/\.(?:ts|rs|js)$/.test(path) ? '//' : '#'} selection scenario\n`);

beforeAll(() => {
    fixture = mkdtempSync(join(tmpdir(), 'webfont-selection-'));
    // Real tracked manifests, directory ownership, and dependency edges. Commands
    // only query packages; application builds/tests never execute in the fixture.
    const archive = execFileSync('git', ['archive', 'HEAD'], { cwd: repo, maxBuffer: 64 * 1024 * 1024 });
    execFileSync('tar', ['-xf', '-'], { cwd: fixture, input: archive });
    symlinkSync(join(repo, 'node_modules'), join(fixture, 'node_modules'), 'dir');
    git('init', '-q');
    git('config', 'user.name', 'Selection fixture');
    git('config', 'user.email', 'selection@example.invalid');
    git('config', 'core.hooksPath', '/dev/null');
    git('config', 'commit.gpgsign', 'false');
    base = commit();
});

beforeEach(() => {
    git('checkout', '--detach', base);
    git('reset', '--hard', base);
});
afterAll(() => {
    if (fixture) rmSync(fixture, { recursive: true, force: true });
});

describe('real Git/pnpm affected selection', () => {
    it.each(['packages/webfont-generator/package.json', 'packages/webfont-generator/Cargo.toml'])('keeps manifest-only pushes downstream: %s', path => {
        const file = join(fixture, path);
        writeFileSync(file, `${readFileSync(file, 'utf8')}\n`);
        commit();
        const result = resolveSelection(base, run, true);
        expect(result.full).toBe(false);
        expect(result.packages).toEqual([packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root].toSorted());
        expect(result.rustSuites).toEqual(['adapter']);
    });

    it('fails the CLI with an annotation when a workspace package is unknown', () => {
        const file = join(fixture, 'packages/docs/package.json');
        const manifest = JSON.parse(readFileSync(file, 'utf8'));
        manifest.name = 'unknown-docs-package';
        writeFileSync(file, JSON.stringify(manifest));
        commit();
        const event = join(fixture, 'event.json');
        const output = join(fixture, 'output.txt');
        writeFileSync(event, JSON.stringify({ before: base }));
        writeFileSync(output, '');
        const result = spawnSync(process.execPath, [join(repo, 'scripts/ci/affected-selection.ts')], {
            cwd: fixture,
            encoding: 'utf8',
            env: { ...process.env, GITHUB_EVENT_NAME: 'push', GITHUB_EVENT_PATH: event, GITHUB_OUTPUT: output },
        });
        expect(result.status).toBe(1);
        expect(result.stderr).toContain('::error title=Affected selection failed::Unknown affected packages: unknown-docs-package');
        expect(result.stderr).not.toMatch(/\n\s+at |Node\.js v/);
        expect(readFileSync(output, 'utf8')).toBe('');
        rmSync(event);
        rmSync(output);
    });

    it.each([
        ['crates/webfont-generator/src/lib.rs', [packageNames.engine, packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root], true],
        ['packages/webfont-generator/native/lib.rs', [packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root], true],
        ['packages/webfont-generator/index.js', [packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root], true],
        ['packages/vite-svg-2-webfont/src/index.ts', [packageNames.plugin, packageNames.example], false],
        ['packages/docs/getting-started.md', [packageNames.docs], false],
        ['packages/vite-svg-2-webfont/CHANGELOG.md', [packageNames.plugin, packageNames.example, packageNames.docs], false],
        ['packages/webfont-generator/templates/css.hbs', [packageNames.adapter, packageNames.plugin, packageNames.example, packageNames.root], true],
        ['packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/add.svg', [packageNames.plugin, packageNames.example], false],
    ] as const)('selects %s and accounts for non-package consumers', (path, packages, rust) => {
        change(path);
        commit();
        const result = resolveSelection(base, run);
        expect(result.full).toBe(false);
        expect(result.packages).toEqual([...packages].toSorted());
        expect(result.jobs['rust-coverage']).toBe(rust);
        expect(result.rustSuites).toEqual((packages as readonly string[]).includes(packageNames.engine) ? ['engine', 'cli', 'adapter'] : rust ? ['adapter'] : []);
        expect(result.nativeBuildScope).toBe(rust ? 'full' : 'linux-x64');
        expect(result.jobs.docs).toBe(path.startsWith('packages/docs/') || path === 'packages/vite-svg-2-webfont/CHANGELOG.md');
    });

    it.each(['Cargo.lock', 'pnpm-lock.yaml', '.github/workflows/main.yaml'])('broadens root-only selection for %s', path => {
        change(path);
        commit();
        const result = resolveSelection(base, run);
        expect(result.packages).toEqual(Object.values(packageNames).toSorted());
        expect(result.full).toBe(true);
    });

    it('selects both owners of a cross-package rename', () => {
        renameSync(join(fixture, 'packages/docs/getting-started.md'), join(fixture, 'packages/example/moved.md'));
        commit();
        const result = resolveSelection(base, run);
        expect(result.files).toEqual(['packages/docs/getting-started.md', 'packages/example/moved.md']);
        expect(result.packages).toEqual([packageNames.docs, packageNames.example].toSorted());
        expect(result.jobs.docs).toBe(true);
        expect(result.jobs.build).toBe(true);
    });

    it('retains engine consumers when an engine source is deleted', () => {
        rmSync(join(fixture, 'crates/webfont-generator/src/lib.rs'));
        commit();
        const result = resolveSelection(base, run);
        expect(result.jobs['rust-coverage']).toBe(true);
        expect(result.jobs['test-host']).toBe(true);
    });

    it('compares a diverged PR to its merge-base, excluding base-only docs edits', () => {
        change('packages/docs/getting-started.md');
        const baseTip = commit();
        git('checkout', '--detach', base);
        change('packages/vite-svg-2-webfont/src/index.ts');
        commit();
        const result = resolveSelection(baseTip, run);
        expect(result.base).toBe(base);
        expect(result.files).toEqual(['packages/vite-svg-2-webfont/src/index.ts']);
        expect(result.jobs.docs).toBe(false);
    });

    it('reports no affected packages for an unchanged head', () => {
        const result = resolveSelection(base, run);
        expect(result.full).toBe(false);
        expect(result.packages).toEqual([]);
    });

    it('falls back to full on unavailable history instead of silently skipping', () => {
        const result = resolveSelection('1'.repeat(40), run);
        expect(result.full).toBe(true);
        expect(Object.values(result.jobs).every(Boolean)).toBe(true);
    });
});
