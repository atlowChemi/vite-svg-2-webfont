import { transformerNotationFocus, transformerNotationHighlight } from '@shikijs/transformers';
import { bundledLanguages, createHighlighter, type BundledLanguage, type Highlighter, type ShikiTransformer } from 'shiki';
import catppuccinFrappe from 'shiki/themes/catppuccin-frappe.mjs';
import catppuccinLatte from 'shiki/themes/catppuccin-latte.mjs';
import { codeBackgrounds, codeThemes, withReadableTokens } from '../site/codeThemes.ts';
import { DiskCache, installed } from './diskCache.ts';
import styles from './highlight.module.css';

// Code blocks, highlighted by Shiki at build time. markdown-it renders synchronously, so a page's languages load first
// (`loadLanguages`), and highlighting itself is sync.
let highlighter: Promise<Highlighter> | undefined;
const themes = [withReadableTokens(catppuccinLatte, codeBackgrounds.light), withReadableTokens(catppuccinFrappe, codeBackgrounds.dark)];
const aliases: Record<string, string> = { hbs: 'handlebars', txt: 'text', '': 'text' };
const resolve = (lang: string) => aliases[lang] ?? lang;

export const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

export async function loadLanguages(langs: Iterable<string>): Promise<Highlighter> {
    highlighter ??= createHighlighter({ themes, langs: [] });
    const shiki = await highlighter;
    const loaded = new Set(shiki.getLoadedLanguages());
    const missing = new Set(
        Iterator.from(langs)
            .map(resolve)
            .filter((l): l is BundledLanguage => !loaded.has(l) && l in bundledLanguages),
    )
        .values()
        .toArray();
    if (missing.length) await shiki.loadLanguage(...missing);
    return shiki;
}

/**
 * Extracts highlighted line numbers and an optional title from a code fence's info string.
 * Example: '{1,3-5} [vite.config.ts]' → lines {1, 3, 4, 5}, title 'vite.config.ts'.
 */
export function fenceMeta(meta = ''): { lines: Set<number>; title?: string } {
    const lines = new Set<number>();

    // Extract the comma-separated line numbers and ranges inside {...}.
    const ranges = /\{([\d,\s-]+)\}/.exec(meta)?.[1];

    if (ranges) {
        // Match individual numbers or inclusive ranges (e.g. '3' or '5-8').
        for (const match of ranges.matchAll(/(\d+)(?:\s*-\s*(\d+))?/g)) {
            const from = Number(match[1]);
            const to = match[2] ? Number(match[2]) : from;

            // Expand each inclusive range into individual line numbers.
            for (let n = from; n <= to; n++) lines.add(n);
        }
    }

    // Extract the optional filename/title enclosed in [...].
    const title = /\[([^\]]+)\]/.exec(meta)?.[1];

    return { lines, title };
}

// `// [!code highlight]` marks a line like `{n}` does. `// [!code focus]` blurs the other lines; hovering, tabbing to
// or tapping the block brings them back.
const notations = [
    transformerNotationHighlight({ classActiveLine: styles.highlighted, classActivePre: '' }),
    transformerNotationFocus({ classActiveLine: styles.focused, classActivePre: styles.hasFocused }),
];

// Highlighting is most of a build's work, nearly all of it Shiki compiling each grammar's patterns on first use; a block
// whose code hasn't changed comes from disk instead. This file, the themes, Shiki and its transformers, and the class
// names decide the HTML.
const blocks = new DiskCache('highlight', ['src/build/highlight.ts', 'src/site/codeThemes.ts'], {
    packages: ['shiki', '@shikijs/transformers'].map(installed),
    themes,
    styles,
});

/** After a full build: drops the stored code blocks no page used, and writes the rest. */
export const pruneHighlights = () => blocks.prune();

export interface CodeBlockOptions {
    /** Lines to highlight, 1-based, as a fence's `{n-m}`. */
    lines?: Set<number>;
}

/**
 * A highlighted code block: its language label, a copy button, and the code, with both themes' colors as Shiki's CSS
 * variables.
 */
export function highlight(shiki: Highlighter, code: string, lang: string, { lines = new Set() }: CodeBlockOptions = {}): string {
    return blocks.getOrCreate([lang, [...lines].join(','), code], () => highlightNow(shiki, code, lang, lines));
}

function highlightNow(shiki: Highlighter, code: string, lang: string, lines: Set<number>): string {
    const language = shiki.getLoadedLanguages().includes(resolve(lang)) ? resolve(lang) : 'text';
    const marks: ShikiTransformer = {
        line(node, line) {
            if (lines.has(line)) this.addClassToHast(node, styles.highlighted!);
        },
        pre(node) {
            node.properties.tabindex = '0';
        },
    };
    const pre = shiki.codeToHtml(code, { lang: language, themes: codeThemes, defaultColor: false, transformers: [...notations, marks] });
    const label = escape(lang || 'txt');
    return `<div class="${styles.block}" data-lang="${label}"><button type="button" class="${styles.copy}" title="Copy code" aria-label="Copy code"></button><span class="${styles.lang}">${label}</span>${pre}</div>`;
}

/** A code block under a bar that names its file. */
export const titled = (html: string, title: string) =>
    `<div class="${styles.titled}"><div class="${styles.titleBar}"><span class="${styles.title}" data-title="${title}">${title}</span></div>${html}</div>`;
