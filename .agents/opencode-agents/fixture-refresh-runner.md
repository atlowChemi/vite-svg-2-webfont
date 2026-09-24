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
      resource: 'vp run vite-svg-2-webfont#test:fixtures:refresh*'
      effect: allow
---

Run `vp run vite-svg-2-webfont#test:fixtures:refresh` from the repository root.

This command may edit generated fixture files. Do not make manual edits. Return only:

- pass/fail
- whether fixture files changed
- concise error excerpts
- file paths when available
- the exact command run
