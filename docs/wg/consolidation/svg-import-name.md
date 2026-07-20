---
title: SVG Import IR Name and Math Study
description: "Evidence for NAME: the strict crate boundary, affine vocabulary, two-surface reading, and preservation gates for the Phase 3 SVG import cut."
tags:
  - internal
  - wg
  - program
  - consolidation
  - svg
format: md
---

# SVG Import IR Name and Math Study

**Date:** 2026-07-21

**Status:** Evidence complete; registry decision **NAME remains open**
pending the owner's explicit GO. Phase 3 remains behind its Phase 1 and
Phase 2 entry gates.

**Genre:** program decision study. This document performs the naming
exercise required by the [charter](./charter.md), inventories the seam that
the name must protect, and supplies proposed decision wording. It does not
take NAME, cut a crate, grant SVG import to the chassis, classify a
scoreboard row, or supersede the SVG domain documents. The
[SVG import mapping](../format/svg.md) and sibling SVG RFDs win on conflict;
a mismatch with them is recorded here as a finding.

## Decision in one sentence

Phase 3 needs a name and a math vocabulary for the model-agnostic product
consumed by either document model, while keeping its source producer, the
browser-cascade SVG renderer, and the retained-source animation frontend from
becoming one ambiguous contract.

The evidence supports the following owner decision:

> Name the Cargo package **`svg-import-ir`**. Give it the existing
> `math2`-backed f32 affine vocabulary, retaining `cg::CGTransform2D` and
> `cg::CGRect` at the public IR boundary during the zero-behavior cut. Do not
> expose or depend on `kurbo` or `n0-model` math. Confirm that
> import-to-document and render-to-pixels are distinct SVG surfaces. Make the
> package the canonical shared IR consumption boundary; preserve the existing
> `cg::svg` definitions and legacy raw-string façade during the first
> compatibility cut rather than creating a dependency cycle. Keep the usvg
> producer behind a separate, loss-aware surface that does not leak its tree
> type into model consumers.

That wording is a proposal, not a registry update. If accepted, the Rust
crate identifier is `svg_import_ir`, and directory and Cargo package names
remain aligned at `crates/svg-import-ir` / `name = "svg-import-ir"`.

## Bedrock

The naming exercise begins from constraints already settled by the program.

1. **The product is an import IR.** The existing sink inversion names
   `SVGPackedScene` and the `IRSVG*` tree as the reusable product. The legacy
   scene graph is one consumer, not part of the product.
2. **The second consumer arms extraction.** The crate cut is justified by the
   legacy packer and a new chassis packer compiling against the same IR. It is
   not justified by moving the first consumer's module early.
3. **The IR is static and normalized.** `usvg` has already resolved CSS,
   references, basic shapes, and other authored distinctions. This is not an
   editable or round-trippable source representation.
4. **Adapters own model policy.** A construct that one model cannot express
   is an honest `UNSUPPORTED` result for that capability entry. The shared IR
   does not grow a lowest-common-denominator shim, and the packer does not
   conceal an upstream loss it could never observe.
5. **Hosts supply resources; the frontend owns semantics.** Hosts provide
   exact bytes, resource availability, and environment identity. The SVG/CSS
   frontend owns declared resolution and fallback semantics. Invalid-XML
   repair is separate host policy. Ambient system state is not a pure-core
   input, and a host may not silently invent a different cascade or fallback.
6. **A cut preserves legacy behavior exactly.** Moving or replacing the
   current producer is a zero-behavior change and therefore owes
   byte-identical output over the declared corpora. The legacy engine is not
   an oracle for the later chassis capability grant.
7. **Dependency direction is one-way.** Each model adapter may depend on the
   IR and its model. `n0-model` depends on neither the IR nor the legacy
   engine, and the IR depends on neither model.
8. **Animation retains source identity separately.** The chassis SVG
   animation profiles deliberately do not invent a general static importer;
   their source snapshot and target identity cannot be recovered from the
   normalized static tree.
