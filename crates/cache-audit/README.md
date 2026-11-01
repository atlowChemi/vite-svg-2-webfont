# Cache audit

Private Linux tooling for observing a command and checking its filesystem accesses
against pnpm's effective task inputs and outputs. Normal builds still use Vite+.

## Build and test

Install Node, Git, and Rust `nightly-2026-08-02` (with rustfmt and Clippy).
Linux regression tests also require Python 3, a C compiler, static libc, and
the installed pnpm on `PATH` for graph parity checks.
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

## Resolve task graphs

The binary embeds pnpm 12.15.0's configuration, workspace discovery, project graph,
and task graph APIs at commit `91774e72a04d52ce61db06339696a433d383c764`.
Task and pipeline definitions live in `pnpm-tasks.yaml`, delegated by the root
workspace's native `include`. Validation uses pnpm's merged local-include document;
negative checks mutate the owning file only inside the temporary copy. Config-dependency
includes are explicitly unsupported by the validator rather than silently omitted.
These unpublished APIs are pinned independently of the installed pnpm version.

```sh
crates/cache-audit/target/debug/cache-audit graph --pipeline next-build
crates/cache-audit/target/debug/cache-audit graph --task next:native
crates/cache-audit/target/debug/cache-audit graph --all --verify-pnpm
```

Use `--workspace PATH` for another checkout. Exactly one selection is required;
output is a JSON object keyed by pipeline or task name. Graph resolution loads
pnpm's merged configuration and detects cycles without executing tasks or
installing dependencies. It selects the full workspace, not affected projects.
Graph-only configuration loading skips pnpm's store-placement probes.

`--verify-pnpm` compares each selected pipeline with the installed
`pnpm pipeline NAME --full --dry-run --json` and fails on drift. CI runs this for
every pipeline. Fixture tests cover default and explicit dependencies, catalogs,
missing-script nodes, root inclusion, cycles, and invalid selection.
Graph resolution alone does not execute commands or provide sandbox isolation.

## Run a graph-aware audit

```sh
vp run @atlowchemi/cache-audit#audit -- --task next:native
vp run @atlowchemi/cache-audit#audit -- --pipeline next-assets
vp run @atlowchemi/cache-audit#audit -- --all
```

The equivalent binary entry point is `cache-audit audit --workspace PATH` with
one of those selectors. `--task` audits matching tasks and prepares prerequisites;
`--pipeline` audits every executable node in that pipeline. `--all` audits only
tasks listed in the reviewed native/build policy files, preparing their dependency
closure once. Select servers, publishing, and other open-ended pipelines explicitly.
`--policy PATH` supplies another policy file; `--reports PATH` selects the report
directory. The legacy package command `audit -- TASK [POLICY]` is also accepted.

Rust uses pnpm's graph builder and topological ordering. Execution is serial so
overlapping output paths cannot contaminate simultaneous observations. Missing-script
nodes pass dependency status through. Failed commands block descendants; findings
from successful commands are reported while consumers can still run. Independent
tasks continue after failures. A single task returns 2 for audit findings; aggregate
failures return 1. SIGINT/SIGTERM cancels subprocess groups and returns 130.

Each audited task gets a fresh temporary Cargo target, removed after its command
and before the addon-load check. Reports contain observed accesses only for that task, plus
logs and statuses for prerequisites. The runner prepares WASM tools outside observation.
It resolves pnpm's store location before observation and requires dependencies to
be current, preventing pnpm's automatic install path from mutating the lockfile
during a task. Run `vp install` beforehand when manifests have changed.
Source workspaces still receive normal build outputs; this is observation, not an
OS sandbox. Use the isolated negative checker for declaration mutations.

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
              reason: 'System toolchain; upgrades require separate task-cache invalidation'
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
narrow and review the declared environment inputs and toolchain cache policy separately.
`policy.native.yaml` describes the reviewed native-build environment. A clean
filesystem audit does not replace environment declarations/toolchain invalidation or enable
result caching.

`policy.build.yaml` covers SVG optimization, WASM, both library packs, social
images, docs, and the example. `directory: true` restricts an exemption to paths
that are directories; it cannot waive regular-file reads. `cleanupOutputs`
accepts retired files only when they were declared outputs before execution and
are absent afterward.

