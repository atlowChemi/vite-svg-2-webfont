# SVG Fixtures

This directory contains the Rust SVG generator test fixtures used by [`tests.rs`](../tests.rs).

Layout:

- [`icons/`](./icons/)
    - source SVG inputs used by the Rust tests
- [`expected/`](./expected/)
    - exact SVG font snapshots produced by the current Rust implementation

How fixtures should be populated:

- `icons/` should contain only the SVG input sets we actively want to test.
- `expected/` should contain repo-owned snapshots generated from this Rust implementation for those same cases.
- Inputs may originally come from upstream projects, but once copied here they should be treated as native test fixtures.
- We do not keep upstream `src/tests` copies here.
- If generator behavior intentionally changes, regenerate the matching files in `expected/` and review the diffs like any other snapshot update.

How to regenerate `expected/`:

1. Run the Rust crate tests with snapshot updates enabled:
    ```sh
    UPDATE_SVG_FIXTURES=1 pnpm pipeline rust-test --full --no-cache
    ```
2. Re-run the native crate tests normally to verify the snapshots:
    ```sh
    pnpm pipeline rust-test --full --no-cache
    ```
3. Rebuild the native NAPI addon and run the full JS test suite:
    ```sh
    pnpm pipeline native --full
    pnpm pipeline test --full
    ```

Other useful commands:

| Command                               | Description                                   |
| ------------------------------------- | --------------------------------------------- |
| `pnpm pipeline native --full`         | Build the native NAPI addon (dev profile)     |
| `pnpm pipeline native-release --full` | Build the native NAPI addon (release profile) |
| `pnpm pipeline rust-test --full`      | Run all RS tests                              |
| `pnpm pipeline test --full`           | Run all JS tests (includes compat suite)      |
| `pnpm pipeline engine-bench --full`   | Run Rust Criterion benchmarks                 |
| `pnpm pipeline bench --full`          | Run Vitest benchmarks (upstream vs new core)  |

When `UPDATE_SVG_FIXTURES=1` is set, the Rust tests rewrite every matching file in `expected/` from the current implementation before asserting.
The tests read inputs from `icons/` and `unit/`. The small `unit/` fixtures are engine-owned copies of the original plugin examples so the engine's snapshots do not depend on downstream packages. They can evolve independently of the plugin fixtures.

Fixture scope today:

- upstream-derived input sets we want for SVG behavior coverage
- repo-specific cases such as explicit codepoints, ligatures, metadata, custom font metrics, and optimize-output behavior

The goal is simple: stable inputs in `icons/`, stable exact outputs in `expected/`, and byte-for-byte assertions in Rust tests.
