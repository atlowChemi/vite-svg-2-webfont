---
description: Rebuild a font after icons are added, changed, or removed, reprocessing only what changed, in Node.js, the browser, Rust, and the Vite dev server.
---

# Incremental Updates

An incremental result keeps the engine's work in memory, so it can be updated when icons are added, changed, renamed, or removed. Only the changed icons are parsed and processed again; every requested format is then rebuilt from the kept work, and CSS and HTML are reused when no icon names or codepoints changed. The result is byte-identical to a fresh build of the same icons.

Use it for watch mode, dev servers, and editors: anywhere one process rebuilds the same font many times. Leave it off for one-off builds, so the kept work isn't held in memory for nothing. To reuse work across processes instead, use the [engine cache](/guide/features/caching); the two combine.

## By tool

| Tool    | How                                                                                                                                                                                              |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Vite    | Automatic in the dev server. SVG changes rebuild the font and update the page through HMR; see [development](/vite/how-it-works#development).                                                    |
| Node.js | Generate with `incremental: true`, then call `regenerate`, `regenerateAsync`, or the single-file `add`, `update`, and `remove` helpers. See [Node.js usage](/node/api#regenerate-files-changes). |
| Browser | Generate with `incremental: true`, then call `add`, `update`, `remove`, or `regenerate`. See [browser usage](/browser/usage#incremental-updates).                                                |
| Rust    | Generate with `incremental: Some(true)`, then call the `regenerate` family or, for in-memory sources, `add`, `update`, and `remove`. See [Rust usage](/rust/usage#incremental-rebuild).          |
| CLI     | Not available: each run is a separate process. Use the [engine cache](/guide/features/caching) to reuse work between runs.                                                                       |

```ts [Node.js]
let result = await generateWebfonts({ files, dest: 'dist/fonts', incremental: true });

// On a watch event:
result = await result.updateAsync('icons/search.svg');
result = await result.addAsync('icons/bell.svg');
```

## Two ways to describe a change

- **One icon at a time:** `add`, `update`, and `remove` change a single icon, using the result's current list of icons. An added icon goes at the end; an updated icon keeps its position and, unless you rename it, its name.
- **The complete list:** `regenerate` takes every icon the font should contain, in the order a fresh build would use (for example, your glob result), plus optional hints about what changed. Icons missing from the list are dropped. Without hints, the engine re-reads every icon and works out what changed. Use it for batches, explicit ordering, or moving icons between designs.

## Families

[Variant families](/guide/features/variants) update the same way, with a list per design. Every design must be listed exactly once, in any order, and each list is the complete membership of that design. The designs and their weights are fixed: adding, removing, or reordering designs needs a fresh generation. Codepoints, shared metrics, and fallback icons are recomputed after every update.

## When an update fails

Invalid input, a missing file, or a failed font build rejects the update and keeps the previous result, so you can fix the input and retry. The browser package queues updates in the order you call them. In Node.js and Rust, an async update returns a replacement result: use it for the next update, and don't start another update before the previous one finishes.

Writing files is not transactional. When a result writes its output to disk, a write error can leave some files updated and others not; retrying the update, or regenerating from the complete list, finishes the writes. The exact contract differs between synchronous and async methods; see each tool's reference.

## Restrictions

- In Node.js, results created with `cssContext` or `htmlContext` callbacks can't be updated, because the callbacks can't run again during an update.
- Options such as formats, [color selection](/guide/features/color-glyphs), and templates are fixed for a result's lifetime. To change them, generate again.
