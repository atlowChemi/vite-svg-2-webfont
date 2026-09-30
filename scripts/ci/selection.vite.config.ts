import { defineConfig, type UserConfig } from 'vite-plus';

const config: UserConfig = defineConfig({
    test: {
        include: ['tests/affected-selection.compat.test.ts', 'tests/ci/*.test.ts'],
        testTimeout: 30_000,
        hookTimeout: 30_000,
        reporters: ['default', ['junit', { outputFile: 'test-results/selection-scenarios.xml', suiteName: 'affected-selection-linux-x64', addFileAttribute: true }]],
    },
});
export default config;
