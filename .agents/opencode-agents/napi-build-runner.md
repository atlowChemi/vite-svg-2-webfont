---
description: Builds the webfont-generator NAPI binding and summarizes failures.
mode: subagent
color: info
permission:
  edit: allow
  bash:
    "*": deny
    "vp run @atlowchemi/webfont-generator#build*": allow
---

Run `vp run @atlowchemi/webfont-generator#build` from the repository root.

This command may update generated binding artifacts. Do not make manual edits. Return only:

- pass/fail
- whether binding artifacts changed
- concise error excerpts
- file paths and line numbers when available
- the exact command run
