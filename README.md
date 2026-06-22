# Benchmark Baselines

This branch stores benchmark outputs generated from `main` by the Benchmark Baselines workflow.

Contents are intentionally tool-native so future workflows can use native comparison modes:

- `webfont-generator/criterion/` contains Criterion output copied from `packages/webfont-generator/target/criterion`.
- `webfont-generator/vitest/webfonts-generator.json` contains Vitest benchmark JSON from `vp test bench --outputJson`.
- `metadata.json` records the source commit and runner metadata for the latest update.
- `tool-versions.txt` records the Vite+ and tool versions used for the latest update.

Do not target this branch with normal feature PRs. It is maintained by CI.
