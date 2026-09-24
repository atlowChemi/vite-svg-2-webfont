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
        '*.rs': 'cargo fmt --manifest-path packages/webfont-generator/Cargo.toml --',
    },
    run: {
        tasks: {
            test: {
                command: 'vp test',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'test:compat': {
                command: 'vp test --project=compat',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            coverage: {
                command: "vp test --coverage --project='!*-browser*'",
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
        },
    },
    test: {
        // Vitest v4 compatibility: preserve mock call history.
        // Remove after tests no longer rely on calls from setup or earlier tests.
        // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
        // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
        clearMocks: false,
        // Vitest v4 compatibility: keep separate Vite servers for inline projects.
        // Remove when plugins and config hooks can run once for shared projects.
        // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
        // https://vitest.dev/guide/migration/#inline-projects-share-the-vite-server-by-default
        sharedViteServer: false,
        fsModuleCache: true,
        coverage: {
            provider: 'v8',
            exclude: ['packages/example/**', 'packages/vite-svg-2-webfont/src/fixtures/**', 'packages/webfont-generator/binding.*'],
        },
        projects: [
            'packages/!(example)/vite.config.ts',
            'packages/!(example)/vite.browser.config.ts',
            {
                // Vitest v4 compatibility: keep this inline project independent of the root config.
                // Remove to inherit root options, including plugins and setup files.
                // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
                // https://vitest.dev/guide/migration/#inline-projects-inherit-the-root-config-by-default
                extends: false,
                test: {
                    // Vitest v4 compatibility: preserve mock call history.
                    // Remove after tests no longer rely on calls from setup or earlier tests.
                    // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
                    // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
                    clearMocks: false,
                    name: 'compat',
                    include: ['tests/**/*.compat.test.ts'],
                    benchmark: { include: ['tests/**/*.bench.ts'] },
                },
            },
        ],
    },
});
export default config;
