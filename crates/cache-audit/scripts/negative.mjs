import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const { parse, stringify } = createRequire(new URL('../../../package.json', import.meta.url))('yaml');
const path = join(root, 'pnpm-workspace.yaml');
const original = readFileSync(path, 'utf8');
const reports = process.env.CACHE_AUDIT_REPORTS || join(root, 'artifacts/cache-audit');
try {
    for (const [name, field, declaration, suffix, diagnostic] of [
        ['source', 'inputs', 'crates/webfont-generator/**', '/crates/webfont-generator/src/lib.rs', 'read not covered'],
        ['output', 'outputs', 'packages/webfont-generator/binding.d.ts', '/packages/webfont-generator/binding.d.ts', 'undeclared write'],
    ]) {
        const config = parse(original);
        const values = config.tasks['next:native'][field];
        assert.ok(values.includes(declaration), `Missing baseline declaration: ${declaration}`);
        config.tasks['next:native'][field] = values.filter(value => value !== declaration);
        writeFileSync(path, stringify(config));
        const directory = join(reports, `negative-${name}`);
        const result = spawnSync(process.execPath, [fileURLToPath(new URL('./audit.mjs', import.meta.url)), 'next:native'], {
            cwd: root,
            stdio: 'inherit',
            env: { ...process.env, CACHE_AUDIT_REPORTS: directory },
        });
        if (result.error) throw result.error;
        assert.equal(result.status, 2, `${name}: expected findings, not a command/configuration failure`);
        const report = JSON.parse(readFileSync(join(directory, 'next-native.json'), 'utf8'));
        assert.equal(report.exit_code, 0);
        assert.ok(
            report.audit.findings.some(([file, reason]) => file.endsWith(suffix) && reason.includes(diagnostic)),
            `${name}: omission was not detected`,
        );
        console.log(`Verified deliberate ${name} omission is detected.`);
    }
} finally {
    writeFileSync(path, original);
}
