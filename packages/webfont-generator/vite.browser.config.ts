import { defineProject } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';
import { generatedWeightFixture } from './tests/browser/generated-weight-fixture';

export default defineProject({
    plugins: [generatedWeightFixture()],
    publicDir: 'tests/browser/fixtures',
    test: {
        // Vitest v4 compatibility: preserve mock call history.
        // Remove after tests no longer rely on calls from setup or earlier tests.
        // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
        // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
        clearMocks: false,
        name: 'webfont-generator-browser',
        include: ['tests/browser/**/*.test.ts'],
        typecheck: { enabled: false },
        browser: {
            locators: {
                // Vitest v4 compatibility: keep partial, case-insensitive locator matching.
                // Remove after updating locators for full, case-sensitive matches.
                // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
                // https://vitest.dev/guide/migration/#locators-are-strict-by-default
                exact: false,
            },
            enabled: true,
            headless: true,
            instances: [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            provider: playwright(),
            screenshotFailures: false,
        },
    },
});
