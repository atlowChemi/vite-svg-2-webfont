# fspy mutation tracking

`fspy-mutations.patch` applies to upstream revision
`8db455ef9c2b1c7b33cf70d8ba370c1f2c49d4c9` (MIT licensed).
It extends the Unix preload and Linux syscall paths with hard-link, rename,
and deletion access reporting. Source reads and destination writes use the
existing fspy record format, including attempted operations that fail.

The package build script checks out that exact revision and applies this patch
in a disposable, patch-content-addressed directory under `target`. Cargo uses
local overrides for the three changed crates; all upstream sources stay pinned.
This keeps the maintained delta reviewable without vendoring the whole upstream
workspace or relying on an unpublished fork commit. Use the package scripts to
build/test; bare Cargo does not apply the patch.

The regression fixture covers links, rename/exchange, deletion, relative directory
descriptors, direct Linux syscalls, and dynamically/static linked executables.
This is filesystem-access evidence, not a claim to intercept every Linux syscall.
