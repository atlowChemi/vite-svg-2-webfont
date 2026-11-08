import type { HighlighterCore } from 'shiki/core';
import { codeBackgrounds, codeThemes, withReadableTokens } from '../../site/codeThemes.ts';

let highlighter: Promise<HighlighterCore> | undefined;

/** Load Shiki on first use with the site's code themes and the CSS grammar; other grammars load when first needed. */
export function loadHighlighter(): Promise<HighlighterCore> {
    highlighter ??= (async () => {
        const [{ createHighlighterCore }, { createJavaScriptRegexEngine }] = await Promise.all([import('shiki/core'), import('shiki/engine/javascript')]);
        return createHighlighterCore({
            themes: [
                import('shiki/themes/catppuccin-latte.mjs').then(({ default: theme }) => withReadableTokens(theme, codeBackgrounds.light)),
                import('shiki/themes/catppuccin-frappe.mjs').then(({ default: theme }) => withReadableTokens(theme, codeBackgrounds.dark)),
            ],
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

const grammars = new Map<HighlightLanguage, Promise<HighlighterCore>>();

/**
 * Shiki with this grammar loaded, each grammar once. The viewer calls it when a tab is hovered or focused too, so the
 * grammar is usually there by the time the tab is chosen.
 */
export function loadLanguage(lang: HighlightLanguage): Promise<HighlighterCore> {
    let loaded = grammars.get(lang);
    if (!loaded) {
        loaded = loadHighlighter().then(async shiki => {
            if (lang !== 'css') await shiki.loadLanguage(lazyGrammars[lang]());
            return shiki;
        });
        // A failed load is tried again next time.
        loaded.catch(() => grammars.delete(lang));
        grammars.set(lang, loaded);
    }
    return loaded;
}

/** Highlight generated output with the same themes and light/dark variables as the site's code blocks. */
export async function highlight(code: string, lang: HighlightLanguage): Promise<string> {
    const shiki = await loadLanguage(lang);
    return shiki.codeToHtml(code, { lang, themes: codeThemes, defaultColor: false });
}
