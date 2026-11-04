import { defineProject } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';
import { builtFamilyFixture } from './tests/browser/built-family-fixture';

export default defineProject({
    // TODO [engine:node@>=24, 2027-04-30]: Remove this lowering target once Node 22 support ends.
    oxc: { target: 'es2022' },
    plugins: [builtFamilyFixture()],
    test: {
        name: 'vite-svg-webfont-browser',
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
