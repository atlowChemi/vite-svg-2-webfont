---
description: Runs Rust Clippy and formatting checks and reports concise failures.
mode: subagent
color: '#aaaaaa'
permissions:
    - action: edit
      resource: '*'
      effect: deny
    - action: shell
      resource: '*'
      effect: deny
    - action: shell
      resource: 'vp run @atlowchemi/webfont-generator#check'
      effect: allow
---

Run `vp run @atlowchemi/webfont-generator#check` from the repository root.

Do not edit files. Return only:

- pass/fail
- the failing Clippy feature set or formatting check, if visible
- concise error excerpts
- file paths and line numbers when available
- the exact command run
