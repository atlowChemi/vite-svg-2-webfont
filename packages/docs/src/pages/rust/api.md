---
description: The functions, result types, and supporting types of the webfont-generator crate.
---

# Rust API

The crate's public API, by task. Every option is described in [configuration](/rust/configuration), and the complete reference with every type is on [docs.rs](https://docs.rs/webfont-generator).

## Generating from files

These functions need the [`fs` feature](/rust/usage#feature-flags), enabled by default. They read SVG files and, unless [`write_files`](/rust/configuration#write-files) is `Some(false)`, write the output to [`dest`](/rust/configuration#dest).

### `generate()`

```rust
pub async fn generate(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult>
```

Generates the fonts on Tokio's blocking pool, so it needs a running [Tokio](https://tokio.rs/) runtime. `rename` is an optional [`RenameFn`](#renamefn).

### `generate_sync()`

```rust
pub fn generate_sync(
    options: GenerateWebfontsOptions,
    rename: Option<RenameFn>,
) -> std::io::Result<GenerateWebfontsResult>
```

The same, for code without a Tokio runtime: it creates one internally.

### `generate_with_hooks()`

`generate_with_hooks(options, &hooks)` is the async entry point for adapters, such as the Node.js package, that need callbacks. It takes an implementation of `GenerationHooks` and returns `Result<GenerateWebfontsResult, H::Error>`, where the hook's error type implements `From<std::io::Error> + Send`. Implementations must be `Send + Sync` and return `Send` futures.

`generate_with_hooks(options, &hooks)` accepts an implementation of `GenerationHooks`.
It returns `Result<GenerateWebfontsResult, H::Error>`, where the hook error type implements
`From<std::io::Error> + Send`. Implementations must be `Send + Sync` and return `Send` futures.

| Method                  | Default            | Contract                                                                                                                                                      |
| ----------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rename(paths)`         | `Ok(None)`         | Borrows `&[String]` after source loading; return one name per ordered path, or `None` for file stems. Variant paths are flattened in configured design order. |
| `has_css_context()`     | `false`            | Enable CSS context mutation.                                                                                                                                  |
| `has_html_context()`    | `false`            | Enable HTML context mutation.                                                                                                                                 |
| `css_context(context)`  | Unmodified context | Runs before HTML; engine-owned variant mode cannot be overridden.                                                                                             |
| `html_context(context)` | Unmodified context | Receives the HTML context, with variant styles derived from the finalized CSS context.                                                                        |

The engine owns generation, rendering caches and output writes. Adapters own callback
transport and boundary types. `()` implements the trait with no callbacks. Results made
with context mutation enabled reject incremental regeneration, matching the Node API.

### `RenameFn`

```rust
pub type RenameFn = Box<dyn Fn(&str) -> String + Send + Sync>;
```

An optional callback that maps file paths to custom glyph names. When `None`, glyph names are derived from the file stem.

```rust
let rename: webfont_generator::RenameFn = Box::new(|path| {
    // Use only the filename without extension, lowercased
    std::path::Path::new(path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_lowercase()
});

let result = webfont_generator::generate_sync(options, Some(rename)).unwrap();
```

## Generating from SVG text

These functions need the [`in_memory` feature](/rust/usage#feature-flags), enabled by default. They take SVG text instead of paths and never touch the file system. See [in-memory options](/rust/configuration#in-memory-options) for how their options differ.

### `generate_from_sources_sync()`

`generate_from_sources_sync(options)` returns `std::io::Result<GenerateWebfontsFromSourcesResult>`, without a runtime.

```rust
use webfont_generator::{generate_from_sources_sync, GenerateWebfontsFromSourcesOptions, SvgSource};

fn main() -> std::io::Result<()> {
    let result = generate_from_sources_sync(GenerateWebfontsFromSourcesOptions {
        sources: vec![SvgSource {
            id: "icon-42".into(),
            name: "square".into(),
            svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H22V22H2Z"/></svg>"#.into(),
        }],
        incremental: Some(true),
        ..Default::default()
    })?;
    let css = result.generate_css()?;
    let outputs = result.collect_outputs()?;
    println!("{} bytes of CSS, {} outputs", css.len(), outputs.len());
    Ok(())
}
```

### `generate_from_sources()`

`generate_from_sources(options).await` runs the same work on Tokio's blocking pool, and also needs the `async` feature and a running runtime. Both functions use Rayon for CPU parallelism, capped by [`threads`](/rust/configuration#threads) when set.

Options and source updates implement `serde::Deserialize` with camelCase names, rejecting unknown fields. `SourceChange` uses a `type` tag (`added`, `changed`, `removed`), and `SourceMembership` accepts `{ "sources": [...] }` or `{ "variants": [...] }`. `cache` and `threads` aren't deserialized.

## `GenerateWebfontsResult`

The result of generating from files.

### Font bytes

Unrequested formats return `None`. Multi-variant families return one shared resource per requested
format; their SVG/EOT getters return `None`. Output writes are non-transactional and can leave
a partial bundle if a write fails.

| Method          | Return type     | Description         |
| --------------- | --------------- | ------------------- |
| `eot_bytes()`   | `Option<&[u8]>` | EOT font bytes      |
| `svg_string()`  | `Option<&str>`  | SVG font XML string |
| `ttf_bytes()`   | `Option<&[u8]>` | TTF font bytes      |
| `woff_bytes()`  | `Option<&[u8]>` | WOFF font bytes     |
| `woff2_bytes()` | `Option<&[u8]>` | WOFF2 font bytes    |

### Rendering

| Method                      | Return type               | Description                                     |
| --------------------------- | ------------------------- | ----------------------------------------------- |
| `generate_css_pure(urls?)`  | `io::Result<String>`      | Render CSS with optional URL overrides          |
| `generate_html_pure(urls?)` | `io::Result<String>`      | Render HTML preview with optional URL overrides |
| `generate_ts()`             | `io::Result<Arc<String>>` | Render and share the fixed TypeScript module    |

The CSS and HTML methods accept `Option<HashMap<FontType, String>>` for the `urls` parameter. Results are cached internally for repeated calls with the same arguments.

`None` uses generated URLs. `Some` replaces all defaults, leaving omitted entries empty;
multi-variant results reject SVG/EOT URL overrides. See the shared [Templates reference](/guide/templates)
for context fields, generated CSS, HTML previews, and SCSS usage.

### `generate_ts()`

`generate_ts()` lazily renders and shares a fixed TypeScript source module:

<!-- prettier-ignore -->
```ts
export const iconBaseSelector = ".icon" as const;
export const iconClassPrefix = "icon-" as const;
export const icons = [
  "add",
  "remove"
] as const;

export type IconName = (typeof icons)[number];
```

The same method and output contract apply to `GenerateWebfontsFromSourcesResult`.
Set `ts: Some(true)` to include the module in written outputs or SVG-text collection;
explicit `generate_ts()` calls work regardless of this flag. File output uses `ts_dest`
and respects `write_files`. SVG-text collection identifies the module as `OutputKind::Ts`.
The CLI manifest spells the destination `tsDest`.

Names follow source order and are deduplicated across variant designs. `baseSelector` and
`classPrefix` come from template options, or the effective CSS context hook for file results.
Missing or non-string values use `.icon` and `icon-`, matching the built-in CSS renderer;
explicit empty strings are preserved.
HTML context hooks do not customize this module. The metadata remains the same when CSS
output is disabled or a custom CSS template does not use these fields.
String literals are escaped; no TS template, template override, or TS context callback exists.

Incremental additions, removals, renames, and reordering refresh the module. Geometry-only
edits reuse the cached module when names are unchanged. This is `.ts` source with runtime
constants, not a `.d.ts` file.

### `flush_cache()`

| Method                | Return type      | Description                                                                  |
| --------------------- | ---------------- | ---------------------------------------------------------------------------- |
| `flush_cache()`       | `io::Result<()>` | Block until background cache writes finish, retrying a previous failure once |
| `flush_cache_async()` | `io::Result<()>` | Await the same flush on Tokio's blocking pool                                |

Await outstanding rebuilds before flushing. Both methods are no-ops without an incremental filesystem cache; persistence errors do not invalidate generated outputs.

With [`cache_location`](/rust/configuration#cache-location) and `incremental`, every committed update queues a background cache write; a single writer per result lineage coalesces them, and encoding and disk I/O don't block rebuilding. Failed rebuilds that roll back don't publish cache snapshots. Call `flush_cache()` or `flush_cache_async()` after outstanding rebuilds and before shutdown: dropping a result doesn't wait, and abrupt termination can lose pending writes. Initial generation still saves synchronously.

## Updating a file result

A result generated with [`incremental: Some(true)`](/rust/configuration#incremental) can be [updated](/guide/features/incremental) after files change.

### `regenerate()` and friends

| Method                                      | Return type                     | Description                                                                 |
| ------------------------------------------- | ------------------------------- | --------------------------------------------------------------------------- |
| `regenerate(files, changes)`                | `io::Result<()>`                | Rebuild after known file changes, reusing unchanged glyphs                  |
| `regenerate_all(files)`                     | `io::Result<()>`                | Re-read/hash the full file set and infer added/changed/removed paths        |
| `regenerate_async(files, changes)`          | `Result<Self, RegenerateError>` | Consume the result and rebuild on Tokio's blocking pool                     |
| `regenerate_all_async(files)`               | `Result<Self, RegenerateError>` | Consume the result, re-diff, and rebuild on Tokio's blocking pool           |
| `regenerate_snapshot_async(files, changes)` | `io::Result<Self>`              | Borrow the result, preserving readable output while returning a replacement |

Requires the result to have been generated with `incremental: Some(true)` (errors otherwise).
Synchronous methods take `files: &RegenerationFiles`; async methods take an owned `RegenerationFiles`.
Each contained file list is the complete file set after the changes, in the order a fresh build
would use (e.g. the glob result); the rebuilt glyphs are ordered to match it, so the result is
byte-identical to a fresh build of that set — additions included, even when they sort before
existing glyphs. Any path absent from its list is dropped. `changes: &[(String, GlyphChange)]`
names the affected files: added/changed files are re-read from disk. Every format is refreshed in
memory, and — when the result was built with `write_files` — refreshed fonts are written to disk
too, while unchanged CSS/HTML companion files are skipped. Rendered
CSS/HTML is reused when glyph names and codepoints are unchanged. Use `regenerate_all` when you have
the fresh ordered file set but no reliable watcher change batch; existing glyph names are preserved,
and added paths derive their glyph name from the file stem.

`regenerate_async` and `regenerate_all_async` take owned inputs and consume the result, so Rust rejects stale-result
reuse after a successful rebuild. Assign the returned generation:

```rust
result = result.regenerate_async(files, changes).await?;
```

For ordinary regeneration failures, `RegenerateError::into_result()` returns the consumed result
so it can be retried. It returns `None` only if Tokio cancelled the blocking task before the result
could be returned. The consuming futures are not cancellation-safe: dropping one does not stop an
already-started blocking task, filesystem writes may continue, and the consumed result cannot be
recovered. Panics resume unwinding on the awaiting task.

`regenerate_snapshot_async` supports adapters that need the receiver to remain readable.
It borrows `&self` and takes `changes: Option<Vec<(String, GlyphChange)>>`: `Some(changes)`
supplies explicit changes; `None` re-diffs the complete membership. Success returns a new
result and prevents further regeneration of the old snapshot. Overlapping attempts are
rejected. Failure restores the old snapshot's regeneration state for retry, including
reconciliation after partial variant writes. Worker panics are returned as I/O errors.
Dropping this future does not cancel blocking work or writes: if the work succeeds, the
old snapshot remains replaced, but the new result is lost. Prefer the consuming methods
when Rust ownership can enforce replacement.

For Node.js results, `regenerate()` errors if the initial build used `cssContext` or `htmlContext`
callbacks because the synchronous method cannot re-run JavaScript callbacks during the rebuild.

```rust
pub enum GlyphChange {
    Added { name: Option<String> },   // new file; `name` overrides the file-stem glyph name
    Changed { name: Option<String> }, // content changed; `name` overrides the glyph name
    Removed,                          // file deleted
}

// result built with `incremental: Some(true)`
let files = RegenerationFiles::Single(vec!["icons/add.svg".to_owned(), "icons/remove.svg".to_owned()]);
result.regenerate(&files, &[("icons/add.svg".to_owned(), GlyphChange::Changed { name: None })])?;
// Or re-diff the full set when watcher hints are unavailable/untrusted:
result.regenerate_all(&files)?;
```

### Single-file helpers

To change one file at a time without listing every file, use `add(path, name, variant)`,
`update(path, name)`, and `remove(path)`, plus the consuming `add_async`, `update_async`,
and `remove_async`. Async adapters can use `add_snapshot_async`, `update_snapshot_async`,
and `remove_snapshot_async` to retain readable receivers, with the same lineage and cancellation
rules as `regenerate_snapshot_async`.

Additions append and require a target variant for families (`None` for ordinary fonts).
Updates preserve position and the existing name when `name` is `None`.
Updates/removals affect every design consuming the ID/path. Duplicate additions, unknown
update/remove targets, and empty-design removals reject. File helpers preserve existing
regeneration's filesystem-write semantics; removing a path never deletes the source file.
Use `regenerate`, `regenerate_sync`, or `regenerate_async` for batches, explicit ordering, or membership reassignment.

### Families

Enable `incremental: Some(true)` when generating a family to retain per-design parse/process
caches. The same [incremental methods](#regenerate-and-friends) support ordinary fonts and families.
Select the matching source mode with `RegenerationFiles`:

```rust
use webfont_generator::{GlyphChange, RegenerationFiles, VariantFileSet};

let files = RegenerationFiles::Variants(vec![
    VariantFileSet { variant: "light".into(), files: vec!["light/add.svg".into()] },
    VariantFileSet { variant: "bold".into(), files: vec!["bold/add.svg".into()] },
]);
let changes = vec![("bold/add.svg".into(), GlyphChange::Changed { name: None })];
result = result.regenerate_async(files, changes).await?;
```

#### `RegenerationFiles`

| Case       | Payload               | Meaning                                                |
| ---------- | --------------------- | ------------------------------------------------------ |
| `Single`   | `Vec<String>`         | Complete ordered inputs for an ordinary font           |
| `Variants` | `Vec<VariantFileSet>` | Complete ordered file sets for every configured design |

#### `VariantFileSet`

| Field     | Type          | Meaning                                                          |
| --------- | ------------- | ---------------------------------------------------------------- |
| `variant` | `String`      | Existing design name, present exactly once per update            |
| `files`   | `Vec<String>` | Nonempty, duplicate-free authoritative file list for this design |

Every update supplies every configured design. The update's design order does not change the
configured order; file ordering is independent within each design. Regeneration cannot change
the design list or weights themselves. Unknown/duplicate design names and invalid lists fail
before reading sources.

`GlyphChange` hints apply to paths across the family. `Added` introduces a path not previously
used by any design; `Changed` updates every final consumer of an existing path; `Removed`
requires that no design retain the path. Optional names apply to all consumers too. Membership
changes, including moving or sharing an existing path, need only changes to the file lists.
New memberships are loaded even without a hint. Use `regenerate_all` or `regenerate_all_async`
to read every file and infer additions, changes, and removals.

Successful updates match a fresh build of the final inputs, including union/codepoint assignment,
shared metrics, and fallback consumers. Unchanged parse/process work is reused; effective changes
reassemble the shared font bundle. No-op updates retain in-memory output and can retry writes.

Validation, source loading, and font-build failures preserve the previous state. State commits
before writes: a write failure leaves the new in-memory output available and may leave partial
disk files. Retry to complete output writes. The consuming async methods take `self`; recover the result
with `RegenerateError::into_result` on failure. Dropping the future does not cancel started
blocking work or its writes. Results created through Node context callbacks reject regeneration.

## `GenerateWebfontsFromSourcesResult`

The result of generating from SVG text.

| Method                                                        | Result                                                                                       |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `svg_string()`                                                | `Option<&str>`                                                                               |
| `ttf_bytes()`, `eot_bytes()`, `woff_bytes()`, `woff2_bytes()` | `Option<&[u8]>`                                                                              |
| `generate_css()`, `generate_html()`                           | `std::io::Result<Arc<String>>`; renders once and shares the cached text.                     |
| `generate_ts()`                                               | `std::io::Result<Arc<String>>`; shares the fixed [TypeScript module](#generate-ts).          |
| `collect_outputs()`                                           | `std::io::Result<Vec<CollectedOutput>>`; shares font payloads and enabled companion renders. |
| `cache_bytes()`                                               | `Option<Vec<u8>>`; persistent cache bytes to store, see [above](#cache-bytes).               |

`CollectedOutput` contains `kind: OutputKind` (`Font(FontType)`, `Css`, `Html`, or `Ts`) and
`contents: OutputContents` (`Bytes(Arc<Vec<u8>>)` or `Text(Arc<String>)`). Use `as_bytes()`
to consume either representation. The caller chooses download names or other destinations.

Font getters never initialize templates. CSS requests never compile HTML. HTML embeds the
cached CSS. TS requests never initialize templates. Collection respects `css`/`html`/`ts`;
explicit render methods work even when disabled.
Template compile/render errors are deferred until that companion is requested.

Memory fingerprints include ordered IDs, names, content, variant membership, resolved settings,
and supplied template text. They need not match legacy file-mode URL hashes.

### `cache_bytes()`

Set `cache: Some(bytes)` (an empty vector starts cold) to reuse work from an earlier run without
a filesystem, for example from browser storage. The bytes use the same format as
[`cache_location`](/rust/configuration#cache-location) files, and the same [rules](/guide/features/caching#what-gets-reused) apply: unchanged icons and formats
are reused, output is byte-identical, and bytes from another engine version or other settings are
ignored. `result.cache_bytes()` (`&mut self`) encodes the current state, including incremental
updates, and returns `None` when caching is off, nothing changed since the bytes were loaded or
last returned, or a failed update left the result without its glyph data (the next successful
update restores it).

`fallback_ts` supplies the TTF timestamp on targets without a system clock. Unlike
`format_options.ttf.ts`, it isn't part of cache keys, so later runs with another clock still reuse
cached fonts, which keep the timestamp of the run that created them.

## Updating an in-memory result

For single actions on `GenerateWebfontsFromSourcesResult`, use
`add_sync(source, variant)`, `update_sync(id, svg, name)`, or `remove_sync(id)`.
With the `async` feature, consuming `add(source, variant)`, `update(id, svg, name)`,
and `remove(id)` return the next result, with `SourceRegenerateError` recovery on failure.
These methods require `incremental: Some(true)` and derive membership from retained sources.

```rust
use webfont_generator::{GenerateWebfontsFromSourcesResult, SourceChange, SourceMembership};

fn update(result: &mut GenerateWebfontsFromSourcesResult) -> std::io::Result<()> {
    result.regenerate_sync(
        SourceMembership::Sources(vec!["icon-42".into()]),
        vec![SourceChange::Changed {
            id: "icon-42".into(),
            name: None, // Keep the name; Some(...) renames without changing identity.
            svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M0 0L24 24H0Z"/></svg>"#.into(),
        }],
    )
}
```

- `SourceChange::Added(SvgSource)` introduces an unknown ID.
- `Changed { id, svg, name }` replaces a known source.
- `Removed { id }` removes a known source from all designs.
- `SourceMembership::Sources(ids)` supplies complete ordinary membership and order.
- `SourceMembership::Variants(Vec<VariantSourceSet { name, ids }>)` supplies every configured
  variant in its original order. A shared-ID update affects every membership.

Unchanged source text stays in Rust; submit only changed payloads. Complete membership also
permits reorder or removal without resending retained text. Added/changed IDs must appear in
final membership, removed IDs cannot remain, and each design must remain nonempty.
Duplicate changes, conflicting modes and unknown IDs return errors.

Generation settings, named color selection, variant configuration and templates remain fixed.
Invalidating a named color selection by removal or rename returns an error. No-op updates keep
font/render caches. Changed builds reuse geometry/encoder caches and invalidate companion renders.
Companions independent of changed fields retain their cached strings using the shared template
dependency rules. Failed builds retain old outputs and sources but discard partially mutated build caches before retry.

The consuming async method is `result.regenerate(membership, changes).await`. It returns
`Result<GenerateWebfontsFromSourcesResult, SourceRegenerateError>`. Recover ordinary failures
with `into_result()` or `into_parts()`; worker panic/cancellation returns no recoverable result.
For synchronous failures, the borrowed receiver remains usable.

Dropping an async future does not cancel already-started blocking work and does not return
the consumed result. CPU work may continue, with no memory-mode filesystem side effects.

These functions are Rust-only; the npm/NAPI exports and CLI still accept files.

## `clear_cache()`

```rust
use webfont_generator::clear_cache;

fn main() -> std::io::Result<()> {
    let report = clear_cache(".webfont-cache")?;
    println!("{} files, {} bytes", report.files_removed, report.bytes_removed);
    Ok(())
}
```

`clear_cache(cache_location)` requires the `fs` feature. It removes only files the cache created
(plus leftovers from interrupted writes), leaves other files alone, and returns
`ClearCacheReport { files_removed, bytes_removed }`. A missing directory is not an error.
Stop incremental updates and flush every active result lineage before clearing or deleting the
cache directory; outstanding background writes can otherwise recreate removed files.

## Types

### `FontType`

```rust
pub enum FontType {
    Svg,
    Ttf,
    Eot,
    Woff,
    Woff2,
}
```

Methods:

- `css_format() -> &'static str` -- Returns the CSS `format()` value (e.g., `"woff2"`, `"truetype"`)
- `as_extension() -> &'static str` -- Returns the file extension (e.g., `"woff2"`, `"ttf"`)

### `FontVariant`

One design in `GenerateWebfontsOptions::variants`. Supply at least two designs, each with
a unique name and a nonempty file list; exactly one must set `default: Some(true)`.
Leave the top-level `files` empty when using `variants`.

```rust
pub struct FontVariant {
    pub name: String,
    pub files: Vec<String>,
    pub weight: Option<u16>,
    pub default: Option<bool>,
}
```

| Field     | Meaning                                                                              |
| --------- | ------------------------------------------------------------------------------------ |
| `name`    | Design name, also used in its CSS modifier class. Whitespace and NUL are rejected.   |
| `files`   | SVG paths for this design. Match filenames across designs to identify the same icon. |
| `weight`  | Optional weight from 1–1000. Explicit weights must increase in variant order.        |
| `default` | Whether this design is used without a modifier class.                                |

With `weight: None`, weights are assigned automatically; see [the rules](/guide/features/variants#rules).

### `MissingGlyphBehavior`

Controls what happens when an icon exists in the family but is absent from one design.

```rust
pub enum MissingGlyphBehavior {
    Blank,
    Error,
    Fallback,
}
```

| Value      | Behavior                                                                       |
| ---------- | ------------------------------------------------------------------------------ |
| `Blank`    | Use an empty outline while retaining the icon's advance width. Default policy. |
| `Error`    | Reject the family and report missing design/icon pairs.                        |
| `Fallback` | Reuse the outline from a named design containing every icon.                   |

### `MissingGlyphOptions`

Sets the family-wide missing-icon policy through `GenerateWebfontsOptions::missing_glyphs`.

```rust
pub struct MissingGlyphOptions {
    pub behavior: MissingGlyphBehavior,
    pub variant: Option<String>,
}
```

Set `variant` to the fallback design's name only with `Fallback`; use `None` for `Blank` or `Error`.

```rust
use webfont_generator::{MissingGlyphBehavior, MissingGlyphOptions};

let blank = MissingGlyphOptions { behavior: MissingGlyphBehavior::Blank, variant: None };
let error = MissingGlyphOptions { behavior: MissingGlyphBehavior::Error, variant: None };
let fallback = MissingGlyphOptions {
    behavior: MissingGlyphBehavior::Fallback,
    variant: Some("Regular".to_owned()),
};
```

Set `missing_glyphs: Some(fallback)` (or another policy) in the generation options. The fallback
design must contain every logical icon; blank cells retain the logical icon's advance.
