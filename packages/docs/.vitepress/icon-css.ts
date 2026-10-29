import type { CSSOptions } from 'vite-plus';

export const iconCss: Exclude<NonNullable<CSSOptions['postcss']>, string> = {
    plugins: [
        {
            postcssPlugin: 'stable-group-icon-selectors',
            Once(root) {
                // The plugin sorts rules before we normalize their discovery-ordered selectors.
                const rules = root.nodes
                    .filter(node => node.type === 'rule')
                    .filter(node => node.selector.includes('[data-title=') && node.nodes.some(child => child.type === 'decl' && child.prop === '--icon'));
                for (const rule of rules) rule.selectors = rule.selectors.toSorted();
                const sorted = rules.map(rule => rule.clone()).toSorted((a, b) => (a.selector < b.selector ? -1 : a.selector > b.selector ? 1 : 0));
                rules.forEach((rule, index) => rule.replaceWith(sorted[index]!.clone({ raws: rule.raws })));
            },
        },
    ],
};
