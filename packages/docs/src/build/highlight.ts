import { transformerNotationDiff, transformerNotationErrorLevel, transformerNotationFocus, transformerNotationHighlight } from '@shikijs/transformers';
import { bundledLanguages, createHighlighter, type BundledLanguage, type Highlighter, type ShikiTransformer } from 'shiki';
import catppuccinFrappe from 'shiki/themes/catppuccin-frappe.mjs';
import catppuccinLatte from 'shiki/themes/catppuccin-latte.mjs';
import { codeBackgrounds, codeThemes, withReadableTokens } from '../site/codeThemes.ts';

// Shiki, emitting VitePress's code-block markup so the vendored CSS applies unchanged. markdown-it renders synchronously,
// so a page's languages load first (`loadLanguages`), and highlighting itself is sync.
let highlighter: Promise<Highlighter> | undefined;
const aliases: Record<string, string> = { hbs: 'handlebars', txt: 'text', '': 'text' };
const resolve = (lang: string) => aliases[lang] ?? lang;

export const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

export async function loadLanguages(langs: Iterable<string>): Promise<Highlighter> {
    highlighter ??= createHighlighter({
        themes: [withReadableTokens(catppuccinLatte, codeBackgrounds.light), withReadableTokens(catppuccinFrappe, codeBackgrounds.dark)],
        langs: [],
    });
    const shiki = await highlighter;
    const loaded = new Set(shiki.getLoadedLanguages());
    const missing = [...new Set([...langs].map(resolve))].filter(l => !loaded.has(l) && l in bundledLanguages) as BundledLanguage[];
    if (missing.length) await shiki.loadLanguage(...missing);
    return shiki;
}

/** `{7-9}` and `[vite.config.ts]` from a VitePress fence's info string. */
export function fenceMeta(meta = ''): { lines: Set<number>; title?: string } {
    const lines = new Set<number>();
    for (const range of /\{([\d,\s-]+)\}/.exec(meta)?.[1]?.split(',') ?? []) {
        const [from, to = from] = range.split('-').map(Number);
        for (let n = from!; n <= to!; n++) lines.add(n);
    }
    return { lines, title: /\[([^\]]+)\]/.exec(meta)?.[1] };
}

// VitePress applies the `// [!code …]` notations by default; the docs' examples rely on them.
const notations = [
    transformerNotationDiff(),
    transformerNotationFocus({ classActiveLine: 'has-focus', classActivePre: 'has-focused-lines' }),
    transformerNotationHighlight(),
    transformerNotationErrorLevel(),
];

export function highlight(shiki: Highlighter, code: string, lang: string, lines: Set<number> = new Set(), active = false): string {
    const language = shiki.getLoadedLanguages().includes(resolve(lang)) ? resolve(lang) : 'text';
    const marks: ShikiTransformer = {
        line(node, line) {
            if (lines.has(line)) this.addClassToHast(node, 'highlighted');
        },
        pre(node) {
            if (lines.size) this.addClassToHast(node, 'has-highlighted');
            node.properties.tabindex = '0';
        },
    };
    const pre = shiki.codeToHtml(code, { lang: language, themes: codeThemes, defaultColor: false, transformers: [...notations, marks] });
    const label = lang || 'txt';
    return `<div class="language-${label}${active ? ' active' : ''}"><button title="Copy code" class="copy"></button><span class="lang">${label}</span>${pre}</div>`;
}

export const titled = (html: string, title: string) =>
    `<div class="vp-code-block-title"><div class="vp-code-block-title-bar"><span class="vp-code-block-title-text" data-title="${title}">${title}</span></div>${html}</div>`;
