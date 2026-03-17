import type { UserConfig } from 'vite';
import { defineConfig } from 'vitest/config';

const config: UserConfig = defineConfig({
    test: {
        experimental: {
            fsModuleCache: true,
        },
        coverage: {
            provider: 'istanbul',
            exclude: ['example/**', '.eslint*', 'src/fixtures/**'],
        },
    },
});
export default config;
