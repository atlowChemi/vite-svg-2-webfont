import { defineConfig } from 'vite-plus';

export default defineConfig({
    run: {
        tasks: {
            'optimize-svg': {
                command: './node_modules/.bin/svgo -r -f public',
            },
            'social-card': {
                command: 'node ./scripts/generate-social-card.ts',
                dependsOn: ['optimize-svg'],
            },
            dev: {
                cache: false,
                command: './node_modules/.bin/vitepress dev .',
                dependsOn: ['social-card'],
            },
            build: {
                command: './node_modules/.bin/vitepress build .',
                dependsOn: ['social-card'],
                env: ['GITHUB_ACTIONS'],
            },
            preview: {
                cache: false,
                command: './node_modules/.bin/vitepress preview .',
                dependsOn: ['build'],
            },
        },
    },
});
