import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

// Stable Cargo has no per-doctest JUnit output. Record each actually executed Cargo
// doctest invocation as an explicitly named aggregate case; retain the full output.
const root = resolve(import.meta.dirname, '../..');
const output = resolve(root, 'test-results/doctests.xml');
mkdirSync(resolve(root, 'test-results'), { recursive: true });
const escape = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
let failed = 0;
const cases = [];
for (const mode of ['default', 'cli']) {
    const start = performance.now();
    const result = spawnSync('cargo', ['test', '--locked', '-p', 'webfont-generator', '--doc', ...(mode === 'cli' ? ['--features', 'cli'] : [])], {
        cwd: root,
        encoding: 'utf8',
        maxBuffer: 16 * 1024 * 1024,
    });
    const stdout = result.stdout ?? '';
    const stderr = result.stderr ?? '';
    process.stdout.write(stdout);
    process.stderr.write(stderr);
    const failure = result.status !== 0;
    if (failure) failed++;
    cases.push(
        `<testcase classname="rust-doctest-invocations" name="${mode} (aggregate)" time="${(performance.now() - start) / 1000}">${failure ? `<failure message="Cargo doctests failed">${escape(result.error?.message ?? stderr)}</failure>` : ''}<system-out>${escape(stdout)}</system-out><system-err>${escape(stderr)}</system-err></testcase>`,
    );
    // Write after each invocation, so a later failure cannot discard earlier results.
    writeFileSync(
        output,
        `<?xml version="1.0" encoding="UTF-8"?><testsuites><testsuite name="rust-doctest-invocations" tests="${cases.length}" failures="${failed}" errors="0">${cases.join('')}</testsuite></testsuites>`,
    );
}
process.exitCode = failed ? 1 : 0;
