# Selective COLR v1 Color Glyphs PRD And Implementation Plan

## Status

- Last updated: 2026-10-04.
- Product direction: PR 1 implementation is authorized on `colr-v1/paint-pipeline`: internal selection, ordinary paint extraction, per-layer geometry, and cache support. Later stack layers follow the delivery plan.
- Authority: the current engine, adapter, and tests are authoritative for existing variant behavior. The previously referenced `local-docs/multi-weight-variable-font-plan.md` is absent; this plan is self-contained and does not depend on that document.
- Scope: full ordinary and multi-variant color support, INCLUDING the existing incremental regeneration lifecycle.
- Feasibility status: browser compatibility and initial SVG scope are resolved. Ship COLR v1 with documented WebKit monochrome fallback and basic ordinary solid-fill extraction. Advanced CSS, complex evenodd geometry, and comprehensive unsupported-source detection are deferred without special rejection machinery. Production integration and font-table bounds still require verification.
- Delivery method: three stacked pull requests, opened incrementally as drafts and merged bottom-up after the complete stack passes its release gate.
- Initial implementation scope: Rust engine/CLI in `crates/webfont-generator`, NAPI/Node adapter in `packages/webfont-generator`, and their public documentation.
- Vite plugin support is excluded until the generator feature is shipped.

## Decision Log

- 2026-10-04: User authorized committing, pushing, and opening PR 1 as a draft after review fixes. Draft checkpoints may be opened before the complete stack is implemented; keep the stack draft until its full release gate passes. This supersedes the original all-at-once submission timing.

- 2026-10-04: User authorized starting PR 1 after approving the basic SVG scope. Implement Phases 1–2 with focused verification, retaining the local stack foundation; public font emission/API work belongs to subsequent layers.

- 2026-09-03: Approved an opt-in all-or-named logical-glyph selection.
- 2026-09-03: SVG's unauthored initial fill maps to foreground; authored fills inherited from root/groups/styles remain fixed.
- 2026-09-03: Active color selection rejects EOT and legacy SVG font output.
- 2026-09-03: COLR v1 is required so foreground paint can retain independent fill opacity.
- 2026-09-03: The feature will account for multi-weight glyph identity, `rvrn`, conditioned `liga`, and sparse variants from the start. The historical static legacy-output assumption is superseded below.
- 2026-09-03: Delivery uses three stacked PRs opened after the complete stack is green.
- 2026-10-04: Reconciled against the split engine/adapter implementation. Variants already reject EOT/SVG and default to WOFF/WOFF2; ordinary defaults remain EOT/WOFF/WOFF2.
- 2026-10-04: Variant regeneration is existing functionality and mandatory for color support, including fallback propagation, post-rename selection, fresh/incremental parity, and failure recovery.
- 2026-10-04: Approved concurrent plan reconciliation and independent test-only feasibility work, not full implementation. All feasibility gates remain pending until evidence is reviewed and recorded.
- 2026-10-04: Approved a basic first release with best-effort handling of advanced SVG input. Defer paint-dependent selectors, unproven CSS resets/invalid declarations, complex intersecting evenodd conversion, and source-level unsupported-content preflight. No dedicated detection, warning, or rejection is required for these cases; visually incorrect or incomplete results are accepted limitations to improve through user issues/PRs. Reuse usvg stylesheet handling rather than building a separate stylesheet validator. This supersedes earlier strict source-rejection and exact-conversion gates below.

### 2026-10-04 Feasibility Results

Historical research record: strict rejection/exact-fidelity proposals and initial blockers in this section through Step 1 are superseded by the approved basic first-release scope in Product Decisions and Supported SVG Paint Contract.

Test-only evidence is in `crates/webfont-generator/src/sfnt/builder/tests/color_proof.rs`, `crates/webfont-generator/src/svg/color_proof.rs`, and `packages/webfont-generator/tests/browser/color-variable-font.test.ts`. Reproducible TTF/WOFF/WOFF2 fixtures live in that browser directory's `fixtures/` directory.

- TTF semantic parsing passed with COLR v1/CPAL, fvar/STAT, rvrn, conditioned liga, and appended unmapped layer glyphs.
- WOFF1 decompressed tables match TTF. WOFF2 reference decoding preserves color/layout tables and expanded outlines/metrics.
- Chromium 153.0.8010.12 and Firefox 155.0 passed color assertions. WebKit 26.6 rendered monochrome fallback in all three containers: the three-browser color gate FAILED. This observation does not yet isolate general COLR v1 support from proof-font or variable-layout compatibility.
- Browser samples cover direct U+E001 and `ab` ligature, weights 100/300/400/499/500/501/600/700/900, green/magenta foreground, invariant red/blue fixed paint, 25% foreground alpha, and equal advances.
- Zero advance for layer glyphs works, but correct sidebearings must be retained; zeroing the entire hmtx record displaced layer geometry.
- Locked usvg is 0.48.1 and has no `Options::color` field. A root color/fill sentinel approximation fails explicit/inherited currentColor with authored color, including attribute, inline-style, group, root, and stylesheet cases. The original sentinel proposal is rejected as an implementation strategy.
- Source content can disappear before tree inspection: foreignObject, unavailable images, and text; unresolved filters can remove paths and unresolved clip references disappear. Source-level preflight is required alongside resolved-tree validation.
- CSS reset fixtures observed inherited red for `inherit`, and black for `initial`, `unset`, `revert`, and `revert-layer`. These observations are not an approved product policy for resets.

Commands reported by the proof runner:

| Command                                                                           | Result                                                              |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `UPDATE_COLOR_PROOF_FIXTURE=1 vp run @atlowchemi/webfont-engine#test:color-proof` | Fixtures generated; five proofs passed                              |
| `vp run @atlowchemi/webfont-engine#test:color-proof`                              | Passed                                                              |
| `vp run @atlowchemi/webfont-generator#check`                                      | Passed                                                              |
| `vp run @atlowchemi/webfont-generator#test`                                       | Passed, including existing variant incremental tests                |
| `vp run @atlowchemi/webfont-generator#test:browser`                               | 15 passed; explicitly records WebKit fallback, NOT color acceptance |
| `VITE_COLOR_PROOF_STRICT=1 vp run @atlowchemi/webfont-generator#test:browser`     | 12 passed; three WebKit color failures                              |
| `vp check`                                                                        | Passed formatting, lint, and types                                  |

At this initial checkpoint the strict browser command tested the original all-engines color gate. That requirement is superseded by the compatibility decision below: strict mode remains an optional diagnostic, not a release gate. Supported browser builds regenerated native artifacts with no tracked binding changes reported.

Next investigations: isolate WebKit using a minimal static COLR v1 control and a known-good control font, then compare with the combined proof; investigate a usvg provenance hook retaining initial/authored/currentColor classification through its existing cascade; define reset semantics and source-preflight handling for referenced versus unused content. An upstream change or maintained parser patch requires an explicit implementation/dependency decision. Full color-aware incremental behavior, fill-rule conversion, and layer/palette boundary handling are not established by these spikes.

### Historical Cross-Check

- The original multi-weight implementation conversation (`ses_fa41ce1ebffeMUIxFgFzrJaru9`, 2026-09-01 results) records an apparent WebKit failure caused by a non-conforming proof: missing STAT and fvar.axisNameID set to 1 instead of the required 256..32767 range. Correcting both made all three browsers pass without adding gvar. This was an implementation defect, not a browser capability limit.
- The current color proof copies fvar/STAT from the corrected `build_proof_font`; existing assertions check axis name ID 256. The same missing-metadata defect is not apparent here, but the new color proof still needs specification-level validation before attributing its fallback to WebKit support. Current tests observe successful weight switching with monochrome fallback, a different symptom from the historical failure.
- Upstream usvg 0.47.0 and 0.48.0 Options definitions both lack `color`, as does the locked 0.48.1 version. Repository history records the 0.47-to-0.48 upgrade on 2026-08-07, before the color plan's 2026-09-03 date. Evidence points to an incorrect planning API assumption rather than that upgrade removing the option. The parser itself exists and works; the missing item is only the proposed configuration field.
- Failure of root sentinel injection does not prove that every provenance-preserving transformation is impossible or that a parser fork is required. Investigate available stylesheet injection and cascade behavior as well as parser hooks before choosing a dependency change.

### Follow-Up: Independent WebKit Control And Revised Paint Contract

