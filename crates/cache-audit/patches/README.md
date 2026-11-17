# fspy mutation tracking

`fspy-mutations.patch` applies to upstream revision
`2011a0b9decc8ac330b2415d6dba6f3e0749721d` (MIT licensed).
It extends the Unix preload and Linux syscall paths with hard-link, rename,
and deletion access reporting. Source reads and destination writes use the
existing fspy record format, including attempted operations that fail.
Upstream now records rename destinations and hard-link writes; the patch retains
the auditor's source reads, rename-source deletions, exchange writes on both sides,
and unlink/rmdir reporting. Upstream mkdir and symlink tracking remains enabled.

The root preinstall hook checks out that exact revision and applies this patch
under `.pnpm/cache-audit-fspy`, verifying the revision and complete patched diff.
Root Cargo configuration points `fspy` at that source for installs and builds;
its transitive workspace crates use the same patched checkout.
This keeps the maintained delta reviewable without vendoring the whole upstream
workspace or relying on an unpublished fork commit. After `pnpm install`, ordinary
Cargo commands work too. The manifest patch explicitly identifies fspy's upstream
workspace so Cargo does not attach its path dependency to our product workspace.
When updating the revision or patch, remove `.pnpm/cache-audit-fspy` and reinstall;
unexpected source modifications fail verification instead of being overwritten.

The regression fixture covers links, rename/exchange, deletion, relative directory
descriptors, direct Linux syscalls, and dynamically/static linked executables.
This is filesystem-access evidence, not a claim to intercept every Linux syscall.