9. **The cg cut made compatibility a contract.** Existing `cg::svg` and
   `grida::cg::svg` paths, serde spellings, and the legacy
   `SVGPackedScene` constructors cannot disappear merely because a stricter
   package name is now available.

## Patrol: the current seam

The current pipeline is:

```text
raw SVG
  -> legacy source repair
  -> legacy-configured usvg parse
  -> usvg::Tree
  -> IRSVG tree / SVGPackedScene
  -> v1 scene-graph adapter
  -> v1 Paint projection
  -> optional frozen .grida encoding
```

The useful boundary is narrower than `crates/grida/src/import/svg/`.

| Current part | Observed responsibility | NAME disposition |
|---|---|---|
| [`formats/svg/sanitize.rs`](../../../crates/grida/src/formats/svg/sanitize.rs) | Repairs bare ampersands in otherwise invalid XML | Outside. Source-repair policy is not IR semantics. |
| [`formats/svg/parse.rs`](../../../crates/grida/src/formats/svg/parse.rs) | Builds `usvg::Tree` with embedded Geist, optional system fonts, remapped generics, and default usvg resource behavior | Outside as written. A future raw-source producer must take an explicit closed environment. |
| [`import/svg/from_usvg.rs`](../../../crates/grida/src/import/svg/from_usvg.rs) | Maps usvg values into `cg` and SVG-domain values | Producer-layer content, separate from the model-consumer IR surface; every loss must become attached diagnostics. |
| [`import/svg/packed_scene.rs`](../../../crates/grida/src/import/svg/packed_scene.rs) | Lowers `usvg::Tree` to ordered `IRSVG*` nodes | Producer-layer content, separate from the model-consumer IR surface; its Skia path dependency must be removed without changing bytes. |
| [`cg/src/svg.rs`](../../../crates/cg/src/svg.rs) | Defines the serializable SVG paint and `IRSVG*` vocabulary | Remains physically in `cg` for the first compatibility cut and is re-exported by the named IR package. A later physical move needs its own compatibility decision. |
| [`import/svg/pack.rs`](../../../crates/grida/src/import/svg/pack.rs) | Projects the IR into the legacy scene graph | Stays a legacy consumer. |
| [`import/svg/paint.rs`](../../../crates/grida/src/import/svg/paint.rs) | Bakes SVG attributes into legacy runtime paints | Stays a legacy consumer policy. |
| [`import/svg/grida.rs`](../../../crates/grida/src/import/svg/grida.rs) | Repairs source, invokes the legacy packer, assigns deterministic IDs, and encodes frozen FlatBuffers | Stays a v1 compatibility sink. |
| future n0 packer | Projects supported IR values into `n0-model::Document` | New consumer; depends on IR + `n0-model`. |
| [`htmlcss::svg`](../../../crates/grida/src/htmlcss/svg/README.md) | Resolves browser-like SVG/CSS and renders to pixels | Separate render-to-pixels surface; never a consumer requirement for this crate. |
| [`n0-model::svg_animation`](../../../crates/n0-model/src/svg_animation.rs) | Retains animation-bearing source and target identity for explicit-time profiles | Separate animation frontend; never folded into the static import IR. |

The existing
[`svg_import_architecture.rs`](../../../crates/grida/tests/svg_import_architecture.rs)
already proves three useful facts: the IR layer does not name the v1 node
model, runtime-paint projection is outside the vocabulary, and only the two
legacy sink files may touch the node model. It does **not** yet prove that the
producer is graphics-backend-free or host-policy-free.

### Backend and host leaks found by patrol

The present IR values are Skia-free, but the producer is not yet an agnostic
crate.

- Path lowering converts a tiny-skia path to `skia_safe::Path`, offsets it by
  the usvg bounds, and calls Skia's SVG serializer. The resulting path string
  and offset are observable legacy behavior. Removing Skia is therefore a
  byte-gated replacement, not a mechanical import edit.
- Relative transforms are reconstructed with `math2`: invert the parent's
  absolute matrix, compose it with the child's absolute matrix, and fall back
  to the child matrix when the parent is singular. Path-bound offsets are then
  added directly to translation components.
