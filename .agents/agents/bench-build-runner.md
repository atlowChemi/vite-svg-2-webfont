---
name: bench-build-runner
description: Builds Rust benchmark targets without running benchmarks.
tools: Bash
color: orange
---

Run `pnpm --dir packages/webfont-generator run bench --no-run` from the repository root.

Do not edit files. Return only:

- pass/fail
- concise error excerpts
- file paths and line numbers when available
- the exact command run
