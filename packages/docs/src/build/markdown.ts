import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import MarkdownIt, { type Env as MarkdownEnv, type Token } from 'markdown-it';
import container from 'markdown-it-container';
import { parse as parseYaml } from 'yaml';
import { extractBlocks, type Block } from './blocks.ts';
import tabStyles from './codeTabs.module.css';
import { escape, fenceMeta, highlight, loadLanguages, titled } from './highlight.ts';
import { expandIncludes } from './includes.ts';
import styles from './markdown.module.css';
import { diagramKey, marker, renderDiagrams } from './mermaid.ts';

// Markdown → HTML with markdown-it, plus the syntaxes the docs were written in for VitePress: includes and snippets,
// `::: tip`-style containers, code groups, titled and line-highlighted fences, and heading ids from VitePress's slugger,
// so every existing `#hash` link still lands. The markup is the site's own, styled by `markdown.module.css`,
// `codeTabs.module.css` and `highlight.module.css`.

// This module runs from `src/build` in dev and from `.cache/server` once built: both two folders below the docs.
export const docsRoot = join(import.meta.dirname, '../..');

export interface Heading {
    level: number;
    title: string;
    slug: string;
    children: Heading[];
}

export interface Frontmatter {
    title?: string;
    titleTemplate?: string;
    description?: string;
    /** `page`: no rail or page strip, for the landing page and the Playground. */
    layout?: 'doc' | 'page';
}

export interface Markdown {
    title: string;
    frontmatter: Frontmatter;
    html: string;
    headings: Heading[];
    /** The page's Markdown after includes, for llms.txt. */
    source: string;
    /** Every file the page was read from, for the dev server's reloads. */
    deps: Set<string>;
}

/** Turns one Markdown block into HTML; its slots are rendered first, with `renderSlot`. */
export type BlockRenderer = (block: Block, renderSlot: (markdown: string) => string) => string;

