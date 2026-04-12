import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const config: UserProjectConfigExport = defineProject({
    run: {
        tasks: {
            'optimize-svg': {
                command: 'svgo -r -f public',
            },
            'social-card': {
                command: 'node ./scripts/generate-social-card.ts',
                dependsOn: ['optimize-svg'],
            },
            dev: {
                cache: false,
                command: 'vitepress dev .',
                dependsOn: ['social-card'],
            },
            build: {
                command: 'vitepress build .',
                dependsOn: ['social-card'],
                env: ['GITHUB_ACTIONS'],
            },
            preview: {
                cache: false,
                command: 'vitepress preview .',
                dependsOn: ['build'],
            },
        },
    },
});

export default config;
