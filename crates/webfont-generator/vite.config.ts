import { defineProject } from 'vite-plus';

// Cargo owns incremental compilation; CI caches target directories explicitly.
// Vite+ must not restore competing build outputs or replay Rust test executions.

export default defineProject({
    run: {
        tasks: {
            check: {
                cache: false,
                command: 'cargo clippy -p webfont-generator -- -D warnings && cargo clippy -p webfont-generator --features cli -- -D warnings && cargo fmt --all -- --check',
            },
            test: {
                cache: false,
                command: 'cargo test -p webfont-generator && cargo test -p webfont-generator --features cli',
                dependsOn: ['check'],
            },
            bench: { cache: false, command: 'cargo bench -p webfont-generator --features bench' },
            'test:coverage': {
                cache: false,
                command: 'bash ../../scripts/ci/rust-coverage.sh engine',
            },
            'test:coverage:cli': {
                cache: false,
                command: 'bash ../../scripts/ci/rust-coverage.sh cli',
            },
            'test:doctests': {
                cache: false,
                command: 'node ../../scripts/ci/doctests.mjs',
            },
        },
    },
});
