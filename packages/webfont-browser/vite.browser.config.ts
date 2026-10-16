import { defineProject, type UserProjectConfigExport } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

const config: UserProjectConfigExport = defineProject({
    test: {
        name: 'webfont-wasm-browser',
        include: ['tests/browser.test.ts'],
        typecheck: { enabled: false },
        testTimeout: 30_000,
        browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: process.env.WEBFONT_COVERAGE ? [{ browser: 'chromium' }] : [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            screenshotFailures: false,
        },
    },
});

export default config;
