import { defineProject, type UserProjectConfigExport } from 'vite-plus';
import { keepCssModules, scopedName } from './config/cssModules.ts';
import { fonteDocs } from './config/staticSite.ts';

// The docs as static pages: see `config/staticSite.ts` for the build and the dev server.

// The site's own plugins build fonts with vite-svg-2-webfont, so they load only once it and its native module exist.
let site: typeof import('./config/plugins/index.ts') | undefined;
try {
    site = await import(import.meta.resolve('./config/plugins/index.ts'));
} catch {
    // Task discovery runs before the plugin and native module are built.
}

const config: UserProjectConfigExport = defineProject({
    // Holds the engine cache the icon fonts build into, kept across CI runs (see main.yaml and docs.yaml).
    cacheDir: '.cache',
    css: { modules: { generateScopedName: scopedName } },
    plugins: [fonteDocs(site), keepCssModules(), ...(site?.sitePlugins() ?? [])],
    test: {
        name: 'docs',
        include: ['config/**/*.test.ts', 'src/**/*.test.{ts,tsx}'],
    },
    run: {
        tasks: {
            'optimize-svg': {
                command: 'svgo -r -f assets/svg -o public',
            },
            // Regenerates the committed logo and wordmark SVGs in assets/svg; run by hand after brand changes.
            brand: {
                command: 'node ./scripts/generate-brand.ts',
            },
            'social-card': {
                command: 'node ./scripts/generate-social-card.ts',
                dependsOn: ['optimize-svg'],
            },
            dev: {
                cache: false,
                command: 'vp dev',
                dependsOn: ['social-card', 'vite-svg-2-webfont#pack', '@atlowchemi/webfont-browser-generator#pack'],
            },
            build: {
                command: 'vp build',
                cache: { env: ['DOCS_URL'] },
                dependsOn: ['social-card', 'vite-svg-2-webfont#pack', '@atlowchemi/webfont-browser-generator#pack'],
            },
            preview: {
                cache: false,
                command: 'vp preview',
                dependsOn: ['build'],
            },
        },
    },
});

export default config;
