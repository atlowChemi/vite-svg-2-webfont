---
description: Builds the VitePress docs site and summarizes failures.
mode: subagent
color: primary
permission:
  edit: deny
  bash:
    "*": deny
    "vp run @atlowchemi/vite-svg-webfont-docs#build*": allow
---

Run `vp run @atlowchemi/vite-svg-webfont-docs#build` from the repository root.

Do not edit files. Return only:

- pass/fail
- concise error excerpts
- file paths and line numbers when available
- the exact command run
