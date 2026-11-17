import { codecovRollupPlugin } from '@codecov/rollup-plugin';
import { defineConfig } from 'tsdown';

export default defineConfig({
    // No dotenv files or ambient TSDOWN_* variables are build inputs.
    envPrefix: [],
    entry: ['src/index.ts'],
    dts: true,
    format: ['esm', 'cjs'],
    minify: true,
    fixedExtension: false,
    nodeProtocol: true,
    deps: { onlyBundle: false, neverBundle: true },
    outputOptions: {
        comments: { annotation: false },
        exports: 'named',
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
});
