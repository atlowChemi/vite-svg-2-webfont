import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const config: UserProjectConfigExport = defineProject({
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
    run: {
        tasks: {
            dev: {
                command: 'vp pack --watch',
            },
            pack: {
                command: 'vp pack',
                dependsOn: ['@atlowchemi/webfont-generator#build'],
            },
            'pack:tgz': {
                command: 'pnpm pack',
                dependsOn: ['pack'],
            },
            test: {
                command: 'vp test',
                dependsOn: ['pack'],
            },
            'test:fixtures:refresh': {
                command: 'node ./scripts/refresh-font-fixtures.ts',
            },
            publish: {
                cache: false,
                command: 'vp exec -c "pnpm publish vite-svg-2-webfont-*.tgz --no-git-checks"',
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
