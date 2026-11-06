// The server side of the build: Markdown in, finished HTML documents out. Vite builds this module for Node, and the
// plugin in `config/staticSite.ts` calls it, for every page during the build and per request in dev.
import { createHash } from 'node:crypto';
import { readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import generatedCss from 'virtual:docs-examples-css';
import { CacheFileLayout } from '../components/blocks/CacheFileLayout.tsx';
import { ExampleCss } from '../components/blocks/ExampleCss.tsx';
import { ExampleIcons } from '../components/blocks/ExampleIcons.tsx';
import { ExampleOutput } from '../components/blocks/ExampleOutput.tsx';
import { ExampleTabs } from '../components/blocks/ExampleTabs.tsx';
import { SvgAuthoringCompare } from '../components/blocks/SvgAuthoringCompare.tsx';
import { Landing } from '../components/landing/Landing.tsx';
import { NotFound } from '../components/NotFound.tsx';
import { PlaygroundPage } from '../components/playground/PlaygroundPage.tsx';
import { DocShell, PageShell } from '../components/shell/DocShell.tsx';
import { pageNav } from '../site/nav.ts';
import { site } from '../site/site.ts';
import { loadLanguages } from './highlight.ts';
import { pagesRoot, renderMarkdown, type BlockRenderer, type Frontmatter, type Heading } from './markdown.ts';
export { deadLinks } from './links.ts';
import { llmsFullTxt, llmsTxt, markdownFiles, mdPath, searchIndex } from './outputs.ts';
import { specimens } from './specimens.ts';

/** Browser entries a page can link besides `main`, each with its own stylesheet or script. */
export const clientEntries = new Map([
    ['main', 'src/client/main.ts'],
    ['examples', 'src/client/examples.ts'],
    ['svg-authoring', 'src/client/svgAuthoring.ts'],
    ['landing', 'src/client/landing.ts'],
    ['playground', 'src/client/playground.tsx'],
    ['not-found', 'src/client/notFound.ts'],
] as const);
export type ClientEntry = typeof clientEntries extends Map<infer K, unknown> ? K : never;
/** Entries that only carry styles: pages link their stylesheet, not their empty script. */
export const styleOnly: ReadonlySet<ClientEntry> = new Set<ClientEntry>(['examples', 'svg-authoring', 'not-found']);

export interface Page {
    route: string;
    /** The source under packages/docs; empty for pages that have none (404). */
    file: string;
    title: string;
    description?: string;
    frontmatter: Frontmatter;
    headings: Heading[];
    /** The rendered Markdown, without the shell. */
    html: string;
    source: string;
    deps: Set<string>;
    entries: Set<ClientEntry>;
}

const cssHighlighter = await loadLanguages(['css']);

// Components the Markdown can use, rendered to static HTML at build time, and the browser entry each one needs.
type BlockName = 'ExampleTabs' | 'ExampleIcons' | 'ExampleCss' | 'ExampleOutput' | 'SvgAuthoringCompare' | 'CacheFileLayout' | 'FonteLanding' | 'Playground';
type BlockComponent = { entry?: ClientEntry; render: (this: void, props: Record<string, unknown>, slots: Record<string, string>) => ReactNode };
const blocks: Record<BlockName, BlockComponent> = {
    ExampleTabs: { entry: 'examples', render: (p, slots) => <ExampleTabs tabs={p.tabs as Record<string, string>} slots={slots} /> },
    ExampleIcons: { entry: 'examples', render: p => <ExampleIcons example={p.example as string} /> },
    ExampleCss: {
        entry: 'examples',
        render: p => <ExampleCss css={generatedCss[p.example as string] ?? ''} highlight={p.highlight as string[] | undefined} shiki={cssHighlighter} />,
    },
    ExampleOutput: { entry: 'examples', render: (p, slots) => <ExampleOutput {...(p as { example: string })} slots={slots} /> },
    SvgAuthoringCompare: {
        entry: 'svg-authoring',
        render: p => (
            <SvgAuthoringCompare
                cases={p.cases as { id: string; label: string }[]}
                text={p.text as [string, string] | undefined}
                codepoints={p.codepoints as number[] | undefined}
                size={p.size as number | undefined}
            />
        ),
    },
    CacheFileLayout: { render: () => <CacheFileLayout /> },
    FonteLanding: { entry: 'landing', render: (_, slots) => <Landing install={slots} /> },
    Playground: { entry: 'playground', render: (_, slots) => <PlaygroundPage intro={slots.default ?? ''} /> },
};
const blockNames = new Set(Object.keys(blocks) as BlockName[]);

/** Every page by route, from `src/pages`: `guide/index.md` is `/guide/`, `guide/engine.md` is `/guide/engine`. */
function discover(): Record<string, string> {
    const pages: Record<string, string> = {};
    for (const entry of readdirSync(pagesRoot, { recursive: true, withFileTypes: true })) {
        if (!entry.isFile() || !entry.name.endsWith('.md')) continue;
        // `name` is only the file's own name; the route needs its path below `src/pages`.
        const file = relative(pagesRoot, join(entry.parentPath, entry.name));
        pages[`/${file.replace(/(^|\/)index\.md$/, '$1').replace(/\.md$/, '')}`] = file;
    }
    return pages;
}

const pages = discover();
/** Every route the site serves, the 404 page last. */
// oxlint-disable-next-line unicorn/no-array-sort -- We are creating a new array here. No reason not to mutate it.
export const routes = Object.keys(pages).sort();
routes.push('/404');

/** Load a single route, render it's markdown, including it's rendered React components. */
export async function loadPage(route: string): Promise<Page | undefined> {
    if (route === '/404') {
        return { route, file: '', title: 'Page not found', frontmatter: { layout: 'page' }, headings: [], html: '', source: '', deps: new Set(), entries: new Set(['not-found']) };
    }
    const file = pages[route];
    if (!file) return undefined;
    const entries = new Set<ClientEntry>();
    let rendered = 0;
    const renderBlock: BlockRenderer<BlockName> = (block, renderSlot) => {
        const { render, entry } = blocks[block.name];
        if (entry) entries.add(entry);
        const slots = Object.fromEntries(Object.entries(block.slots).map(([name, markdown]) => [name, renderSlot(markdown)]));
        // Each block is its own render, so its `useId`s get a prefix of their own: unique on the page and beside the shell's.
        return renderToStaticMarkup(render(block.props, slots), { identifierPrefix: `b${++rendered}-` });
    };
    const markdown = await renderMarkdown(join(pagesRoot, file), renderBlock, blockNames);
    const html = markdown.frontmatter.layout === 'page' ? markdown.html : specimens(markdown.html, route.split('/', 2)[1] ?? '');
    return { route, file, description: markdown.frontmatter.description, ...markdown, html, entries };
}

/** The stylesheets and scripts of each entry: hashed build output, or Vite's dev URLs. */
export type Assets = Record<ClientEntry, { css: string[]; js: string[] }>;

// Applied before the first paint, so a stored or system dark preference never flashes light. VitePress's key, so a
// reader's choice carries over.
const themeScript = `try{const p=localStorage.getItem('vitepress-theme-appearance')||'auto';document.documentElement.classList.toggle('dark',p==='dark'||(p==='auto'&&matchMedia('(prefers-color-scheme: dark)').matches))}catch{}`;
// Chromium prerenders a page once the pointer rests on its link; the page script prefetches in other browsers. The
// Playground is only prefetched: prerendering runs its script, which loads and compiles the generator's WASM.
const speculation = JSON.stringify({
    prerender: [{ where: { and: [{ href_matches: '/*' }, { not: { href_matches: '/playground' } }] }, eagerness: 'moderate' }],
    prefetch: [{ where: { href_matches: '/playground' }, eagerness: 'moderate' }],
});

const hash = (script: string) => `'sha256-${createHash('sha256').update(script).digest('base64')}'`;
/**
 * The pages' Content-Security-Policy: everything comes from this origin. The two inline scripts above are allowed by
 * hash, so the policy follows them when they change. The Playground's worker compiles the generator's WASM; the
 * rendered pages carry `style` attributes and diagrams carry `<style>` elements; the file icons are `data:` SVGs, and
 * Vite inlines small fonts, the example fonts among them, as `data:` URLs.
 */
export const contentSecurityPolicy = [
    "default-src 'self'",
    `script-src 'self' ${hash(themeScript)} ${hash(speculation)} 'wasm-unsafe-eval'`,
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self' data:",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
].join('; ');

function body(page: Page): ReactNode {
    const nav = pageNav(page.route);
    if (page.route === '/404')
        return (
            <PageShell nav={nav}>
                <NotFound />
            </PageShell>
        );
    if (page.frontmatter.layout === 'page')
        return (
            // The landing page ends in a footer of its own.
            <PageShell nav={nav} footer={page.route !== '/'}>
                {page.route === '/' && (
                    <div hidden data-nosnippet="">
                        Are you an LLM? View /llms.txt for optimized Markdown documentation, or /llms-full.txt for full documentation bundle
                    </div>
                )}
                <div dangerouslySetInnerHTML={{ __html: page.html }} />
            </PageShell>
        );
    return <DocShell page={{ nav, title: page.title, file: page.file, headings: page.headings, html: page.html }} />;
}

/** The page's `<title>`: `title | fonte`, or the frontmatter's `titleTemplate` in place of `fonte`. */
const documentTitle = (page: Page) => (page.route === '/' ? `${site.name} | ${page.frontmatter.titleTemplate ?? site.subtitle}` : `${page.title} | ${site.name}`);

export function renderDocument(page: Page, assets: Assets, docsUrl: string): string {
    const linkedAssets = (['main', ...page.entries] as ClientEntry[]).map(entry => assets[entry]);
    const description = page.description ?? site.description;
    const url = new URL(page.route === '/404' ? '' : page.route.slice(1), docsUrl).href;
    const image = new URL('social-card.png', docsUrl).href;
    const html = renderToStaticMarkup(
        <html lang="en-US">
            <head>
                <meta charSet="utf-8" />
                <meta name="viewport" content="width=device-width,initial-scale=1" />
                <title>{documentTitle(page)}</title>
                <meta name="description" content={description} />
                <meta name="theme-color" content="#faf7f2" media="(prefers-color-scheme: light)" />
                <meta name="theme-color" content="#1c1a1f" media="(prefers-color-scheme: dark)" />
                <link rel="icon" href="/logo.svg" />
                <meta property="og:type" content="website" />
                <meta property="og:site_name" content={site.name} />
                <meta property="og:title" content={page.route === '/' ? site.name : page.title} />
                <meta property="og:description" content={description} />
                <meta property="og:url" content={url} />
                <meta property="og:image" content={image} />
                <meta property="og:image:alt" content='The fonte wordmark and the line "Melted SVGs, cast into fonts", beside a large T filled with molten metal.' />
                <meta name="twitter:card" content="summary_large_image" />
                <meta name="twitter:title" content={page.route === '/' ? site.name : page.title} />
                <meta name="twitter:description" content={description} />
                <meta name="twitter:image" content={image} />
                {page.file && <link rel="alternate" type="text/markdown" href={mdPath(page.route)} />}
                <link rel="describedby" href="/llms.txt" />
                <script dangerouslySetInnerHTML={{ __html: themeScript }} />
                <script type="speculationrules" dangerouslySetInnerHTML={{ __html: speculation }} />
                {new Set(linkedAssets.flatMap(a => a.css))
                    .values()
                    .map(href => <link key={href} rel="stylesheet" href={href} />)
                    .toArray()}
                {new Set(linkedAssets.flatMap(a => a.js))
                    .values()
                    .map(src => <script key={src} type="module" src={src} />)
                    .toArray()}
            </head>
            <body>{body(page)}</body>
        </html>,
    );
    return `<!DOCTYPE html>${html}`;
}

/** Every file the site serves besides the pages: the search index, llms.txt and each page's Markdown. */
export function siteFiles(all: Page[]): Map<string, string> {
    const sources = all.filter(p => p.file);
    // The landing page is all components, with nothing to find; the Playground is found by its own text.
    const searchable = sources.filter(p => p.route !== '/');
    return new Map([
        ['search-index.json', searchIndex(searchable)],
        ['llms.txt', llmsTxt(sources)],
        ['llms-full.txt', llmsFullTxt(sources, blockNames)],
        ...markdownFiles(sources, blockNames),
    ]);
}

function* labelsIn(all: Page[]) {
    for (const { html } of all) {
        for (const match of html.matchAll(/data-title="([^"]*)"/g)) {
            yield match[1]!;
        }
    }
}

/** Every code-group and example-tab label, for the file icons in `virtual:file-icons.css`. */
export const tabLabels = (all: Page[]) => new Set(labelsIn(all));
