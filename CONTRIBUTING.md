# Contributing

Thanks for contributing to `vite-svg-2-webfont`.

## Development Setup

This repository uses:

- The Node version pinned in [`.node-version`](./.node-version)
- Vite+ through the `vp` CLI for installs, checks, tests, builds, and task execution. See the [Vite+ docs](https://viteplus.dev/guide/) for more information about the toolchain and workflow, and how to install it.
- `pnpm` underneath the hood as the package manager, managed through `vp`
- `oxlint` for linting, type-aware checks, and type checking through Vite+ and `tsgo`
- `oxfmt` for formatting through Vite+
- A monorepo workspace with packages under `packages/`
- Stable Rust with Cargo, Clippy, and rustfmt for the native generator. The JavaScript test and plugin build tasks build this native binding automatically.

Install dependencies from the repository root:

```bash
vp install
vp exec playwright install chromium firefox webkit
```

On Linux, use `vp exec playwright install --with-deps chromium firefox webkit` to install browser system dependencies as well. Rerun `vp install` after pulling dependency changes.

## Common Commands

Run these from the repository root:

```bash
vp check                                         # format, lint, TypeScript checks
vp run test                                      # JavaScript, browser, and type tests; builds the debug binding
vp run coverage                                  # non-browser JavaScript tests with coverage
vp run bench:vitest                              # Vitest benchmarks with release binding and per-test results
vp run vite-svg-2-webfont#pack                   # build the Vite plugin
vp run @atlowchemi/webfont-generator#build       # build the native addon
vp run @atlowchemi/webfont-generator#check       # Clippy (default, CLI, NAPI) and rustfmt checks
vp run @atlowchemi/webfont-generator#test        # Rust checks and tests
vp run @atlowchemi/webfont-generator#bench       # run Rust Criterion benchmarks
vp run @atlowchemi/vite-svg-webfont-docs#dev     # docs dev server
vp run @atlowchemi/vite-svg-webfont-docs#build   # build docs
vp run example#dev                               # run example app
```

### Regenerating test fixtures

After adding, removing, or modifying SVG icons in the plugin's fixture directory (`packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/`), regenerate the expected font fixtures:

```bash
vp run vite-svg-2-webfont#test:fixtures:refresh
```

### Benchmarks

The repository has two benchmark layers:

- Rust Criterion benchmarks in `crates/webfont-generator/benches/` isolate native generator internals, pipeline stages, incremental regeneration, output formats, templates, write paths, and scaling behavior.
- Vitest benchmarks in `tests/webfonts-generator.bench.ts` exercise the JavaScript-facing API and compare against upstream behavior through the Node/NAPI boundary.

Run Rust benchmarks through the package Vite+ task:

```bash
vp run @atlowchemi/webfont-generator#bench
```

Use compile-only validation while editing benchmark targets or benchmark-only support code:

```bash
vp run @atlowchemi/webfont-generator#bench --no-run
```

Run targeted Criterion filters when investigating a specific path, for example:

```bash
vp run @atlowchemi/webfont-generator#bench --bench pipeline -- pipeline_stages/woff2_output_only/300
```

Run JavaScript-facing Vitest benchmarks with:

```bash
vp run bench:vitest
```

This task builds the optimized release binding, including its existing Rust check/test dependencies, and runs the benchmarks once with fresh measurements. The verbose reporter prints each comparison and its results as it completes. Skipped comparisons are hidden when filtering; individual iterations within a running comparison are not printed.

For a focused comparison, use Vitest's long-form filter flag (`-t` is a Vite+ task-runner option):

```bash
vp run bench:vitest --testNamePattern='batched vs separate content edits — 600 glyphs'
```

Use this task when collecting timings: `vp run test` builds a debug binding, which can make expensive benchmarks exceed their timeout. The benchmark task rebuilds the release binding before measuring. Vitest v5 sampling defaults apply except where a scenario specifies its own options; a sampling time is a minimum measurement window, not a maximum runtime.

Benchmark CI also uses verbose reporting and writes a JSON report for historical comparisons.

The Rust benchmarks prefer Iconify JSON fixtures from the workspace `node_modules` when available and fall back to deterministic synthetic SVGs. Keep this fallback so `cargo bench` still works after a clean Rust-only checkout, but use `vp install` first when collecting numbers intended for comparison.

## Tools

- `vp` is the entry point for the development workflow in this repository.
- `oxlint` is the main linting tool and is run through `vp lint` and `vp check`.
- `oxlint` is configured for type-aware linting and type checking through Vite+, powered by `tsgo`.
- `oxfmt` is the formatter and is run through `vp fmt` and `vp check`.
- `vitest` is used for unit tests and is run through `vp test`.

## Project Structure

This monorepo has a root Cargo workspace and the following packages:

- `packages/vite-svg-2-webfont/`: the Vite plugin — source code, tests, and build config
- `crates/webfont-generator/`: published Rust engine and CLI; its private npm manifest connects the task graph
- `packages/webfont-generator/`: `@atlowchemi/webfont-generator` — npm API and unpublished `webfont-generator-napi` adapter crate
- `packages/example/`: Vite app used for local development and manual verification
- `packages/docs/`: VitePress documentation site, published to GitHub Pages
- `tests/`: cross-package compatibility tests and benchmarks (at root level)

Cargo uses the root `Cargo.lock` and `target/`. Templates are tracked solely under
`packages/webfont-generator/templates/` and ship directly in the npm package. Rust rendering
parity tests read those files through `test_helpers::npm_template`; they require a repository
checkout. The library and CLI do not need those files for default rendering.
Release Please links engine/adapter versions. The engine has its own changelog at
`crates/webfont-generator/CHANGELOG.md` and GitHub releases tagged `webfont-engine-v*`;
those releases trigger crates.io publication. The npm adapter retains its own changelog
and `webfont-generator-v*` releases. npm/crates.io identities are unchanged, and the
engine's private npm task package is never published.

Release-PR preparation updates Cargo.lock, runs
`vp run @atlowchemi/webfont-generator#binding:regenerate`, and formats changes with `vp fmt`.
The regeneration task uses the NAPI generator to update only the JS loader, preserving
exports from the last full binding build without compiling Rust. Changes to the native API
still require the normal binding build to refresh both exports and TypeScript declarations.

## Pull Requests

Before opening a pull request, please:

1. Install dependencies with `vp install`.
2. Build the plugin with `vp run vite-svg-2-webfont#pack`.
3. Run `vp check` for formatting, lint, and TypeScript checks.
4. Run `vp run test` when your change affects JavaScript or browser behavior; use `vp run coverage` to check non-browser JavaScript coverage.
5. Run `vp run @atlowchemi/webfont-generator#test` when changing Rust code; this includes Rust checks and tests. The root JavaScript test task does not run the Rust test suite.
6. Run `vp run @atlowchemi/webfont-generator#bench --no-run` when changing Rust benchmark targets or benchmark-only support code.
7. Run targeted Rust or Vitest benchmark filters when changing measured performance behavior.
8. Verify the example app with `vp run example#dev` or `vp run example#build` for user-facing changes.
9. Verify the docs site with `vp run @atlowchemi/vite-svg-webfont-docs#build` when you change site documentation or docs config.

## Commit Conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) format for commit messages and PR titles. This is required for automated changelog generation via release-please.

The format is:

```
type(scope): description
```

Common types:

- `feat` - A new feature
- `fix` - A bug fix
- `chore` - Maintenance tasks, dependency updates
- `docs` - Documentation changes
- `refactor` - Code restructuring without behavior changes
- `test` - Adding or updating tests
- `ci` - CI/CD configuration changes
- `perf` - Performance improvements

Examples:

```
feat: add support for custom font formats
fix(vite-svg-2-webfont): handle empty SVG directory gracefully
docs: update configuration reference
chore: bump dependencies
```

Scopes are optional; when used, they must match `scope-enum` in [`commitlint.config.js`](./commitlint.config.js). The `commit-msg` hook validates commit messages, and CI validates PR titles. You can validate either before submitting:

```bash
printf '%s\n' 'chore(benchmarks): show per-test progress locally and in CI' | vp exec commitlint
```

## Before You Commit

Commits should be created only after the code passes the repository checks.

The checked-in hooks in `.vite-hooks/` run:

- `pre-commit`: `vp staged`, then `vp test`. Staged-file tasks run `vp check --fix` and Rust formatting as configured in the root `vite.config.ts`.
- `commit-msg`: `vp exec commitlint --edit` to validate the message.

`vp install` invokes `vp config` to set up hooks. If Git already has a custom `core.hooksPath`, Vite+ may leave it in place; check your local hook configuration rather than assuming these hooks are active.

Run `vp run test` before committing to ensure the native binding is built. The hook's direct `vp test` command does not build it or run the Rust test suite, and hooks do not replace the change-specific checks above.

## Notes

- Keep lockfile changes in `pnpm-lock.yaml` when dependencies change.
- Do not commit `package-lock.json` files to the workspace.
- Use `vp` instead of calling `pnpm`, `vite`, `vitest`, `oxlint`, or `oxfmt` directly for normal repository workflows.
- The docs site is driven by Vite+ run tasks in `packages/docs/`, so prefer `vp run @atlowchemi/vite-svg-webfont-docs#dev` over direct `vitepress` commands.
