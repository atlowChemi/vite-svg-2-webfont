import { defineProject } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

export default defineProject({
    test: {
        name: 'webfont-wasm-browser',
        include: ['tests/browser.test.ts'],
        typecheck: { enabled: false },
        testTimeout: 30_000,
        browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            screenshotFailures: false,
        },
    },
});
