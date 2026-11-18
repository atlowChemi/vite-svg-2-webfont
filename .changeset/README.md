# Release intents

Run `pnpm change` and commit the generated intent with your change.
Use `pnpm change status` to preview pending releases.

Engine changes target `@atlowchemi/webfont-generator`; the private engine npm
manifest is only a task shim. Include `@atlowchemi/webfont-browser-generator`
when an engine change should also ship in WASM.

Maintainers prepare a release PR with `pnpm run release:prepare`.
See [Releasing packages](../CONTRIBUTING-reference.md#releasing-packages) for
review, publication, and npm staging approval.
