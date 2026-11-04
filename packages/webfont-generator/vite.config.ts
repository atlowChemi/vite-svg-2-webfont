import { defineProject } from 'vitest/config';

export default defineProject({
    test: {
        fsModuleCache: true,
        typecheck: { enabled: true },
        benchmark: { include: [] },
        exclude: ['tests/browser/**'],
        include: ['tests/**/*.test.ts'],
        name: 'webfont-generator',
    },
});
