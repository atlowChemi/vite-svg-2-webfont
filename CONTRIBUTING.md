# Contributing

Thanks for contributing to `vite-svg-2-webfont`.

## Development Setup

This repository uses:

- Node `24.14.0` from `.node-version`
- Vite+ through the `vp` CLI for installs, checks, tests, builds, and task execution. See the [Vite+ docs](https://viteplus.dev/guide/) for more information about the toolchain and workflow, and how to install it.
- `pnpm` underneath the hood as the package manager, managed through `vp`
- `oxlint` for linting, type-aware checks, and type checking through Vite+ and `tsgo`
- `oxfmt` for formatting through Vite+
- A monorepo workspace with packages under `packages/`

Install dependencies from the repository root:

```bash
vp install
```

## Common Commands

Run these from the repository root:

```bash
vp check                                         # format, lint, TypeScript checks
vp run test                                      # run all tests
vp run coverage                                  # run tests with coverage
vp run vite-svg-2-webfont#pack                   # build the Vite plugin
vp run @atlowchemi/webfont-generator#build       # build the native addon
vp run @atlowchemi/vite-svg-webfont-docs#dev     # docs dev server
vp run @atlowchemi/vite-svg-webfont-docs#build   # build docs
vp run example#dev                               # run example app
```

### Regenerating test fixtures

After adding, removing, or modifying SVG icons in the plugin's fixture directory (`packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/`), regenerate the expected font fixtures:

```bash
vp run vite-svg-2-webfont#test:fixtures:refresh
```

## Tools

- `vp` is the entry point for the development workflow in this repository.
- `oxlint` is the main linting tool and is run through `vp lint` and `vp check`.
- `oxlint` is configured for type-aware linting and type checking through Vite+, powered by `tsgo`.
- `oxfmt` is the formatter and is run through `vp fmt` and `vp check`.
- `vitest` is used for unit tests and is run through `vp test`.

## Project Structure

This is a monorepo with the following packages under `packages/`:

- `packages/vite-svg-2-webfont/`: the Vite plugin — source code, tests, and build config
- `packages/webfont-generator/`: `@atlowchemi/webfont-generator` — Rust NAPI native addon
- `packages/example/`: Vite app used for local development and manual verification
- `packages/docs/`: VitePress documentation site, published to GitHub Pages

## Pull Requests

Before opening a pull request, please:

1. Install dependencies with `vp install`.
2. Build the plugin with `vp run vite-svg-2-webfont#pack`.
3. Run `vp run test` or `vp run coverage` when your change affects behavior.
4. Verify the example app with `vp run example#dev` or `vp run example#build` for user-facing changes.
5. Verify the docs site with `vp run @atlowchemi/vite-svg-webfont-docs#build` when you change documentation or docs config.

## Commit Conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) format for all commit messages. This is required for automated changelog generation via release-please.

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
fix(options): handle empty SVG directory gracefully
docs: update configuration reference
chore: bump dependencies
```

Commit messages are validated automatically by a `commitlint` hook on `commit-msg`. Messages that do not follow the conventional format will be rejected.

## Before You Commit

Commits should be created only after the code passes the repository checks.

- Formatting, linting, and tests are handled automatically by the configured Vite+ hooks.
- Commit messages are validated against the Conventional Commits format by the `commitlint` hook.
- You can still run `vp check` and `vp test` manually if you want an earlier check before committing.

In practice, assume formatting, linting, and unit tests are part of the validation flow for changes, with all three enforced during commit.

## Notes

- Keep lockfile changes in `pnpm-lock.yaml` when dependencies change.
- Do not commit `package-lock.json` files to the workspace.
- Use `vp` instead of calling `pnpm`, `vite`, `vitest`, `oxlint`, or `oxfmt` directly for normal repository workflows.
- The docs site is driven by Vite+ run tasks in `packages/docs/`, so prefer `vp run @atlowchemi/vite-svg-webfont-docs#dev` over direct `vitepress` commands.
