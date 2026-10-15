import { defineProject } from 'vite-plus';

export default defineProject({
    pack: {
        entry: ['src/index.ts', 'src/worker.ts'],
        format: ['esm'],
        platform: 'browser',
        target: 'es2022',
        dts: true,
        minify: true,
        copy: ['generated/webfont_browser_bg.wasm'],
    },
    run: {
        tasks: {
            build: { cache: false, command: 'node scripts/build.mjs' },
            pack: { command: 'vp pack', dependsOn: ['build'] },
            'pack:tgz': { cache: false, command: 'pnpm pack', dependsOn: ['pack'] },
            'test:coverage': { cache: false, command: 'node scripts/coverage.mjs', dependsOn: ['pack'] },
            publish: {
                cache: false,
                command: 'vp run pack:tgz && vp exec -c "pnpm stage publish atlowchemi-webfont-browser-generator-*.tgz --access public --no-git-checks"',
            },
            check: {
                cache: false,
                command: 'cargo clippy -p webfont-browser --target wasm32-unknown-unknown --locked -- -D warnings && cargo fmt --all -- --check',
                dependsOn: ['build'],
            },
            test: { cache: false, command: 'vp test --root ../.. --project=webfont-wasm-browser --project=webfont-browser-build', dependsOn: ['check', 'pack'] },
        },
    },
    test: {
        include: ['tests/bundle.test.ts', 'tests/worker.test.ts', 'tests/client.test.ts'],
        benchmark: { include: [] },
        name: 'webfont-browser-build',
        testTimeout: 30_000,
        unstubGlobals: true,
    },
});
