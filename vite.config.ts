import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        coverage: {
            provider: 'istanbul',
            exclude: ['example/**', '.eslint*', 'src/fixtures/**'],
        },
    },
});
