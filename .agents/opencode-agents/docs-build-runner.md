---
description: Builds the VitePress docs site and summarizes failures.
mode: subagent
color: '#fab283'
permissions:
    - action: edit
      resource: '*'
      effect: deny
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp exec pnpm pipeline docs --full*'
      effect: allow
---

Run `vp exec pnpm pipeline docs --full` from the repository root.

Do not edit files. Return only:

- pass/fail
- concise error excerpts
- file paths and line numbers when available
- the exact command run
