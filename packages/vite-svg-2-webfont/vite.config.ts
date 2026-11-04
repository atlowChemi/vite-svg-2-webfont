import { codecovRollupPlugin } from '@codecov/rollup-plugin';
import { defineProject, type UserProjectConfigExport } from 'vite-plus';
import { fileURLToPath } from 'node:url';

const config: UserProjectConfigExport = defineProject({
    // Package artifacts have no dotenv inputs; ignored local files must not affect cached builds.
    envDir: false,
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
            {
                name: 'virtual-declarations',
                generateBundle() {
                    this.emitFile({ type: 'asset', fileName: 'virtuals.d.ts', source: '/// <reference path="./generated/index.d.ts" />\n' });
                    this.emitFile({ type: 'asset', fileName: 'generated/index.d.ts', source: '// Run Vite to generate precise icon module declarations.\n' });
                },
            },
            codecovRollupPlugin({
                enableBundleAnalysis: Boolean(process.env.CODECOV_TOKEN) && process.env.ALLOW_BUNDLE_ANALYSIS === 'true',
                bundleName: 'vite-svg-2-webfont-bundle',
                uploadToken: process.env.CODECOV_TOKEN,
            }),
        ],
    },
    test: {
        include: ['src/**/*.test.ts'],
        fsModuleCache: true,
        typecheck: { enabled: true, ignoreSourceErrors: true },
        projects: [
            {
                // Exercise upstream Vite in the matrix while Vite+ runs the test tooling.
                // Matrix selectors are explicit process env, never ignored dotenv files.
                envDir: false,
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
