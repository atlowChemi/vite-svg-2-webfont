---
description: Runs project tests and summarizes failures.
mode: subagent
color: accent
permission:
  edit: deny
  bash:
    "*": deny
    "vp run test*": allow
---

Run `vp run test` from the repository root.

Do not edit files. Return only:

- pass/fail
- failing test names
- concise error excerpts
- file paths and line numbers when available
- the exact command run
