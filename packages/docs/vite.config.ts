import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { defineProject, type Plugin, type UserProjectConfigExport, type ViteDevServer } from 'vite-plus';
import { iconCss } from './src/build/iconCss.ts';

// The docs as static pages. Vite builds two things: `src/build/render.tsx` for Node, and the browser assets (one shared
// stylesheet and script, plus an entry per kind of page that needs more). Every page is rendered to HTML that links its
// entries' hashed files. Nothing hydrates: the HTML is the page, and the scripts only add behavior. In dev, the same
// renderer answers each page request.

type Renderer = typeof import('./src/build/render.tsx');
type Assets = Parameters<Renderer['renderDocument']>[1];
const renderer = 'src/build/render.tsx';
const serverDir = '.cache/server';
// Absolute links for crawlers and link previews (`og:url`, `og:image`). CI sets `DOCS_URL` to the site a build is deployed
// to (`next`, or a pull request's preview); everything else is the production site.
const docsUrl = (process.env.DOCS_URL || 'https://fonte.atlow.dev').replace(/\/?$/, '/');

async function loadAll(render: Renderer) {
    const pages = [];
    // One page at a time: markdown-it's fence renderer and the diagram browser are shared.
    // oxlint-disable-next-line no-await-in-loop
    for (const route of render.routes) pages.push((await render.loadPage(route))!);
    return pages;
}

const htmlPath = (route: string) => (route === '/404' ? '404.html' : route.endsWith('/') ? `${route}index.html` : `${route}.html`).replace(/^\//, '');

type SitePlugins = typeof import('./src/build/plugins.ts');
let site: SitePlugins | undefined;
let siteError: unknown;
try {
    site = (await import(import.meta.resolve('./src/build/plugins.ts'))) as SitePlugins;
} catch (error) {
    // Task discovery runs before vite-svg-2-webfont and its native module are built; `dev` and `build` depend on them.
    siteError = error;
}
/** The site plugins, or the reason they didn't load, once a build or the dev server actually needs them. */
const sitePlugins = () => {
    if (!site) throw siteError;
    return site;
};

function fonteDocs(): Plugin {
    let server: ViteDevServer | undefined;
    return {
        name: 'fonte-docs',
        config: () => ({
            environments: {
                client: { build: { manifest: true } },
                // Outside `dist`, which the browser build empties. Like `src/build`, two folders below this one.
                ssr: { build: { outDir: serverDir, rolldownOptions: { input: renderer } } },
            },
            builder: {
                // The pages render first, so the browser build knows every tab label; then the HTML links its assets.
                async buildApp(builder) {
                    sitePlugins();
                    const client = builder.environments.client!;
                    const outDir = client.config.build.outDir;
                    await builder.build(builder.environments.ssr!);
                    const render = (await import(pathToFileURL(join(builder.config.root, serverDir, 'render.js')).href)) as Renderer;
                    const pages = await loadAll(render);
                    await sitePlugins().tellGroupIcons(render.tabLabels(pages));
                    client.config.build.rolldownOptions.input = Object.values(render.clientEntries);
                    await builder.build(client);
                    const manifest = JSON.parse(await readFile(join(outDir, '.vite/manifest.json'), 'utf8')) as Record<string, { file: string; css?: string[] }>;
                    const assets = Object.fromEntries(
                        Object.entries(render.clientEntries).map(([name, entry]) => {
                            const chunk = manifest[entry]!;
                            return [name, { css: (chunk.css ?? []).map(f => `/${f}`), js: render.styleOnly.has(name as keyof Assets) ? [] : [`/${chunk.file}`] }];
                        }),
                    ) as Assets;
                    const files = new Map(pages.map(page => [htmlPath(page.route), render.renderDocument(page, assets, docsUrl)]));
                    const dead = render.deadLinks(pages.map(page => ({ route: page.route, html: files.get(htmlPath(page.route))! })));
                    if (dead.length) throw new Error(`Links to pages or headings that don't exist:\n${dead.join('\n')}`);
                    await Promise.all(
                        [...files, ...render.siteFiles(pages)].map(async ([file, content]) => {
                            await mkdir(dirname(join(outDir, file)), { recursive: true });
                            await writeFile(join(outDir, file), content);
                        }),
                    );
                    await rm(join(outDir, '.vite'), { recursive: true });
                },
            },
        }),
        configureServer(dev) {
            sitePlugins();
            server = dev;
            dev.middlewares.use((req, res, next) => {
                void (async () => {
                    try {
                        const found = await serve(dev, req.url ?? '/');
                        if (!found) return next();
                        res.statusCode = found.status;
                        res.setHeader('content-type', found.type);
                        res.end(found.body);
                    } catch (error) {
                        dev.ssrFixStacktrace(error as Error);
                        next(error);
                    }
                    return undefined;
                })();
            });
            // The Markdown isn't in Vite's module graph: reload the page when any of it changes.
            dev.watcher.add(join(dev.config.root, '**/*.md'));
            dev.watcher.on('change', file => {
                if (file.endsWith('.md')) dev.ws.send({ type: 'full-reload' });
            });
        },
        // Dev: the group icons learn every label from the rendered pages before their CSS is first served.
        async load(id) {
            if (!server || id !== '\0virtual:group-icons.css') return undefined;
            const render = (await server.ssrLoadModule(`/${renderer}`)) as Renderer;
            await sitePlugins().tellGroupIcons(render.tabLabels(await loadAll(render)));
            return undefined;
        },
    };
}

async function serve(server: ViteDevServer, url: string): Promise<{ type: string; body: string; status: number } | undefined> {
    const path = new URL(url, 'http://localhost').pathname.replace(/\.html$/, '');
    const isSiteFile = /^\/(search-index\.json|llms(-full)?\.txt|.+\.md)$/.test(path);
    if (!isSiteFile && (/\.\w+$/.test(path) || /^\/(@|src\/|node_modules\/)/.test(path))) return undefined;
    const render = (await server.ssrLoadModule(`/${renderer}`)) as Renderer;
    if (isSiteFile) {
        // The search index and llms.txt need every page; build them only when asked for.
        const body = render.siteFiles(await loadAll(render)).get(path.slice(1));
        return body === undefined ? undefined : { type: path.endsWith('.json') ? 'application/json' : 'text/plain', body, status: 200 };
    }
    const route = [path, `${path}/`].find(r => render.routes.includes(r));
    const page = (await render.loadPage(route ?? '/404'))!;
    const assets = Object.fromEntries(Object.entries(render.clientEntries).map(([name, entry]) => [name, { css: [] as string[], js: [`/${entry}`] }])) as Assets;
    const html = render.renderDocument(page, assets, 'http://localhost/');
    return { type: 'text/html', body: await server.transformIndexHtml(url, html), status: route ? 200 : 404 };
}

const config: UserProjectConfigExport = defineProject({
    // Holds the engine cache the icon fonts build into, kept across CI runs (see main.yaml and docs.yaml).
    cacheDir: '.cache',
    css: { postcss: iconCss },
    plugins: [fonteDocs(), ...(site?.sitePlugins() ?? [])],
    test: {
        name: 'docs',
        include: ['src/**/*.test.{ts,tsx}'],
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
