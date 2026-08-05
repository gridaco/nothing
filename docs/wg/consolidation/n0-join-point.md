---
title: "Finding: where n0 joins the shared downstream"
description: "Where n0 joins the shared downstream, decided per fact kind on staged evidence: glyphless visual facts join high (2026-07-23); shaped text joins low on the two-producer spike (2026-08-05), keeping text artifacts, font registries, and glyph replay policy engine-private."
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

**Status:** the staged decision is **complete**. The **vector stage was taken
on 2026-07-23**, coupled to the glyphless vector scope of **D-C**, in the
[charter's registry](./charter.md): source-neutral glyphless facts join high
into the shared downstream — one engine-private compiler, private drawlist,
painter, damage policy, and cache policy — with `cg` as the shared
leaf-vocabulary seat, adoption per-leaf, and the proving-shell downstream
retired rather than surviving as a peer engine. The **text stage was taken on
2026-08-05: shaped text joins low**, on the two-producer spike this finding
required ([the text-stage evidence](#the-text-stage-evidence) below;
[gridaco/nothing#73](https://github.com/gridaco/nothing/pull/73)). The
complete vector evidence bar remains recorded in the
[Web renderer adoption patrol](./web-renderer-adoption.md).

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
question high for glyphless vector facts; the shaped-text stage was answered
separately — **low** — by [the two-producer spike](#the-text-stage-evidence).

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
| Shaped text (glyph layout + font registry) | Bound to n0's font environment: the shaped-text artifact references a font registry kept opaque and private to n0, shaped through n0's own oracle | **Low — taken 2026-08-05** | the two-producer spike measured the candidate neutral boundary: shaping facts join, but metric facts arrive scaler-tinted, glyph replay is pixel-visible raster policy, and the fact itself is a resource reference the contract refuses — see [the text-stage evidence](#the-text-stage-evidence) |

## The deciding factor

Everything except text proved eligible to converge high, and the vector-stage
compiler-equivalence evidence selected that join. `cg` now owns the shared
leaf-vocabulary seat, while adoption remains gated per leaf. Shaped text was a
separate later question where
"emit the common contract" and "join below the private drawlist" genuinely
diverged:

- The amendment's MAY list *does* admit "shaped-text artifacts" and "declared
  font/image/resource environments" — so a neutral shaped-text contract is not
  forbidden. The blocker was that n0's shaped text is *implemented* coupled to a
  private font registry and its own oracle; there is no neutral font-key boundary.
- So the choice was: (a) define that neutral shaped-text + font-key boundary and
  push text into the high join too, or (b) let text join *low* — each engine's
  private compiler and executor retain its own text artifact, font registry,
  glyph replay policy, and text item. Sharing then stops at backend glyph/raster
  utilities that require no common font key or shaped-text representation.
  (a) is more work and more sharing, (b) is less of both. Until the Web family
  gained a real producer, the deciding evidence could not exist; with
  `textlayout` live, the two-producer spike ran and chose **(b)** — see
  [the text-stage evidence](#the-text-stage-evidence).

## Decision by stage

- **The vector stage joins high.** Source-neutral glyphless visual primitives
  enter the shared downstream: one engine-private compiler, private drawlist,
  painter, damage policy, and cache policy. `cg` is the shared leaf-vocabulary
  seat; adoption remains per-leaf rather than becoming a wholesale type
  replacement.
- **The boundary remains per-fact.** Facts that cannot normalize without
  source semantics stay below the join or remain excluded pending evidence.
- **The text stage joins low** (taken 2026-08-05). The smallest deciding spike
  this entry required — a text run from *both* producers pushed at a candidate
  neutral shaped-text + font-key contract — ran once the Web family gained a
  real producer, and the boundary failed at three seams: the measured
  replay-policy divergence, the metric-fact identity, and the resource
  reference the contract's standing identity refuses. Each engine keeps
  its own text artifact, font registry, glyph replay policy, and text item;
  sharing stops at backend glyph/raster utilities.
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

## The text-stage evidence

The two-producer spike this finding required ran on 2026-08-05 as
[`crates/n0/src/text_join_spike.rs`](../../../crates/n0/src/text_join_spike.rs)
— in-tree and unit-test-only, like the vector spike, surviving as the witness
behind the decision rather than as a proposal. The Web family's producer is
[`crates/textlayout`](../../../crates/textlayout/src/lib.rs) at oracle v0
(`textlayout` enters `n0` as a dev-dependency for this evidence alone, and the
spike locks it out of the shipping graph); n0's producer is its Skia Paragraph
oracle over its private font registry. Both were pushed at one spike-local
candidate fact: a content-digest font key (digest, face index, variation
coordinates), font size, baseline-relative glyph placements, advance, and line
metrics — no live object, no registry address, no raster-facing flags.

- **The shaping facts join.** For the overlapping slice — printable-ASCII LTR
  text in one declared face — the producers state bit-identical *measured*
  shaping facts: the same glyph identities, pen positions, and advance. (The
  content key matches by construction, not by measurement: n0's artifact
  cannot state it — the fourth bullet — so the spike joins it back from the
  host's declaration.) The inputs genuinely match at the shaping layer, which
  licenses the low join's shared glyph/raster-utility seat by measurement and
  means the low join hides no shaping divergence.
- **The metric facts do not join.** The same resolution's ascent, descent, and
  baseline arrive exact from the Web producer's font-unit arithmetic and one
  scaler quantum off from n0's paragraph backend — 2⁻¹⁴ per metric on the
  declared Darwin-arm64 build; a new platform declares its own value through
  one loud CI round-trip. The skew is below coverage resolution (the outline
  arm proves the quantized baseline moves no pixel), so this leg is not a
  pixel break — it is a fact-identity break: a neutral contract would have to
  legislate metric derivation, rebuilding n0's oracle output, or carry one
  fact with two meanings.
- **Outlines are meaning; glyph replay is policy.** Two outline extractions of
  the same candidate fact — Skia `get_path` over environment bytes, and the
  artifact's own ttf-parser stream — are byte-identical through the one
  shared path-fill rasterizer at every probed anchor, on and off the integer
  lattice, and bilevel on it (the admitted-domain law); the controlled
  question is whether lowering to outlines loses anything, and it loses
  nothing. n0's replay through its oracle's live font, which arrives with
  hinting, subpixel positioning, and edging set by the paragraph backend,
  paints a measured non-bilevel fringe against those same outlines *on the
  lattice itself* (432 bytes at the probed cell on Darwin-arm64; control
  isolation attributes the fringe to the anti-alias glyph-mask pipeline at
  this cell — an alias-edged replay matches the outlines byte-exactly) —
  where the external oracle gates byte-exactly. Carrying the fact without the
  policy makes that fringe a silent wrong pixel; carrying the policy puts
  raster flags into a meaning-only contract; and the remaining branch — one
  downstream-mandated uniform replay policy — either mandates n0's policy
  (breaking the Web route's byte-exact gate by exactly the measured fringe)
  or mandates outline realization (fixing replay as contract meaning and
  rewriting n0's product raster with no oracle to certify it). None is
  admissible.
- **The fact is a resource reference.** Realizing the candidate requires a
  declared digest→bytes environment and gains an undeclared-key refusal — the
  exact boundary `rframe`'s standing identity refuses ("no fact that
  references a resource") and the `n0::glyphless` route is named for. This
  leg is a structural cost, not a pixel measurement: admitting the fact
  spends that refusal. And n0's artifact itself carries process-local
  identity only: the content digest never flows through its oracle and had to
  be joined back from the host's declaration.
- **Beyond the overlap the second producer still does not exist.** Line
  structure is a typed refusal in the Web producer's v0 profile, and styled
  runs and variable instances are not even expressible in its input surface —
  while n0 resolves all three today. Everywhere the join would have
  substance, the amendment's two-producers-first rule still fails.

**The decision: shaped text joins low.** Each engine's private compiler and
executor retain their own text artifact, font registry, glyph replay policy,
and text item. Sharing stops at backend glyph/raster utilities — licensed by
the shaping-agreement measurement, requiring no common font key or shaped-text
fact in the contract. Content-digest font identity remains a *host-seam*
convention, which the tree already shares by convergence rather than by
contract: the CLI `--font` surface, `textlayout::FontKey`, and the bake
manifests all state fonts as sha256 content identities. The Web route keeps
lowering glyphs to resolved outline geometry before `rframe`, which the
evidence shows is no degradation: outline realization is the one realization
every party agrees on byte-exactly where the oracle gates.

The decision fixes where the *contract* boundary sits; it does not forbid the
engines from later sharing a shaping oracle as a utility below it. Re-opening
is a new registered decision and requires both of: a declared resource
environment entering the shared contract for some other fact kind on its own
evidence, and a measured need for cross-contract glyph replay. Producer
maturity alone — a `textlayout` that grows styled runs, line structure, and
variable instances — does not re-open it, because the legs that decided it
(the metric fact identity, the pixel-visible replay policy, the one-meaning
tripwire) are independent of how much text the Web producer can state.
Neither re-opening condition exists today; speculative sharing infrastructure
is forbidden by the amendment's evidence-first discipline, and a
replay-performance motive would additionally need the optimization law's
measurement.

## The registered stages

**D-M** is registered in the [charter's decision registry](./charter.md) with
independent stages, and both are now taken. The **vector stage**, coupled with
D-C and the leaf-vocabulary seat, is taken high on the compiler-read
inventory, paint/stroke gap report, and normalized-input equivalence spike,
including its mixed-fact composition condition. The proving-shell downstream
therefore retires after the n0 route earns its gates; it does not grow into a
peer engine. The **text stage** is taken low on the two-producer font-key
spike above: shaped text stays below the join, in each engine's private tier,
with sharing confined to backend glyph/raster utilities.
