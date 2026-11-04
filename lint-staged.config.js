export default {
    '*.{js,mjs,cjs,ts,tsx,jsx}': ['oxfmt', 'oxlint --fix --type-aware --type-check'],
    '*.{json,jsonc,yaml,yml,md,html,css,scss,vue}': 'oxfmt',
    '*.rs': () => 'cargo fmt --all',
};
