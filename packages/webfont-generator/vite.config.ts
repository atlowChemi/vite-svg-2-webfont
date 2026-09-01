import { defineProject } from 'vite-plus';

const cargoCache = {
    input: [{ auto: true } as const, '!target/**'],
    output: [{ auto: true } as const, '!target/**'],
};

export default defineProject({
    run: {
        tasks: {
            check: {
                ...cargoCache,
                command: 'cargo clippy -- -D warnings && cargo clippy --features cli -- -D warnings && cargo clippy --features napi -- -D warnings && cargo fmt -- --check',
            },
            test: {
                ...cargoCache,
                command: 'cargo t && cargo t --features cli && cargo t --features napi',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES'],
            },
            'test:coverage': {
                ...cargoCache,
                command:
                    'cargo llvm-cov clean --workspace && cargo llvm-cov --no-report && cargo llvm-cov --no-report --features cli && cargo llvm-cov --no-report --features napi && cargo llvm-cov report --lcov --output-path rust-lcov.info',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES'],
            },
            build: {
                ...cargoCache,
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts -- --features napi',
            },
            bench: {
                cache: false,
                command: 'cargo bench --features bench',
            },
            'build:release': {
                ...cargoCache,
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts --release -- --features napi',
                dependsOn: ['test'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        typecheck: { enabled: true },
        projects: [
            {
                test: {
                    name: 'webfont-generator',
                    include: ['tests/**/*.test.ts'],
                    benchmark: { include: [] },
                },
            },
        ],
    },
});
