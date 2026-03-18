# Contributing

Thanks for contributing to `vite-svg-2-webfont`.

## Development Setup

This repository uses:

- Node `24.14.0` from `.nvmrc`
- `pnpm` as the package manager
- `oxlint` for linting, type-aware checks, and type checking through `oxlint-tsgolint` (`tsgo`)
- `oxfmt` for formatting
- A workspace containing the library package and the `example` app

Install dependencies from the repository root:

```bash
pnpm install
```

## Common Commands

Run these from the repository root:

```bash
pnpm build
pnpm lint
pnpm format
pnpm format-check
pnpm test
pnpm coverage
```

Run the example app against the local workspace package:

```bash
pnpm example:dev
```

Build the example app:

```bash
pnpm example:build
```

Preview the example production build:

```bash
pnpm example:preview
```

## Tools

- `oxlint` is the main linting tool used in this repository.
- `oxlint` is also configured with `oxlint-tsgolint`, so it performs type-aware checking using `tsgo` in addition to regular lint rules.
- `oxfmt` is the formatter used for code style checks and formatting updates.
- `vitest` is used for unit tests.

## Project Structure

- `src/`: plugin source code and tests
- `example/`: Vite app used for local development and manual verification
- `dist/`: generated build output

## Pull Requests

Before opening a pull request, please:

1. Install dependencies with `pnpm install`.
2. Run `pnpm build`.
3. Run `pnpm test` or `pnpm coverage` when your change affects behavior.
4. Verify the example app with `pnpm example:dev` or `pnpm example:build` for user-facing changes.

## Before You Commit

Commits should be created only after the code passes the repository checks.

- Linting, formatting, and tests are handled automatically by the Husky pre-commit hooks.
- You can still run `pnpm lint`, `pnpm format-check`, and `pnpm test` manually if you want an earlier check before committing.

In practice, assume formatting, linting, and unit tests are part of the validation flow for changes, with all three enforced during commit.

## Notes

- Keep lockfile changes in `pnpm-lock.yaml` when dependencies change.
- Do not commit `package-lock.json` files to the workspace.
- Husky is installed through the root `prepare` script.
