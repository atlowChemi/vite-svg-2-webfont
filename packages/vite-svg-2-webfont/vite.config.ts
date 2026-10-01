import { codecovRollupPlugin } from '@codecov/rollup-plugin';
import { defineProject, type UserProjectConfigExport } from 'vite-plus';
import { fileURLToPath } from 'node:url';

const config: UserProjectConfigExport = defineProject({
    pack: {
        format: ['esm', 'cjs'],
        minify: true,
        fixedExtension: false,
        nodeProtocol: true,
        deps: {
            onlyBundle: false,
            neverBundle: true,
        },
        outputOptions: {
            // Strip annotation comments (including coverage directives) from published bundles.
            comments: { annotation: false },
            exports: 'named',
            // Develop against Vite+, while consumers use the plugin's existing Vite peer.
            paths: { 'vite-plus': 'vite' },
        },
        plugins: [
            codecovRollupPlugin({
                enableBundleAnalysis: Boolean(process.env.CODECOV_TOKEN) && process.env.ALLOW_BUNDLE_ANALYSIS === 'true',
                bundleName: 'vite-svg-2-webfont-bundle',
                uploadToken: process.env.CODECOV_TOKEN,
            }),
        ],
    },
    run: {
        tasks: {
            dev: {
                command: 'vp pack --watch',
            },
            pack: {
                command: 'vp pack',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
                cache: { env: ['CODECOV_TOKEN', 'ALLOW_BUNDLE_ANALYSIS'] },
            },
            'pack:tgz': {
                command: 'pnpm pack',
                dependsOn: ['pack'],
            },
            test: {
                command: 'vp test',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'test:browser': {
                cache: false,
                command: 'vp test --root ../.. --project=vite-svg-webfont-browser',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'test:fixtures:refresh': {
                command: 'node ./scripts/refresh-font-fixtures.ts',
                dependsOn: ['pack'],
            },
            publish: {
                cache: false,
                // Release-only: package external dependencies without compiling the native addon.
                command: 'vp run --ignore-depends-on pack && vp run --ignore-depends-on pack:tgz && vp exec -c "pnpm stage publish vite-svg-2-webfont-*.tgz --no-git-checks"',
            },
        },
    },
    test: {
        include: ['src/**/*.test.ts'],
        fsModuleCache: true,
        typecheck: { enabled: true, ignoreSourceErrors: true },
        projects: [
            {
                // Exercise upstream Vite in the matrix while Vite+ runs the test tooling.
                resolve: {
                    alias: process.env.VITE_COMPAT_MAJOR ? [{ find: /^vite-plus$/, replacement: fileURLToPath(import.meta.resolve('vite-compat')) }] : [],
                },
                test: {
                    name: 'vite-plugin',
                    include: ['src/**/*.test.ts'],
                    benchmark: { include: [] },
                },
            },
        ],
    },
});

export default config;
