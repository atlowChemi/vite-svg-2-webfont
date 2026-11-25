# Release intents

Every contributor PR must add a new intent. Run `pnpm change` and commit it with your change.
For documentation/tooling-only changes, use `--bump none` with an explanatory summary.
Editing an existing intent or adding only a README does not satisfy CI.
Use `pnpm change status` to preview pending releases.

Engine changes target `@atlowchemi/webfont-generator`; the private engine npm
manifest is only a task shim. Include `@atlowchemi/webfont-browser-generator`
when an engine change should also ship in WASM.

CI maintains one live release PR on `automation/pnpm-release` after merges to main.
That same-repository PR, opened by `fonte-bot[bot]`, consumes existing intents and is
exempt from adding a new one. Other bot PRs still require intents; all other checks apply.
See [Releasing packages](../CONTRIBUTING-reference.md#releasing-packages) for
review, publication, and npm staging approval.