- Raw parsing loads embedded Geist, loads system fonts outside Emscripten,
  remaps every generic family to Geist, and inherits usvg's default resource
  behavior. Those choices can change the normalized tree across hosts.
- Text-family lowering falls back to the embedded Geist family name. That is
  current legacy policy, not a universal IR default.

An honest cut keeps the IR contract distinct from its usvg implementation.
The model consumers must not expose or inherit a particular `usvg::Tree`
version. The producer may be a separately named adapter package or an
equivalent non-default/private layer, but its public source boundary must use
an explicit closed environment and return a certified outcome. It cannot move
the current raw-string constructor unchanged and call the result pure.

### Compatibility topology for the first cut

Moving `cg::svg` into a package that itself depends on general `cg` leaves
would make preservation of `cg::svg` require a `cg → svg-import-ir → cg`
cycle. Dropping the old path instead would violate the compatibility promise
recorded by the cg cut. The first cut must therefore use this acyclic shape:

```text
svg-import-ir -> cg -> math2
grida -> svg-import-ir + cg
narrow SVG producer -> svg-import-ir + explicit frontend dependencies
n0 SVG adapter -> svg-import-ir + n0-model
```

`svg-import-ir` becomes the canonical package model consumers compile
against. It re-exports the existing SVG values from `cg`, introduces an
honestly named canonical `ImportTree`, and owns its invariants, validation,
certification, and typed outcome vocabulary. The legacy crate keeps a real
`SVGPackedScene` wrapper, not a type alias, so its public `svg` field and
`new`, `new_from_tree`, and `new_from_svg_str` methods retain their current
behavior. Direct `cg::svg` and `grida::cg::svg` paths therefore keep the same
defining types.

Physical ownership of the SVG values can move later only with a separate
compatibility decision or a lower common vocabulary that does not create a
cycle. The program must not copy the definitions to make the graph look
clean.

### Honest outcomes are layered

The normalized tree alone cannot report semantics that usvg has already
erased. `UNSUPPORTED` is therefore an entry-point property, not a promise the
n0 packer can fulfill in isolation:

```text
raw-source preflight
  -> explicit-environment parse
  -> loss-aware producer outcome
  -> certified ImportTree
  -> model packer outcome
```

The source preflight is conservative and loss-complete across CSS, `<use>`,
and resource-derived features; a tag scan is insufficient. Parsing uses a
closed environment. The producer outcome keeps typed diagnostics attached to
the tree for every drop or collapse it can observe. Validation certifies the
IR invariants. The model packer reports values its model cannot express and
commits a document only after the whole chain succeeds. An already-built
`usvg::Tree` without the same certification is not chassis-eligible.

The outcome classes are distinct: invalid source or environment, producer
normalization loss, invalid import-tree invariant, and consumer projection
unsupported. `Result<_, String>` and a partial document are not sufficient.
One unsupported field rejects the whole import atomically. The legacy
raw-string façade may deliberately consume the lossy outcome and discard
additive diagnostics to preserve current bytes; that exception is
legacy-only.

The exact Rust result types are not part of NAME. The semantic requirement is
that no layer turns an unobserved or unrepresentable construct into silent
success.

## Naming exercise

The name is evaluated by what future contents it refuses.

| Candidate | What it tells the reader | Refusal test | Verdict |
|---|---|---|---|
| `svg-import-ir` | A normalized intermediate representation for the import-to-document surface | Refuses source repair/acquisition, both model packers, FlatBuffers, painting, browser rendering, animation, and editor-source fidelity | **Recommend.** It repeats the charter's exact subject and is the narrowest honest package claim. |
| `svg-ir` | Some intermediate representation related to SVG | Refuses model packers and paint, but appears to claim the ecosystem's sole SVG IR | Reject. The browser renderer, animation frontend, and editor-source work have different legitimate representations. |
| `svg-import` | The SVG import-to-document surface | Distinguishes import from rendering, but can absorb repair, resource acquisition, and both model projections | Reject. It names the whole process rather than the shared product. |
| `svg` | The SVG domain | Refuses almost nothing | Reject. It invites import, browser rendering, animation, authoring, and export into one crate. |

