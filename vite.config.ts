import type { UserConfig } from 'vite';
import { defineConfig } from 'vitest/config';

const config: UserConfig = defineConfig({
    test: {
        coverage: {
            provider: 'istanbul',
            exclude: ['example/**', '.eslint*', 'src/fixtures/**'],
        },
    },
});
export default config;
