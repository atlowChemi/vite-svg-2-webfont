import { defineProject, type UserProjectConfigExport } from 'vitest/config';

const config: UserProjectConfigExport = defineProject({
    test: {
        name: 'docs',
        include: ['.vitepress/**/*.test.ts'],
    },
});

export default config;
