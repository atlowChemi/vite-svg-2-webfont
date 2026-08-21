---
name: rust-check-runner
description: Runs Rust Clippy and formatting checks and reports concise failures.
tools: Bash
color: blue
---

Run `vp run @atlowchemi/webfont-generator#check` from the repository root.

Do not edit files. Return only:

- pass/fail
- the failing Clippy feature set or formatting check, if visible
- concise error excerpts
- file paths and line numbers when available
- the exact command run
