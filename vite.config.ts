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
        ignorePatterns: ['dist', 'dist-*', 'webfont', 'node_modules', 'coverage/*'],
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
        ignorePatterns: ['dist', 'node_modules', 'packages/vite-svg-2-webfont/src/fixtures', 'coverage', 'packages/example/src/webfont/icons.ts', 'packages/example/dist'],
    },
    staged: {
        '*': 'vp check --fix',
    },
    run: {
        tasks: {
            test: {
                command: 'vp test',
                dependsOn: ['vite-svg-2-webfont#pack'],
            },
            coverage: {
                command: 'vp test --coverage',
                dependsOn: ['vite-svg-2-webfont#pack'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        coverage: {
            provider: 'v8',
            exclude: ['packages/example/**', 'packages/vite-svg-2-webfont/src/fixtures/**'],
        },
        projects: ['packages/vite-svg-2-webfont/vite.config.ts'],
    },
});
export default config;
