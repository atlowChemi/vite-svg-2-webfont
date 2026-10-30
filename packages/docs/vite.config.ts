import { defineProject, type UserProjectConfigExport } from 'vite-plus';

const config: UserProjectConfigExport = defineProject({
    test: {
        name: 'docs',
        include: ['.vitepress/**/*.test.ts'],
    },
    run: {
        tasks: {
            'optimize-svg': {
                command: 'svgo -r -f assets/svg -o public',
            },
            'social-card': {
                command: 'node ./scripts/generate-social-card.ts',
                dependsOn: ['optimize-svg'],
            },
            dev: {
                cache: false,
                command: 'vitepress dev .',
                dependsOn: ['social-card', 'vite-svg-2-webfont#pack', '@atlowchemi/webfont-browser-generator#pack'],
            },
            build: {
                command: 'vitepress build .',
                dependsOn: ['social-card', 'vite-svg-2-webfont#pack', '@atlowchemi/webfont-browser-generator#pack'],
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
