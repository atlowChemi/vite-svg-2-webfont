# Contributing

Thanks for helping improve `vite-svg-2-webfont`. You only need to run the checks relevant to your change; CI handles the platform matrix.

## Set up once

Install:

- **Node.js:** the version in [`.node-version`](./.node-version).
- **pnpm:** the version in [`package.json`](./package.json), using the [pnpm installation instructions](https://pnpm.io/installation).
- **Rust via [rustup](https://rustup.rs/):** the repository selects its pinned toolchain and components automatically. Install Rust before dependencies, since pnpm installs both JavaScript and Rust dependencies.

From the repository root:

```sh
pnpm install
```

Rerun this after pulling dependency changes.

For JavaScript/browser tests or the docs site, also install the browsers:

```sh
pnpm exec playwright install chromium firefox webkit
```

On Linux, add `--with-deps` to install their system dependencies.

## Find your code

| What you want to change      | Where to look                      |
| ---------------------------- | ---------------------------------- |
| Vite plugin                  | `packages/vite-svg-2-webfont/src/` |
| Font generation or Rust CLI  | `crates/webfont-generator/`        |
| Node.js API / native adapter | `packages/webfont-generator/`      |
| Browser / WASM API           | `packages/webfont-browser/`        |
| Documentation                | `packages/docs/src/pages/`         |
| Example app                  | `packages/example/`                |

Tests live alongside the code or in the package's `tests/` directory.

To work on the site or example app:

```sh
pnpm pipeline docs-dev --full
pnpm pipeline example-dev --full
```

Choose one. These commands build their prerequisites before starting the dev server.

## Check your change

Run formatting, lint, and TypeScript checks from the root:

```sh
pnpm run check
```

Then choose the relevant checks below. Pipelines build their own prerequisites; you do not need to build the plugin separately first.

| Your change                         | Run                                 |
| ----------------------------------- | ----------------------------------- |
| Docs content or site configuration  | `pnpm pipeline docs --full`         |
| JavaScript / TypeScript behavior    | `pnpm pipeline test --full`         |
| Rust engine, CLI, or native adapter | `pnpm pipeline rust-test --full`    |
| Browser / WASM adapter              | `pnpm pipeline browser-test --full` |
| Example app                         | `pnpm pipeline example --full`      |

For changes across language boundaries, run both relevant suites. Run build/test pipelines sequentially in the same checkout because they share generated bindings.

Add or update a focused test for behavior changes. Let builds regenerate native bindings; include generated binding changes with the source changes that produced them.

Need a specialized check? See [coverage](./CONTRIBUTING-reference.md#running-coverage-locally), [benchmarks](./CONTRIBUTING-reference.md#benchmarks), or [fixture regeneration](./CONTRIBUTING-reference.md#regenerating-test-fixtures).

## Open a pull request

1. Describe what changed and why; link a related issue if there is one.
2. Mention the checks you ran. You do not need every platform or coverage tool installed locally.
3. Use a [Conventional Commit](https://www.conventionalcommits.org/) title, for example `fix: handle empty SVG directories` or `docs: clarify browser setup`. Scopes are optional; allowed scopes are in [`commitlint.config.js`](./commitlint.config.js).
4. Review CI results and address failures before merging.

Commits use the same title convention. Installation enables hooks that format/check staged files and validate commit messages. **The pre-commit hook does not run the full test suite**; use the checks above before opening your PR.

## When you need more detail

The [contributor reference](./CONTRIBUTING-reference.md) contains the full command list, task graphs, cache troubleshooting, CI selection, and release procedures. The [cache auditor README](./crates/cache-audit/README.md) is for work on build infrastructure.
