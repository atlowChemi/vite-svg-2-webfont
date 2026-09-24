export default {
    extends: ['@commitlint/config-conventional'],
    rules: {
        // Keep in sync with the PR title scopes in .github/workflows/pr-title.yaml.
        'scope-enum': [2, 'always', ['benchmarks', 'ci', 'deps', 'deps-dev', 'docs', 'example', 'main', 'tests', 'vite-svg-2-webfont', 'webfont-generator']],
    },
};
