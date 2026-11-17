import { defineProject, type UserProjectConfigExport } from 'vitest/config';
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
    // Ignored local dotenv files must not affect cached site builds.
    envDir: false,
    // Tests on Node.js 22, which can't parse `using`, get lowered syntax; builds run on Node.js 26 and keep it as written.
    // TODO [engine:node@>=24, 2027-04-30]: Remove this lowering target once Node 22 support ends.
    ...(process.env.VITEST && Number(process.versions.node.split('.')[0]) < 24 ? { oxc: { target: 'es2022' } } : {}),
    // Holds the engine cache the icon fonts build into, kept across CI runs (see main.yaml and docs.yaml).
    cacheDir: '.cache',
    css: { modules: { generateScopedName: scopedName } },
    plugins: [fonteDocs(site), keepCssModules(), ...(site?.sitePlugins() ?? [])],
    test: {
        name: 'docs',
        include: ['config/**/*.test.ts', 'src/**/*.test.{ts,tsx}'],
        setupFiles: ['config/testSetup.ts'],
    },
});

export default config;
