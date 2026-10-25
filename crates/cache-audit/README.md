# Cache audit

Private Linux tooling for observing a command and checking its filesystem accesses
against pnpm's effective task inputs and outputs. Normal builds still use Vite+.

## Build and test

Install Node, Git, and Rust `nightly-2026-08-02` (with rustfmt and Clippy).
Linux regression tests also require Python 3, a C compiler, and static libc.
From the repository root, use the workspace package:

```sh
vp run @atlowchemi/cache-audit#build
vp run @atlowchemi/cache-audit#test
vp run @atlowchemi/cache-audit#check
```

The package scripts fetch the pinned upstream revision, apply the checked-in
mutation-tracking patch, and verify the resulting diff. Downloads are reused by
revision and patch hash under `target`. Cargo overrides point at those patched
sources. See [patch maintenance](patches/README.md). Bare Cargo does not apply the
patch; use these scripts. `packages/cache-audit` exposes these commands in the root
pnpm workspace. The Rust project remains a separate Cargo workspace under
`crates/cache-audit`; its nested pnpm workspace boundary keeps its nightly
dependencies out of root Cargo discovery. Builds avoid inheriting pnpm's vendored
Cargo sources.

## Observe or audit

Run the binary directly to avoid inheriting Cargo's build-time environment.
The command runs in the caller's working directory; `--project` selects the owner
of the **cache task**, not the command's working directory.

```sh
# From the repository root: raw observation, no coverage verdict.
crates/cache-audit/target/debug/cache-audit -- pnpm --dir packages/webfont-generator run next:build > accesses.json

# Audit the root-owned cache wrapper's declarations, executing its raw command.
crates/cache-audit/target/debug/cache-audit --workspace . --project . --task next:native --policy crates/cache-audit/policy.native.yaml -- pnpm --dir packages/webfont-generator run next:build > audit.json
```

Install dependencies and prepare prerequisite outputs **before** observation.
Use a fresh `CARGO_TARGET_DIR` to exercise compiler accesses; disabling task-result
caching does not disable Cargo reuse. The auditor executes exactly the command
supplied, without installing dependencies or invoking prerequisite tasks.

Child stdout/stderr go to stderr. Stdout is one JSON document with `version`,
`exit_code`, `accesses`, and `audit` (null in observation mode). Findings identify
absolute paths and reasons. Exit 2 means a successful command with coverage
findings; a failing command preserves its exit code. Observer/configuration errors
and incomplete tracking fail rather than reporting a clean audit.

## Explicit policy

Pass `--policy path/to/policy.yaml`. Policies are separate from pnpm declarations:

```yaml
version: 1
tasks:
    '.#next:native':
        ignore:
            - pattern: '/usr/**'
              reason: 'System toolchain; version fingerprinting is handled separately'
            - pattern: '.'
              kinds: [read, read_dir]
              reason: 'Known working-directory probes'
        neverCache:
            - pattern: 'target/**'
              reason: 'Cargo intermediate state; final artifacts live in the package'
```

Paths inside the workspace are workspace-relative (`.` for its root); paths
outside it are absolute. Patterns use wax, like pnpm. `ignore` accepts optional
`kinds`: `read`, `write`, `read_write`, `read_dir`. Read and write exemptions
apply independently to a `read_write` access; `read_write` exempts both halves
of that combined access only. Every rule requires a reason.
Rules can also specify `contents` for a literal path: its exact UTF-8 contents
must match both before and after execution, otherwise the audit fails. The native
policy uses this for pnpm's Git-ignored Cargo installation shim; custom Cargo
configuration cannot silently inherit that exemption. These assertions are audit
preconditions, not extra pnpm hash inputs. Cache activation must enforce them or
fingerprint the installation state on cache lookups too.
`missingInputs` lists reviewed regular-file probe patterns (each with a reason).
It opts those paths into the checked-absence handling below; it does not exempt
existing reads or writes. Do not use it for arbitrary existence tests or empty
directory dependencies, which pnpm's Git file enumeration does not hash.
`neverCache` both exempts understood accesses and rejects matching paths found in
the actual input/output selections; it cannot be restricted by access kind.
Policies are opt-in: external paths are not silently ignored. Keep exceptions
narrow and review the declared environment/toolchain fingerprints separately.
`policy.native.yaml` describes the reviewed native-build environment. A clean
filesystem audit does not replace environment/toolchain fingerprints or enable
result caching.

`policy.build.yaml` covers SVG optimization, WASM, both library packs, social
images, docs, and the example. `directory: true` restricts an exemption to paths
that are directories; it cannot waive regular-file reads. `cleanupOutputs`
accepts retired files only when they were declared outputs before execution and
are absent afterward.

A read-only literal-path rule can specify `environment: VARIABLE`. The variable
must appear explicitly in the task's or a dependency's `env` inputs and equal the
file's SHA-256 (or `missing`) before and after execution. This covers Git-ignored
local example configuration without hashing its contents into every task.

