import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, test } from 'vite-plus/test';

const source = readFileSync(new URL('../scripts/ci/set-vite-major.ts', import.meta.url), 'utf8');
const workspace = readFileSync(new URL('../pnpm-workspace.yaml', import.meta.url), 'utf8');
const packageJson = readFileSync(new URL('../package.json', import.meta.url), 'utf8');
let directory: string;
let script: string;
let workspaceFile: string;
let packageFile: string;

beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'vite-matrix-'));
    mkdirSync(join(directory, 'scripts', 'ci'), { recursive: true });
    script = join(directory, 'scripts', 'ci', 'set-vite-major.ts');
    workspaceFile = join(directory, 'pnpm-workspace.yaml');
    packageFile = join(directory, 'package.json');
    writeFileSync(script, source);
    writeFileSync(packageFile, packageJson);
    writeFileSync(workspaceFile, workspace);
});

afterEach(() => {
    rmSync(directory, { recursive: true, force: true });
});

function run(major: number, ...args: string[]) {
    // Like CI, run before installing dependencies: this fixture has no node_modules.
    return spawnSync(process.execPath, [script, String(major), ...args], { cwd: directory, encoding: 'utf8' });
}

test.each([6, 7, 8])('selects upstream Vite %i without changing the Vite+ toolchain or unrelated dependencies', major => {
    const result = run(major);
    expect(result.stderr).toBe('');
    expect(result.status).toBe(0);
    const expected = JSON.parse(packageJson);
    expected.devDependencies['vite-compat'] = `npm:vite@^${major}.0.0`;
    expect(JSON.parse(readFileSync(packageFile, 'utf8'))).toEqual(expected);
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});

test('validates a dry run without rewriting either manifest', () => {
    expect(run(6, '--dry-run').status).toBe(0);
    expect(readFileSync(packageFile, 'utf8')).toBe(packageJson);
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});

test('replaces a previously selected matrix version', () => {
    expect(run(6).status).toBe(0);
    expect(run(8).status).toBe(0);
    expect(JSON.parse(readFileSync(packageFile, 'utf8')).devDependencies['vite-compat']).toBe('npm:vite@^8.0.0');
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});
