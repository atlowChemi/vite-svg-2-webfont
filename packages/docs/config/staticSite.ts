import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isCSSRequest, isFileLoadingAllowed, isRunnableDevEnvironment, type BuildEnvironment, type Connect, type Plugin, type ViteDevServer } from 'vite-plus';
import type { Page } from '../src/build/render.tsx';
type SitePlugins = typeof import('./plugins/index.ts');

// The docs as static pages. Vite builds two things: `src/build/render.tsx` for Node, and the browser assets (one shared
// stylesheet and script, plus an entry per kind of page that needs more). Every page is rendered to HTML that links its
// entries' hashed files. Nothing hydrates: the HTML is the page, and the scripts only add behavior. In dev, the same
// renderer answers each page request.

// TODO [@types/node>=26]: Only needed as long as we don't have Node 26 types.
declare global {
    interface IteratorConstructor {
        concat<T extends readonly Iterable<unknown>[]>(...iterables: T): IteratorObject<T[number] extends Iterable<infer U> ? U : never>;
    }
}
type Renderer = typeof import('../src/build/render.tsx');
type Assets = Parameters<Renderer['renderDocument']>[1];
const renderer = 'src/build/render.tsx';
const serverDir = '.cache/server';
// Absolute links for crawlers and link previews (`og:url`, `og:image`). CI sets `DOCS_URL` to the site a build is deployed
// to (`next`, or a pull request's preview); everything else is the production site.
const docsUrl = (process.env.DOCS_URL || 'https://fonte.atlow.dev').replace(/\/?$/, '/');

async function loadAll(render: Renderer): Promise<Page[]> {
    // Held across the batch, so the pages' diagrams share one Chromium instead of each page launching and closing its own.
    await using _diagrams = render.diagramBrowser.use();
    // Pages render at the same time: each keeps its own state in markdown-it's `env`, and the order follows `routes`.
    return await Promise.all(render.routes.map(async route => (await render.loadPage(route))!));
}

type Manifest = Record<string, { file: string; css?: string[]; imports?: string[] }>;

/**
 * An entry's stylesheets, including those of the chunks it imports: CSS modules that several entries import (the
 * wordmark's, for one) land in a shared chunk of their own. Imported chunks' styles come first, as they load first.
 */
function stylesheets(manifest: Manifest, key: string, seen = new Set<string>(), result: string[] = []): string[] {
    if (seen.has(key)) return result;
    seen.add(key);

    const chunk = manifest[key]!;

    for (const imported of chunk.imports ?? []) {
        stylesheets(manifest, imported, seen, result);
    }

    if (chunk.css) Array.prototype.push.apply(result, chunk.css);

    return result;
}

/**
 * Parses a `_headers` file into rules, each consisting of a path pattern
 * (where `*` matches any sequence) and its indented `Name: value` headers.
 */
function headerRules(text: string): { pattern: RegExp; headers: [string, string][] }[] {
    const rules: { pattern: RegExp; headers: [string, string][] }[] = [];
    for (const line of text.split('\n')) {
        const trimmed = line.trim();
        // Skip empty lines and comments.
        if (!trimmed || trimmed.startsWith('#')) continue;

        // Unindented lines define a new path pattern.
        if (line[0] === trimmed[0]) {
            const source = trimmed
                // Escape regex metacharacters, preserving '*' as a wildcard.
                .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
                // Convert wildcards to regex patterns matching any sequence.
                .replaceAll('*', '.*');

            // Anchor the regex to match the entire path.
            rules.push({ pattern: new RegExp(`^${source}$`), headers: [] });
            continue;
        }

        // Indented lines define headers for the preceding path pattern.
        const [name, value] = trimmed.split(/:\s*(.*)/s, 2);
        rules.at(-1)?.headers.push([name!, value ?? '']);
    }
    return rules;
}

const htmlPath = (route: string) => {
    if (route === '/404') return '404.html';
    if (route[0] === '/') route = route.slice(1);
    if (!route || route.endsWith('/')) return `${route}index.html`;
    return `${route}.html`;
};

