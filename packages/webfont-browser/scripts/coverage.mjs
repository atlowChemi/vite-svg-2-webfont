import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cli, setup } from './setup.mjs';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const run = (command, args, env = {}) => execFileSync(command, args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } });
setup();
run(
    'vp',
    ['test', '--project=webfont-wasm-browser', '--project=webfont-browser-build', '--coverage', '--coverage.include=src/**', '--coverage.reportsDirectory=coverage/browser-js'],
    {
        WEBFONT_COVERAGE: '1',
        CI_TEST_REPORT: 'coverage/browser-js/junit.xml',
        CI_TEST_SUITE: 'browser-coverage',
        // Bundle analysis belongs to the ordinary production smoke test, not coverage runs.
        ALLOW_BUNDLE_ANALYSIS: 'false',
    },
);
run('node', ['scripts/ci/validate-native-coverage.mjs', 'coverage/browser-js/lcov.info', 'browser-js']);

mkdirSync(`${root}/coverage/browser-rust`, { recursive: true });
// wasm-bindgen-test supplies minicov's WASM profiling runtime. Coverage alone needs
// nightly; release builds continue using stable and contain no profiling runtime.
run(
    'cargo',
    [
        `+${process.env.WEBFONT_COVERAGE_TOOLCHAIN ?? 'nightly-2026-09-14'}`,
        'llvm-cov',
        'test',
        '--locked',
        '-p',
        'webfont-browser',
        '--lib',
        '--target',
        'wasm32-unknown-unknown',
        '--ignore-filename-regex',
        '(/\\.pnpm/|/native/tests\\.rs$)',
        '--lcov',
        '--output-path',
        'coverage/browser-rust/lcov.info',
    ],
    {
        CARGO_TARGET_DIR: `${root}/target/browser-coverage`,
        CARGO_LLVM_COV_TARGET_DIR: `${root}/target/browser-coverage`,
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER: `${dirname(cli)}/wasm-bindgen-test-runner${process.platform === 'win32' ? '.exe' : ''}`,
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: '-Cinstrument-coverage -Zno-profiler-runtime -Clink-args=--no-gc-sections --cfg=wasm_bindgen_unstable_test_coverage',
    },
);
run('node', ['scripts/ci/validate-native-coverage.mjs', 'coverage/browser-rust/lcov.info', 'browser-rust']);
