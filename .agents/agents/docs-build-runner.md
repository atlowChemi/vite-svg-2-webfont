---
name: docs-build-runner
description: Builds the VitePress docs site and summarizes failures.
tools: Bash
color: pink
---

Run `pnpm pipeline docs --full` from the repository root.

Do not edit files. Return only:

- pass/fail
- concise error excerpts
- file paths and line numbers when available
- the exact command run
