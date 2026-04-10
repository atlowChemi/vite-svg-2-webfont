import { defineConfig } from 'vite-plus';

export default defineConfig({
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
            test: {
                command: 'vp test',
                dependsOn: ['pack'],
            },
            'test:fixtures:refresh': {
                command: 'node ./scripts/refresh-font-fixtures.ts',
            },
            publish: {
                cache: false,
                command: 'pnpm publish --no-git-checks',
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
            exclude: ['src/fixtures/**'],
        },
    },
});
