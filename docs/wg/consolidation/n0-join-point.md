---
title: "Finding: where n0 joins the shared downstream"
description: "The amendment defers whether n0 emits the common resolved contract or keeps a private drawlist and joins only at the leaf-paint/backend tier. A staged gap analysis names the evidence needed for glyphless visual facts and, later, shaped text."
tags:
  - internal
  - wg
  - program
format: md
---

# Finding: where n0 joins the shared downstream

**Genre:** finding — grounded evidence for a **staged owner decision**. Not a
spec and not a plan. It reframes a question the
[Web-First Amendment](./web-first.md) defers, using n0's actual downstream
types, so the decision can be taken on evidence when it is ripe.

**Status:** the **D-M vector stage was taken on 2026-07-23**, coupled to the
glyphless vector scope of **D-C**, in the
[charter's registry](./charter.md). Source-neutral glyphless facts join high
into the shared downstream: one engine-private compiler, private drawlist,
painter, damage policy, and cache policy. `cg` is the shared leaf-vocabulary
seat; adoption remains per-leaf, and the proving-shell downstream does not
survive as a peer engine. The text stage remains **not yet ripe** because its
second producer does not exist. The complete evidence bar remains recorded in
the [Web renderer adoption patrol](./web-renderer-adoption.md).

## The crux

The amendment originally left one question to a later spike: does n0 **emit the common
resolved contract** (one compiler, one private drawlist, one executor, and
shared frame/damage/cache behavior), or does it **join below its private
drawlist** (n0 and Web each retain a resolved form, compiler, private drawlist,
and private executor while sharing only the leaf-paint vocabulary and
realization utilities plus the raster backend)? The latter is called the low
join here. It does not mean that one executor consumes two unrelated private
drawlist types.

The [prototype](./web-first.md) showed that a *high* join is possible for the
trivial case: the n0 canary lowers a resolved rectangle into `rframe::Frame`
and paints it through the shared downstream. It did not show that n0's real
compiler can stop reading authored/effective model state. A rectangle proves
nothing about the facts that actually differ between producers. The amendment
already supplies the resolving principle — *sharing begins only where the
inputs genuinely match* — so the real question is **per fact kind**, not one
global switch. The completed vector spike and owner decision answer this
question high for glyphless vector facts; shaped text remains open.

## The reframing

n0's downstream is an ordered primitive stream. Each primitive kind carries a
set of facts. Classify each as a **source-neutral candidate** (eligible for a
high join after equivalence evidence) or **n0-coupled** (would leak an
n0/authoring concept into the contract, or is bound to n0's environment and
therefore pushes the join lower). The join point is then the *lowest* fact that
must stay coupled — and it need not be uniform across kinds. Eligibility is not
proof: the current compiler also reads document topology, payload kinds, and
effective values, as the adoption patrol records.

| n0 downstream fact | Nature | Candidate join | Condition / blocker |
| --- | --- | --- | --- |
| Opacity scope, clip-rect, painter order | Structurally neutral (isolation / clips / order — all on the MAY list) | **High** | the vector-input arm separated the visual facts from n0's bracket placement and traversal policy |
| Geometry — rect / oval / line / path, resolved bounds | Neutral (geometry + bounds are on the MAY list) | **High** | each adopted geometry still needs a neutral path/geometry seat and explicit coordinate-space/bounds laws |
| Ordered paint stacks (solid / gradient / image), strokes | Neutral *concepts*, but carried as n0-model value types | **High**, with per-leaf gates | `cg` is the shared leaf-vocabulary seat; each leaf must conform before its mapping is deleted |
| Corner smoothing (squircle) | An authoring semantic carried as a *parameter* in n0's drawlist and resolved in n0's painter | **High only after resolution** | n0 must resolve smoothed corners to neutral geometry *before* the contract; carrying the parameter would leak an n0/authoring concept — forbidden |
| Shaped text (glyph layout + font registry) | Bound to n0's font environment: the shaped-text artifact references a font registry kept opaque and private to n0, shaped through n0's own oracle | **Undecided — the deciding fact** | needs a *neutral* shaped-text representation **and** a neutral font-key/registry boundary that both n0's and the Web family's shapers can produce; neither exists |

## The deciding factor

Everything except text proved eligible to converge high, and the vector-stage
compiler-equivalence evidence selected that join. `cg` now owns the shared
leaf-vocabulary seat, while adoption remains gated per leaf. Shaped text is a
separate later question where
"emit the common contract" and "join below the private drawlist" genuinely
diverge:

- The amendment's MAY list *does* admit "shaped-text artifacts" and "declared
  font/image/resource environments" — so a neutral shaped-text contract is not
  forbidden. The blocker is that n0's shaped text is *implemented* coupled to a
  private font registry and its own oracle; there is no neutral font-key boundary.
- So the choice is: (a) define that neutral shaped-text + font-key boundary and
  push text into the high join too, or (b) let text join *low* — each engine's
  private compiler and executor retain its own text artifact, font registry,
  glyph replay policy, and text item. Sharing then stops at backend glyph/raster
  utilities that require no common font key or shaped-text representation.
  Both remain D-M candidates; (a) is more work and more sharing, (b) is less of
  both. The evidence that should decide it does not exist yet, because there is
  only one shaped-text producer today.

## Decision by stage

- **The vector stage joins high.** Source-neutral glyphless visual primitives
  enter the shared downstream: one engine-private compiler, private drawlist,
  painter, damage policy, and cache policy. `cg` is the shared leaf-vocabulary
  seat; adoption remains per-leaf rather than becoming a wholesale type
  replacement.
- **The boundary remains per-fact.** Facts that cannot normalize without
  source semantics stay below the join or remain excluded pending evidence.
  The separate shaped-text stage stays open.
- **Name the smallest deciding spike, and gate it on a second text producer.**
  When the Web family gains a real shaped-text producer, push a text run from
  *both* it and n0 toward a candidate neutral shaped-text + font-key contract,
  and observe whether a neutral font-key boundary holds (→ text joins high) or
  forces the boundary down (→ text joins low). Until that second producer
  exists, per the amendment's "two real producers first," the text join stays
  deliberately undecided.
- **Prove mixed-fact composition before treating the stages as independent.**
  The vector spike must preserve order, scopes, identity, damage, and cache
  behavior for a frame that mixes admitted vector facts with still-private
  text. Failure collapses the stages into one later decision; it is not
  permission for a second compositor.

## Current vector-input evidence

The first bounded arm of the vector-input/mixed-composition spike passed on
2026-07-23:

- An independently constructed normalized input and n0's authored and
  immutable-effective views agree for rectangle, ellipse, path, and line
  geometry; even-odd fill; ordered fills and strokes; opacity and clip scopes;
  ordinary corners; painter order; and primitive-specific admissibility.
- Both inputs produce the same existing private drawlist and exact raw raster.
  No second painter or executor participates.
- Real n0-private shaped text, including its exact private font environment,
  interleaves with those vector facts in one ordered frame and contributes the
  same pixels. This proves that the vector and later text stages can compose at
  this bounded orchestration seam; it does not decide the text join.
- Two independently constructed normalized frames retain the same complete
  identity/provenance keys across a fill transition. They enter the same
  complete-frame comparison policy as n0's ordinary frame products, report
  only the changed rectangle, produce its exact world-space coverage, and
  leave the interleaved real private text undamaged. Projecting the ordinary
  frame result into the candidate identity domain gives the same attribution
  and coverage. Inactive-paint edits remain undamaged in both arms because
  their compiled visual material is unchanged. The policy ignores
  document-specific draw-item slots. The candidate's full identity/provenance
  pair is only an opaque, arm-local owner key here; this evidence does not
  decide which part owns replacement identity in a future public contract.
- The same two frames, still mixed with real private shaped text, enter n0's
  one preview cache. Cold input, exact reuse, visible replacement, and exact
  replacement reuse report `true, false, true, false`; each candidate-cache
  raster is byte-identical to the corresponding ordinary n0-cache raster.
  Cache material compares paint-consumed order and facts, the private text
  replay registry, and the paint environment while ignoring the diagnostic
  document slot for raster reuse. Preflight failures do consume that slot for
  diagnostics, so exact reuse retains the cached diagnostic owner; any
  promotion must settle diagnostic provenance separately. Source-owned
  invalidation remains separate and coherent, including transactional
  environment, gradient, and image failure and safe return from a candidate
  replacement to an ordinary n0 frame.
- The cache probe also corrected an overclaim in the preview policy. During
  local Darwin-arm64 integration, the margin-translated backend raster for this
  antialiased fixture differed from direct accurate-frame execution at two
  pixels, four channels total, with a maximum channel delta of one; a manually
  translated offscreen raster reproduced the cache exactly. This observation
  locates the cause in backend device-translation antialiasing rather than
  comparison or blitting, but is not itself a durable conformance gate. The
  load-bearing gates compare like duty cycles exactly: candidate preview to n0
  preview, and candidate direct execution to n0 direct execution. No threshold
  is introduced. Accurate static and exact-time export therefore remain direct
  immutable-frame duties, never cache-output duties.
- Nonzero corner smoothing is refused rather than carried as an authoring
  parameter. It must become resolved geometry before a high join or remain
  below that join. Invalid line fills, non-rectangular clips in this arm, and
  inadmissible stroke states also fail explicitly.

The arm completed the evidence bar for the vector-stage decision. Its opaque
identity and provenance drive local fact lookup, painter order, mixed-text
placement, and the engine's one private complete-frame damage policy without
becoming a public runtime identity. Its compiled visual material also reaches
the one private preview-cache policy without making source identity part of
raster reuse. The decision does not admit source-specific fields, settle
cross-frame replacement identity, or authorize parallel policies.

## The registered stages

**D-M** is registered in the [charter's decision registry](./charter.md) with
independent stages. The **vector stage**, coupled with D-C and the
leaf-vocabulary seat, is taken high on the compiler-read inventory,
paint/stroke gap report, and normalized-input equivalence spike, including its
mixed-fact composition condition. The proving-shell downstream therefore
retires after the n0 route earns its gates; it does not grow into a peer
engine. The **text stage** later chooses high or low for shaped text after the
two-producer font-key spike.
