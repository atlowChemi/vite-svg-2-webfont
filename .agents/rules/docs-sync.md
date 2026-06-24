# Documentation Sync Rule

When changing `@atlowchemi/webfont-generator` public APIs, options, CLI flags, exported types, or generated bindings, keep these in sync in the same change:

- Rust doc comments in `packages/webfont-generator/src/`.
- User-facing docs in `packages/docs/webfont-generator/`.
- Package README at `packages/webfont-generator/README.md`.

Do not duplicate the webfont-generator changelog in docs; `packages/docs/webfont-generator/changelog.md` includes `packages/webfont-generator/CHANGELOG.md`.
