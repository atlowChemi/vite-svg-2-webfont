import { codecovRollupPlugin } from '@codecov/rollup-plugin';
import { defineProject, type UserProjectConfigExport } from 'vite-plus';

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
            exports: 'named',
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
                env: ['CODECOV_TOKEN', 'ALLOW_BUNDLE_ANALYSIS']
            },
            'pack:tgz': {
                command: 'pnpm pack',
                dependsOn: ['pack'],
            },
            test: {
                command: 'vp test',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'test:fixtures:refresh': {
                command: 'node ./scripts/refresh-font-fixtures.ts',
                dependsOn: ['pack'],
            },
            publish: {
                cache: false,
                command: 'vp exec -c "pnpm stage publish vite-svg-2-webfont-*.tgz --no-git-checks"',
                dependsOn: ['pack:tgz'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        projects: [
            {
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
