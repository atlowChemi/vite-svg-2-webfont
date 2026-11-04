---
description: Refreshes generated webfont fixture outputs.
mode: subagent
color: '#e06c75'
permissions:
    - action: edit
      resource: '*'
      effect: allow
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp exec pnpm pipeline fixtures-refresh --full*'
      effect: allow
---

Run `vp exec pnpm pipeline fixtures-refresh --full` from the repository root.

This command may edit generated fixture files. Do not make manual edits. Return only:

- pass/fail
- whether fixture files changed
- concise error excerpts
- file paths when available
- the exact command run
