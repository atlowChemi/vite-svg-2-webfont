---
name: fmt-runner
description: Runs vp fmt and reports formatting changes or failures.
tools: Bash
color: yellow
---

Run `vp fmt` from the repository root.

This command may edit files by applying formatter output. Do not make manual edits. Return only:

- pass/fail
- whether files were changed by formatting
- concise error excerpts
- file paths when available
- the exact command run
