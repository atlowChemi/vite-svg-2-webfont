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
        ignorePatterns: ['dist', 'node_modules', 'src/fixtures', 'coverage', 'example/src/webfont/icons.ts', 'example/dist', 'example/vite.config.ts'],
    },
    pack: {
        format: ['esm', 'cjs'],
        minify: true,
        fixedExtension: false,
        nodeProtocol: true,
        deps: {
            onlyBundle: false,
            skipNodeModulesBundle: true,
        },
        outputOptions: {
            exports: 'named',
        },
    },
    staged: {
        '*': 'vp check --fix',
    },
    run: {
        tasks: {
            dev: {
                command: 'vp pack --watch',
            },
            pack: {
                command: 'vp pack',
            },
            coverage: {
                command: 'vp test --coverage',
                dependsOn: ['pack'],
            },
            test: {
                command: 'vp test',
                dependsOn: ['pack'],
            },
            publish: {
                cache: false,
                command: 'pnpm publish',
                dependsOn: ['pack'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        coverage: {
            provider: 'istanbul',
            exclude: ['example/**', '.eslint*', 'src/fixtures/**'],
        },
    },
});
export default config;
