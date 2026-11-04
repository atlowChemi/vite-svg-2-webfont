import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const config: UserProjectConfigExport = defineProject({
    test: {
        name: 'docs',
        include: ['.vitepress/**/*.test.ts'],
    },
});

export default config;
