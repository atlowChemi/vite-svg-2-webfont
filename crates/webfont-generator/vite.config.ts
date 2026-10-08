import { defineProject, type UserWorkspaceConfig } from 'vite-plus';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;
type TaskCache = Exclude<NonNullable<TaskDefinition['cache']>, boolean>;

const cargoCache: TaskCache = {
    input: [{ auto: true }, { pattern: 'Cargo.{toml,lock}', base: 'workspace' }, { pattern: '!target/**', base: 'workspace' }],
    output: [{ auto: true }, { pattern: '!target/**', base: 'workspace' }],
};

export default defineProject({
    run: {
        tasks: {
            check: {
                cache: cargoCache,
                command: 'cargo clippy -p webfont-generator -- -D warnings && cargo clippy -p webfont-generator --features cli -- -D warnings && cargo fmt --all -- --check',
            },
            test: {
                cache: { ...cargoCache, env: ['UPDATE_SVG_FIXTURES'] },
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
            'test:color-proof': {
                cache: false,
                command: 'cargo test -p webfont-generator color_proof -- --nocapture',
            },
        },
    },
});
