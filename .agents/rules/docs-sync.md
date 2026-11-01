# Documentation Sync Rule

When changing `@atlowchemi/webfont-generator` public APIs, options, CLI flags, exported types, or generated bindings, keep these in sync in the same change:

- Rust doc comments in `crates/webfont-generator/src/` and `packages/webfont-generator/native/`.
- User-facing docs in `packages/docs/`: each tool's own section (`node/`, `rust/`, `cli/`, `browser/`), and `guide/` for concepts shared by every tool.
- Package READMEs at `crates/webfont-generator/README.md` and `packages/webfont-generator/README.md`.

Do not duplicate the webfont-generator changelog in docs; `packages/docs/node/changelog.md` includes `packages/webfont-generator/CHANGELOG.md`.
