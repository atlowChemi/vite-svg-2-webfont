import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vite-plus/test';

it.each([false, true])('retains every feature report and propagates failures (failure=%s)', fail => {
    const root = mkdtempSync(join(tmpdir(), 'rust-coverage-matrix-'));
    try {
        mkdirSync(join(root, 'scripts/ci'), { recursive: true });
        mkdirSync(join(root, '.config'));
        mkdirSync(join(root, 'bin'));
        for (const file of ['rust-coverage.sh', 'validate-native-coverage.mjs']) {
            copyFileSync(join(import.meta.dirname, file), join(root, 'scripts/ci', file));
        }
        writeFileSync(join(root, '.config/nextest.toml'), '');
        writeFileSync(
            join(root, 'bin/cargo'),
            `#!/usr/bin/env bash
set -eu
echo "$*" >> commands.log
if [[ "$2" == nextest ]]; then
  report="$(dirname "$CARGO_TARGET_DIR")/nextest/ci/junit.xml"
  mkdir -p "$(dirname "$report")"
  printf '<testsuites><testsuite name="%s" tests="1"/></testsuites>' "$*" > "$report"
  if [[ "$FAIL_MODE" == yes && "$*" == *'--features in_memory' ]]; then exit 7; fi
else
  for output; do :; done
  printf 'SF:crates/webfont-generator/src/lib.rs\nDA:1,1\nLF:1\nLH:1\nend_of_record\n' > "$output"
fi
`,
            { mode: 0o755 },
        );
        const run = spawnSync('bash', ['scripts/ci/rust-coverage.sh', 'engine'], {
            cwd: root,
            env: { ...process.env, PATH: `${join(root, 'bin')}:${process.env.PATH}`, FAIL_MODE: fail ? 'yes' : 'no' },
            encoding: 'utf8',
        });
        expect({ status: run.status, stderr: run.stderr }).toEqual({ status: fail ? 7 : 0, stderr: '' });
        const output = join(root, 'coverage/rust-engine');
        expect(readdirSync(output).toSorted()).toEqual(['junit-cli.xml', 'junit-fs.xml', 'junit-in_memory-async.xml', 'junit-in_memory.xml', 'junit.xml', 'rust.lcov']);
        expect(readFileSync(join(output, 'junit-in_memory.xml'), 'utf8')).toContain('--no-default-features --features in_memory');
        expect(readFileSync(join(output, 'junit-cli.xml'), 'utf8')).toContain('--no-default-features --features cli');
        expect(readFileSync(join(root, 'commands.log'), 'utf8').trim().split('\n')).toHaveLength(6);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});
