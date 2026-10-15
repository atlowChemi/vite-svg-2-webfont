import { fileURLToPath } from 'node:url';
import { codecovRollupPlugin } from '@codecov/rollup-plugin';
import { chromium } from 'playwright';
import { build, preview } from 'vite-plus';
import { expect, it } from 'vite-plus/test';

it('loads the emitted worker and WASM from a production build under a URL prefix', async () => {
    const config = {
        configFile: false as const,
        root: fileURLToPath(new URL('./fixture', import.meta.url)),
        base: '/nested/',
        plugins: [
            codecovRollupPlugin({
                enableBundleAnalysis: Boolean(process.env.CODECOV_TOKEN) && process.env.ALLOW_BUNDLE_ANALYSIS === 'true',
                bundleName: 'webfont-browser-generator-bundle',
                uploadToken: process.env.CODECOV_TOKEN,
            }),
        ],
        build: { outDir: fileURLToPath(new URL('../dist/test', import.meta.url)), emptyOutDir: true, target: 'esnext' },
        preview: { host: '127.0.0.1', port: 0 },
    };
    await build(config);
    const server = await preview(config);
    try {
        const browser = await chromium.launch({ headless: true });
        try {
            const page = await browser.newPage();
            await page.goto(server.resolvedUrls!.local[0]!);
            await expect.poll(() => page.locator('body').getAttribute('data-state')).toBe('passed');
        } finally {
            await browser.close();
        }
    } finally {
        await new Promise<void>((resolve, reject) => server.httpServer.close(error => (error ? reject(error) : resolve())));
    }
});
