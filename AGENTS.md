# Agent Notes

## Tooling

- Use `vp` for normal repo workflows: `vp install`, `vp check`, `vp fmt`, `vp run test`, `vp run coverage`, `vp run <package>#<task>`. Do not call `pnpm`, `vite`, `vitest`, `oxlint`, `oxfmt`, or `vitepress` directly unless a checked-in Vite+ task itself does so.
- Import Vite/Vitest APIs from `vite-plus` (`vite-plus` or `vite-plus/tests`), not from direct `vite` or `vitest` packages.
- After pulling remote changes, run `vp install` before validation.
- Use Conventional Commit messages and PR titles: `type(scope): description` or `type: description`. Before creating or editing either, read `scope-enum` in `commitlint.config.js`; use a listed scope or omit it when none fits. Validate the proposed message/title with `printf '%s\n' 'your title' | vp exec commitlint` before committing or submitting it.

## Focused Commands

- Never manually modify or restore `packages/webfont-generator/binding.{js,d.ts}` or platform `.node` artifacts unless the user explicitly authorizes it. Build and dependent tasks may regenerate them; stage and commit those generated updates with the source changes that produced them.
- `vp run @atlowchemi/webfont-generator#build` builds the Rust/NAPI binding and can update `packages/webfont-generator/binding.{js,d.ts}` and platform `.node` artifacts.
- `vp run @atlowchemi/webfont-generator#binding:regenerate` regenerates only the JS loader for version updates, preserving the last full build's exports; API changes still require `#build`.
- After changing Rust code, run `vp run @atlowchemi/webfont-generator#check`; it checks the engine's default/CLI builds, the separate NAPI adapter, and workspace Rust formatting.
- `vp run @atlowchemi/webfont-generator#test` runs Rust checks/tests via the package task; workspace `vp run test` first depends on the NAPI build, then runs JS/Vitest tests.
- `vp run coverage` measures JS only. Rust coverage tasks require `cargo-llvm-cov`, `cargo-nextest`, and `llvm-tools-preview`; see CONTRIBUTING.md for engine/CLI/adapter task commands. `@atlowchemi/webfont-generator#test:coverage:native` instruments Node execution; keep its addon override package-scoped. Nextest omits doctests, so retain `@atlowchemi/webfont-engine#test:doctests` separately.
- `vp run @atlowchemi/webfont-generator#bench --no-run` is the compile-only check for Rust benchmark target changes; run targeted Criterion filters only when measured behavior changes.
- `vp run @atlowchemi/vite-svg-webfont-docs#build` is the docs build task; it runs the docs `social-card` and `optimize-svg` dependencies.
- `vp run vite-svg-2-webfont#test:fixtures:refresh` regenerates expected font fixtures after changing SVG icons in `packages/vite-svg-2-webfont/src/fixtures/webfont-test/svg/`.

## Public API Sync

- When changing generator public APIs, options, CLI flags, or exported types, update Rust doc comments, `packages/docs/webfont-generator/`, and both engine/adapter READMEs together.
- Keep templates in `packages/webfont-generator/templates/`; Rust rendering parity tests read them through `test_helpers::npm_template`.
- Docs changelog pages include their source files: npm history from `packages/webfont-generator/CHANGELOG.md`, engine history from `crates/webfont-generator/CHANGELOG.md`. Do not copy release entries into docs.

## Verification Subagents

- Prefer verification subagents for long/noisy commands so logs stay out of the main context.
- Use `vp-check` for `vp check`.
- Use `fmt-runner` for isolated `vp fmt` runs.
- Use `rust-check-runner` after changing Rust code.
- Use `test-runner` for `vp run test`.
- Use `coverage-runner` for `vp run coverage`.
- Use `napi-build-runner` for `vp run @atlowchemi/webfont-generator#build`.
- Use `bench-build-runner` for `vp run @atlowchemi/webfont-generator#bench --no-run`.
- Use `docs-build-runner` for `vp run @atlowchemi/vite-svg-webfont-docs#build`.
- Use `fixture-refresh-runner` for `vp run vite-svg-2-webfont#test:fixtures:refresh`.
- Run independent verification subagents in parallel and have them return pass/fail, concise failure excerpts, file paths/line numbers, generated-file changes, and the exact command run.
