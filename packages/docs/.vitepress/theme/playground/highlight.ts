import type { HighlighterCore } from 'shiki/core';
import { codeThemes } from '../../codeThemes';

let highlighter: Promise<HighlighterCore> | undefined;

/** Load Shiki on first use with the site's code themes and the CSS grammar; other grammars load when first needed. */
export function loadHighlighter(): Promise<HighlighterCore> {
    highlighter ??= (async () => {
        const [{ createHighlighterCore }, { createJavaScriptRegexEngine }] = await Promise.all([import('shiki/core'), import('shiki/engine/javascript')]);
        return createHighlighterCore({
            themes: [import('shiki/themes/catppuccin-latte.mjs'), import('shiki/themes/catppuccin-frappe.mjs')],
            langs: [import('shiki/langs/css.mjs')],
            engine: createJavaScriptRegexEngine(),
        });
    })();
    return highlighter;
}

export type HighlightLanguage = 'css' | 'html' | 'xml' | 'ts';

/** Fetch each grammar only when its output is shown. */
const lazyGrammars = {
    ts: () => import('shiki/langs/typescript.mjs'),
    html: () => import('shiki/langs/html.mjs'),
    xml: () => import('shiki/langs/xml.mjs'),
};

/** Highlight generated output with the same themes and light/dark variables as the site's code blocks. */
export async function highlight(code: string, lang: HighlightLanguage): Promise<string> {
    const shiki = await loadHighlighter();
    if (lang !== 'css' && !shiki.getLoadedLanguages().includes(lang)) await shiki.loadLanguage(lazyGrammars[lang]());
    return shiki.codeToHtml(code, { lang, themes: codeThemes, defaultColor: false });
}
