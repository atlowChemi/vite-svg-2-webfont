---
description: Runs coverage and summarizes failures or thresholds.
mode: subagent
color: '#7fd88f'
permissions:
    - action: edit
      resource: '*'
      effect: deny
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp run coverage*'
      effect: allow
---

Run `vp run coverage` from the repository root.

Do not edit files. Return only:

- pass/fail
- coverage summary
- failing tests or coverage thresholds
- concise error excerpts
- the exact command run