/** VitePress's heading slug. */
export const slug = (heading: string) =>
    heading
        .replace(/[\s~`!@#$%^&*()\-_+=[\]{}|\\;:"'“”‘’<>,.?/]+/g, '-')
        .replace(/^-+|-+$/g, '')
        .replace(/^(\d)/, '_$1')
        .toLowerCase();

/** Removes a trailing `{#id}` from a heading and returns the id. */
function customId(inline: Token): string | undefined {
    const last = inline.children?.at(-1);
    const match = last?.type === 'text' ? /\s*\{#([\w-]+)\}\s*$/.exec(last.content) : null;
    if (!last || !match) return undefined;
    last.content = last.content.slice(0, match.index);
    if (!last.content) inline.children!.pop();
    inline.content = inline.content.replace(/\s*\{#[\w-]+\}\s*$/, '');
    return match[1];
}

const inlineText = (token: Token) => (token.children ?? []).map(t => (t.type === 'text' || t.type === 'code_inline' ? t.content : '')).join('');

let tabGroup = 0;
function createMarkdown() {
    const md = new MarkdownIt({ html: true, linkify: false, typographer: false });

    // Callouts: tip, info, warning, danger and details, with an optional custom title.
    for (const kind of ['tip', 'info', 'warning', 'danger', 'details'] as const) {
        md.use(container, kind, {
            render(tokens: Token[], i: number) {
                const token = tokens[i]!;
                if (token.nesting !== 1) return kind === 'details' ? '</details>\n' : '</div>\n';
                const title = md.renderInline(token.info.trim().slice(kind.length).trim() || kind.toUpperCase());
                const className = `${styles.callout} ${styles[kind]}`;
                return kind === 'details' ? `<details class="${className}"><summary>${title}</summary>\n` : `<div class="${className}"><p class="${styles.title}">${title}</p>\n`;
            },
        });
    }

    // A code group: one radio tab per fence, labelled by the fence's `[title]`; the fences render as its panels.
    md.use(container, 'code-group', {
        render(tokens: Token[], i: number) {
            if (tokens[i]!.nesting !== 1) return '</div></div>\n';
            const fences: Token[] = [];
            for (let j = i + 1, depth = 0; j < tokens.length; j++) {
                const t = tokens[j]!;
                if (t.type === 'container_code-group_open') depth++;
                if (t.type === 'container_code-group_close' && depth-- === 0) break;
                if (t.type === 'fence' && depth === 0) fences.push(t);
            }
            const group = `group-${++tabGroup}`;
            const tabs = fences.map((f, n) => {
                f.meta = { group: true, hidden: n > 0 };
                const label = escape(fenceMeta(f.info).title ?? f.info.trim().split(/\s/)[0] ?? '');
                return `<input type="radio" name="${group}" id="${group}-${n}"${n ? '' : ' checked'}><label for="${group}-${n}" data-title="${label}">${label}</label>`;
            });
            return `<div class="${tabStyles.group}"><div class="${tabStyles.tabs}">${tabs.join('')}</div><div class="${tabStyles.panels}">\n`;
        },
    });

    // External links open in a new tab, as VitePress's did.
    md.renderer.rules.link_open = (tokens, i, options, _env, self) => {
        const token = tokens[i]!;
        if (/^(?:[a-z][a-z\d+.-]*:)?\/\//i.test(String(token.attrGet('href') ?? ''))) {
            token.attrSet('target', '_blank');
            token.attrSet('rel', 'noreferrer');
        }
        return self.renderToken(tokens, i, options);
    };

    // Wide tables scroll sideways, so they take focus to scroll from the keyboard.
    md.renderer.rules.table_open = (tokens, i, options, _env, self) => {
        tokens[i]!.attrSet('tabindex', '0');
        return self.renderToken(tokens, i, options);
    };

    // Heading ids from VitePress's slugger (deduplicated per page), its permalink anchor, and the page's outline. A
    // trailing `{#id}` sets the id itself, as VitePress's attribute syntax did.
    md.core.ruler.push('fonte-headings', state => {
        const env = state.env as Env;
        for (let i = 0; i < state.tokens.length; i++) {
            const open = state.tokens[i]!;
            if (open.type !== 'heading_open') continue;
            const inline = state.tokens[i + 1]!;
            const custom = customId(inline);
            const text = inlineText(inline);
            const base = custom ?? slug(text);
            const n = env.slugs.get(base) ?? 0;
            env.slugs.set(base, n + 1);
            const id = n && !custom ? `${base}-${n}` : base;
            open.attrSet('id', id);
            open.attrSet('tabindex', '-1');
            const anchor = new state.Token('html_inline', '', 0);
            anchor.content = ` <a class="${styles.anchor}" href="#${id}" aria-label="Permalink to “${escape(text)}”">&#8203;</a>`;
            inline.children?.push(anchor);
            const level = Number(open.tag.slice(1));
            if (level === 1) env.title ??= text;
            if (level === 2) env.headings.push({ level, title: text, slug: id, children: [] });
            if (level === 3) env.headings.at(-1)?.children.push({ level, title: text, slug: id, children: [] });
        }
    });
    return md;
}

interface Env extends MarkdownEnv {
    headings: Heading[];
    slugs: Map<string, number>;
    title?: string;
}

const md = createMarkdown();
const blockMarker = (i: number) => `<!--fonte-block:${i}-->`;

export async function renderMarkdown(file: string, renderBlock: BlockRenderer, blockNames: Set<string>): Promise<Markdown> {
    const path = join(docsRoot, file);
    const deps = new Set([path]);
    const raw = readFileSync(path, 'utf8');
    const head = /^---\n([\s\S]*?)\n---\n/.exec(raw);
    const frontmatter = (head ? parseYaml(head[1]!) : {}) as Frontmatter;
    const source = expandIncludes(raw.slice(head?.[0].length ?? 0), path, docsRoot, deps);

    const shiki = await loadLanguages([...source.matchAll(/^[ \t]*(?:`{3,}|~{3,})\s*([\w-]+)/gm)].map(m => m[1]!));
    const diagrams = new Map<string, string>();
    md.renderer.rules.fence = (tokens, i) => {
        const token = tokens[i]!;
        const lang = /^[\w-]*/.exec(token.info.trim())![0];
        const meta = token.info.trim().slice(lang.length);
        if (lang === 'mermaid') {
            const key = diagramKey(token.content);
            diagrams.set(key, token.content);
            return marker(key);
        }
        const { lines, title } = fenceMeta(meta);
        const group = token.meta as { group?: boolean; hidden?: boolean } | null;
        const html = highlight(shiki, token.content.replace(/\n$/, ''), lang, { lines, hidden: group?.hidden });
        return title && !group?.group ? titled(html, escape(title)) : html;
    };

    const env: Env = { headings: [], slugs: new Map() };
    const render = (markdown: string): string => {
        const { source: text, blocks } = extractBlocks(markdown, blockNames, blockMarker);
        let html = md.render(text, env);
        blocks.forEach((block, i) => {
            html = html.replace(blockMarker(i), () => renderBlock(block, render));
        });
        return html;
    };
    let html = render(source);
    for (const [key, svg] of await renderDiagrams(diagrams)) html = html.replace(marker(key), () => svg);
    return { title: frontmatter.title ?? env.title ?? '', frontmatter, html, headings: env.headings, source, deps };
}
