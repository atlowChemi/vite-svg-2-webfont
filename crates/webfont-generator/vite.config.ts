import { defineProject, type UserWorkspaceConfig } from 'vite-plus';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;

const cargoCache: TaskDefinition = {
    input: [{ auto: true }, { pattern: 'Cargo.{toml,lock}', base: 'workspace' }, { pattern: '!target/**', base: 'workspace' }],
    output: [{ auto: true }, { pattern: '!target/**', base: 'workspace' }],
};

export default defineProject({
    run: {
        tasks: {
            check: {
                ...cargoCache,
                command: 'cargo clippy -p webfont-generator -- -D warnings && cargo clippy -p webfont-generator --features cli -- -D warnings && cargo fmt --all -- --check',
            },
            test: {
                ...cargoCache,
                command: 'cargo test -p webfont-generator && cargo test -p webfont-generator --features cli',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES', 'UPDATE_VARIABLE_PROOF_FIXTURE'],
            },
            bench: { cache: false, command: 'cargo bench -p webfont-generator --features bench' },
            'test:coverage': {
                ...cargoCache,
                command:
                    'cargo llvm-cov clean --workspace && cargo llvm-cov -p webfont-generator --no-report && cargo llvm-cov -p webfont-generator --no-report --features cli && cargo llvm-cov report -p webfont-generator --lcov --output-path rust-lcov.info',
                dependsOn: ['check'],
            },
        },
    },
});
