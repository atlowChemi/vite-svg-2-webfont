import type { UserConfig } from 'vite';
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
                dependsOn: ['@atlowchemi/webfont-generator#build', 'vite-svg-2-webfont#pack'],
            },
            'test:compat': {
                command: 'vp test --project=compat',
                dependsOn: ['@atlowchemi/webfont-generator#build', 'vite-svg-2-webfont#pack'],
            },
            coverage: {
                command: 'vp test --coverage',
                dependsOn: ['@atlowchemi/webfont-generator#build', 'vite-svg-2-webfont#pack'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        coverage: {
            provider: 'v8',
            exclude: ['packages/example/**', 'packages/vite-svg-2-webfont/src/fixtures/**', 'packages/webfont-generator/binding.*'],
        },
        projects: [
            'packages/!(example)/vite.config.ts',
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
