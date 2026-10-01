import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, expect, it } from 'vite-plus/test';

const script = fileURLToPath(new URL('./ci/set-vite-major.ts', import.meta.url));
const workspace = readFileSync(new URL('../pnpm-workspace.yaml', import.meta.url), 'utf8');
const packageJson = readFileSync(new URL('../package.json', import.meta.url), 'utf8');
let directory: string;
let workspaceFile: string;
let packageFile: string;

beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'vite-matrix-'));
    workspaceFile = join(directory, 'pnpm-workspace.yaml');
    packageFile = join(directory, 'package.json');
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

it.each([6, 7, 8])('selects upstream Vite %i without changing the Vite+ toolchain or unrelated dependencies', major => {
    const result = run(major);
    expect(result.stderr).toBe('');
    expect(result.status).toBe(0);
    const expected = JSON.parse(packageJson);
    expected.devDependencies['vite-compat'] = `npm:vite@^${major}.0.0`;
    expect(JSON.parse(readFileSync(packageFile, 'utf8'))).toEqual(expected);
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});

it('validates a dry run without rewriting either manifest', () => {
    expect(run(6, '--dry-run').status).toBe(0);
    expect(readFileSync(packageFile, 'utf8')).toBe(packageJson);
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});

it('replaces a previously selected matrix version', () => {
    expect(run(6).status).toBe(0);
    expect(run(8).status).toBe(0);
    expect(JSON.parse(readFileSync(packageFile, 'utf8')).devDependencies['vite-compat']).toBe('npm:vite@^8.0.0');
    expect(readFileSync(workspaceFile, 'utf8')).toBe(workspace);
});
