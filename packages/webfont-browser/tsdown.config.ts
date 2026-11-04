import { defineConfig } from 'tsdown';

export default defineConfig({
    // No dotenv files or ambient TSDOWN_* variables are build inputs.
    envPrefix: [],
    entry: ['src/index.ts', 'src/worker.ts'],
    format: ['esm'],
    platform: 'browser',
    target: 'es2022',
    dts: true,
    minify: true,
    copy: ['generated/webfont_browser_bg.wasm'],
});
