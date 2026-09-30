import { defineProject, type UserWorkspaceConfig } from 'vite-plus';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;
type TaskCache = Exclude<NonNullable<TaskDefinition['cache']>, boolean>;

const cargoCache: TaskCache = {
    input: [
        { auto: true },
        { pattern: 'Cargo.{toml,lock}', base: 'workspace' },
        { pattern: 'crates/webfont-generator/**', base: 'workspace' },
        { pattern: '!target/**', base: 'workspace' },
        '!target/**',
    ],
    output: [{ auto: true }, { pattern: '!target/**', base: 'workspace' }, '!target/**'],
};

const napiBuildCache: TaskCache = {
    ...cargoCache,
    input: [
        ...cargoCache.input!,
        // NAPI's atomic output transactions are intermediates, not build inputs.
        '!.*.tmp',
        '!.napi-rs-filesystem-*',
        '!.napi-rs-filesystem-*/**',
        '!*.node',
        '!binding.js',
        '!binding.d.ts',
        { pattern: '!packages/.webfont-generator.napi-stage-*/**', base: 'workspace' },
    ],
    output: ['*.node', 'binding.js', 'binding.d.ts'],
};

export default defineProject({
    run: {
        tasks: {
            check: {
                cache: cargoCache,
                command: 'cargo clippy -p webfont-generator-napi -- -D warnings && cargo fmt --all -- --check',
                dependsOn: ['@atlowchemi/webfont-engine#check'],
            },
            test: {
                cache: { ...cargoCache, env: ['UPDATE_SVG_FIXTURES'] },
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
                cache: napiBuildCache,
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
                cache: napiBuildCache,
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
