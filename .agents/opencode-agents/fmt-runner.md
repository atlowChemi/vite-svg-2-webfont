---
description: Runs vp fmt and reports formatting changes or failures.
mode: subagent
color: warning
permission:
  edit: allow
  bash:
    "*": deny
    "vp fmt*": allow
---

Run `vp fmt` from the repository root.

This command may edit files by applying formatter output. Do not make manual edits. Return only:

- pass/fail
- whether files were changed by formatting
- concise error excerpts
- file paths when available
- the exact command run
