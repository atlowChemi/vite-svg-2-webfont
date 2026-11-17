import { defineProject, type UserProjectConfigExport } from 'vitest/config';

const config: UserProjectConfigExport = defineProject({
    // Package artifacts have no dotenv inputs; ignored local files must not affect cached builds.
    envDir: false,
    test: {
        include: ['tests/**/*.test.ts'],
        exclude: ['tests/browser.test.ts'],
        benchmark: { include: [] },
        name: 'webfont-browser-build',
        typecheck: { enabled: true },
        testTimeout: 30_000,
        unstubGlobals: true,
    },
});

export default config;
