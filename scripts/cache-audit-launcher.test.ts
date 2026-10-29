import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, it } from 'vite-plus/test';

it.skipIf(process.platform === 'win32')('forwards audit selectors with or without the package-runner separator', () => {
    const directory = mkdtempSync(join(tmpdir(), 'audit-launcher-'));
    try {
        const binary = join(directory, 'audit-stub');
        writeFileSync(binary, '#!/usr/bin/env node\nconsole.log(JSON.stringify(process.argv.slice(2)));\n', { mode: 0o755 });
        const launcher = fileURLToPath(new URL('../packages/cache-audit/scripts/run.mjs', import.meta.url));
        for (const options of [['--all'], ['--', '--all'], ['--', 'next:native']]) {
            const result = JSON.parse(
                execFileSync(process.execPath, [launcher, 'audit', ...options], {
                    encoding: 'utf8',
                    env: { ...process.env, CACHE_AUDIT_BIN: binary },
                }),
            );
            expect(result.slice(0, 2)).toEqual(['audit', '--workspace']);
            expect(result.slice(3)).toEqual(options.at(-1) === 'next:native' ? ['--task', 'next:native'] : ['--all']);
        }
    } finally {
        rmSync(directory, { recursive: true, force: true });
    }
});
