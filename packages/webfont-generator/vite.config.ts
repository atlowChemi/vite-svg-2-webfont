import { defineProject } from 'vite-plus';

// Cargo owns incremental compilation; CI owns target caching, not Vite+ outputs.

export default defineProject({
    run: {
        tasks: {
            check: {
                cache: false,
                command: 'cargo clippy -p webfont-generator-napi -- -D warnings && cargo fmt --all -- --check',
                dependsOn: ['@atlowchemi/webfont-engine#check'],
            },
            test: {
                cache: false,
                command: 'cargo test -p webfont-generator-napi --lib',
                dependsOn: ['check', '@atlowchemi/webfont-engine#test'],
            },
            'test:browser': {
                cache: false,
                command: 'vp test --root ../.. --project=webfont-generator-browser',
                dependsOn: ['build'],
            },
            'test:coverage': {
                cache: false,
                command: 'bash ../../scripts/ci/rust-coverage.sh adapter',
            },
            'test:coverage:native': {
                cache: false,
                command: 'bash ../../scripts/ci/native-coverage.sh',
            },
            build: {
                cache: false,
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts',
            },
            'binding:regenerate': {
                cache: false,
                command: 'node ../../scripts/regenerate-webfont-binding.mjs',
            },
            bench: {
                cache: false,
                command: 'cargo bench -p webfont-generator --features bench',
            },
            'build:release': {
                cache: false,
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts --release',
                dependsOn: ['test'],
            },
        },
    },
    test: {
        fsModuleCache: true,
        typecheck: { enabled: true },
        benchmark: { include: [] },
        exclude: ['tests/browser/**'],
        include: ['tests/**/*.test.ts'],
        name: 'webfont-generator',
    },
});
