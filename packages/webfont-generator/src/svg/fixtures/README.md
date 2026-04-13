# SVG Fixtures

This directory contains the Rust SVG generator test fixtures used by [`tests.rs`](/Users/chemiatlow/Documents/repos/vite-svg-2-webfont/packages/webfont-generator/src/svg/tests.rs).

Layout:

- [`icons/`](/Users/chemiatlow/Documents/repos/vite-svg-2-webfont/packages/webfont-generator/src/svg/fixtures/icons)
    - source SVG inputs used by the Rust tests
- [`expected/`](/Users/chemiatlow/Documents/repos/vite-svg-2-webfont/packages/webfont-generator/src/svg/fixtures/expected)
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
    UPDATE_SVG_FIXTURES=1 cargo test --manifest-path packages/webfont-generator/Cargo.toml
    ```
2. Re-run the native crate tests normally to verify the snapshots:
    ```sh
    cargo test --manifest-path packages/webfont-generator/Cargo.toml
    ```
3. Rebuild the native NAPI addon and run the full JS test suite:
    ```sh
    vp run native:build
    vp run test
    ```

Other useful commands:

| Command                                              | Description                                   |
| ---------------------------------------------------- | --------------------------------------------- |
| `vp run @atlowchemi/webfont-generator#build`         | Build the native NAPI addon (dev profile)     |
| `vp run @atlowchemi/webfont-generator#build:release` | Build the native NAPI addon (release profile) |
| `vp run @atlowchemi/webfont-generator#test`          | Run all RS tests                              |
| `vp run test`                                        | Run all JS tests (includes compat suite)      |
| `vp test bench`                                      | Run Vitest benchmarks (upstream vs new core)  |

When `UPDATE_SVG_FIXTURES=1` is set, the Rust tests rewrite every matching file in `expected/` from the current implementation before asserting.
The tests read inputs from `icons/` plus the small repo fixtures in `src/fixtures/webfont-test/svg`.

Fixture scope today:

- upstream-derived input sets we want for SVG behavior coverage
- repo-specific cases such as explicit codepoints, ligatures, metadata, custom font metrics, and optimize-output behavior

The goal is simple: stable inputs in `icons/`, stable exact outputs in `expected/`, and byte-for-byte assertions in Rust tests.