## Supported coverage semantics and boundaries

- Supports stable pnpm **12.x starting at 12.13** and uses **wax 0.7.0**.
  Routine minor/patch upgrades do not require editing a version allowlist. CI runs
  `packages/cache-audit/scripts/compat.mjs` against the installed pnpm to check replacement/default/
  additive inputs, ignored and untracked files, dependency/environment invalidation,
  and output restoration. Explicit unsupported configuration still fails. New
  majors and prereleases require a semantics review. The cache implementation and
  pipeline configuration code are unchanged between 12.13.0 and 12.14.0.
- Git enumerates tracked and untracked nonignored project-local files **before**
  execution. Parent-path input patterns cannot extend that set. Missing files
  are skipped, outputs removed, replacement/additive patterns applied, and the
  workspace lockfile included as pnpm does. Empty/all-additive input lists retain
  the default set.
- Explicit same-project task dependencies contribute source coverage and their
  prepared output files. Dependency outputs are trusted only on the assumption
  that their producer is separately audited. Upstream `^task` dependencies,
  implicit task configurations, package overrides, Cargo snapshots, and
  environment-expanded YAML are rejected, not approximated.
- Symlink input text is not treated as coverage of referenced content. Symlink
  ancestors, submodules, invalid paths, and unsupported task keys fail explicitly.
  For unresolved observed traversals, an explicit exemption is accepted only
  when both the original path and resolved target are exempt for every access
  component. Missing suffixes are resolved through their existing ancestor for
  this policy check only. This never supplies input coverage or proves success.
- Final outputs are enumerated using pnpm's wax walk/filter rules. Deleted or
  temporary writes require a policy; deleted files cannot be restored outputs.
- Reads of newly generated declared outputs are covered when a write was observed.
  Reads of preexisting outputs remain findings unless explicitly accounted for.
  Directory enumeration requires an explicit policy. Native discovery exclusions
  apply only to directory accesses, not reads of files discovered there.
- Reviewed `missingInputs` probes are covered only if the path was absent in a pre-run
  filesystem snapshot, remains absent afterward, has no observed write to it or
  an ancestor, and would enter the effective input selection if created. Git
  ignore rules and output exclusions still apply. Snapshot traversal excludes
  dependency stores, `.git`, `node_modules`, and `target`; those paths cannot gain
  absence coverage. Symlink ancestors are rejected. This relies on the observed
  mutation stream and stable filesystem/Git-ignore configuration; concurrent
  outside modifications are not supported. An absent declaration is still a
  finding, even when the attempted file read fails.
  Reviewed absent optional manifests can also be covered by a dependency's
  output namespace. ENOTDIR probes require an existing keyed file ancestor.
- The fspy patch adds hard-link, rename (including exchange), unlink, and rmdir
  tracking through Unix preload and Linux syscall paths. Records describe
  **attempted** accesses, not syscall success, and omit PID/error information.
  Other filesystem mutations are not claimed to be completely observed.
- Tracking ends with the root command; it must wait for its subprocesses.
  Filesystem observation does not discover environment dependencies. A passing
  audit applies to observed execution paths under these assumptions, not all
  possible executions. Keep caches dormant until task policies are accepted.

The tests exercise actual observation, dynamic/static mutation calls, effective
input selection, dependency coverage, excluded Git files, parent-path failures,
must-not-cache policies, and deliberately missing inputs and outputs.

## Reproduce phase-2 verification

From the repository root, after installing dependencies and building the auditor
on Linux:

```sh
vp exec node scripts/verify-pipelines.mjs
vp run @atlowchemi/cache-audit#test:compat
vp exec node scripts/verify-pipelines.mjs --run
vp run @atlowchemi/cache-audit#audit -- --all
vp run @atlowchemi/cache-audit#audit:negative
```

The runtime comparison executes all eight dormant graphs uncached and any
selected task not reached by those graphs. The audit runner prepares prerequisites,
installs WASM tools before observation, uses a fresh temporary Cargo target, and
checks that the generated native addon loads. `negative.mjs` deliberately removes
native source/output declarations and restores the workspace YAML in `finally`.
Run mutation checks in an isolated checkout without concurrent builds.

`CACHE_AUDIT_BIN` selects the compiled auditor; `CACHE_AUDIT_REPORTS` overrides
`artifacts/cache-audit`. Reports include command logs, audit JSON, and task status.

Use `pnpm run next:pipeline next-assets` for dormant cached execution. The launcher
computes toolchain/build-environment identity before pnpm chooses cache keys;
the cached SVG wrapper rejects a missing fingerprint. Normal workflows still use
Vite+. The manual/scheduled `cache-audit.yaml` workflow executes dormant graphs
and collects report-only filesystem audits; adding the file does not establish
that a GitHub run has passed.