- WebKit issue [233496, Support for COLRv1 color vector fonts](https://bugs.webkit.org/show_bug.cgi?id=233496) remains NEW. Maintainer comments place Apple-platform support in the platform text libraries. This is a missing-support report, not evidence that our proof triggered a specific implementation bug.
- A separate `vp exec node --input-type=module` Playwright diagnostic loaded Google Fonts' `twemoji_smiley-glyf_colr_1.ttf` from the `googlefonts/color-fonts` repository. Its table directory has no fvar or STAT. The diagnostic mapped only private-use U+E001 to the existing smiley GID 3 and recomputed checksums, preventing system emoji fallback from masquerading as control-font rendering. No repository fixture was replaced.
- Chromium 153.0.8010.12 rendered 10,657 opaque non-green pixels; Firefox 155.0 rendered 10,677. WebKit 26.6 rendered zero. `CSS.supports('font-tech(color-COLRv1)')` returned true, true, and false respectively. Together with the combined proof's monochrome fallback, this supports a COLR v1 capability limitation in the tested WebKit/platform combination, independent of weight switching. It is not a claim about every future WebKit/OS version.
- Targeted COLR specification review found no contradiction in CPAL presence, sorted base records, layer/palette references, foreground alpha, zero layer advances, or bounded PaintGlyph graphs without optional clip boxes. This is not a completed whole-font conformance audit.
- Revised product direction from the user: honor SVG-authored `color` when resolving currentColor. Root, group, shape, and stylesheet color declarations resolve through normal cascade/inheritance; descendant color overrides remain effective. Only currentColor without an authored effective color maps to host foreground. Root `fill` and root `color` are different properties: fill directly sets inherited paint; color supplies the value used by currentColor.
- Under this revision, the existing seven authored-color provenance cases already yield the desired fixed red. Their historical test name/comment describes failure against the superseded unconditional-foreground contract. Root-marker extraction is a candidate again, not production-proven: reset keywords, invalid declarations, XML rewriting, references, and path correspondence still need coverage. A parser patch is not presently demonstrated to be necessary.
- At this checkpoint browser release policy was unresolved; the subsequent compatibility decision below supersedes that blocker.

### Approved Browser Compatibility Contract

- 2026-10-04: Accept and document the browser limitation. COLR v1 remains the color representation; no additional WebKit color representation is required for this release.
- Chromium and Firefox must pass full fixed-color, host-foreground, alpha, layer-order, weight-selection, ligature, and advance assertions.
- The tested WebKit/platform combination must render the monochrome glyf fallback correctly: selected weight outline, direct/ligature parity, host text color, and shared advances. Fixed colors and independent per-layer opacity are unavailable in that fallback. Full-color WebKit rendering is not a release gate.
- Retain automated coverage in all three engines and record versions/platforms. Update capability expectations deliberately when browser support changes; do not dynamically excuse a color regression in an engine expected to support COLR v1. The existing research fallback branch must become an explicit compatibility assertion with this contract documented in the test.
- Public docs must state the tested compatibility matrix and fallback behavior without implying a permanent limitation in all future Safari/WebKit versions. CSS uses the same font resource; applications do not need an alternative API to obtain the built-in fallback.
- 2026-10-04: User accepted the limitation after viewing both authored-root-fill and foreground-layer demos in all three browsers. Shipping documentation must explicitly describe loss of fixed colors and per-layer opacity in the tested WebKit fallback and link to [WebKit issue #233496: Support for COLRv1 color vector fonts](https://bugs.webkit.org/show_bug.cgi?id=233496).

### Step 1: Expanded Extraction And Geometry Research

2026-10-04: Expanded test-only proofs under `src/svg/color_proof.rs`, `src/svg/color_proof/{preflight,geometry}.rs`, and `src/sfnt/builder/tests/color_proof.rs`. This step records evidence, including counterexamples; it does not expose a production option or silently narrow the supported SVG contract.

**Established by focused tests:**

- Authored effective color, descendant overrides, root/group/path fills, whitespace in inline declarations, CSS specificity/important precedence, inherited fill, and local use references produce the expected paint under root-marker extraction in the tested cases.
- A source-range-based root QName insertion preserves XML declarations, DTD entities, namespace prefixes, comments containing apparent SVG tags, quoted greater-than signs, IDs, and self-closing roots. Existing attributes are not replaced. A resolved-color collision fixture selects a different marker. Source path order and absolute transforms are unchanged in a referenced/transformed two-layer fixture.
- A structural source-preflight prototype detects dropped images/text/foreignObject, explicit unsupported effects, unresolved/external/cyclic use references, and unsupported content in referenced definitions while ignoring unused definitions. This prototype intentionally does NOT claim complete CSS/visibility validation; a stylesheet filter can still remove a path before normalized-tree inspection. Production validation needs both source CSS inspection and resolved-tree checks, with deterministic per-state error context.
- Recursive PaintColrLayers grouping round-trips 1, 255, 256, 511, and 65,026 leaves in exact source order. Groups contain at most 255 children; the largest fixture exercises more than two levels. This removes the need for a 255-source-layer product cap. These are semantic table tests, not browser stress/performance acceptance for 65,026 layers.
- One CPAL palette with 65,535 entries serializes and parses; 65,536 is rejected before narrowing. Fixed indices end at 65,534; 0xFFFF remains foreground. Glyph-count and offset limits still require checked validation during production allocation/serialization.

**Counterexamples requiring implementation decisions:**

1. Root-marker attributes alter selector matching. `<style>svg[fill] path {fill:red}</style>` does not match an originally fill-less root, but does match after injection. Thus a universal claim that root injection preserves the authored cascade is false. Injecting a normal `svg {fill:marker}` stylesheet also overrides an authored root presentation attribute; it is not an initial-value API. A provenance-preserving parser hook is the general solution; a deliberately restricted selected-SVG CSS contract is an alternative requiring user agreement. Do not silently reject previously promised stylesheet cases or silently return altered paint.
2. CSS `inherit` works for the tested descendant fill. usvg returns black for `initial`, `unset`, `revert`, and `revert-layer` in the tested inherited-red case. In particular, `unset` should inherit an inherited property, so observed black is not evidence of correct reset semantics. Proposed first-release handling is to support proven inherit cases and fail clearly for unproven reset/invalid declarations, subject to user approval, rather than interpreting parser fallback black as reliably authored paint.
3. Color nonzero paths must preserve their original contour winding. Existing monochrome containment normalization creates a hole in a deliberately filled nested nonzero path. For evenodd, nesting-only reversal handles simple nested holes but fails overlapping contours; retain fill-rule metadata and use a proven per-path conversion. Never apply the fallback heuristic indiscriminately to a color layer.
4. The existing oxvg_path optional boolean API was tested separately against nested/opposite-winding rectangles, overlaps, a bow-tie, and a curved ring. Polygon fixtures matched sampled SVG membership for both rules. The curved ring produced 12 membership mismatches per rule, with sampled disagreement up to 2.407 source units from the original boundary on a 100-unit fixture. This is not acceptable evidence of exact geometry conversion; do not adopt the API unconditionally based on polygon success.

The optional algorithm feature is not enabled in the workspace. A test-only activation attempt failed because the pnpm-managed Cargo source directory lacks i_float ^3.0 (it contains 1.16.0); `vp install` and a forced install did not populate that optional dependency. The temporary dev-dependency change was removed. The isolated geometry experiment used the exact workspace-vendored oxvg_path source with algorithm enabled in a temporary standalone crate and registry-resolved optional dependencies. `vp run proof -- --nocapture` reported measurements, not a successful geometry acceptance gate. Its source and manifest are under the approved temporary directory's `color-geometry-proof/`; no workspace dependency or lockfile change is retained.

**Decision resolved:** the user approved the basic, best-effort boundary below. The counterexamples remain research evidence, not release blockers or requirements to add rejection machinery. Earlier preflight and exact-conversion proposals in this research history are superseded.

Verification for this research checkpoint: `vp run @atlowchemi/webfont-engine#test:color-proof` passed 16 focused tests; generator `#check` and `#test` passed; generator `#test:browser` passed 15 tests under the approved fallback contract; the verification runner reported `vp run test` passed 585 tests with one skipped. No tracked binding, dependency, or lockfile changes resulted. The research tests include assertions demonstrating limitations, so their success does not clear those limitations as implementation gates.

Created the local stack foundation after the research step: `main ← colr-v1/paint-pipeline`. Changes remain uncommitted; no PRs have been opened. The planned next layers remain font construction and public API/lifecycle, added only when their work begins.

### PR 1 Implementation Checkpoint

2026-10-04: Implemented the internal Phases 1–2 pipeline on `colr-v1/paint-pipeline`:

- `SvgOptions` accepts internal all/named/disabled logical selection; public option conversion still supplies no selection.
- `svg/parse/color.rs` performs collision-free root-marker extraction using parsed XML root ranges. Parsed glyphs retain optional ordered solid paint, alpha, path indices, and computed fill rules. Geometry and paint come from the same marked tree, avoiding cross-tree positional matching. Deferred input uses the approved best-effort behavior.
- Processing derives layers from the same transformed/centered paths as fallback, preserves nonzero winding, normalizes basic nested evenodd holes per path, and rounds/optimizes each layer independently. Fallback keeps the existing merged containment behavior.
- Parsed/processed caches retain layers. Per-path reuse validates selection mode, including logical renames; content-hash reuse requires matching mode. The content-hash map retains its most recently parsed mode while per-path entries can retain both modes for identical source bytes. Variant processed caches invalidate on paint/source or selection changes and reuse resolved fallback layers at the shared advance.
- Eight production-pipeline tests cover paint inheritance/order/alpha, authored currentColor, XML entities/namespaces/marker collisions, empty selection/glyphs, ordinary curves and holes, reflected/shared transforms, optimized geometry, no-op reuse, selection/rename transitions, and fresh-versus-cached ordinary/variant parity with blank/fallback states and paint edits.

Verification passed: generator `#check`; generator `#test` (760 executions across default/CLI engine, integration, doctests, and adapter); `vp check`; and `vp run test` (585 passed, one skipped, including the three-browser research fixtures). Existing `ordinary_generation_matches_phase_zero_hashes` and variant regression tests remain green. The build produced no tracked binding/platform artifact changes. Performance measurements and the complete release gate remain later work; browser color fixtures are still manually assembled research fonts until PR 2 connects font emission. Changes are uncommitted and no PR has been opened.

### PR 1 Review Follow-Up

Independent GPT-6 Astra and Opus 5.5 reviews identified two actionable findings and focused coverage gaps. The follow-up changes are:

- Selected evenodd layers now classify ordinary non-intersecting nesting using actual curve winding and signed area via the existing kurbo dependency. This fixes a small nested hole close to a cubic boundary that the six-sample monochrome containment polygon missed. Original curve segments are preserved when reversing contours; the existing monochrome fallback heuristic is unchanged. Complex intersections remain best-effort as approved.
- Engine color proofs validate generated font/container data without reading downstream files. They optionally export bytes to `COLOR_PROOF_OUTPUT_DIR`. Adapter task `vp run @atlowchemi/webfont-generator#test:color-proof` owns comparison with its browser fixtures and is a dependency of adapter `#test:browser`. Refresh those fixtures with `UPDATE_COLOR_PROOF_FIXTURE=1 vp run @atlowchemi/webfont-generator#test:color-proof`; the older engine-task refresh command in the historical log is superseded.
- Pipeline tests now explicitly cover the reported curved-edge hole (with optimization on/off), fixed black beside foreground, stroke-only/mixed-fill fallback parity, differently sized variant sources with normalized/non-normalized centering and shared advances, variant no-op reuse counts, and variant logical renames into/out of named selection. Paint-pass errors retain source path context.

Follow-up verification: generator `#check` passed; generator `#test` passed 768 executions (12 production-pipeline tests in each engine configuration); adapter `#test:color-proof` matched all three checked-in browser fixtures; adapter `#test:browser` passed 15 tests. Existing ordinary hash baselines remain green, and no browser fixture refresh was needed.

## Goal

Allow callers to opt selected logical glyphs, or every logical glyph, into solid-color SVG paint preservation. Generate COLR v1 and CPAL data in TTF, WOFF, and WOFF2 while retaining a normal monochrome `glyf` fallback. Support both ordinary single-font input and the discrete multi-weight variant input that uses `wght`, `rvrn`, and conditioned `liga`.

Given this source:

```svg
<svg viewBox="0 0 24 24" fill="black" xmlns="http://www.w3.org/2000/svg">
  <path d="..." fill-opacity="0.1" />
  <path d="..." />
  <path d="..." fill="#F83743" />
</svg>
```

the generated color glyph has three ordered layers:

1. Fixed black at 10% opacity because the root authored `fill="black"` is inherited.
2. Fixed opaque black for the second inherited path.
3. Fixed opaque `#F83743` for the directly painted path.

If the root `fill="black"` is removed, the first two layers instead use `currentColor`, with the first layer retaining 10% opacity.

## Product Decisions

These decisions are fixed unless the product owner changes them explicitly.

1. The Node option is `colorGlyphs?: true | string[]`.
2. `true` selects every logical glyph. A string array selects post-rename logical glyph names. An empty array selects none.
3. Selection is by logical glyph name, not source path. In variant mode one selected logical name applies to every resolved variant state for that name.
4. The Rust API uses a typed all-or-named selection rather than an untyped JSON value. The exact NAPI representation may use a manual conversion and `ts_type` annotation, but the Node behavior is normative.
5. The option is top-level because it affects SVG parsing, SFNT construction, WOFF containers, validation, caching, and result hashes.
6. Omission preserves the existing monochrome path, output bytes, and ordinary-generation performance characteristics.
7. SVG's initial, unauthored black fill maps to font foreground color (`currentColor`).
8. Within the ordinary supported subset, an authored fill from a shape, ancestor group, root element, inline style, or internal stylesheet remains the computed fixed color after usvg cascade and inheritance. Advanced CSS cases follow the best-effort policy below.
9. `currentColor` resolves through authored SVG `color` declarations and normal inheritance/overrides. If an effective authored color exists, preserve that fixed color; otherwise use host foreground. Do not erase authored color to force foreground.
10. Effective `fill-opacity` is preserved for foreground and fixed-color layers.
11. Source paint order is preserved. Layers are composed bottom-to-top in source rendering order.
12. The first release targets ordinary solid-fill icons. Advanced SVG paint, CSS, and compositing are best-effort outside the supported subset.
13. Gradients, patterns, masks, filters, clipping, group/object `opacity`, blend modes, images, text, and strokes have no fidelity guarantee. No dedicated source preflight, detection, warning, or rejection is required; parser omissions or inaccurate rendering are accepted initial limitations. Unselected glyphs keep current monochrome behavior.
14. `fill="none"` contributes no fill layer. Stroke preservation is deferred; selected stroked content may be omitted or rendered incompletely.
15. TTF, WOFF, and WOFF2 are the only supported color outputs.
16. An active color selection combined with EOT or legacy SVG font output is a validation error. This includes EOT selected by the ordinary default output list; variant defaults are already compatible WOFF/WOFF2.
17. Variant requests retain their existing modern-only TTF/WOFF/WOFF2 contract, with or without color. There are no production static EOT companions.
18. COLR v1 is required. COLR v0 cannot apply independent alpha to palette index `0xFFFF`, which represents foreground color.
19. The first release uses static COLR v1 paints per discrete variant glyph. It does not add COLR variation data, a COLR VarStore, `gvar`, or interpolated colors.
20. Every glyph ID that `cmap`, `rvrn`, or conditioned `liga` can select has its own correct COLR presentation or intentionally has no COLR presentation.
21. A normal merged `glyf` outline remains on each selectable base/variant glyph as the fallback for renderers that ignore COLR.
22. Full multi-variant incremental regeneration is required in the initial color release. Extend the existing regeneration APIs, caches, validation, commit/rollback, and retry behavior rather than introducing a second lifecycle.
23. The multi-weight JSON manifest accepts `colorGlyphs`. No additional positional CLI flag is required initially.
24. No new dependency is added when `usvg`, `roxmltree`, `write-fonts`, or the standard library can provide the required behavior.

## Terminology

### Logical glyph

The name and codepoint shared across variants, such as `wifi` at `U+F101`. `colorGlyphs` string entries select this identity.

### Selectable glyph

A glyph ID that text layout can return. In ordinary mode this is the codepoint/ligature glyph. In variant mode it is a default or alternate variant glyph targeted by `cmap`, `rvrn`, or conditioned `liga`.

### Color base glyph

COLR terminology for a selectable glyph ID with a color presentation. Every painted default or alternate variant glyph is a COLR base glyph; “default glyph” separately means the default variant's presentation.

### Layer glyph

An auxiliary, unmapped `glyf` outline referenced by COLR `PaintGlyph`. Layer glyphs are never targets of `cmap`, `rvrn`, or `liga`.

### Foreground paint

The host text color, represented by COLR palette index `0xFFFF`. In CSS this normally follows the element's `color` property.

### Fixed paint

An authored solid SVG color stored in the font's CPAL palette and unaffected by the host text color.

## Rendering Model

OpenType layout completes before COLR painting. The generated paths are:

```text
Direct codepoint:
Unicode -> cmap -> default variant GID -> rvrn -> selected variant GID -> COLR presentation

Name ligature:
characters -> conditioned liga -> selected variant GID -> COLR presentation
```

COLR must be keyed by the final selectable glyph IDs. It must not duplicate `rvrn`, add color-specific substitutions, or target layer glyphs from GSUB.

For a selected logical glyph in variant mode:

- The default variant glyph receives the default source's presentation.
- Each non-default alternate receives its own variant source's presentation.
- A blank missing-glyph state remains an empty selectable glyph with no COLR record.
- A fallback state reuses the resolved fallback source's geometry and paint.
- Two variants may share a selectable glyph ID only if doing so preserves geometry, advance, missing-state semantics, and the complete ordered paint presentation.

## Supported SVG Paint Contract

### Supported

- Solid SVG colors accepted and resolved by `usvg`, including named colors, hex colors, and `rgb()` forms supported by the existing parser.
- `currentColor`.
- SVG's unauthored initial fill, mapped to foreground paint.
- Fill declarations on supported shape elements.
- Fill inherited from root elements and groups.
- Fill declarations from presentation attributes, inline style, and ordinary internal stylesheets through `usvg`; advanced selector/reset fidelity is deferred below.
- Effective `fill-opacity` in the range accepted by `usvg`.
- Existing SVG transforms and root viewBox correction.
- Existing supported path-producing shapes after `usvg` converts them to paths.
- Multiple paths and multiple subpaths in deterministic source rendering order.

### Deferred / Best-Effort For Selected Glyphs

- Linear or radial gradients.
- Patterns.
- Strokes, including elements with both fill and stroke.
- Group or object `opacity`; only `fill-opacity` is supported initially.
- Masks, filters, blend modes, and isolation/compositing behavior.
- Clip paths unless a later approved implementation converts the clipped result to stable geometry before layer construction.
- Raster images, foreign objects, and text content.
- External paint resources that cannot be resolved deterministically during generation.
- Paint-dependent CSS selectors affected by root-marker insertion, such as `svg[fill] path`.
- Unproven CSS reset keywords (`initial`, `unset`, `revert`, `revert-layer`) and invalid declarations.
- Intersecting or self-intersecting evenodd contours requiring geometry reconstruction.
- Other paint forms outside ordinary ordered solid COLR v1 layers.

These cases do not require detection or an error. Let usvg resolve stylesheets and normalize input using its existing behavior; do not add a CSS cascade, selector validator, parser fork, or source-preflight system for this release. Ordinary stylesheet fills can work through that path, but full stylesheet fidelity is not promised. Unsupported content may disappear or render incorrectly without a warning. Document this boundary and invite reproducible issues/PRs as users encounter gaps.

The initial geometry solution preserves nonzero contour winding and handles ordinary non-intersecting nested evenodd holes per path. Complex intersection detection and exact conversion are deferred. Existing parser/build errors still propagate normally. API/format validation and checked font-table construction remain part of implementation; this scope decision removes SVG-fidelity gates, not those existing correctness requirements.

## Public API Contract

### Node

```ts
interface GenerateWebfontsBaseOptions {
    colorGlyphs?: true | string[];
}
```

Examples:

```ts
await generateWebfonts({
    files,
    colorGlyphs: true,
    types: ['woff2', 'woff'],
    dest,
});

await generateWebfonts({
    files,
    rename: path => customName(path),
    colorGlyphs: ['status', 'warning'],
    types: ['woff2'],
    dest,
});
```

In variant mode the option remains in the shared base options:

```ts
await generateWebfonts({
    variants,
    colorGlyphs: ['status'],
    types: ['woff2', 'woff'],
    dest,
});
```

### Rust

Use a typed representation with these semantics:

```rust
pub enum ColorGlyphSelection {
    All,
    Glyphs(Vec<String>),
}

pub struct GenerateWebfontsOptions {
    // Existing fields.
    pub color_glyphs: Option<ColorGlyphSelection>,
}
```

`None` and `Glyphs(Vec::new())` select no glyphs. `All` selects every logical glyph. `Glyphs` entries are resolved after rename and logical-union construction.

If NAPI cannot derive the desired union directly, implement conversion for this field or use a narrow NAPI input adapter. Do not replace the typed Rust API with `serde_json::Value`, duplicate the complete options object, or expose an internal sentinel string.

### CLI Manifest

The variant JSON manifest accepts either:

```json
{ "colorGlyphs": true }
```

or:

```json
{ "colorGlyphs": ["status", "warning"] }
```

An empty array disables color generation. Positional CLI mode does not gain another flag unless users demonstrate a need after manifest support ships.

## Validation Rules

Validation is split according to when the required information exists.

### Input-Time Validation

- Reject a non-boolean/non-array Node or manifest value.
- Reject `false`; omission or an empty array is the disabled form.
- Reject non-string array members.
- Resolve requested formats using the same defaulting logic as generation.
- If selection is active, reject any resolved `svg` or `eot` output.
- Report the public camelCase path `options.colorGlyphs` and incompatible format names.
- Preserve the existing variant-mode EOT/SVG rejection when color is disabled.

Because ordinary default formats are `['eot', 'woff', 'woff2']`, this request is invalid:

```ts
generateWebfonts({ files, colorGlyphs: true, dest });
```

The error instructs the caller to provide compatible `types`, for example `['woff2', 'woff']`.

Variant defaults are `['woff', 'woff2']`, so `{ variants, colorGlyphs: true, dest }` is format-compatible without an explicit `types`. Do not copy ordinary defaults into variant validation. An empty selection preserves each mode's existing format behavior.

### Post-Load Validation

- Resolve string entries against final post-rename logical names.
- Reject unknown names together in deterministic input-list order.
- Duplicate names are harmless and may be deduplicated while preserving first occurrence; do not add a separate failure unless implementation ambiguity appears.
- In variant mode validate against the logical union, not independently against every variant's sparse file list.
- Missing states do not make a selected logical name unknown.

### Selected-Source Processing

- Extract ordinary solid paint after SVG cascade and missing-glyph resolution identify the actual source used by each state.
- Reuse resolved fallback sources and retain existing parser/build error context.
- Do not add unsupported-feature error aggregation or source/CSS preflight. Deferred cases are best-effort and need not produce diagnostics.

## Data Model

Current engine types are authoritative: `ParsedGlyph`, `CachedGlyph`, `ProcessedGlyph`, `CachedProcessedGlyph`, `PreparedVariantFamily`, and `ProcessedVariantGlyph` in `crates/webfont-generator/src/svg/types.rs`, plus `VariantGlyphCache` in `src/svg/mod.rs`. Extend these rather than introducing parallel source or glyph models.

The minimum ordinary parsed representation is conceptually:

```rust
enum ResolvedLayerPaint {
    Foreground { alpha: f32 },
    Solid { red: u8, green: u8, blue: u8, alpha: f32 },
}

struct ParsedColorLayer {
    path_index: usize,
    paint: ResolvedLayerPaint,
}

struct ParsedGlyph {
    paths: Vec<TinyPath>,
    color_layers: Option<Box<[ParsedColorLayer]>>,
    // Existing identity and dimensions.
}
```

Use path indices so color metadata does not duplicate source geometry. Retain each selected path's computed fill rule alongside paint. Keep `color_layers` absent when color processing is disabled, with no allocations proportional to glyph count for color-only state on that path.

After geometry processing, selected glyphs need optional ordered layer geometry:

```rust
struct ProcessedColorLayer {
    outline: Arc<BezPath>,
    outline_hash: u64,
    paint: ResolvedLayerPaint,
}

struct ProcessedGlyph {
    // Existing merged fallback geometry/path data.
    color_layers: Option<Box<[ProcessedColorLayer]>>,
}
```

The current processed carrier is `ttf_path: Option<Arc<BezPath>>`, `ttf_path_hash`, and `path_data: Arc<str>`. Follow that seam; recheck it if geometry changes before implementation.

For variants, attach optional paint layers to the `ProcessedGlyph` values in `ProcessedVariantGlyph.outlines: Box<[Option<ProcessedGlyph>]>`. `None` represents blank geometry; resolved fallback entries reuse source geometry with the logical glyph's shared advance. Do not add a separate color-only variant matrix. Cached parsed and processed equivalents must retain the same metadata.

## Distinguishing Default Fill From Authored Fill

`usvg` exposes computed paint but may not expose whether computed black came from SVG's initial fill or from an authored declaration. The distinction is required because initial black becomes foreground while authored black remains fixed.

Historical candidate using a nonexistent API, rejected by the 2026-10-04 spike (the actual initial strategy follows below):

1. Parse the selected SVG normally and collect every resolved fixed RGB color.
2. Choose an RGB sentinel not present in that resolved color set.
3. Parse the selected SVG for paint extraction with `usvg::Options::color` set to the sentinel so explicit `currentColor` resolves to it.
4. If the root has no authored fill attribute or inline fill declaration, inject a root presentation-attribute fill using the same sentinel before the paint parse. Descendant declarations and applicable stylesheet rules override inherited root presentation paint through the normal cascade.
5. Interpret a resolved sentinel solid fill as foreground paint.
6. Interpret every other resolved solid fill as fixed paint.
7. Preserve `usvg`'s computed fill opacity.

For the revised root-marker strategy, choose the sentinel from resolved colors rather than hard-coding it; an author may legitimately use any RGB value. Limit source rewriting to inserting missing root fill/color presentation attributes and preserve XML validity, DTD handling, IDs, and source ordering.

If the current `usvg` version exposes reliable specified-versus-initial paint provenance, use that API and skip the second parse. Do not add a private CSS cascade implementation.

### Initial Extraction Strategy

The engine locks `usvg` 0.48.1 with default features disabled. Use the tested two-pass root-marker approach for the ordinary subset, subject to production integration tests:

- `Options::color` does not exist in the locked version. Root marker injection must preserve authored effective color as fixed and classify currentColor without authored effective color as foreground; the original unconditional-foreground requirement is superseded.
- Distinguish unauthored fill from fixed root/group/path/inline paint, including ordinary inherited `currentColor` and tested descendant `inherit`.
- Choose a marker absent from resolved colors, preserve XML structure and authored attributes, and retain path order/correspondence for ordinary shapes and local references.
- Reuse usvg's existing stylesheet cascade. Paint-dependent selectors, resets, and invalid declarations remain best-effort; no additional selector/reset detection is required.
- Omit source-level preflight. Content dropped or changed by normalization is an accepted limitation outside the supported subset.

Keep research counterexamples as evidence for future improvements, without making their detection or correction an initial release gate.

## Geometry Processing

Color and monochrome processing share one geometry transform:

1. Parse source paths using the existing `usvg` flow.
2. Apply absolute transforms and root viewBox correction once.
3. Apply the shared ordinary or variant family scale, Y flip, centering, font height, ascent, and descent plan.
4. Retain transformed per-path geometry only for selected color glyphs.
5. Build the existing merged monochrome fallback from all visible filled geometry.
6. Run the existing containment/winding normalization across merged paths only for the fallback.
7. Preserve nonzero winding; normalize ordinary nested evenodd contours within each individual layer. Complex evenodd intersections are best-effort. Never use another painted layer as a knockout.
8. Apply rounding consistently to fallback and layer geometry.
9. If `optimizeOutput` is active, optimize each color-layer outline independently and optimize the merged fallback through the existing route.
10. Keep the same shared advance for every variant presentation of a logical glyph.

One transformed source path may contain multiple contours but remains one paint layer. Do not split contours into independently painted layers.

### Initial Fill-Rule Scope

SVG supports `nonzero` and `evenodd`; TrueType outlines do not carry an SVG per-path fill-rule switch. Retain the computed rule. Preserve nonzero contour direction and use a basic per-path containment approach for ordinary non-intersecting nested evenodd holes, preserving curves. Test ordinary nesting, disjoint contours, and reflected transforms. Intersecting/self-intersecting evenodd conversion and reliable detection of such geometry are deferred; inaccurate output in these cases is accepted without a special error. Do not adopt the optional boolean library merely to broaden initial scope. Keep the existing color-disabled fallback path unchanged.

## Glyph Store And ID Allocation

Start from the current SFNT builders. Ordinary allocation/dedup is in `sfnt/builder/glyphs.rs`; variant allocation is the `physical` vector, matrix, `add_presentation`, and `checked_gids` in `sfnt/builder/variants/mod.rs`. There is no pre-existing unified layer-glyph store abstraction. Extend these seams coherently, sharing only the necessary allocation/assembly helpers, with assigned IDs as the sole authority.

Deterministic order when color is active:

1. `.notdef` remains glyph ID 0.
2. Existing selectable ordinary or variant glyphs retain their current allocation order.
3. Existing ligature placeholders retain their current order.
4. Color layer glyphs are appended in selectable-glyph ID order, then source layer order.

Appending layers after existing selectable IDs prevents color auxiliaries from perturbing `cmap`, `rvrn`, or conditioned `liga` target calculations. All table builders must consume assigned IDs from the glyph store rather than repeat positional arithmetic.

Layer glyph requirements:

- No `cmap` entry.
- No GSUB target.
- Zero advance width is sufficient because COLR paints it at the color base glyph's origin.
- A deterministic internal PostScript-safe glyph name if `post` format 2 remains in use.
- Inclusion in `glyf`, `loca`, `maxp`, `hmtx`, and `post` counts.
- Validation that the expanded store fits the 65,535-glyph limit before narrowing indices.

Do not deduplicate auxiliary layer outlines in the initial implementation. Add that optimization only if enabled-color font-size measurements justify it. Selectable-glyph deduplication, however, is correctness-sensitive and must include the complete presentation identity.

## Selectable Glyph Identity And Deduplication

The existing path/advance deduplication may alias codepoints or variant states to one glyph ID. With color enabled, equality must include:

- Merged fallback outline.
- Advance width and relevant metrics.
- Whether a COLR presentation exists.
- Number and order of layers.
- Each layer outline.
- Foreground versus fixed paint.
- Fixed RGB color.
- Effective alpha.
- Missing-state semantics where required by `rvrn` construction.

Two glyphs with the same fallback outline but different colors or opacity cannot share a selectable glyph ID because COLR is keyed by glyph ID. A selected glyph and an unselected glyph should not alias merely because both currently look like opaque foreground; keeping selection in the identity avoids accidental future coupling.

A missing fallback may share the fallback source's selectable glyph ID when the existing variant substitution design allows it and the entire presentation matches. A blank state uses the existing empty alternate behavior and has no COLR record.

## COLR v1 Construction

Use the repository's locked `write-fonts` table types; consult `Cargo.toml` when implementation starts.

For each selected selectable glyph with at least one visible layer:

1. Add a COLR v1 `BaseGlyphPaintRecord` keyed by the selectable glyph ID.
2. Build a `PaintColrLayers` root in source order.
3. For each layer, build `PaintGlyph(layer_gid, PaintSolid(...))`.
4. Use palette index `0xFFFF` for foreground paint.
5. Use a CPAL palette index for fixed paint.
6. Encode effective opacity in the COLR `PaintSolid` alpha field.
7. Quantize alpha deterministically to F2DOT14 using `write-fonts`/font-types conventions.

CPAL policy:

- Emit CPAL whenever COLR is emitted; COLR is ignored without CPAL.
- Emit one default palette initially.
- Store unique fixed RGB colors in deterministic first-use order across selectable glyph IDs and layer order.
- Store CPAL colors with alpha 255; layer-specific effective opacity belongs in COLR.
- If every layer uses foreground paint, emit the minimum valid CPAL palette/record even though no paint references that record.
- Do not expose palette customization or alternate palettes in this feature.
- Reserve `0xFFFF` exclusively for foreground. With one palette and 16-bit CPAL entry/record counts, permit at most 65,535 fixed entries with indices `0x0000..=0xFFFE`; validate counts before conversion, including the all-foreground dummy-record case. Check serialized offsets and total record bounds as well as palette indices.

COLR policy:

- Emit version 1 only when at least one selected glyph has a color presentation.
- Do not add COLR variation data or variable alpha/color stops.
- Sort base-glyph records as required by the OpenType specification.
- Preserve bottom-to-top source layer order.
- Do not emit competing v0 records unless a later compatibility measurement justifies a mixed v0/v1 table.
- `PaintColrLayers.numLayers` is an 8-bit count, not an unlimited vector. Support presentations exceeding 255 layers using a deterministic nested `PaintColrLayers` graph (chunks of at most 255 child paints, recursively grouped as needed), retaining source order. Confirm nested graph serialization and renderer support in the proof; never truncate or wrap. Check global LayerList counts, first-layer indices, offsets, graph depth, and expanded glyph-count limits before narrowing. A resource limit/rejection instead of supported nesting requires an explicit reviewed decision.
- Pending boundary tests: 255 and 256 layers, multi-chunk ordering/alpha, legal fixed palette index `0xFFFE`, foreground `0xFFFF`, and palette/count/offset overflow. Exercise palette limits at the builder boundary without requiring an impractically large SVG fixture.

## Interaction With `fvar`, `STAT`, `rvrn`, And `liga`

- `fvar` and `STAT` remain unchanged by color selection.
- `rvrn` continues substituting the default selectable glyph ID with a variant selectable glyph ID.
- Conditioned `liga` continues outputting the variant selectable glyph ID directly.
- Each target selectable glyph ID receives the matching COLR record.
- Layer glyph IDs never appear in GSUB.
- COLR paint does not need a VarStore because variant changes are discrete glyph substitutions.
- Browser tests must cover exact named weights, quantized midpoint boundaries, and min/max clamping. The existing `variant_range` normalizes to F2DOT14, includes the upper variant at its quantized lower boundary, and ends the preceding range one F2DOT14 unit earlier; retain that behavior.
- Browser tests must cover both direct codepoint and ligature access because their GSUB paths differ.
- Shared advances must remain identical across painted variants.

## Output Format Behavior

### TTF

One ordinary or variable SFNT contains fallback outlines plus optional `COLR` and `CPAL`. Variant fonts also retain `fvar`, `STAT`, and FeatureVariations GSUB.

### WOFF1

The existing generic table packaging should carry `COLR` and `CPAL` unchanged. Tests compare decompressed table bytes with TTF and confirm variation tables remain intact in the same file.

### WOFF2

The existing known-tag handling already recognizes `COLR` and `CPAL`. Tests must verify the expanded `glyf`/`loca` store and all color, variation, and GSUB tables survive reference decoding together.

### EOT

Reject active color selection before ordinary EOT wrapping. Variant EOT is already invalid independent of color, and production variant generation has no EOT/static-companion construction path.

### Legacy SVG Font

Reject active color selection. Do not implement child-path SVG-font paint as part of this feature.

### CSS And HTML

- Existing `color` CSS continues controlling foreground layers.
- Fixed layers ignore host text color.
- Variant CSS continues using exact-weight faces and one shared modern variable URL.
- Variant requests already omit EOT sources because EOT cannot be requested, regardless of color.
- Existing output-format filtering and nullable legacy result fields remain authoritative.
- The default HTML preview may add one color/currentColor demonstration only if it can reuse existing context without new template API; otherwise browser fixtures cover it.

## Caching And Regeneration

### Ordinary Incremental Regeneration

- Parsed cache identity includes whether paint extraction was requested and all retained paint metadata.
- Content-hash reuse must not cross incompatible monochrome/color parse modes unless cached data contains everything required by both.
- Processed cache identity includes layer geometry, paint, shared metrics, optimization, rounding, and color mode.
- A rename-only change can move a source into or out of a named selection and must re-evaluate its presentation without stale reuse.
- Fresh generation and regeneration produce byte-identical TTF/WOFF/WOFF2 for the same final files and names.
- Existing parse/build/selection failures preserve the prior usable result and retry behavior.
- CSS/output hashes include color selection and resulting font bytes so URLs change when relevant paint changes.

### Variant Regeneration

This is required shipping scope. Extend `incremental/variants.rs`, `prepare_variant_family_cached`, and `svg::VariantGlyphCache` through the existing Rust/NAPI/Node lifecycle:

- Preserve `RegenerationFiles::Variants(Vec<VariantFileSet>)`, complete ordered file lists for every configured variant exactly once, and family-wide `Added`/`Changed`/`Removed` hints. Membership-only changes require no hint; `regenerate_all` re-reads the final lists. Keep existing incremental opt-in, callback restrictions, duplicate/unknown/missing-variant validation, and adapter busy/stale-result handling.
- Per-variant parsed caches and content-hash sharing retain paint metadata or partition by paint-extraction mode. Processed cache keys currently include source variant, path, and shared advance, with a family geometry signature; extend them with selection/presentation identity so neither name-only nor paint-only changes reuse stale layers.
- Rebuild the logical union, codepoints, missing states, and named selection after rename/membership changes. A selected name missing from the new union is an unknown-name validation failure, not permission to use a stale selection. Cover source renames into/out of an existing selected logical name as well as removal of a selected identity.
- A paint edit to one source invalidates all its direct and resolved fallback consumers, their presentation deduplication, COLR/CPAL ordering, fonts, and relevant hashes. Shared-path reads must remain consistent across variants. Blank states have no color record; fallback-to-source and source-to-blank transitions must clear stale records.
- Fresh versus incremental output must be byte-identical for fixed timestamp/options and identical final file/name order, including TTF/WOFF/WOFF2, COLR/CPAL, layout IDs, metadata, and rendered output. Cover hinted regeneration and full re-diff, no-ops, reordered files, additions/removals, and renamed files with identical content.
- Validation, parsing, geometry, or font-build failures before commit preserve previous usable fonts, names, hashes, and outputs. `RegenerationStateLease` discards dirty caches so a corrected retry is safe; test that no partial color state leaks.
- Preserve the existing variant write-error contract: new in-memory output commits before disk writes; write failure keeps that usable generation and `variant_write_pending` for retry. Do not promise disk-write rollback or substitute the ordinary write-error behavior. Test retry after a color edit and write failure, including a no-op retry batch.
- Exercise Rust synchronous/async/snapshot regeneration and the NAPI/Node result lifecycle with color variants; existing result ownership and retry semantics remain authoritative.

## Test Strategy

Prefer semantic table parsing and browser pixels over opaque binary snapshots.

### SVG Paint Fixtures

Add the smallest hand-readable fixtures needed for:

- No authored fill and opaque foreground.
- No authored fill plus `fill-opacity="0.1"`.
- Root-authored black inherited by paths.
- Group-authored color and opacity inheritance.
- Direct fixed color overriding inherited color.
- Direct and inherited `currentColor`.
- `currentColor` with authored/inherited/CSS `color` and sentinel-collision colors.
- Inline-style and internal-stylesheet fills.
- `fill="none"`.
- Multiple ordered overlapping layers.
- A path with multiple contours and a hole.
- Both fill rules with ordinary same/opposite contour directions, non-intersecting nesting, disjoint contours, and reflected transforms.
- Retain existing research counterexamples for resets, complex geometry, and dropped content as observations, not mandatory fidelity/rejection tests. Add future regression fixtures when reported cases are addressed.

Do not refresh unrelated existing SVG-font fixtures because color mode rejects SVG output and disabled mode must remain byte-identical.

### Rust Parser And Processing Tests

- Assert foreground versus fixed provenance.
- Assert effective alpha and inherited paint.
- Assert source layer order.
- Assert transforms and shared variant geometry apply identically to fallback and layers.
- Assert fallback winding behavior is unchanged.
- Assert one painted layer cannot become a knockout based on another layer.
- Keep deferred SVG cases outside the required fidelity assertions; no unsupported-feature rejection tests are required.

### SFNT Tests

- Parse TTF with `read-fonts`.
- Assert `COLR` version 1 and a valid CPAL table.
- Assert every expected default/alternate selectable glyph has the correct base paint record.
- Assert blank and unselected glyphs have no record.
- Assert layer GIDs, order, outline coordinates, palette indices, and F2DOT14 alpha.
- Assert foreground uses `0xFFFF`.
- Assert fixed colors are deduplicated in first-use order.
- Assert same outline/different paint does not alias selectable GIDs.
- Assert all-foreground fonts still contain a minimally valid CPAL table.
- Assert `maxp`, `hmtx`, `loca`, `glyf`, and `post` agree on expanded glyph count.
- Assert overflow is rejected before glyph ID narrowing.
- Assert 255/256-layer nesting and multi-chunk order without alpha changes; palette and LayerList bounds fail before narrowing.

### Layout Tests

- Assert `cmap` targets the default selectable glyph.
- Assert `rvrn` targets alternate selectable glyphs, never layer glyphs.
- Assert conditioned `liga` targets the same variant glyph as direct access.
- Assert fallback and blank substitutions preserve the multi-weight contract.
- Assert `fvar`, `STAT`, GSUB, COLR, and CPAL coexist in one parseable font.

### Container Tests

- Compare WOFF1-decoded COLR/CPAL bytes with TTF.
- Decode WOFF2 through the existing reference path and compare semantic tables/outlines.
- Assert variable tables and FeatureVariations survive alongside color tables.
- Assert variant format filtering retains the existing modern-only path for color-enabled and disabled requests.

### Browser Tests

Use the existing Vitest browser project in Chromium, Firefox, and WebKit. The color-specific assertions below apply to Chromium and Firefox. For tested WebKit without COLR v1, assert monochrome fallback pixels, host-color changes, correct weight outlines, direct/ligature parity, and equal advances. Do not require fixed paint or independent layer alpha in fallback rendering.

- Set canvas text color to two values and prove foreground pixels change.
- Prove fixed-color pixels remain unchanged.
- Measure the translucent foreground layer against an opaque control with tolerant channel/alpha bounds.
- Render every named weight by direct codepoint and ligature.
- Render at least one in-between coordinate on each side of an `rvrn` boundary.
- Confirm each weight has the expected distinct outline and paint.
- Confirm advances remain equal.
- Use automated canvas readback; do not rely on screenshots or visual inspection.
- Record browser versions and gate results in this plan's decision log when phases execute.

### NAPI, Node, And Manifest Tests

- Type acceptance for `true`, named arrays, and empty arrays.
- Type rejection for `false`, arbitrary strings, and mixed arrays.
- Runtime validation agrees with TypeScript declarations.
- Selection occurs after rename.
- Unknown names aggregate deterministically.
- Both single and variant discriminated inputs accept the shared option.
- Ordinary default EOT selection causes the documented incompatible-format error; variant default WOFF/WOFF2 selection succeeds.
- Explicit modern-only types succeed.
- JSON manifest behavior matches Node and Rust behavior.
- Generated `binding.d.ts` and handwritten `index.d.ts` expose the intended surface.

### Regression And Performance Tests

- Keep a recorded current-code ordinary-output hash baseline green when color is omitted.
- Add a variant-output semantic baseline showing color omission does not add COLR/CPAL or alter existing tables.
- Benchmark disabled ordinary generation against the recorded current-code baseline at 1, 100, 300, and 600 glyphs.
- Retain this plan's proposed 5% ordinary regression ceiling, with baseline and measurement protocol pending Phase 0 review; investigate any statistically significant regression even below the ceiling. This is a release requirement, not a measured result.
- Measure enabled generation separately for all-color and sparse-selection corpora, but do not set an arbitrary optimization gate before data exists.
- Track output-size growth by number of selected glyphs and layers.
- Do not add auxiliary-layer deduplication or fine-grained table caching without measured need.

### Incremental Lifecycle Tests (Both Modes Required)

Extend existing engine `src/incremental/` tests, `tests/variant_incremental.rs`, result snapshot tests, and adapter result/Node tests. Cover named selection after rename, paint-only changes, identical-content reuse across selected/unselected names, sparse variant membership edits, fallback propagation, blank transitions, and shared-source consumers. For each successful batch compare a fresh final-input build with hinted and full re-diff regeneration at a fixed timestamp, including bytes, metadata, hashes, and rendered URLs. For invalid selection and existing parse/build failures assert the previous generation remains usable and corrected retries match fresh output. Separately assert variant write failure retains committed new output and retries pending writes without requiring another edit. Include no-ops and concurrent/stale adapter calls so paint caching does not bypass existing lifecycle checks.

## Delivery Plan

The implementation uses three stacked PRs. PR 1 (Phases 0–2) is authorized after feasibility review and the approved scope deferrals. Implement and verify its internal pipeline as one work unit; separate phase-by-phase approval was not requested. For each later layer, define focused tests, implement the minimum green change, run the gate, and update the decision log. All gates below are requirements pending execution, not claims of success.

### Phase 0: Current-Code Reconciliation And Combined Proof

Purpose: replace planning assumptions with the current split engine/adapter implementation and test whether COLR works after `rvrn`/`liga` selection. Reconciliation and research results are recorded above. WebKit fallback and the basic best-effort SVG boundary are accepted; deferred SVG cases no longer block this phase.

Work:

- Re-read current source-mode dispatch, resolved variant states, shared geometry types, ordinary/variant allocation, GSUB builder, output filtering, incremental lifecycle, NAPI adapter, manifest parser, and tests.
- Update this document's architecture map if paths or type names differ.
- Record ordinary root-marker extraction, basic per-layer fill-rule behavior, and >255-layer/palette-bound evidence. Record deferred SVG cases without requiring source preflight or exact complex-geometry conversion. Successful serialization alone is not rendering proof.
- Record ordinary all-format bytes/hashes and color-disabled variant semantic baselines.
- Extend or generate a test-only proof font containing `fvar`, `STAT`, FeatureVariations `rvrn`, conditioned `liga`, COLR v1, and CPAL.
- Use two unrelated variant outlines, different fixed paint, and one translucent foreground layer.
- Verify TTF parsing, WOFF1 preservation, WOFF2 decoding, and three-browser rendering.

Hard gate: ordinary extraction/geometry and font-limit questions have reviewed answers under the approved basic scope; direct codepoint and ligature paths select the correct painted variant in Chromium and Firefox and the correct monochrome fallback variant in tested WebKit. Record exact commands, browser versions/platforms, fixture paths, pass/fail, and any blocked coverage. Deferred SVG fidelity and documented WebKit color limitations are not blockers.

### Phase 1: Internal Selection And Paint Semantics

Purpose: lock SVG-to-paint behavior without changing public output.

Tests first:

- All and named logical selection in ordinary and resolved variant fixtures.
- Default fill versus root/group/path/style authored fill.
- Explicit `currentColor`, including authored `color`, and effective fill opacity.
- Ordinary stylesheet fills through existing usvg resolution, without additional unsupported-feature classification.

Implementation:

- Add a private resolved selection type used by tests and later public resolution.
- Add opt-in paint extraction around the existing parser.
- Implement the ordinary root-marker extraction strategy against the revised authored-color contract. Do not use the nonexistent Options::color field or add source/CSS preflight for deferred cases.
- Retain path-indexed ordered paint metadata without duplicating source geometry.

Gate: paint semantics pass; monochrome parse behavior and ordinary baseline bytes remain unchanged.

### Phase 2: Shared Geometry And Processed Layers

Purpose: carry selected paint through ordinary and variant family geometry.

Tests first:

- Shared transforms, advances, centering, normalization, rounding, and optimization.
- Preserved nonzero winding, ordinary nested evenodd holes, independent layers, and unchanged merged fallback winding.
- Blank and fallback resolved states.
- Same source used by multiple fallback states.

Implementation:

- Extend the current processing seam with optional per-layer geometry.
- Transform source paths once and derive fallback/layers from the same transformed values.
- Store processed color layers only for selected glyphs.
- Extend ordinary and variant parse/process cache signatures and equality, including selection transitions and fallback consumers.

Gate: processed presentations are correct for every ordinary/variant state; existing color-disabled geometry remains semantically unchanged.

PR 1 ends after Phase 2. Its production behavior remains unchanged because no public option is wired yet, but its internal seams are fully exercised and immediately consumed by the next stack PR.

### Phase 3: Paint-Aware Selectable Glyph Identity

Purpose: make deduplication safe before adding COLR records.

Tests first:

- Same fallback outline with different color, opacity, layer order, or selection state receives distinct selectable IDs.
- Identical complete presentations may reuse IDs where existing substitution semantics allow.
- Blank/fallback targets retain correct IDs.
- Existing color-disabled glyph ordering remains unchanged.

Implementation:

- Add presentation identity to ordinary dedup keys/equality and the variant `add_presentation` exact equality scan. Do not assume the variant path already has hash buckets.
- Keep paint hashes non-cryptographic and in-process only, with exact equality after bucketing.
- Preserve existing deterministic selectable ordering in both builders.

Gate: `cmap`, `rvrn`, and conditioned `liga` target presentation-safe selectable IDs.

### Phase 4: Auxiliary Layer Glyph Store

Purpose: allocate every outline referenced by COLR without disturbing layout IDs.

Tests first:

- Deterministic appended layer order.
- No layer appears in cmap/GSUB.
- Expanded table counts and zero layer advances are valid.
- Glyph-limit overflow reports deterministic context.

Implementation:

- Extend the existing ordinary/variant allocation and shared assembly seams to represent auxiliary layers.
- Append layer glyphs after existing selectable/placeholder IDs.
- Thread assigned IDs into table assembly; remove any positional arithmetic invalidated by the expanded store.
- Add deterministic internal names only if required by the final `post` representation.

Gate: a fallback-only TTF with appended layer glyphs parses and all non-color layout tables retain correct targets.

### Phase 5: COLR v1 And CPAL Tables

Purpose: emit the complete color presentation.

Tests first:

- Foreground `0xFFFF`, fixed palette colors, alpha quantization, layer order, empty/unselected behavior, and all-foreground CPAL validity.
- Default and alternate variant base records.
- Table sorting and deterministic bytes for fixed timestamp/options.
- Nested roots at 255/256 layers and CPAL/LayerList limits, including overflow errors.

Implementation:

- Build one deterministic CPAL palette.
- Build COLR v1 PaintColrLayers/PaintGlyph/PaintSolid graphs.
- Add tables to existing SFNT assembly and cache/table identity.
- Keep fallback outlines on selectable glyphs.

Gate: `read-fonts` validates complete ordinary and variable SFNTs and semantic assertions pass.

### Phase 6: Combined Layout And Browser Correctness

Purpose: prove real layout engines paint the final selected GID.

Tests first:

- Every named weight through direct codepoint and ligature.
- Approved midpoint boundaries and endpoint clamping.
- Variant-specific fixed colors and foreground alpha.
- Equal measured advances.

Implementation:

- Fix integration defects between the current GSUB builder, glyph allocation, and COLR records.
- Do not add a color-specific GSUB path.

Hard gate: generated WOFF2 passes Chromium/Firefox color canvas assertions and tested WebKit monochrome-fallback canvas assertions for direct and ligature rendering at all tested coordinates. Both paths preserve advances and weight selection.

### Phase 7: WOFF Containers And Format Filtering

Purpose: preserve color and variation tables through every supported container and reject legacy outputs early.

Tests first:

- TTF/WOFF1/WOFF2 table preservation.
- Combined variation/color/GSUB table survival.
- Ordinary and variant EOT/SVG incompatibility.
- Ordinary default-format EOT incompatibility and compatible variant defaults.
- Existing modern-only variant output filtering regardless of color.

Implementation:

- Reuse existing generic WOFF1 and WOFF2 paths.
- Change serializers only if semantic round-trip tests identify an actual defect.
- Add early format validation at the shared resolved-options boundary.

Gate: all supported containers parse and render; incompatible requests fail before font construction.

PR 2 ends after Phase 7 with complete internal color-font generation and format behavior.

### Phase 8: Public Rust, NAPI, Node, And Manifest API

Purpose: expose the completed feature consistently.

Tests first:

- Rust all/named/empty behavior.
- NAPI conversion and generated declaration shape.
- Node single/variant discriminated options.
- Rename-before-selection and unknown-name errors.
- Manifest all/named/empty behavior.

Implementation:

- Add `ColorGlyphSelection` and `GenerateWebfontsOptions.color_glyphs`.
- Resolve selection after ordinary rename or variant logical-union construction.
- Add the handwritten Node union and minimal wrapper adaptation.
- Add manifest deserialization using the same semantic type.
- Regenerate NAPI artifacts only through `vp run @atlowchemi/webfont-generator#build`.

Gate: Rust, raw NAPI, package wrapper, TypeScript, and manifest tests agree.

### Phase 9: Ordinary And Variant Regeneration, Results, Hashes, And CSS

Purpose: finish full color lifecycle behavior through existing ordinary and variant regeneration APIs.

Tests first:

- Ordinary and variant rename/membership changes re-resolve named selection, including deterministic unknown-name errors.
- Paint-only source edits invalidate font bytes and hashes.
- Variant fallback consumers receive source paint changes; blank/fallback transitions remove or restore the correct COLR records.
- Fresh/regenerated parity for both modes, via hinted changes and full re-diff, including additions/removals/reordering, identical-content renames, shared sources, and no-ops.
- Ordinary and variant pre-commit rollback on selection, parse/build/limit failures, with unchanged prior bytes/names/hashes and corrected retries.
- Variant post-commit write failures retain new output and pending-write retry semantics; test a no-op retry and adapter ownership/busy/stale-result behavior.
- Modern-only variant CSS/results retain ordered metadata and compatible defaults.

Implementation:

- Complete ordinary and variant cache invalidation, fallback propagation, and output hash integration.
- Reuse current result/output filtering; do not add color-specific result getters.
- Extend existing variant regeneration and adapter conversions rather than introducing separate color result APIs.

Gate: lifecycle parity passes and existing result/CSS behavior remains unchanged when color is disabled.

### Phase 10: Documentation, Performance, And Release Gate

Purpose: make the stack reviewable and releasable.

Documentation:

- Rust API comments and examples.
- `crates/webfont-generator/README.md` (engine/CLI).
- `packages/webfont-generator/README.md`.
- `packages/docs/webfont-generator/node.md`, `rust.md`, CLI manifest documentation, and relevant reference pages.
- Supported/best-effort SVG paint matrix, accepted visual limitations, and guidance to report reproductions or contribute improvements.
- COLR v1/browser compatibility note.
- Explicit ordinary-versus-variant format defaults and ordinary default EOT incompatibility.
- Variant examples showing paint changes across weights.
- Ordinary and variant incremental examples with selection, fallback propagation, and the existing distinct failure/retry contracts.
- Engine and npm changelog entries in their own source files as appropriate, without duplicating them into docs include pages.

Verification:

```text
vp fmt
vp run @atlowchemi/webfont-generator#check
vp run @atlowchemi/webfont-generator#build
vp run @atlowchemi/webfont-generator#test
vp run @atlowchemi/webfont-generator#test:browser
vp run @atlowchemi/webfont-engine#test:doctests
vp run @atlowchemi/vite-svg-webfont-docs#build
vp check
vp run test
vp run coverage
```

The adapter `#check` depends on engine checks (default and CLI builds); adapter `#test` depends on engine default/CLI tests and checks. Browser `#test:browser` depends on the NAPI build. Workspace `vp run test` builds NAPI before JS tests. `vp run coverage` measures JS only; when Rust coverage is required, use engine `#test:coverage`, `#test:coverage:cli`, adapter `#test:coverage`, and adapter `#test:coverage:native` per CONTRIBUTING.md; retain doctests separately from nextest. Engine fixtures must be engine-owned; shipped-template parity tests belong to the adapter.

Run relevant benchmarks against the recorded current-code baseline using `vp run @atlowchemi/webfont-generator#bench` (delegates to the engine), with `--no-run` for benchmark-only compilation changes. Report generated `binding.js`, `binding.d.ts`, platform `.node` artifacts, and any proof-font fixture changes. Do not manually edit generated bindings. These are future implementation gates; a plan-only reconciliation does not require builds or artifact regeneration.

Release gate: all checks, coverage, three-browser tests, docs, ordinary byte baselines, color-disabled variant semantic baselines, and performance thresholds pass at the top of the stack.

PR 3 ends after Phase 10 and exposes the public feature.

## Stacked Pull Request Strategy

### PR 1: Paint-Aware SVG And Geometry Pipeline

Contains Phases 0-2.

- Base: current engine/adapter implementation after Phase 0 evidence review and implementation approval.
- Public feature remains unavailable.
- Reviews SVG paint semantics, optional data shape, shared geometry, and cache boundaries.
- Must preserve all color-disabled outputs.

### PR 2: COLR v1 Variable SFNT Integration

Contains Phases 3-7.

- Base: PR 1.
- Reviews presentation-safe glyph identity, shared glyph allocation, COLR/CPAL assembly, layout interaction, browsers, and containers.
- Uses internal selection until the public API lands.
- Must pass combined `rvrn`/`liga`/COLR browser gates.

### PR 3: Public Surface And Release Readiness

Contains Phases 8-10.

- Base: PR 2.
- Reviews public Rust/NAPI/Node/manifest behavior, validation, ordinary AND variant regeneration, results, docs, and release gates.
- Regenerates checked-in binding artifacts.

Open completed layers incrementally as draft PRs. Keep them draft until PR 3's head passes the full release gate. Each PR must also pass its narrower phase gates. Merge bottom-up, rebase dependent branches after each merge, and rerun the full gate after the final rebase.

Do not create separate PRs for WOFF transport, documentation, generated bindings, or browser fixtures; those changes are too coupled or too small to justify additional stack maintenance.

## Architecture Map

Reconciled on 2026-10-04. Paths below identify current implementation evidence, not proof of color feasibility. Recheck them before implementation if concurrent work changes these seams.

### Engine (`crates/webfont-generator`)

- Rust crate `webfont-generator`; Vite+ workspace package `@atlowchemi/webfont-engine`. Public options/types are in `src/types.rs`; resolution/defaults and variant EOT/SVG rejection are in `src/input/options/mod.rs` (`resolved_font_types`, `validate_variants`).
- `src/input/files/mod.rs` owns source loading, logical union, and missing-source resolution. `src/lib.rs::prepare_variant_family_cached` rebuilds the union, resolves missing states, assigns codepoints, and dispatches cached geometry.
- `src/svg/parse.rs` parses XML with DTD support and `usvg`, applies absolute transforms/root viewBox correction, and currently collects geometry without paint or fill-rule provenance. `src/svg/types.rs` owns parsed/processed/cache carriers. `src/svg/mod.rs` owns shared family metrics and `VariantGlyphCache`; `src/svg/incremental.rs` owns ordinary parsed/processed reuse.
- `src/sfnt/builder/glyphs.rs`, `cache.rs`, `outlines.rs`, `types.rs`, and `tables.rs` own ordinary dedup/compilation/assembly. `src/sfnt/builder/variants/mod.rs` owns physical presentations, `presentation_gids`, exact outline/advance dedup, `fvar`/`STAT`, `rvrn`, conditioned `liga`, and quantized variant ranges. Layer support must extend actual allocation rather than assume a unified store already exists.
- `src/pipeline/mod.rs::build_variant_font_outputs` serializes modern variable resources only; ordinary output assembly also handles legacy formats. `src/sfnt/serialize.rs` and `src/formats/` implement SFNT and containers.
- `src/result/`, `src/output/`, and `src/rendering/` own result state, writes, rendering, and hashes. `src/incremental/mod.rs` owns regeneration dispatch/state leases; `src/incremental/variants.rs` implements existing family regeneration and post-commit pending-write retry. Tests live under `src/incremental/tests/` and `src/incremental/variants/tests.rs` as well as result tests.
- `src/main.rs` and `src/manifest.rs` implement the optional CLI and JSON `--config` path, deserializing the engine options with field-path errors and manifest-relative file expansion.
- Rust tests/fixtures and benchmarks live in this crate (`src/**/tests*`, `tests/`, `benches/`). The existing test-only SFNT proof helper is `src/sfnt/builder/tests/proof_font.rs`. Engine tests must not consume downstream adapter assets.

### Adapter (`packages/webfont-generator`)

- Rust crate `webfont-generator-napi` has `[lib] path = "native/lib.rs"` in its Cargo.toml and depends on the engine. `native/types.rs`, `native/conversions.rs`, `native/result.rs`, and `native/result/tests.rs` own NAPI DTOs, conversion, result/regeneration lifecycle, and adapter tests. The engine remains NAPI-free; extend the existing adapter options object rather than creating another full copy solely for color.
- `index.js`, `index.d.ts`, and `validations.js` implement the public Node wrapper, discriminated single/variant types, rename hook, and validation. `binding.js`/`binding.d.ts` and platform `.node` files are generated by the supported NAPI build.
- `tests/` contains Node/runtime/type tests; `tests/browser/` and `vite.browser.config.ts` define browser coverage. `templates/` remains the shipped template location, with parity checked by adapter tests.
- `vite.config.ts` in each workspace package defines the actual Vite+ tasks and cross-package dependencies listed in Phase 10. Adapter benchmark tasks invoke the engine benchmarks.
- Public docs: `crates/webfont-generator/README.md`, `packages/webfont-generator/README.md`, Rust comments, and `packages/docs/webfont-generator/`. Engine/npm changelogs stay in their own source files and are included by docs.

### Existing Variant Contract To Preserve

Variants are ordered, at least two, with exactly one default and strictly increasing resolved weights in 1..=1000. The default weight resolves to 400 when omitted; missing glyphs default to blank, with explicit fallback/error modes. Union/codepoint order and final per-variant file ordering must match fresh builds. Each logical glyph has one advance shared across states; family `fixedWidth` can extend that across glyphs. Defaults are WOFF/WOFF2, EOT/SVG are rejected, and the requested modern files share one variable font. Layout uses discrete `wght` substitutions (no outline interpolation), default cmap presentations, and conditioned ligatures. Incremental opt-in is supported now; preserve both pre-commit rollback and variant post-commit write retry as detailed above.

## Risks And Mitigations

### COLR After FeatureVariations

Risk: layout selects the right outline but COLR is attached only to the default GID.

Mitigation: attach records to every selectable target and test direct/ligature access at every weight in three engines.

### Unsafe Glyph Deduplication

Risk: equal fallback outlines with different paint alias to one GID and therefore one COLR presentation.

Mitigation: include complete presentation identity in selectable dedup keys and exact equality.

### Default Black Versus Authored Black

Risk: computed SVG black loses provenance, causing fixed black where foreground was intended or vice versa.

Mitigation: test ordinary authored `color` with `currentColor`, initial fill, inherited fills, and marker collisions. Document advanced selectors/resets as best-effort rather than blocking initial extraction on full CSS fidelity.

### Incorrect Alpha Composition

Risk: flattening group opacity into each child changes overlap results.

Mitigation: promise only fill-opacity preservation; document group/object opacity as deferred, without adding rejection machinery.

### Layer Winding Corruption

Risk: the monochrome containment heuristic turns overlapping painted paths into holes.

Mitigation: retain existing fallback behavior, preserve nonzero winding, and handle ordinary nested evenodd holes per layer. Document complex evenodd intersections as best-effort; defer detection and exact conversion.

### Glyph Count Growth

Risk: one auxiliary glyph per path approaches the SFNT glyph limit or inflates files.

Mitigation: validate limits before narrowing, measure output growth, and defer auxiliary dedup until measurements justify it.

### Container Interaction

Risk: WOFF2 transformed `glyf` handling succeeds for variation or color independently but fails with both.

Mitigation: round-trip one combined font containing expanded outlines, FeatureVariations, `fvar`, `STAT`, COLR, and CPAL.

### Legacy Output Ambiguity

Risk: callers assume EOT/SVG silently degrade to monochrome.

Mitigation: reject active selection against resolved formats, including defaults, before expensive work and document explicit modern `types`.

### Variant Lifecycle Regression

Risk: caches reuse monochrome or stale painted data after name-only edits, shared-source changes, or fallback transitions; failed rebuilds corrupt a usable generation.

Mitigation: include presentation/selection in cache identity, re-resolve the logical union and fallback consumers, require fresh/incremental parity, and test pre-commit rollback separately from the existing post-commit write-retry behavior. Full variant regeneration is mandatory shipping scope.

### Parser Loss And Table Bounds

Risk: successful parsing hides unsupported content that was dropped; layer or palette counts silently narrow and change output.

Mitigation: accept parser loss for deferred SVG content and document that limitation; no source preflight is required. Use checked nested-layer/palette construction with semantic boundary tests for generated font tables.

### Disabled-Path Regression

Risk: optional paint fields or extra parser passes slow every existing build.

Mitigation: branch once at source-mode/selection boundaries, allocate layer metadata only for selected glyphs, retain byte baselines, and enforce the existing performance gate.

## Definition Of Done

The feature is complete when:

1. `colorGlyphs: true` and named selection work in ordinary and variant modes.
2. Selection uses final logical names after rename.
3. Initial SVG fill maps to foreground while authored/inherited fills remain fixed.
4. Fill opacity is preserved for foreground and fixed paints.
5. Deferred SVG cases are documented as best-effort, with no required detection or diagnostics; unselected behavior is preserved.
6. Every selectable default/alternate variant GID has the correct COLR presentation.
7. Direct codepoints and conditioned ligatures render the same painted variant at every tested weight.
8. Fallback and blank missing states preserve their agreed geometry, paint, and advance behavior.
9. TTF, WOFF, and WOFF2 parse and render with fallback `glyf`, COLR v1, CPAL, `fvar`, `STAT`, and FeatureVariations intact.
10. Active color selection rejects EOT and SVG, including ordinary default EOT selection; compatible modern-only variant defaults succeed.
11. Ordinary incremental regeneration has fresh-build parity and rollback behavior.
12. Variant incremental regeneration has fallback propagation, post-rename selection, fresh-build parity, pre-commit rollback, and existing post-commit write retry through Rust/NAPI/Node APIs.
13. Color-disabled ordinary output bytes and variant semantics remain unchanged.
14. Chromium and Firefox pass automated color/alpha pixel and advance assertions; tested WebKit passes monochrome fallback, host color, weight selection, direct/ligature parity, and advance assertions. Public docs explain the capability difference.
15. Rust, NAPI, Node, TypeScript, and CLI manifest behavior agree.
16. Public docs, both engine/adapter READMEs, Rust comments, and source changelogs are synchronized.
17. Generated bindings are refreshed through the supported build task.
18. Formatting, checks, tests, browser tests, coverage, docs build, and performance gates pass at the final stack head.
19. Ordinary provenance includes authored `color` with `currentColor`; basic nonzero/nested-evenodd geometry and >255-layer/palette bounds are covered. Advanced CSS, complex evenodd conversion, and dropped-content detection are explicitly deferred under the approved scope.

## Executor Checklist

At the start of each phase:

- Read this self-contained plan and the current implementation/tests identified in the architecture map.
- Inspect the current implementation rather than assuming provisional type names survived.
- Check the worktree and preserve unrelated user/agent changes.
- Follow the implementation workflow agreed with the user; PR 1 implementation is authorized without separate phase-by-phase test approvals.

At the end of each phase:

- Run focused tests and the package check appropriate to touched code.
- Confirm color-disabled ordinary baselines remain green.
- Confirm color-disabled variant generation AND incremental lifecycle semantics remain green.
- Report generated files and benchmark/browser versions when relevant.
- Record approved deviations and gate results in this document's decision log.
- Complete the authorized PR layer, then report its gate results before starting the next layer.
