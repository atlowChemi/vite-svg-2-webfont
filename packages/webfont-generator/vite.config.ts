import { defineProject, type UserWorkspaceConfig } from 'vite-plus';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;

const cargoCache: TaskDefinition = {
    input: [
        { auto: true },
        { pattern: 'Cargo.{toml,lock}', base: 'workspace' },
        { pattern: 'crates/webfont-generator/**', base: 'workspace' },
        { pattern: '!target/**', base: 'workspace' },
        '!target/**',
    ],
    output: [{ auto: true }, { pattern: '!target/**', base: 'workspace' }, '!target/**'],
};

export default defineProject({
    run: {
        tasks: {
            check: {
                ...cargoCache,
                command: 'cargo clippy -p webfont-generator-napi -- -D warnings && cargo fmt --all -- --check',
                dependsOn: ['@atlowchemi/webfont-engine#check'],
            },
            test: {
                ...cargoCache,
                command: 'cargo test -p webfont-generator-napi --lib',
                dependsOn: ['check', '@atlowchemi/webfont-engine#test'],
                env: ['UPDATE_SVG_FIXTURES', 'UPDATE_VARIABLE_PROOF_FIXTURE'],
            },
            'test:browser': {
                cache: false,
                command: 'vp test --root ../.. --project=webfont-generator-browser',
                dependsOn: ['build'],
            },
            'test:coverage': {
                ...cargoCache,
                command:
                    'cargo llvm-cov clean --workspace && cargo llvm-cov -p webfont-generator --no-report && cargo llvm-cov -p webfont-generator --no-report --features cli && cargo llvm-cov -p webfont-generator-napi --lib --no-report && cargo llvm-cov report --lcov --output-path rust-lcov.info',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES', 'UPDATE_VARIABLE_PROOF_FIXTURE'],
            },
            build: {
                ...cargoCache,
                command: 'node ../../scripts/sync-webfont-templates.mjs && napi build --platform --esm --js binding.js --dts binding.d.ts',
            },
            bench: {
                cache: false,
                command: 'cargo bench -p webfont-generator --features bench',
            },
            'build:release': {
                ...cargoCache,
                command: 'node ../../scripts/sync-webfont-templates.mjs && napi build --platform --esm --js binding.js --dts binding.d.ts --release',
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
