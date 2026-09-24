---
description: Builds Rust benchmark targets without running benchmarks.
mode: subagent
color: '#5c9cf5'
permissions:
    - action: edit
      resource: '*'
      effect: deny
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp run @atlowchemi/webfont-generator#bench --no-run*'
      effect: allow
---

Run `vp run @atlowchemi/webfont-generator#bench --no-run` from the repository root.

Do not edit files. Return only:

- pass/fail
- concise error excerpts
- file paths and line numbers when available
- the exact command run
