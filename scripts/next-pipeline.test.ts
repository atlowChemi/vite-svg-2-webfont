import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vite-plus/test';

it.skipIf(process.platform === 'win32')(
    'fingerprints are stable and include compiler flags before pipeline lookup',
    () => {
        const script = fileURLToPath(new URL('./next-pipeline.mjs', import.meta.url));
        expect(spawnSync(process.execPath, [script, '--require-fingerprint'], { env: { ...process.env, CACHE_TOOLCHAIN_FINGERPRINT: '' } }).status).toBe(1);
        const fingerprint = (flags: string) =>
            JSON.parse(
                execFileSync(process.execPath, [script, '--fingerprint'], {
                    encoding: 'utf8',
                    env: { ...process.env, RUSTFLAGS: flags },
                }),
            );
        const original = fingerprint('');
        expect(fingerprint('').fingerprint).toBe(original.fingerprint);
        const changed = fingerprint('--cfg cache_audit_fingerprint_test');
        expect(changed.fingerprint).not.toBe(original.fingerprint);
        expect(changed.identity.buildEnvironment.RUSTFLAGS).toBe('--cfg cache_audit_fingerprint_test');
    },
    30_000,
);

it.skipIf(process.platform === 'win32')(
    'keys ignored local env files without coupling them to the compiler identity',
    () => {
        const root = mkdtempSync(join(tmpdir(), 'pipeline-fingerprint-'));
        try {
            for (const directory of ['scripts', '.cargo', 'packages/example']) mkdirSync(join(root, directory), { recursive: true });
            const script = join(root, 'scripts/next-pipeline.mjs');
            copyFileSync(fileURLToPath(new URL('./next-pipeline.mjs', import.meta.url)), script);
            writeFileSync(join(root, '.cargo/config.toml'), '# fixture');
            const read = () => JSON.parse(execFileSync(process.execPath, [script, '--fingerprint'], { encoding: 'utf8' }));
            const before = read();
            expect(before.fileInputs.CACHE_INPUT_EXAMPLE_ENV_LOCAL).toBe('missing');
            writeFileSync(join(root, 'packages/example/.env.local'), 'abc');
            const after = read();
            expect(after.fileInputs.CACHE_INPUT_EXAMPLE_ENV_LOCAL).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
            expect(after.fingerprint).toBe(before.fingerprint);
        } finally {
            rmSync(root, { recursive: true, force: true });
        }
    },
    30_000,
);
