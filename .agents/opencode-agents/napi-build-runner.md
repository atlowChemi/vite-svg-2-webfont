---
description: Builds the webfont-generator NAPI binding and summarizes failures.
mode: subagent
color: '#56b6c2'
permissions:
    - action: edit
      resource: '*'
      effect: allow
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp exec pnpm pipeline native --full*'
      effect: allow
---

Run `vp exec pnpm pipeline native --full` from the repository root.

This command may update generated binding artifacts. Do not make manual edits. Return only:

- pass/fail
- whether binding artifacts changed
- concise error excerpts
- file paths and line numbers when available
- the exact command run
