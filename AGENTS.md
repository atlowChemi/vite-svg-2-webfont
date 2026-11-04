# Agent Notes

## Tooling

- Use `pnpm install` and `pnpm pipeline NAME --full` for installation and task orchestration. Use `pnpm run check` / `pnpm run fmt` for standalone Oxc checks/formatting, and `pnpm exec vitest` for focused tests.
- Keep `--full`: use the conservative CI selector before invoking a pipeline, not pnpm's task-level affected selection. `pnpm run` scripts are raw commands without prerequisites; use them only when prerequisites are already supplied or for forwarding tool arguments.
- Import Vite APIs from `vite`, test APIs from `vitest`, and test configuration from `vitest/config`. Package bundling uses separate `tsdown.config.ts` files; include them in cached task inputs.
- After pulling remote changes, run `pnpm install` before validation.
- Keep `runtimeOnFail: error`: Rust is managed by rustup so installed cross-target libraries and LLVM tools match the compiler used inside pnpm scripts.
- Unified `pnpm install` requires Rust first. Use `pnpm add crate:NAME@RANGE` in the crate directory to update Rust dependencies; `pnpm update crate:…` does not update them in pnpm 12.6.0.
- Use Conventional Commit messages and PR titles: `type(scope): description` or `type: description`. Before creating or editing either, read `scope-enum` in `commitlint.config.js`; use a listed scope or omit it when none fits. Validate the proposed message/title with `printf '%s\n' 'your title' | pnpm exec commitlint` before committing or submitting it.

## Focused Commands

- Never manually modify or restore `packages/webfont-generator/binding.{js,d.ts}` or platform `.node` artifacts unless the user explicitly authorizes it. Build and dependent tasks may regenerate them; stage and commit those generated updates with the source changes that produced them.
- `pnpm pipeline native --full` builds the Rust/NAPI binding and can update `packages/webfont-generator/binding.{js,d.ts}` and platform `.node` artifacts.
- `pnpm pipeline binding-regenerate --full` regenerates only the JS loader for version updates, preserving the last full build's exports; API changes still require the `native` pipeline.
- After changing Rust code, run `pnpm pipeline rust-check --full`; it checks the engine's default/CLI builds, the separate NAPI adapter, and workspace Rust formatting.
- `pnpm pipeline rust-test --full` runs Rust checks/tests; `pnpm pipeline test --full` builds the NAPI/browser packages, then runs JS/Vitest tests.
- `pnpm pipeline coverage --full` measures JS only. Rust coverage tasks require `cargo-llvm-cov`, `cargo-nextest`, and `llvm-tools-preview`; see CONTRIBUTING.md. `native-coverage` instruments Node execution; keep its addon override package-scoped. Nextest omits doctests, so retain the `doctests` pipeline separately.
- `pnpm --dir packages/webfont-generator run bench --no-run` is the compile-only check for Rust benchmark target changes; run targeted Criterion filters only when measured behavior changes.
- `pnpm pipeline docs --full` builds docs and their asset/package prerequisites.
- `pnpm pipeline fixtures-refresh --full` regenerates expected font fixtures after changing SVG icons in `packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/`.

## Public API Sync

- Never change any `CHANGELOG.md` file without explicit user permission for that change. This includes creating, editing, deleting, renaming, regenerating, or formatting it, directly or through tools. General implementation, documentation, commit, or release requests do not grant permission.
- When changing generator public APIs, options, CLI flags, or exported types, update Rust doc comments, the affected tool pages in `packages/docs/src/pages/` (`node/`, `rust/`, `cli/`, `browser/`; shared concepts in `guide/`; option descriptions every tool shares in `packages/docs/src/includes/options/`), and both engine/adapter READMEs together.
- Keep templates in `packages/webfont-generator/templates/`; shipped-template parity belongs in adapter tests. Engine tests use engine-owned inputs, never downstream package assets.
- Docs changelog pages include their source files: npm history from `packages/webfont-generator/CHANGELOG.md`, engine history from `crates/webfont-generator/CHANGELOG.md`. Do not copy release entries into docs.

## Verification Subagents

- Prefer verification subagents for long/noisy commands so logs stay out of the main context.
- Use `check-runner` for `pnpm run check`.
- Use `fmt-runner` for isolated `pnpm run fmt` runs.
- Use `rust-check-runner` after changing Rust code.
- Use `test-runner` for `pnpm pipeline test --full`.
- Use `coverage-runner` for `pnpm pipeline coverage --full`.
- Use `napi-build-runner` for `pnpm pipeline native --full`.
- Use `bench-build-runner` for `pnpm --dir packages/webfont-generator run bench --no-run`.
- Use `docs-build-runner` for `pnpm pipeline docs --full`.
- Use `fixture-refresh-runner` for `pnpm pipeline fixtures-refresh --full`.
- Run independent verification subagents in parallel and have them return pass/fail, concise failure excerpts, file paths/line numbers, generated-file changes, and the exact command run.
