import { defineConfig, type UserConfigExport } from 'vite-plus';

const config: UserConfigExport = defineConfig({
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
        coverage: {
            provider: 'istanbul',
            exclude: ['src/fixtures/**'],
        },
    },
});

export default config;
