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

**Genre:** finding — grounded evidence for an **open owner decision**. Not a
spec and not a plan. It reframes a question the
[Web-First Amendment](./web-first.md) defers, using n0's actual downstream
types, so the decision can be taken on evidence when it is ripe.

**Status:** open as staged **D-M**, coupled to **D-C**, in the
[charter's registry](./charter.md). A bounded vector-input arm now proves
drawlist/raster equivalence and mixed composition with still-private text, but
the vector stage is **not yet ripe**: stable identity and provenance do not
reach the shared damage/cache policies, and the leaf-vocabulary seat remains
open. The text stage is also **not yet ripe** because its second producer does
not exist. The complete evidence bar remains recorded in the
[Web renderer adoption patrol](./web-renderer-adoption.md).

## The crux

The amendment leaves one question to a later spike: does n0 **emit the common
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
global switch.

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
| Opacity scope, clip-rect, painter order | Structurally neutral (isolation / clips / order — all on the MAY list) | **High candidate** | the vector-input arm of the spike must separate the visual facts from n0's bracket placement and traversal policy |
| Geometry — rect / oval / line / path, resolved bounds | Neutral (geometry + bounds are on the MAY list) | **High candidate** | a neutral path type and explicit coordinate-space/bounds laws; n0's current path is kurbo-backed |
| Ordered paint stacks (solid / gradient / image), strokes | Neutral *concepts*, but carried as n0-model value types | **High candidate**, conditionally | the **leaf-vocabulary seat** — n0 must lower its paints/strokes into a neutral vocabulary, which is itself a deferred decision |
| Corner smoothing (squircle) | An authoring semantic carried as a *parameter* in n0's drawlist and resolved in n0's painter | **High candidate**, if resolved first | n0 must resolve smoothed corners to neutral geometry *before* the contract; carrying the parameter would leak an n0/authoring concept — forbidden |
| Shaped text (glyph layout + font registry) | Bound to n0's font environment: the shaped-text artifact references a font registry kept opaque and private to n0, shaped through n0's own oracle | **Undecided — the deciding fact** | needs a *neutral* shaped-text representation **and** a neutral font-key/registry boundary that both n0's and the Web family's shapers can produce; neither exists |

## The deciding factor

Everything except text is eligible to converge high or is blocked on the
**leaf-vocabulary seat** decision, but eligibility still needs the vector-stage
compiler equivalence evidence. Shaped text is a separate later question where
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

## Recommendation (for the owner to decide, by stage)

- **Do not take a uniform A-vs-B decision.** The honest shape is a *per-fact*
  boundary: glyphless visual primitives are high-join candidates subject to
  the vector equivalence spike, while shaped text is separately open.
- **Couple this decision with the leaf-vocabulary seat.** n0 cannot emit the
  common contract for paint/stroke facts until the neutral leaf vocabulary
  exists; deciding one without the other is deciding on air.
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
- Nonzero corner smoothing is refused rather than carried as an authoring
  parameter. It must become resolved geometry before a high join or remain
  below that join. Invalid line fills, non-rectangular clips in this arm, and
  inadmissible stroke states also fail explicitly.

The arm deliberately does **not** complete D-M. Its opaque identity and
provenance drive local fact lookup, painter order, and mixed-text placement,
but n0's frame comparison, damage, and cache policies still consume
document-specific identity and resolved storage. No shared two-frame damage or
cold/reuse/change/reuse cache evidence exists. The path-vocabulary ownership
and D-C leaf-vocabulary seat also remain open. These are decision blockers, not
permission to add parallel policies or to widen the provisional contract.

## The registered stages

**D-M** is registered in the [charter's decision registry](./charter.md) with
independent stages. The **vector stage**, coupled with D-C and the
leaf-vocabulary seat, chooses the glyphless join and proving-shell disposition
after the compiler-read inventory, paint/stroke gap report, and normalized-input
equivalence spike, including its mixed-fact composition condition. The **text
stage** later chooses high or low for shaped text after the two-producer
font-key spike. D-C does not, by itself, choose either stage. Until its
applicable stage is taken, the n0 canary stays a deliberately tiny invariant
probe, not a widening integration.
