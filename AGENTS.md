# Agent Notes

## Tooling

- Use `vp` for normal repo workflows: `vp install`, `vp check`, `vp fmt`, `vp run test`, `vp run coverage`, `vp run <package>#<task>`. Do not call `pnpm`, `vite`, `vitest`, `oxlint`, `oxfmt`, or `vitepress` directly unless a checked-in Vite+ task itself does so.
- Import Vite/Vitest APIs from `vite-plus` (`vite-plus` or `vite-plus/tests`), not from direct `vite` or `vitest` packages.
- After pulling remote changes, run `vp install` before validation.
- Use Conventional Commit messages if the user asks you to commit; `commitlint` enforces `type(scope): description`.

## Focused Commands

- `vp run @atlowchemi/webfont-generator#build` builds the Rust/NAPI binding and can update `packages/webfont-generator/binding.{js,d.ts}` and platform `.node` artifacts.
- `vp run @atlowchemi/webfont-generator#test` runs Rust checks/tests via the package task; workspace `vp run test` first depends on the NAPI build, then runs JS/Vitest tests.
- `vp run @atlowchemi/webfont-generator#bench --no-run` is the compile-only check for Rust benchmark target changes; run targeted Criterion filters only when measured behavior changes.
- `vp run @atlowchemi/vite-svg-webfont-docs#build` is the docs build task; it runs the docs `social-card` and `optimize-svg` dependencies.
- `vp run vite-svg-2-webfont#test:fixtures:refresh` regenerates expected font fixtures after changing SVG icons in `packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/`.

## Public API Sync

- When changing `@atlowchemi/webfont-generator` public APIs, options, CLI flags, or exported types, update Rust doc comments, `packages/docs/webfont-generator/`, and `packages/webfont-generator/README.md` together.
- Do not duplicate the webfont-generator changelog in docs; `packages/docs/webfont-generator/changelog.md` includes `packages/webfont-generator/CHANGELOG.md`.

## Verification Subagents

- Prefer verification subagents for long/noisy commands so logs stay out of the main context.
- Use `vp-check` for `vp check`.
- Use `fmt-runner` for isolated `vp fmt` runs.
- Use `test-runner` for `vp run test`.
- Use `coverage-runner` for `vp run coverage`.
- Use `napi-build-runner` for `vp run @atlowchemi/webfont-generator#build`.
- Use `bench-build-runner` for `vp run @atlowchemi/webfont-generator#bench --no-run`.
- Use `docs-build-runner` for `vp run @atlowchemi/vite-svg-webfont-docs#build`.
- Use `fixture-refresh-runner` for `vp run vite-svg-2-webfont#test:fixtures:refresh`.
- Run independent verification subagents in parallel and have them return pass/fail, concise failure excerpts, file paths/line numbers, generated-file changes, and the exact command run.
