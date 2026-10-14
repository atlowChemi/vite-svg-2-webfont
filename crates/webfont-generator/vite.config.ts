import { defineProject, type UserWorkspaceConfig } from 'vite-plus';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;
type TaskCache = Exclude<NonNullable<TaskDefinition['cache']>, boolean>;

const cargoCache: TaskCache = {
    input: [{ auto: true }, { pattern: 'Cargo.{toml,lock}', base: 'workspace' }, { pattern: '!target/**', base: 'workspace' }],
    output: [{ auto: true }, { pattern: '!target/**', base: 'workspace' }],
};

const isolatedFeatures = ['', 'async', 'fs', 'in_memory', 'in_memory,async', 'cli', 'bench'];
const featureChecks = isolatedFeatures.map(features => `cargo clippy -p webfont-generator --no-default-features${features ? ` --features ${features}` : ''} -- -D warnings`);
const featureTests = ['fs', 'in_memory', 'in_memory,async', 'cli'].map(features => `cargo test -p webfont-generator --no-default-features --features ${features}`);

export default defineProject({
    run: {
        tasks: {
            check: {
                cache: cargoCache,
                command: [
                    'cargo clippy -p webfont-generator -- -D warnings',
                    'cargo clippy -p webfont-generator --features cli -- -D warnings',
                    ...featureChecks,
                    'cargo fmt --all -- --check',
                ].join(' && '),
            },
            test: {
                cache: { ...cargoCache, env: ['UPDATE_SVG_FIXTURES'] },
                command: 'cargo test -p webfont-generator && cargo test -p webfont-generator --features cli',
                dependsOn: ['check', 'test:features'],
            },
            'test:features': {
                cache: { ...cargoCache, env: ['UPDATE_SVG_FIXTURES'] },
                command: featureTests.join(' && '),
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
