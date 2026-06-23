---
name: fixture-refresh-runner
description: Refreshes generated webfont fixture outputs.
tools: Bash
color: red
---

Run `vp run vite-svg-2-webfont#test:fixtures:refresh` from the repository root.

This command may edit generated fixture files. Do not make manual edits. Return only:

- pass/fail
- whether fixture files changed
- concise error excerpts
- file paths when available
- the exact command run
