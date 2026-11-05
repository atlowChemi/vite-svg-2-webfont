import type { CSSOptions } from 'vite-plus';

type Root = Parameters<NonNullable<Extract<NonNullable<Exclude<NonNullable<CSSOptions['postcss']>, string>['plugins']>[number], { Once?: unknown }>['Once']>>[0];
type Rule = Extract<Root['nodes'][number], { type: 'rule' }>;

/** A rule from `virtual:group-icons.css` that sets one or more labels' `--icon`. */
const isIconRule = (rule: Rule) => rule.selector.includes('[data-title=') && rule.nodes.some(child => child.type === 'decl' && child.prop === '--icon');

/**
 * `virtual:group-icons.css` brings each tab label's file icon, plus layout written for VitePress's code blocks. Only the
 * icons stay: the site's own code tabs and titles size them (`codeTabs.module.css`, `highlight.module.css`). The icon
 * rules come out sorted, since the plugin emits labels in the order it discovered them.
 */
export const iconCss: Exclude<NonNullable<CSSOptions['postcss']>, string> = {
    plugins: [
        {
            postcssPlugin: 'group-icons-only',
            Once(root) {
                const rules = root.nodes.filter((node): node is Rule => node.type === 'rule').filter(isIconRule);
                if (!rules.length) return;
                for (const node of root.nodes.toReversed()) if (!(node.type === 'rule' && rules.includes(node))) node.remove();
                for (const rule of rules) rule.selectors = rule.selectors.toSorted();
                const sorted = rules.map(rule => rule.clone()).toSorted((a, b) => (a.selector < b.selector ? -1 : a.selector > b.selector ? 1 : 0));
                rules.forEach((rule, index) => rule.replaceWith(sorted[index]!.clone({ raws: rule.raws })));
            },
        },
    ],
};
