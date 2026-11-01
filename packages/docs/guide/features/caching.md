---
description: Reuse the engine's work across processes, CI runs, and browser sessions with the persistent cache, in every fonte tool.
---

<script setup>
import CacheFileLayout from '../../.vitepress/theme/components/CacheFileLayout.vue';
</script>

# Engine Cache

The engine can save its work and reuse it in a later run, so a fresh process starts warm. The cache keeps two kinds of work:

- **Per-icon work:** each icon's parsed and processed outlines. An edited icon is reprocessed alone; the others reuse their saved work.
- **Finished fonts:** each format's bytes. A format whose inputs didn't change, such as every font after a CSS-only option change, comes straight from the cache.

Output is byte-identical to an uncached run. When nothing changed, a build is about 6–14× faster; see [performance](/guide/under-the-hood#persistent-cache) for measurements.

Within one running process, [incremental updates](/guide/features/incremental) already reuse this work. The persistent cache carries it to the next process: a restarted dev server, a new CLI run, a CI job, or a reloaded browser tab.

## Turn it on

::: code-group

```ts [Vite]
// On by default, stored in Vite's cacheDir (node_modules/.vite).
viteSvgToWebfont({
    context: './icons',
    cache: false, // Opt out: generate from scratch on every run.
});
```

```ts [Node.js]
const result = await generateWebfonts({
    files,
    dest: 'dist/fonts',
    cacheLocation: '.webfont-cache',
});
```

```ts [Browser]
// Pass bytes from an earlier session (or an empty array to start cold), then store the update.
using result = await generateWebfonts({ sources, cache: stored ?? new Uint8Array() });
const updated = await result.exportCache();
if (updated) await saveCache(updated);
```

```sh [CLI]
webfont-generator --dest ./dist/fonts --cache-location .webfont-cache ./icons/
```

```rust [Rust]
let options = GenerateWebfontsOptions {
    dest: "dist/fonts".to_owned(),
    files: vec!["icons/add.svg".to_owned()],
    cache_location: Some(".webfont-cache".to_owned()),
    ..Default::default()
};
```

:::

Node.js, the CLI, and the Rust crate store the cache in a directory. The browser package has no filesystem, so it hands you the bytes to keep in IndexedDB or elsewhere; the Rust in-memory API works the same way. See each tool for the details: [Vite](/vite/how-it-works#caching), [Node.js](/node/configuration#cachelocation), [browser](/browser/usage#persistent-cache), [CLI](/cli/usage#cache-options), and [Rust](/rust/usage#persistent-cache).

## What gets reused

- Icons are keyed by their SVG contents, not their paths, so moved files and other checkouts still hit. See [share one cache between checkouts](#share-one-cache-between-checkouts).
- When only CSS, HTML, or TypeScript options change (templates, `cssFontsUrl`, `templateOptions`, …), every font comes from the cache.
- Formats are cached separately, so builds that differ only in a format option, such as WOFF2 compression quality between development and production, share the rest. Each format keeps its latest entry plus one for the same icons with different format options.
- Builds that request different `types` (for example WOFF2 only in development, all five in production) still share fonts and parsed glyphs, but processed outlines are kept for the latest `types` only, so alternating between them reprocesses paths.
- TypeScript output is rendered from the current icon names on every run, even when all fonts come from the cache.
- A single-design font and a [variant family](/guide/features/variants) with the same `fontName` replace each other's cache entry, so alternating between them runs uncached.
- A cache from another engine version or with other settings is ignored.

::: tip Fix the TTF timestamp
Every TTF records a creation time, which WOFF and WOFF2 carry over. A reused font keeps the timestamp of the run that created it, so without a fixed `formatOptions.ttf.ts`, formats in one output can carry different timestamps. Set `ts` for reproducible output. The Node.js package and the Vite plugin always set it for you.
:::

## Share one cache between checkouts

Because the cache is keyed by contents, the path to an icon doesn't matter: the same SVG hits wherever it lives. Point every checkout of a project at one cache directory outside them, and each checkout reuses the others' work:

- **Git worktrees and coding agents.** Agents that each work in their own worktree, or your own worktrees for parallel branches, start warm instead of rebuilding every icon. A branch that changes three icons only processes those three.
- **Fresh clones and CI checkouts** reuse the work of earlier clones on the same machine.
- **Renamed or moved folders** keep their hits.

```ts [Node.js]
import { homedir } from 'node:os';
import { join } from 'node:path';

await generateWebfonts({
    files,
    dest: 'dist/fonts',
    cacheLocation: join(homedir(), '.cache', 'my-app-icons'),
});
```

With the CLI, pass the same path to `--cache-location` in every checkout. Several processes can use one directory at the same time: each write goes to its own temporary file and then replaces the cache file in one step, so a reader never sees a half-written file.

A few things still keep checkouts apart:

- **One file per font name and settings.** Two checkouts that build the same `fontName` with different settings that change the glyphs, such as `fontHeight`, replace each other's file, and both run cold. Give them different font names or cache directories. Format options and theme palette slots don't count: they share a file.
- **Engine versions.** A checkout with another version of the engine ignores the file and replaces it.
- **The Vite plugin** keeps its cache in Vite's `cacheDir`, inside each checkout's `node_modules` by default.

## Inside the cache file

Each font gets one file in the cache directory, named after `fontName` and a hash of its settings: for example `icons.3f2a9c41d0b7e8a5.wfcache`. A [variant family](/guide/features/variants) gets a separate file from a single-design font. When settings that change the glyphs change, the next run starts a new file and removes the older one for that font name.

It's a compact binary file, not meant for editing. The browser package's `exportCache()` and the Rust in-memory API's `cache_bytes()` return the same format.

<CacheFileLayout />

- **Header:** a `WFGCACHE` signature, the file format version, the engine version, and a fingerprint of the settings that change glyphs. Files from another engine version or with other settings are ignored, so upgrading the engine starts cold once.
- **Per-icon work:** each SVG's parsed outlines, color layers, and processed glyph, keyed by a hash of the SVG's text. It's only decoded when something must be rebuilt. Entries are sorted, so the same icons always produce the same bytes and an unchanged run skips the write.
- **Finished fonts:** up to two entries per format, the latest one and one with other format options. Each is keyed by its format options and an identity of the icon set: names, codepoints, and contents.
- **Checksums:** each section carries a CRC32 and its length. A damaged font section makes the run a miss; a damaged per-icon section is caught and repaired by the next run that needs it.

## Failures are never fatal

The cache is an optimization only. Read or write failures, missing permissions, and corrupt or truncated entries never fail generation: the run recomputes and replaces what it can, and an unwritable location stays uncached. Saved fonts and saved per-icon work carry separate checksums, so accidental corruption is detected rather than reused.

## With incremental updates

When a result is created with both a cache and [`incremental`](/guide/features/incremental), every committed update queues a background cache write. Pending writes are coalesced, and encoding and disk I/O don't block the next rebuild. Before shutdown, wait for them: `result.flushCache()` in Node.js, `flush_cache()` or `flush_cache_async()` in Rust. The Vite plugin does this when the dev server closes, and the browser package writes nothing on its own, since you store what `exportCache()` returns.

## Clear the cache

Delete only what the engine created with `clearCache()` in Node.js, `clear_cache()` in Rust, or `webfont-generator clear-cache --cache-location <DIR>`. Other files in the directory stay. Stop incremental updates and flush pending writes first; otherwise a background write can recreate a removed file. For the Vite plugin, delete the `vite-svg-2-webfont` folder inside Vite's `cacheDir`.

## Keep it between CI runs

The cache directory is a plain directory, so any cache your build system already provides can carry it between runs. Restoring an older cache is still useful: icons that didn't change hit even when others did, and anything stale is ignored.

::: code-group

```yaml [GitHub Actions]
- uses: actions/cache@v6
  with:
      path: .webfont-cache
      # A new key per icon change saves an updated cache; restore-keys reuse the latest older one.
      key: webfont-${{ runner.os }}-${{ hashFiles('icons/**/*.svg', 'pnpm-lock.yaml') }}
      restore-keys: webfont-${{ runner.os }}-
- run: node scripts/build-icons.mjs # generateWebfonts({ ..., cacheLocation: '.webfont-cache' })
```

```yaml [GitLab CI]
build-icons:
    script:
        - node scripts/build-icons.mjs # generateWebfonts({ ..., cacheLocation: '.webfont-cache' })
    cache:
        key: webfont-$CI_COMMIT_REF_SLUG
        fallback_keys: [webfont-$CI_DEFAULT_BRANCH]
        paths: [.webfont-cache/]
```

```dockerfile [Dockerfile]
# syntax=docker/dockerfile:1
FROM node:24 AS icons
WORKDIR /app
COPY . .
# BuildKit keeps the mount between builds; it never becomes part of the image.
RUN --mount=type=cache,id=webfont-cache,target=/app/.webfont-cache \
    node scripts/build-icons.mjs
```

:::

The same setups work for the CLI with `--cache-location`, and for the Vite plugin by caching `node_modules/.vite`.

::: warning Only restore trusted caches
Cached fonts are reused as-is, so a cache is trusted build output. Only restore caches written by trusted builds, and clear the directory if in doubt. GitHub Actions already isolates caches saved by pull requests from the default branch; with other systems, avoid sharing one cache between untrusted branches and release builds. In the browser, only pass bytes your own app stored, never bytes from users or other origins.
:::
