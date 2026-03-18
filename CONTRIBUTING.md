# Contributing

Thanks for contributing to `vite-svg-2-webfont`.

## Development Setup

This repository uses:

- Node `24.14.0` from `.nvmrc`
- Vite+ through the `vp` CLI for installs, checks, tests, builds, and task execution. See the [Vite+ docs](https://vite.plus) for more information about the toolchain and workflow, and how to install it.
- `pnpm` underneath the hood as the package manager, managed through `vp`
- `oxlint` for linting, type-aware checks, and type checking through Vite+ and `tsgo`
- `oxfmt` for formatting through Vite+
- A workspace containing the library package and the `example` app

Install dependencies from the repository root:

```bash
vp install
```

## Common Commands

Run these from the repository root:

```bash
vp run pack
vp check
vp lint
vp fmt
vp run test
vp run coverage
```

Run the example app:

```bash
vp run example#dev
```

Build the example app:

```bash
vp run example#build
```

Preview the example production build:

```bash
vp run example#preview
```

## Tools

- `vp` is the entry point for the development workflow in this repository.
- `oxlint` is the main linting tool and is run through `vp lint` and `vp check`.
- `oxlint` is configured for type-aware linting and type checking through Vite+, powered by `tsgo`.
- `oxfmt` is the formatter and is run through `vp fmt` and `vp check`.
- `vitest` is used for unit tests and is run through `vp test`.

## Project Structure

- `src/`: plugin source code and tests
- `example/`: Vite app used for local development and manual verification
- `dist/`: generated build output

## Pull Requests

Before opening a pull request, please:

1. Install dependencies with `vp install`.
2. Run `vp pack`.
3. Run `vp run test` or `vp run coverage` when your change affects behavior.
4. Verify the example app with `vp run example#dev` or `vp run example#build` for user-facing changes.

## Before You Commit

Commits should be created only after the code passes the repository checks.

- Formatting, linting, and tests are handled automatically by the configured Vite+ hooks.
- You can still run `vp check` and `vp test` manually if you want an earlier check before committing.

In practice, assume formatting, linting, and unit tests are part of the validation flow for changes, with all three enforced during commit.

## Notes

- Keep lockfile changes in `pnpm-lock.yaml` when dependencies change.
- Do not commit `package-lock.json` files to the workspace.
- Use `vp` instead of calling `pnpm`, `vite`, `vitest`, `oxlint`, or `oxfmt` directly for normal repository workflows.
