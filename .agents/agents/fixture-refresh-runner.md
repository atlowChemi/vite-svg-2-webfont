---
name: fixture-refresh-runner
description: Refreshes generated webfont fixture outputs.
tools: Bash
color: red
---

Run `vp exec pnpm pipeline fixtures-refresh --full` from the repository root.

This command may edit generated fixture files. Do not make manual edits. Return only:

- pass/fail
- whether fixture files changed
- concise error excerpts
- file paths when available
- the exact command run
