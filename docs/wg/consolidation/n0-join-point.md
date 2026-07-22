---
title: "Finding: where n0 joins the shared downstream"
description: "The amendment defers whether n0 emits the common resolved contract or joins only at the drawlist boundary. A gap analysis of n0's real downstream facts shows the join point is not one point — visual primitives converge high, shaped text may converge low — and names what would settle it."
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

**Status:** open as **D-M**, coupled to **D-C**, in the
[charter's registry](./charter.md), and **not yet ripe** — the deciding
evidence needs a second producer for the hard fact (shaped text), which does
not exist yet. This finding says what the decision hinges on and names the
smallest spike that would ripen it.

## The crux

The amendment leaves one question to a later spike: does n0 **emit the common
resolved contract** (its resolver output lowers into `rframe::Frame`, and the
whole downstream — drawlist, painter, bounds, damage, caches — is shared), or
does it **join only at the drawlist boundary** (n0 keeps its own resolved form
and its own drawlist; only the painter and raster tiers are shared)?

The [prototype](./web-first.md) already showed the *high* join is possible for
the trivial case: the n0 canary lowers a resolved rectangle into `rframe::Frame`
and paints it through the shared downstream. But a rectangle proves nothing
about the facts that actually differ between producers. The amendment already
supplies the resolving principle — *sharing begins only where the inputs
genuinely match* — so the real question is **per fact kind**, not one global
switch.

## The reframing

n0's downstream is an ordered primitive stream. Each primitive kind carries a
set of facts. Classify each as **source-neutral** (belongs in the common
contract → n0 can join high for it) or **n0-coupled** (would leak an n0/authoring
concept into the contract, or is bound to n0's environment → n0 should join
lower for it). The join point is then the *lowest* fact that must stay coupled —
and it need not be uniform across kinds.

| n0 downstream fact | Nature | Join point | Condition / blocker |
| --- | --- | --- | --- |
| Opacity scope, clip-rect, painter order | Structurally neutral (isolation / clips / order — all on the MAY list) | **High** (common contract) | none |
| Geometry — rect / oval / line / path, resolved bounds | Neutral (geometry + bounds are on the MAY list) | **High** | a neutral path type (n0's is kurbo-backed; the contract needs its own) |
| Ordered paint stacks (solid / gradient / image), strokes | Neutral *concepts*, but carried as n0-model value types | **High**, conditionally | the **leaf-vocabulary seat** — n0 must lower its paints/strokes into a neutral vocabulary, which is itself a deferred decision |
| Corner smoothing (squircle) | An authoring semantic carried as a *parameter* in n0's drawlist and resolved in n0's painter | **High**, if resolved first | n0 must resolve smoothed corners to neutral geometry *before* the contract; carrying the parameter would leak an n0/authoring concept — forbidden |
| Shaped text (glyph layout + font registry) | Bound to n0's font environment: the shaped-text artifact references a font registry kept opaque and private to n0, shaped through n0's own oracle | **Undecided — the deciding fact** | needs a *neutral* shaped-text representation **and** a neutral font-key/registry boundary that both n0's and the Web family's shapers can produce; neither exists |

## The deciding factor

Everything except text either converges high already or converges high behind
the **leaf-vocabulary seat** decision (the two are coupled — resolve them
together). Shaped text is where "emit the common contract" and "join at the
drawlist" genuinely diverge:

- The amendment's MAY list *does* admit "shaped-text artifacts" and "declared
  font/image/resource environments" — so a neutral shaped-text contract is not
  forbidden. The blocker is that n0's shaped text is *implemented* coupled to a
  private font registry and its own oracle; there is no neutral font-key boundary.
- So the choice is: (a) define that neutral shaped-text + font-key boundary and
  push text into the high join too, or (b) let text join *low* — n0 emits its
  own text drawlist item, and only the painter/backend is shared for glyphs.
  Both honor the amendment; (a) is more work and more sharing, (b) is less of
  both. The evidence that should decide it does not exist yet, because there is
  only one shaped-text producer today.

## Recommendation (for the owner to decide, when ripe)

- **Do not take a uniform A-vs-B decision.** The honest shape is a *per-fact*
  boundary: visual primitives converge high, shaped text is open. Record it
  that way.
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

## The decision to file

**D-M** is registered in the [charter's decision registry](./charter.md),
**coupled with D-C and the leaf-vocabulary seat**: *n0's join point per fact
kind* — high (emit the common contract) for the visual primitives once the
neutral vocabulary exists, and high-or-low for shaped text pending the
font-key spike above. D-C's gap report supplies the paint/stroke equivalence
evidence but does not, by itself, choose the shared contract's seat or text
join. D-M's full evidence bar is this gap analysis, that report, and the
two-producer text spike. Until then the n0 canary stays a deliberately tiny
invariant probe, not a widening integration.
