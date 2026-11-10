import { defineProject, type UserProjectConfigExport } from 'vitest/config';
import { fileURLToPath } from 'node:url';

const config: UserProjectConfigExport = defineProject({
    // Package artifacts have no dotenv inputs; ignored local files must not affect cached builds.
    envDir: false,
    test: {
        include: ['src/**/*.test.ts'],
        fsModuleCache: true,
        typecheck: { enabled: true, ignoreSourceErrors: true },
        projects: [
            {
                // Exercise each consumer Vite major while Vitest keeps its own Vite version.
                // Matrix selectors are explicit process env, never ignored dotenv files.
                envDir: false,
                resolve: {
                    alias: process.env.VITE_COMPAT_MAJOR ? [{ find: /^vite$/, replacement: fileURLToPath(import.meta.resolve('vite-compat')) }] : [],
                },
                test: {
                    name: 'vite-plugin',
                    include: ['src/**/*.test.ts'],
                    benchmark: { include: [] },
                },
            },
        ],
    },
});

export default config;
