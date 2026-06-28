# Fresh Worktree Setup Rule

When creating or switching into a fresh git worktree, run `vp i` before any build, test, benchmark, docs, or fixture task.

If benchmarks fail because the default icon set/path is missing, treat the worktree as not installed; do not override `BENCH_ICON_SET` to a smaller/different icon set unless the task explicitly asks to benchmark that set.