/** `site`: the site's own plugins, absent while `vp` discovers tasks (see `vite.config.ts`). */
export function fonteDocs(site: SitePlugins | undefined): Plugin {
    return {
        name: 'fonte-docs',
        config: () => ({
            // There's no `index.html` for Vite to scan, so name the browser entries: their dependencies are then
            // optimized at startup, not on the first visit, which would reload the page once they're done.
            optimizeDeps: { entries: ['src/client/*.{ts,tsx}'] },
            environments: {
                client: { build: { manifest: true } },
                // Outside `dist`, which the browser build empties. Like `src/build`, two folders below this one.
                ssr: { build: { outDir: serverDir, rolldownOptions: { input: renderer } } },
            },
            builder: {
                // The pages render first, so the browser build knows every tab label; then the HTML links its assets.
                async buildApp(builder) {
                    const { client, ssr } = builder.environments as { client: BuildEnvironment; ssr: BuildEnvironment };
                    const { outDir } = client.config.build;

                    // Start by building SSR: the `render.tsx` and all it's module tree.
                    await builder.build(ssr);

                    // Import the built render module
                    const { href: transpiledRenderPath } = pathToFileURL(join(builder.config.root, serverDir, 'render.js'));
                    const render = (await import(transpiledRenderPath)) as Renderer;

                    // Render every route's Markdown to HTML.
                    const pages = await loadAll(render);
                    // Now we have the pages, define what labels we have, so the file icon plugin only adds those icons.
                    site?.setFileIconLabels(render.tabLabels(pages));
                    // Now build the client's app - create the modules each type of page requires
                    client.config.build.rolldownOptions.input = render.clientEntries.values().toArray();
                    await builder.build(client);

                    // Parse vite client's built manifest to get the transpiled asset per input page.
                    const { href: manifestPath } = pathToFileURL(join(outDir, '.vite', 'manifest.json'));
                    const { default: manifest } = (await import(manifestPath, { with: { type: 'json' } })) as { default: Manifest };
                    const assets = Object.fromEntries(
                        render.clientEntries.entries().map(([name, entry]) => {
                            const chunk = manifest[entry]!;
                            return [name, { css: stylesheets(manifest, entry).map(f => `/${f}`), js: render.styleOnly.has(name) ? [] : [`/${chunk.file}`] }];
                        }),
                    ) as Assets;

                    // Build the final HTML for each page, with all the assets it's route needs
                    const files = new Map(pages.map(page => [htmlPath(page.route), render.renderDocument(page, assets, docsUrl)]));
                    // After we built all pages, ensure no page includes a relative link which leads nowhere.
                    const dead = render.deadLinks(pages.map(page => ({ route: page.route, html: files.get(htmlPath(page.route))! })));
                    if (dead.length) throw new Error(`Links to pages or headings that don't exist:\n${dead.join('\n')}`);

                    // Build the final directory structure of the output site.
                    await Promise.all(
                        Iterator.concat(files, render.siteFiles(pages)).map(async ([file, content]) => {
                            await mkdir(dirname(join(outDir, file)), { recursive: true });
                            await writeFile(join(outDir, file), content);
                        }),
                    );
                    // The policy allows the inline scripts by hash, so the build adds it to `public/_headers`. Written from
                    // the source every time, so the rule is there once however the output folder was left.
                    const headers = await readFile(join(client.config.publicDir, '_headers'), 'utf8');
                    await writeFile(
                        join(outDir, '_headers'),
                        `${headers}\n# Every response, the Playground's worker included.\n/*\n  Content-Security-Policy: ${render.contentSecurityPolicy}\n`,
                    );
                    await rm(join(outDir, '.vite'), { recursive: true });
                },
            },
        }),
        // Dev: packages' stylesheets stay linked (see `devStylesheets`), so their scripts add nothing. A second copy of the
        // `@fontsource` faces would load each font again, and paint the fallback font meanwhile. (`?direct` keeps its CSS.)
        transform(_, id) {
            if (this.environment.mode !== 'dev' || this.environment.name !== 'client') return undefined;
            return /\/node_modules\/.+\.css$/.test(id) ? { code: '' } : undefined;
        },
        // `vp preview` answers with the built `_headers`, as the deployed site does, so the policy can be tried locally.
        async configurePreviewServer(preview) {
            const headersFilePath = resolve(preview.config.root, preview.config.build.outDir, '_headers');
            const headersString = await readFile(headersFilePath, 'utf8');
            const rules = headerRules(headersString);
            preview.middlewares.use((req, res, next) => {
                const { pathname } = new URL(req.url ?? '/', 'http://localhost');
                for (const { pattern, headers } of rules) {
                    if (pattern.test(pathname)) {
                        res.setHeaders(new Headers(headers));
                    }
                }
                next();
            });
        },
        configureServer(dev) {
            dev.middlewares.use(packageFonts(dev));
            dev.middlewares.use((req, res, next) => {
                void (async () => {
                    try {
                        const found = await serve(dev, req.url ?? '/', site);
                        if (!found) return next();
                        res.statusCode = found.status;
                        res.setHeader('content-type', found.type);
                        res.end(found.body);
                    } catch (error) {
                        // The ssr environment's runner already maps the stack to the source files.
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
    };
}

/**
 * The renderer, run by the ssr environment's module runner. The runner keeps each module it evaluated and runs one again
 * only once it or an import changed, so this is cheap after the first call and still picks up edits.
 */
async function loadRenderer(server: ViteDevServer): Promise<Renderer> {
    const { ssr } = server.environments;
    if (!isRunnableDevEnvironment(ssr)) throw new Error('The ssr environment must run in this process to render the docs.');
    return ssr.runner.import<Renderer>(`/${renderer}`);
}

async function serve(server: ViteDevServer, url: string, site: SitePlugins | undefined): Promise<{ type: string; body: string; status: number } | undefined> {
    const path = new URL(url, 'http://localhost').pathname.replace(/\.html$/, '');
    const isSiteFile = /^\/(search-index\.json|llms(-full)?\.txt|.+\.md)$/.test(path);
    if (!isSiteFile && (/\.\w+$/.test(path) || /^\/(@|src\/|node_modules\/)/.test(path))) return undefined;
    const render = await loadRenderer(server);
    if (isSiteFile) {
        // The search index and llms.txt need every page; build them only when asked for.
        const body = render.siteFiles(await loadAll(render)).get(path.slice(1));
        return body === undefined ? undefined : { type: path.endsWith('.json') ? 'application/json' : 'text/plain', body, status: 200 };
    }
    const route = [path, `${path}/`].find(r => render.routes.includes(r));
    const page = (await render.loadPage(route ?? '/404'))!;
    // Before the browser asks for the file icons' stylesheet, which this page's tab labels need.
    site?.addFileIconLabels(render.tabLabels([page]));
    const linked = new Set<string>(['main', ...page.entries]);
    const assets = Object.fromEntries(
        await Promise.all(
            render.clientEntries.entries().map(async ([name, entry]) => [name, { css: linked.has(name) ? await devStylesheets(server, entry) : [], js: [`/${entry}`] }]),
        ),
    ) as Assets;
    const html = render.renderDocument(page, assets, 'http://localhost/').replace('</head>', `${dropDevStylesheets}</head>`);
    return { type: 'text/html', body: await server.transformIndexHtml(url, html), status: route ? 200 : 404 };
}

/**
 * What to follow from a module: the site's own source modules, and every stylesheet that is a file, its own or a
 * package's (the `@fontsource` faces, which would otherwise swap in after the first paint). Prebundled dependencies
 * carry no CSS; `?raw` and other queried imports aren't modules; virtual stylesheets (the icon fonts) don't answer
 * `?direct`, so they still arrive through the scripts.
 */
const linkable = (url: string) => !url.includes('?') && (url.startsWith('/src/') || (url.startsWith('/') && isCSSRequest(url)));

/**
 * The stylesheets a browser entry imports, itself or through its modules, in the order Vite applies them. In dev, CSS
 * reaches the page through the entries' scripts, so the page would first paint without it; linking each one as
 * `?direct` (plain CSS) styles the first paint. Transforming each module is work the browser's requests would cause
 * anyway, and later calls reuse it.
 */
async function devStylesheets(server: ViteDevServer, entry: string): Promise<string[]> {
    const client = server.environments.client;
    const seen = new Set<string>();
    const css: string[] = [];
    const visit = async (url: string): Promise<void> => {
        if (seen.has(url)) return;
        seen.add(url);
        await client.transformRequest(url);
        const module = await client.moduleGraph.getModuleByUrl(url);
        if (!module) return;
        if (isCSSRequest(module.url)) {
            css.push(`${module.url}?direct`);
            return;
        }
        for (const imported of module.importedModules) if (linkable(imported.url)) await visit(imported.url);
    };
    await visit(`/${entry}`);
    return css;
}

/**
 * Runs after the entries, once Vite has injected their styles as `<style>` elements, which HMR updates; the linked
 * copies of the site's own stylesheets would keep a stale rule applying after an edit removes it. Packages' stylesheets
 * never change, so their links stay: dropping the `@fontsource` ones would load the fonts again.
 */
const dropDevStylesheets = `<script type="module">for (const link of document.querySelectorAll('link[rel="stylesheet"][href^="/src/"][href$="?direct"]')) link.remove();</script>`;

const fontTypes: Record<string, string> = { woff2: 'font/woff2', woff: 'font/woff', ttf: 'font/ttf', otf: 'font/otf' };
/**
 * Font files from packages, as Vite serves them from outside the root. Each sits at a versioned path that never changes,
 * so the browser may keep them; with Vite's `no-cache`, every page load waits on revalidating them, and the
 * `font-display: swap` faces paint the fallback font until it answers.
 */
function packageFonts(server: ViteDevServer): Connect.NextHandleFunction {
    return (req, res, next) => {
        const file = /^\/@fs(\/.+\/node_modules\/.+\.(woff2?|ttf|otf))$/.exec(new URL(req.url ?? '/', 'http://localhost').pathname);
        if (!file || !isFileLoadingAllowed(server.config, decodeURIComponent(file[1]!))) {
            return next();
        }
        void (async () => {
            let body: Buffer;
            try {
                body = await readFile(decodeURIComponent(file[1]!));
            } catch {
                return next();
            }
            res.setHeader('content-type', fontTypes[file[2]!]!);
            res.setHeader('cache-control', 'public, max-age=31536000, immutable');
            res.end(body);
            return undefined;
        })();
    };
}