The recommended name commits to these contents:

- the canonical public surface for the normalized static SVG `ImportTree` and
  its SVG-domain leaf values, initially by re-export from `cg`;
- tree invariants, validation, certification, and typed outcome vocabulary;
  and
- conformance laws independent of either model or producer implementation.

It refuses these contents:

- invalid-source repair and URL/file acquisition;
- ambient font discovery or a built-in family policy;
- `usvg::Tree` or another producer implementation type in the model-consumer
  contract;
- v1 or n0 node construction;
- legacy runtime-paint projection and any graphics backend;
- `.grida` or `.n0.xml` encoding;
- drawlist construction or raster painting;
- browser-grade CSS resolution and SVG-to-pixels rendering;
- retained-source animation compilation; and
- editable SVG source or round-trip guarantees.

`SVGPackedScene` is also too broad a type name for the public product: the
value is neither a storage pack nor an archive. Renaming public types during
the crate cut would, however, add avoidable churn to the zero-behavior move.
Because inherent constructors cannot be restored through a type alias, the
legacy type remains a wrapper during the cut. The canonical shared type is
`ImportTree`: under `svg_import_ir`, the name says it is the ordered normalized
tree consumed for import and refuses storage/transport claims. Any later
rename needs an explicit compatibility plan.

## Math vocabulary

### The four observed choices

| Vocabulary | Scalar / representation | Current role | Consequence at the cut |
|---|---|---|---|
| `cg::CGTransform2D` + `cg::CGRect` | f32; six named matrix fields and `x/y/width/height`; serializable | Public fields of the current IR | Retaining them preserves field meaning and transform JSON shape. |
| `math2::AffineTransform` | f32 row-major 2x3; optional serde | Current producer algebra for relative transforms and offsets | Keeps the current operation order and singular-parent fallback behavior. |
| `n0_model::math::{Affine, RectF}` | f32 six-field affine and `x/y/w/h`; not serde contract | Chassis-owned document/resolution math | Valid adapter target, invalid IR dependency: it reverses the required dependency direction. |
| `kurbo` | primarily f64 geometry values | Private implementation aid in n0 path analysis | Exposing it changes precision and representation and makes an implementation library part of the import contract. |

The two f32 affine implementations are not interchangeable merely because
their six coefficients can be copied. `math2` treats determinants with
absolute value below `f32::EPSILON` as singular; n0 rejects exact zero and
non-finite determinants. N0 also deliberately emits exact quadrant-rotation
matrices. The current importer does not ask the consumer to recompute its
relative transforms, so those differences need not leak into the IR.

### Recommended math decision

Select the existing **`math2` lineage** for the crate cut:

1. retain `CGTransform2D` and `CGRect` as the public stored values for the
   zero-behavior move;
2. use `math2::AffineTransform` for the producer's existing affine algebra;
3. keep `kurbo` out of the public and normal dependency surface;
4. keep `n0-model` out of the IR dependency graph; and
5. convert the six stored affine coefficients and four rectangle components
   in each consumer adapter.

This is not a claim that `math2` becomes the chassis's math. It selects the
shared import producer's existing behavior while allowing the n0 packer to
project values into its native vocabulary. A later general-math
consolidation, if armed by real consumers and law evidence, is a separate
step.

The n0 adapter owes law-equivalence tests for:

- coefficient copying under the exact affine law
  `x' = m00*x + m01*y + m02`, `y' = m10*x + m11*y + m12`;
- the n0 raw matrix order `[m00, m10, m01, m11, m02, m12]`, including the
  sentinel `matrix(1 2 3 4 5 6)` and both basis points;
- all six coefficients, including skew, reflection, translation, and signed
  zero, without decomposition, inversion, or recomposition;
