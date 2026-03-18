import { defineConfig } from 'oxlint';

export default defineConfig({
    plugins: ['eslint', 'typescript', 'unicorn', 'vitest', 'oxc', 'promise', 'vitest'],
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
    globals: {},
    ignorePatterns: ['dist', 'node_modules', 'src/fixtures', 'coverage', 'example/src/webfont/icons.ts', 'example/dist', 'example/vite.config.ts'],
});
