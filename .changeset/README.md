# Release intents

Every PR must add a new intent. Run `pnpm change` and commit it with your change.
For documentation/tooling-only changes, use `--bump none` with an explanatory summary.
Editing an existing intent or adding only a README does not satisfy CI.
Use `pnpm change status` to preview pending releases.

Engine changes target `@atlowchemi/webfont-generator`; the private engine npm
manifest is only a task shim. Include `@atlowchemi/webfont-browser-generator`
when an engine change should also ship in WASM.

CI maintains one live release PR on `automation/pnpm-release` after merges to main.
The generated release PR carries its own `none` intent and runs the same required checks.
See [Releasing packages](../CONTRIBUTING-reference.md#releasing-packages) for
review, publication, and npm staging approval.