A read-only literal-path rule can specify `environment: VARIABLE`. The variable
must appear explicitly in the task's or a dependency's `env` inputs and equal the
file's SHA-256 (or `missing`) before and after execution. This is supported for
callers that supply a verified pre-key stamp; native pipelines do not generate one.
The example remains uncached because it can read ignored env files; docs disable
dotenv loading before enabling their cache.
The example policy sets `requiresUncached: true`: its narrowly exempted local env
reads are accepted only while the task explicitly declares `cache: false`. Enabling
or defaulting its cache makes policy preparation fail. This is an uncached-command
audit, not approval of a result-cache contract. Other ignored files remain findings.

## Supported coverage semantics and boundaries

- Supports stable pnpm **12.x starting at 12.13** and uses **wax 0.7.0**.
  Routine minor/patch upgrades do not require editing a version allowlist. CI runs
  `cache-audit compat` against the installed pnpm to check replacement/default/
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
vp run @atlowchemi/cache-audit#test:compat
pnpm pipeline next-build --full --no-cache
vp run @atlowchemi/cache-audit#audit -- --all
vp run @atlowchemi/cache-audit#audit:negative
```

The dormant-pipeline CI job executes finite build/test/coverage/tarball pipelines
uncached; servers, publishing,
measurements, and source-mutating maintenance are explicitly invoked separately.
Rust owns audit execution, compatibility checks, and declaration-negative checks;
`packages/cache-audit/scripts/run.mjs` only launches the binary and forwards signals.
`compat` uses a temporary fixture and separate pnpm cache/store/state directories.
`negative` copies the installed workspace, deliberately removes native source/output
declarations there, and checks the observed diagnostics. It copies files rather
than hardlinking them to the source, relocates internal absolute symlinks, and
removes the copy afterward. Root Git metadata, root/auditor Cargo targets, and reports are
excluded; tracked-file membership is reconstructed in the copy. Installed external
tools remain shared under the usual immutable-toolchain assumption. A frozen install
with lifecycle scripts disabled refreshes pnpm's absolute workspace metadata inside
the copy before mutations or observation. Keep the source
checkout idle while taking its copy. Both checks require the compiled auditor.

`CACHE_AUDIT_BIN` selects the compiled auditor; `CACHE_AUDIT_REPORTS` overrides
`artifacts/cache-audit`. Reports include command logs, audit JSON, and task status.

Use `pnpm pipeline next-assets --full` directly for dormant cached SVG execution.
Normal workflows still use Vite+. The PR/manual/scheduled `cache-audit.yaml` workflow executes dormant graphs
and collects report-only filesystem audits; adding the file does not establish
that a GitHub run has passed.

## Run dormant pipelines and reset task caches

`pnpm pipeline next-test --full` preserves the root Vitest task and native/browser
build prerequisites. `next-rust-test` separately runs Rust formatting and the
engine/adapter validation chain; `next-browser-test` retains browser Rust checks
and tests. Commands, publishing bypasses, and prerequisites remain mapped to Vite+.
Read merged configuration with `pnpm config get tasks --location project --json`
(or `pipelines`); the explicit location makes older bootstrap pnpm launchers
delegate to the pinned project version.

Native/WASM builds, browser/plugin packaging, docs, Rust formatting, engine/adapter Rust checks
and tests, browser Clippy, plugin-local tests, doctests, tarballs, and SVG/social assets enable task
caching. Doctest hits restore `test-results/doctests.xml`. The declared `outputs: []` on Rust checks/tests
means a cached success restores no compiler binaries or `target/` intermediates.
Source-mutating fixture-update mode, report-producing variants, coverage, and
publishing must execute unless their separate contracts have been validated.

Browser/plugin packaging, plugin-local tests, and docs explicitly disable Vite
dotenv loading. Use process environment for selectors such as `VITE_COMPAT_MAJOR`
and `NODE_ENV`; those inputs remain keyed.
The example application retains dotenv support and stays uncached because its
ignored local-env files cannot be reliably keyed by pnpm.

The remaining validation restrictions come from executed probes and source traces:

| Task                                   | Why its unrestricted invocation cannot use a success-only cache                                                                                                      |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root JS tests                          | Dynamic JUnit output, fixture-update modes, ignored browser strictness env, and integration tests that archive committed `HEAD` rather than only working-tree inputs |
| Compatibility and plugin browser tests | Optional `CI_TEST_REPORT` may name an arbitrary external report path; env hashing alone cannot restore that report after deletion                                    |
| WASM browser tests                     | Same report-path issue; `WEBFONT_COVERAGE` changes browser selection; bundle analysis can request a fresh upload                                                     |
| Adapter browser tests                  | Ignored `.env.local` can set `VITE_COLOR_PROOF_STRICT`; the strict mode demonstrated WebKit failures while ordinary mode passed                                      |
| Engine color proof                     | `COLOR_PROOF_OUTPUT_DIR` exports fonts to an arbitrary caller-selected directory; a hit must not silently omit those exports                                         |
| Adapter color proof                    | `UPDATE_COLOR_PROOF_FIXTURE` intentionally updates tracked font fixtures                                                                                             |

Ordinary no-report/no-update/no-export variants are feasible, but require an
enforced separate contract: pnpm supports a boolean environment-selected cache
setting, but cannot derive it from arbitrary report paths or update flags, or
interpolate those paths into outputs. These existing entry points
retain all their modes. Coverage, benchmarks, publishing, fixture/binding
regeneration, and dev/preview commands execute for reports, measurements or side effects.

Plugin-local tests do not use the root JUnit reporter. Their ordinary and upstream
Vite matrix modes cache successful validation without test scratch outputs.
Use `--no-cache` for external `NAPI_RS_NATIVE_LIBRARY_PATH`, custom WASI artifacts,
or injected instrumentation: hashing the library path is not hashing its contents
or restoring its externally written reports. The raw `next:*` package scripts
also always execute, without prerequisite orchestration.

Tarball caching does not suppress uncached package-build prerequisites. Native
release repeats can reuse all five Rust validation tasks, formatting, and the release artifact.

Plugin packaging cache hits intentionally skip Codecov bundle-analysis uploads.
The unchanged bundle is reusable; a new per-commit analysis is optional. When a
fresh report is required, supply `CODECOV_TOKEN` through the environment and run:

```sh
ALLOW_BUNDLE_ANALYSIS=true pnpm pipeline next-plugin-tarball --full --no-cache
```

This executes packaging and its prerequisites even when cached, allowing the
existing Codecov plugin to upload fresh analysis. It does not refresh stored cache
entries. Coverage carryforward is separate; no bundle carryforward is assumed.
Debug and release native builds retain their existing shared output paths. Run
them sequentially: switching producers can cause a safe rebuild even with an
unchanged key, because pnpm refuses to restore over bytes changed by another task.

pnpm 12.15 keys the lockfile, Node version, OS/architecture, task commands, declared
files/environment, and prerequisite keys. It does not detect external compiler,
SDK, libc, or tool replacements. Compiler-selection variables such as `RUSTC` are
declared explicitly; they do not detect a replacement at an unchanged path.
Before reusing result caches after an external-tool upgrade:

1. Locate the workspace scope from a pipeline's printed report path:
   `<cacheDir>/pipeline/<workspace-scope>/runs/<run>`.
2. Remove only `<cacheDir>/pipeline/<workspace-scope>/tasks`, or switch to a fresh
   cache namespace. Keep developer and CI namespaces independent; CI restore keys
   must include the target/ABI and an external-tool generation bumped on upgrades.
3. Run the selected pipeline normally and verify it executes before warming again.

`--no-cache` bypasses reads **and writes**; one uncached run does not replace old
results. `--force` is an installation option. `pnpm cache delete` clears registry
metadata, not pipeline results. Default `pnpm config get cache-dir` may be undefined;
use the actual report path rather than constructing a deletion path from it.

Do not drop `--full` on these root-owned tasks: native affected-project selection
currently omits required root tasks on docs-only changes. CI activation should reuse
`scripts/ci/affected-selection.ts` to choose one dependency graph, observe actual
executions once, and keep scheduled full uncached audits. Running separate pipelines
in a loop does not deduplicate shared prerequisites across invocations.
