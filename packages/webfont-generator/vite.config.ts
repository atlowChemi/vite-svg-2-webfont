import { defineProject, type UserWorkspaceConfig } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

type TaskDefinition = Partial<Exclude<NonNullable<NonNullable<UserWorkspaceConfig['run']>['tasks']>[string], string | string[]>>;
type TestProjects = NonNullable<NonNullable<UserWorkspaceConfig['test']>['projects']>;

const cargoCache: TaskDefinition = {
    input: [{ auto: true }, '!target/**'],
    output: [{ auto: true }, '!target/**'],
};

const browser = process.argv.includes('--mode=browser');
const browserProject: TestProjects[number] = {
    publicDir: 'tests/browser/fixtures',
    test: {
        browser: {
            enabled: true,
            headless: true,
            instances: [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            provider: playwright(),
            screenshotFailures: false,
        },
        include: ['tests/browser/**/*.test.ts'],
        name: 'webfont-generator-browser',
    },
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
                env: ['UPDATE_SVG_FIXTURES', 'UPDATE_VARIABLE_PROOF_FIXTURE'],
            },
            'test:browser': {
                cache: false,
                command: 'vp test --mode=browser --project=webfont-generator-browser',
            },
            'test:coverage': {
                ...cargoCache,
                command:
                    'cargo llvm-cov clean --workspace && cargo llvm-cov --no-report && cargo llvm-cov --no-report --features cli && cargo llvm-cov --no-report --features napi && cargo llvm-cov report --lcov --output-path rust-lcov.info',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES', 'UPDATE_VARIABLE_PROOF_FIXTURE'],
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
                    exclude: ['tests/browser/**'],
                    benchmark: { include: [] },
                },
            },
            ...(browser ? [browserProject] : []),
        ],
    },
});
