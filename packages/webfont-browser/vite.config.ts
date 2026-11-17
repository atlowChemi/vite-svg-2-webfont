import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const config: UserProjectConfigExport = defineProject({
    // Package artifacts have no dotenv inputs; ignored local files must not affect cached builds.
    envDir: false,
    pack: {
        entry: ['src/index.ts', 'src/worker.ts'],
        format: ['esm'],
        platform: 'browser',
        target: 'es2022',
        dts: true,
        minify: true,
        copy: ['generated/webfont_browser_bg.wasm'],
    },
    test: {
        include: ['tests/**/*.test.ts'],
        exclude: ['tests/browser.test.ts'],
        benchmark: { include: [] },
        name: 'webfont-browser-build',
        typecheck: { enabled: true },
        testTimeout: 30_000,
        unstubGlobals: true,
    },
});

export default config;