- rectangle component mapping without width/height normalization;
- finite-value certification before the adapter; and
- whole-import `UNSUPPORTED` when a zero or negative reference extent is
  invalid for the receiving path artifact, rather than epsilon inflation.

The producer gate, not the adapter, owns nested composition,
singular-parent fallback, and path-bound offset behavior. A certified tree
requires finite matrices and bounds and nonnegative extents; it preserves
signed-zero coefficients exactly. Legacy compatibility values remain capable
of carrying their current unchecked f32 states. The adapter must not call n0
inverse or composition to recreate values the producer already materialized.

The backend-free path serializer is a prerequisite step, not part of the
crate move. It is replaced and gated while still owned by `grida`; exact path
strings, transforms, `.grida` bytes, and declared rendered artifacts must all
match. Only a later commit may move the already backend-free producer code
across the crate boundary.

## Two SVG surfaces: confirmed by the evidence

NAME should confirm the [topology's two-surface reading](./topology.md), not
collapse it.

### Import-to-document

This surface normalizes static SVG into the shared IR, then uses a
model-specific packer. Its output is authored document content. It is
necessarily limited by what the receiving model can express, and its honest
failure vocabulary includes `UNSUPPORTED`. Source preflight, lowering, and
packing jointly provide that answer; none may claim completeness alone.

### Render-to-pixels

This surface resolves SVG through the browser-grade CSS lineage and paints it
through the one frame entry. Its output is pixels, not document nodes. Its
coverage and correctness are judged against Chromium/consensus corpora, not
against the static import mapping or v1 output.

Sharing an SVG input syntax does not make those products one contract. The
import IR has already crossed usvg's lossy normalization boundary; it cannot
serve as the browser-cascade renderer's source of truth. Conversely, a
display list is not an authored document representation and cannot be the
importer's result.

SVG animation remains a third policy over rendering time and a separate
frontend concern. The normalized static IR may contribute static content in
the future, but it cannot replace the retained source snapshot, target
identity, or explicit-time animation program.

### Initial n0 capability matrix

This matrix is deliberately rejection-biased. It records known fields that
prevent the first n0 packer from pretending that structural construction
equals faithful import.

| Source/IR fact | First n0 disposition | Owning layer |
|---|---|---|
| Image, clip, mask, filter, pattern, stop opacity, dash offset, or miter-clip semantics lost before certification | Whole-import `UNSUPPORTED` | source preflight / producer |
| Non-normal group blend | Whole-import `UNSUPPORTED` until the model and drawlist can express the same isolation law | consumer packer |
| Off-center radial focal geometry | Whole-import `UNSUPPORTED` | consumer packer |
| Retained attributed-run family or nonzero letter/word spacing | Whole-import `UNSUPPORTED` where the chassis text contract cannot retain it | consumer packer |
| Per-run text stroke | Whole-import `UNSUPPORTED` | consumer packer |
| SVG oblique distinct from italic | Whole-import `UNSUPPORTED` while the receiving model has only the collapsed posture | consumer packer |
| Arbitrary finite affine matrix | Copy six coefficients into a raw matrix lens; never decompose | consumer packer |
| Any invalid tree invariant or any unsupported field on any node | Reject the entire import; commit no partial document | validation / consumer packer |

The matrix grows by evidence. A support claim requires both retained source
meaning and a receiving-model representation; the presence of a similarly
named field is not enough.

## Captured-essence ledger

Nothing is deleted or replaced by this evidence step. The ledger below is the
mandatory input to the later cut; a cut that cannot assign every row a real
gate is not ready.

| id | Observed essence or caveat | Provenance | Required disposition at cut |
|---|---|---|---|
| SVG-NAME-01 | Root width/height and child order are preserved in one initial container. | `packed_scene.rs`; `svg_pack.rs` | Carry through the canonical `ImportTree`; unit-test order and dimensions. |
| SVG-NAME-02 | Groups carry relative affine transform, opacity, blend mode, and ordered children. Filters, clipping, masks, and isolation are not carried. | `packed_scene.rs`; `cg/src/svg.rs`; `pack.rs` | Preserve carried fields; name omitted behavior as gaps, never infer it in an adapter. |
| SVG-NAME-03 | Parent-relative transforms are derived from usvg absolute transforms with `math2` inverse/compose and child-world fallback for singular parents. | `packed_scene.rs`; `math2/src/transform.rs` | Move the exact algebra; add coefficient and singular-sentinel tests. |
| SVG-NAME-04 | Paths are converted through Skia, offset to their usvg bounds, serialized back to SVG `d`, and compensated in translation. | `packed_scene.rs`; Skia bridge | Replace the backend dependency only with byte-identical path and downstream artifact evidence. No textual normalization cleanup. |
| SVG-NAME-05 | Solid, linear-gradient, and radial-gradient paint plus fill/stroke opacity, rule, cap, join, miter, dash array, and spread method cross the IR. | `cg/src/svg.rs`; `from_usvg.rs` | Re-export the values and preserve exact serde spellings; keep runtime paint conversion in each consumer. |
| SVG-NAME-06 | Pattern paint becomes transparent; gradient-stop opacity is dropped; miter-clip becomes miter; stroke dash offset is absent. Fill rule crosses the IR but the legacy packer does not project it. | `[MODEL_MISMATCH]` sites in `from_usvg.rs`; `pack.rs`; paint RFD | Preserve current legacy bytes while adding diagnostics. Capability work must use `UNSUPPORTED` or a spec-backed extension, never disguise the loss. |
| SVG-NAME-07 | Images have an empty IR variant but producer image nodes are dropped and the v1 adapter ignores the variant. | `cg/src/svg.rs`; `packed_scene.rs`; `pack.rs` | Do not claim image support. Decide an honest unsupported form before the n0 capability grant. |
| SVG-NAME-08 | Text retains chunks, positions, anchors, uniform styles, and attributed runs with UTF-8 byte offsets, family, size, weight, posture, spacing, fill, and stroke. | `cg/src/svg.rs`; `packed_scene.rs`; attributed SVG tests | Preserve the current value shape and byte-offset semantics. Reconcile the stale domain text page before treating it as a gate. |
| SVG-NAME-09 | Only the first `dx`/`dy` entry for each chunk is applied; named family is preferred and otherwise Geist is selected. | `packed_scene.rs` | Preserve for the cut; isolate family fallback as producer-environment policy before declaring the crate agnostic. |
| SVG-NAME-10 | The legacy IR values derive serde with exact enum tags/renames; transform serialization is the existing 2x3 array shape. Path bounds are skipped/defaulted and text offsets are `usize`, so serialized IR is not a replay-complete archive. No repository consumer currently persists a whole `SVGPackedScene`. | `cg/src/svg.rs`; `cg/src/transform.rs`; consumer search | Treat existing spellings as legacy compatibility behavior, not a new IR semantic or transport contract. Canonical JSON alone cannot gate behavior because skipped bounds drive gradients. |
| SVG-NAME-11 | The v1 adapter owns absolute layout, center stroke alignment, opacity baking, gradient UV normalization, inactive radial-focal fallback, text baseline offsets, attributed-run projection, and group wrappers. | `pack.rs`; `paint.rs` | Leave in the legacy consumer. The n0 packer makes independent, model-honest choices. |
| SVG-NAME-12 | The v1 byte sink sanitizes source, assigns deterministic DFS IDs/positions, and encodes frozen `grida.fbs`; other raw-string entry points do not run the same repair. | `grida.rs`; `pack.rs`; `packed_scene.rs`; FBS round-trip tests | Preserve each existing façade during the cut instead of silently standardizing repair. Compare encoded outputs byte-for-byte. |
| SVG-NAME-13 | Raw parsing depends on embedded/system fonts and default resource behavior; usvg may discard source distinctions before IR lowering. | `formats/svg/parse.rs`; usvg options; usvg tree research | Preserve the ambient path only in the legacy façade. The chassis requires the certified, explicit-environment producer; an arbitrary normalized tree is insufficient. |
| SVG-NAME-14 | The SVG animation frontend retains source and target identity separately and explicitly refuses to be a general importer. | `n0-model/src/svg_animation.rs`; animation RFDs | Keep separate; no adoption or deletion in the static IR cut. |
| SVG-NAME-15 | The committed L0 fixtures, architecture tests, SVG pack tests, FBS round trips, batch tool, reftest harness, and local resvg/W3C/Oxygen corpora form different evidence classes. | verification estate below | Use each only for the claim it can prove; never turn a parse pass into conformance or v1 pixels into an oracle. |
| SVG-NAME-16 | Public compatibility includes `cg::svg`, `grida::cg::svg`, `grida::import::svg::SVGPackedScene`, its public field, three constructors, and serde shape. | cg-cut ledger; repository consumers | Keep definitions in `cg` and a real legacy wrapper in `grida` for the first cut; compile-test every path. No copy and no dependency back-edge. |
| SVG-NAME-17 | Current feature-named tests often prove only parse/no-crash; the batch and converter CLIs may skip failures or flatten recursive basenames. | `svg_pack.rs`; `tool_svg_batch.rs`; `grida_dev svg-to-grida` | Do not cite them as semantic or A/B proof. Build a fail-fast manifest-driven SVG sweep. |
| SVG-NAME-18 | A model packer cannot diagnose source facts erased upstream, and a detached diagnostics list can be ignored accidentally. | producer and consumer patrol | Require conservative source preflight, attached loss diagnostics, certified validation, and whole-import atomic failure. Preserve the lossy legacy façade as the sole explicit exception. |

### Domain-document conflicts found by patrol

Two current pages predate later importer work:

- The [SVG text import model](../feat-svg/text-import.md) still says inline
  style variation is flattened and attributed text is future work, while the
  current IR and packer contain attributed chunks and styled runs added in
  March 2026.
- The [SVG import mapping](../format/svg.md) reports images, clips, masks,
  gradient-stop opacity, and several stroke/effect cases as mapped even where
  the current producer drops or collapses them; it also understates current
  spread-method and attributed-text values.

Those are findings, not permission to choose whichever description is
convenient. The mapping pages must be reconciled against executable evidence
before the Phase 3 capability grant. Until then, this study labels current
code observations rather than declaring new SVG semantics.

## Gate contract for the later cut

The NAME evidence does not satisfy the Phase 3 entry or exit gates. It makes
the required proof shape concrete.

### Evidence available today

On 2026-07-21 the two focused test targets pass: 3/3 seam architecture tests
and 48/48 SVG pack tests. Two clean converter invocations over the 37 tracked
L0 SVGs also produced identical 37-file `.grida` sets. That proves
same-revision determinism only. Many feature-named pack tests assert no more
than successful parsing, and only one tracked L0 fixture is opened directly
by the pack test target.

There is no honest SVG base/head byte sweep today. The existing
`check-legacy-pixel-sweep` design covers the 65 HTML `L0.exact` fixtures, not
this importer. The locally installed corpora currently contain 1,679 resvg,
525 W3C SVG 1.1, and 4,329 Oxygen SVGs, but those counts and revisions are
machine-local facts rather than committed identities. No score-producing
reftest or sealed scoreboard command was invoked for this study.

### Closed dependency and seam gates

- `svg-import-ir` has no dependency or source reference to `grida`,
  `n0-model`, `n0`, Skia, a window system, or a graphics backend.
- The default model-consumer IR surface may depend on `cg`, `math2`, and
  legacy-compatible serde support; it does not expose `usvg` or another
  producer implementation. Any same-package producer layer is non-default or
  private and takes an explicit source environment.
- The legacy packer depends on `svg-import-ir` and the legacy model.
- The n0 packer depends on `svg-import-ir` and `n0-model`.
- `n0-model` depends on neither the IR nor `grida`; an architecture test locks
  the direction.
- The current SVG seam test is retargeted to the new crate and strengthened
  to ban graphics-backend and ambient-resource imports, with an empty or
  shrinking allowlist.
- A Cargo-metadata graph test proves the actual dependency edges rather than
  relying only on source-token scans.
- Compatibility compile tests keep `cg::svg`, `cg::prelude`, the `cg::*` root
  re-exports, `grida::cg::svg`, `grida::cg::prelude`, the `grida::cg::*` root
  re-exports, the legacy scene wrapper, its public field, and all three
  constructors alive. Representative `IRSVG*` and `SVG*` symbols compile
  through every existing route.

### Zero-behavior legacy gates

- First, a dedicated producer-side commit replaces the Skia path serializer
  inside `grida` and passes exact path-string and downstream byte gates. The
  crate cut cannot begin until that commit is independently green.
- IR unit tests cover every semantic field, invariant, UTF-8 run boundary,
  transform coefficient, singular fallback, and path offset. Separate legacy
  serde tests lock existing spellings and omissions without claiming a
  lossless IR fixpoint.
- All SVG pack tests and v1 FBS round-trip/encode-stability tests pass.
- The new declared SVG A/B sweep is byte-identical; the existing 65-fixture
  HTML sweep does not satisfy this claim.
- Import artifacts and rendered outputs for the available resvg test suite,
  W3C SVG 1.1 suite, and Oxygen icon corpus are byte-identical A/B, with the
  exact corpus enumeration and hashes recorded. Local-only corpus presence is
  never implied by the repository.
- A new manifest-driven SVG A/B harness renders detached base and head trees,
  compares exact input/output sets, PNGs, `.grida` bytes, and declared import
  artifacts, and fails on every skip, collision, or byte difference. The
  current converter and score-producing reftest command are not substitutes.
- Workspace checks, architecture locks, clippy, the n0 gate, and the frozen
  wasm build pass; `format/grida.fbs` and the published package contract have
  no diff.

### Chassis capability gates

- Conservative raw-source preflight catches semantics the producer would
  erase; attached lowering diagnostics name every visible drop/collapse; only
  a certified lossless tree reaches the n0 packer; and every representable IR
  construct has a structural fixture and assertion. Every failure retains its
  distinct typed outcome. A valid-source producer loss or consumer projection
  failure becomes an `UNSUPPORTED` scoreboard row; invalid source/environment
  is an input or environment rejection, and an invalid certified-tree
  invariant is a producer defect that fails the gate rather than consuming a
  capability row.
- The n0 packer is atomic: one producer loss, invalid invariant, or
  unsupported field rejects the whole import and commits no partial document.
- The n0 SVG entry is deterministic and uses the same one-frame rendering
  entry as other chassis sources.
- Scores and coverage are recorded only after FLIP is ratified, against the
  Chromium/consensus oracle discipline. V1 similarity is diagnostic, never
  the conformance bar.
- No scoreboard score is produced or inspected by this study.

## Proposed registry wording

For the owner, after the Phase 3 entry gates are satisfied:

> **NAME — taken YYYY-MM-DD.** The shared static import package is
> `svg-import-ir` (`svg_import_ir` in Rust). It is the canonical public
> boundary for the normalized SVG `ImportTree`, its invariants, validation,
> certification, and typed outcome vocabulary; it refuses source
> repair/acquisition, producer implementation types, both model packers,
> format encoding, rendering, animation, and editable-source fidelity. A
> separate loss-aware producer surface uses an explicit closed environment
> and must certify the tree before a chassis consumer accepts it. The first
> compatibility cut keeps the IR values physically in `cg`, re-exports them
> from `svg-import-ir`, and retains the real legacy raw-string wrapper without
> a dependency cycle. The cut retains the existing f32
> `cg::CGTransform2D` / `cg::CGRect` value boundary and `math2` affine algebra;
> it exposes neither `kurbo` nor `n0-model` math. Import-to-document and
> render-to-pixels remain distinct SVG surfaces.

Until the owner supplies that GO, NAME remains open and no crate should be
created under the proposed name.
