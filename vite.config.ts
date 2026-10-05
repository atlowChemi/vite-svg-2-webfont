import type { UserConfig } from 'vite-plus';
import { defineConfig } from 'vite-plus';

const config: UserConfig = defineConfig({
    fmt: {
        printWidth: 180,
        tabWidth: 4,
        useTabs: false,
        semi: true,
        singleQuote: true,
        quoteProps: 'as-needed',
        jsxSingleQuote: false,
        trailingComma: 'all',
        bracketSpacing: true,
        bracketSameLine: false,
        arrowParens: 'avoid',
        rangeStart: 0,
        filepath: 'none',
        requirePragma: false,
        insertPragma: false,
        proseWrap: 'preserve',
        htmlWhitespaceSensitivity: 'css',
        vueIndentScriptAndStyle: false,
        sortPackageJson: false,
        ignorePatterns: ['dist', 'dist-*', 'webfont', 'node_modules', 'coverage/*', 'tests/fixtures/**', 'packages/webfont-generator/binding{.js,.d.ts}', 'target', '*.hbs'],
    },
    lint: {
        plugins: ['eslint', 'typescript', 'unicorn', 'vitest', 'oxc', 'promise'],
        categories: {
            correctness: 'deny',
            suspicious: 'deny',
            perf: 'warn',
        },
        options: {
            typeAware: true,
            reportUnusedDisableDirectives: 'error',
            typeCheck: true,
        },
        settings: {
            jsdoc: {
                ignorePrivate: false,
                ignoreInternal: false,
                ignoreReplacesDocs: true,
                overrideReplacesDocs: true,
                augmentsExtendsReplacesDocs: false,
                implementsReplacesDocs: false,
                exemptDestructuredRootsFromChecks: false,
                tagNamePreference: {},
            },
            vitest: {
                typecheck: true,
            },
        },
        rules: {
            'vitest/consistent-test-it': ['error', { fn: 'it', withinDescribe: 'it' }],
            'typescript/no-unsafe-type-assertion': 'off',
            'vitest/require-mock-type-parameters': 'off',
            'eslint/no-underscore-dangle': 'off',
        },
        env: {
            builtin: true,
        },
        ignorePatterns: [
            'dist',
            'node_modules',
            'coverage',
            'tests/fixtures/**',
            'packages/example/dist',
            'packages/vite-svg-2-webfont/src/fixtures',
            'packages/webfont-generator/binding{.js,.d.ts}',
        ],
    },
    staged: {
        '*': 'vp check --fix',
        '*.rs': 'cargo fmt --all --',
    },
    run: {
        tasks: {
            'bench:vitest': {
                cache: false,
                command: 'vp test bench --run --reporter=verbose --hideSkippedTests',
                dependsOn: ['@atlowchemi/webfont-generator#build:release'],
            },
            test: {
                command: 'vp test',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'test:compat': {
                command: 'vp test --project=compat',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            coverage: {
                cache: false,
                command: "vp test --coverage --project='!*-browser*'",
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'coverage:scripts': {
                cache: false,
                command:
                    "vp test --run --project=scripts --coverage --coverage.autoAttachSubprocess --coverage.include='scripts/**/*.{ts,mjs}' --coverage.reportsDirectory=coverage/scripts",
            },
        },
    },
    test: {
        ...(process.env.CI_TEST_REPORT
            ? {
                  reporters: [
                      'default',
                      [
                          'junit',
                          {
                              outputFile: process.env.CI_TEST_REPORT,
                              suiteName: process.env.CI_TEST_SUITE ?? 'webfont-tests',
                              // Keep test identities stable across environments; upload flags identify the CI matrix.
                              classnameTemplate: '{displayName}::{filename}',
                              addFileAttribute: true,
                          },
                      ],
                  ],
              }
            : {}),
        fsModuleCache: true,
        coverage: {
            provider: 'v8',
            reporter: ['text', 'lcov'],
            reportOnFailure: true,
            exclude: ['**/*.test-d.ts', 'packages/example/**', 'scripts/fixtures/**', 'packages/vite-svg-2-webfont/src/fixtures/**', 'packages/webfont-generator/binding.*'],
        },
        projects: [
            'packages/!(example)/vite.config.ts',
            'packages/!(example)/vite.browser.config.ts',
            {
                test: {
                    name: 'scripts',
                    include: ['scripts/**/*.test.ts'],
                    // Benchmark discovery has its own default include, independent of test.include.
                    benchmark: { include: [] },
                    testTimeout: 30_000,
                    hookTimeout: 30_000,
                },
            },
            {
                test: {
                    name: 'compat',
                    include: ['tests/**/*.compat.test.ts'],
                    benchmark: { include: ['tests/**/*.bench.ts'] },
                },
            },
        ],
    },
});
export default config;
