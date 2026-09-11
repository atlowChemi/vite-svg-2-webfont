import { defineProject } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';
import { generatedWeightFixture } from './tests/browser/generated-weight-fixture';

export default defineProject({
    plugins: [generatedWeightFixture()],
    publicDir: 'tests/browser/fixtures',
    test: {
        name: 'webfont-generator-browser',
        include: ['tests/browser/**/*.test.ts'],
        typecheck: { enabled: false },
        browser: {
            enabled: true,
            headless: true,
            instances: [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            provider: playwright(),
            screenshotFailures: false,
        },
    },
});
