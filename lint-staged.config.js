export default {
    '*.{js,mjs,cjs,ts,tsx,jsx}': ['oxfmt --no-error-on-unmatched-pattern', 'oxlint --fix --type-aware --type-check --no-error-on-unmatched-pattern'],
    '*.{json,jsonc,yaml,yml,md,html,css,scss,vue}': 'oxfmt --no-error-on-unmatched-pattern',
    '*.rs': () => 'cargo fmt --all',
};
