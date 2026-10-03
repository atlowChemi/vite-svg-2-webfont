import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, expect, it } from 'vite-plus/test';

let directory: string;
let file: string;
let preload: string;
beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'package-manager-update-'));
    file = join(directory, 'package.json');
    preload = join(directory, 'registry.mjs');
});
afterEach(() => rmSync(directory, { recursive: true, force: true }));

function run(version: string, metadata: unknown, status = 200) {
    // Exercise the real CLI and filesystem writes; replace only the external registry.
    writeFileSync(
        preload,
        `globalThis.fetch = async url => {
        if (url !== ${JSON.stringify(`https://registry.npmjs.org/pnpm/${version}`)}) throw new Error('Unexpected registry URL: ' + url);
        return new Response(${JSON.stringify(JSON.stringify(metadata))}, { status: ${status} });
    };`,
    );
    return spawnSync(process.execPath, ['--import', preload, join(import.meta.dirname, 'update-pnpm-package-manager.ts'), version, '--file', file], {
        encoding: 'utf8',
        env: { ...process.env, NO_COLOR: '1' },
    });
}

it.each(['', 'not-a-version'])('rejects invalid CLI input without modifying files: %s', version => {
    const original = '{"name":"example"}\n';
    writeFileSync(file, original);
    const result = run(version, {});
    expect(result.status).toBe(2);
    expect(result.stderr).toMatch(/Usage:|Invalid version:/);
    expect(readFileSync(file, 'utf8')).toBe(original);
});

it.each(['12.8.1', 'latest'])('writes the resolved integrity pin for %s while preserving manifest formatting and data', version => {
    const original = { name: 'unicode-字体', packageManager: `pnpm@1.0.0+sha512.${'a'.repeat(128)}`, private: true };
    writeFileSync(file, JSON.stringify(original, null, '\t'));
    const result = run(version, { version: '12.8.1', dist: { integrity: 'sha512-AQIDBA==' } });
    expect(result.status).toBe(0);
    expect(readFileSync(file, 'utf8')).toBe(JSON.stringify({ ...original, packageManager: 'pnpm@12.8.1+sha512.01020304' }, null, '\t'));
});

it.each([
    { metadata: {}, status: 404, error: 'Version not found' },
    { metadata: {}, status: 503, error: 'Could not resolve' },
    { metadata: { version: '12.8.1' }, status: 200, error: 'Missing dist.integrity' },
    { metadata: { dist: { integrity: 'sha512-AQIDBA==' } }, status: 200, error: 'Missing resolved version' },
    { metadata: { version: '12.8.1', dist: { integrity: 'invalid' } }, status: 200, error: 'Invalid integrity format' },
])('leaves the manifest intact on registry failure: $error', ({ metadata, status, error }) => {
    const original = '{\n  "name": "example",\n  "packageManager": "pnpm@12.8.0"\n}\n';
    writeFileSync(file, original);
    const result = run('latest', metadata, status);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain(error);
    expect(readFileSync(file, 'utf8')).toBe(original);
});
