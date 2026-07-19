---
title: Display-List Contract Study
description: "Design study: whether a shared display-list contract between the two engines should exist. Conclusion: no — the shared surface stops at the leaf paint vocabulary."
tags:
  - internal
  - wg
  - canvas
  - rendering
format: md
---

# Display-List Contract Study

**Status:** Design study — findings, not a spec. This document answers
one question and does not define a contract.

Written for an engine developer deciding, at promotion time, whether
the two engines' display lists need a shared specification. Filed from
[gridaco/nothing#33](https://github.com/gridaco/nothing/issues/33) under
the seam program
[gridaco/nothing#27](https://github.com/gridaco/nothing/issues/27),
which states the lean this study tests: *the shared surface stops at
the leaf vocabulary; the display list is per-engine.*

## The question

Both engines compile the same scene vocabulary — nodes carrying ordered
paint stacks and stroke applications, per the
[Paint Model RFD](../feat-painting/paint-model.md) — into an
intermediate form their renderer executes: a display list. Must that
intermediate form be specified once, as a contract both engines
conform to? Or is it engine-internal?

## The two projections

The evidence is the two display lists as they exist. Described in
domain terms:

**The production engine's projection** is a *retained layer tree*. Each
leaf is a self-contained layer of pure draw content — resolved shape,
fill stack, stroke stack, effects — deliberately excluding transform,
opacity, blend, and clip so the recorded content can be cached and
replayed across frames under changing composite state. That composite
state (opacity, blend mode, world transform, clip path, z-index) lives
in a base record *on every layer*. Above the leaves sits a command
tree: draw commands, mask groups pairing mask-command lists with
content-command lists, and effect surfaces — interior nodes that
composite their children offscreen and apply container-level effects
once to the composited result. The design optimizes for **picture
caching and partial invalidation**: a layer records once and redraws
cheaply; a geometry-only change patches one base record in place.

**The v2 proof's projection** is a *flat ordered stream* of items. Each
item pairs one node's leaf primitive — a fill or one stroke application
over a rect, oval, path, line, or resolved text layout — with a world
transform copied verbatim from the resolver, never recomputed. Scoped
state is expressed not as node attributes but as *paired bracket
commands in the stream*: an opacity group opens and closes around a
range of items; a content clip opens after a container's fill and
closes before its own strokes. There are no mask or effect constructs.
The design optimizes for **purity and diffability**: the stream is a
value, comparable by equality between frames, with no camera baked in
so one list paints at any zoom.

### The structural inversion

The two encode the same information in inverse positions:

| Dimension                              | Production projection                                            | v2 projection                                                    |
| -------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- |
| Overall shape                          | Tree of commands over retained layers                            | Flat ordered stream of items                                     |
| Composite state (opacity, blend, clip) | Attribute of each layer's base record                            | Paired begin/end bracket commands scoping a range                |
| World transform                        | Stored per layer, patchable in place                             | Copied per item from the resolver, verbatim                      |
| Grouping constructs                    | Mask groups and effect surfaces as interior tree nodes           | Opacity and clip brackets; no mask or effect construct           |
| Optimized for                          | Picture caching, partial invalidation, per-layer reuse           | Value equality, frame diffing, camera-free replay                |
| Leaf content                           | Shape + fill stack + stroke stack + effects, in one layer        | One primitive per item: a fill *or* one stroke application       |

Attribute-of-node versus bracket-in-stream is not a stylistic
difference. Flattening the tree into a bracketed stream is mechanical;
reconstructing the tree from the stream means re-inferring grouping —
which is exactly each engine's own compile policy. A shared contract
would have to legislate one of the two encodings, and the engine on the
losing side would rebuild its renderer around the other's structure.

### The capability asymmetry

The two lists also do not cover the same feature set. Masks, effect
surfaces, and z-ordering exist only in the production projection;
value-view projection and the pinned text-font registry accompanying
glyph-bearing items exist only in the v2 projection. A shared contract
must be the union of both vocabularies — so each engine would carry
constructs it never produces and cannot execute, or the contract
fragments into profiles, which is the absence of a contract wearing its
costume.

### What is actually shared

Every leaf of both projections pairs a geometry with values from the
same paint vocabulary: an ordered paint stack for fills, a stroke
application for strokes, and — for text — a resolved layout with the
tri-state run-paint ownership of the paint model. The sharing the seam
program needs already exists **one level below the display list**, and
it is specified: the [Paint Model RFD](../feat-painting/paint-model.md)
for the values, and the
[Universal Shaped Text Layout RFD](../feat-paragraph/text-layout.md)
for the resolved text artifact those leaves consume.

### The consumer test

The seam program's standing rule is that a boundary earns a contract
when its second consumer appears — and not before. Applied here: each
display list has exactly one producer (its engine's compiler) and
exactly one consumer (the same engine's executor), living in the same
codebase and shipping on the same commit. No serialized display list
crosses a process, wire, or version boundary. There is no foreign
consumer to protect, so a contract would protect no one — while still
charging both engines its full conformance cost.

## Conclusion

**The lean is confirmed. No shared display-list contract should exist,
and no display-list spec should be authored.**

- The shared surface between the two engines stops at the leaf
  vocabulary — the paint model and the resolved-text artifact. That
  surface is already specified in its own homes.
- The display list is each engine's private projection of that
  vocabulary, shaped by the performance strategy it serves — retained
  caching in one, pure diffable streams in the other. The two
  structures are inverses; a shared contract would rewrite one engine
  in the other's image and freeze both engines' performance strategies
  into a document neither would honor for long.
- A WG contract binds inputs (the scene and paint vocabulary) and
  outputs (pixels, under conformance suites) — not the intermediate a
  renderer chooses on the way from one to the other.

### Conditions that reopen this question

The conclusion is falsifiable. Any one of these creates the foreign
boundary that would demand a spec, scoped to what the new consumer
actually needs:

1. **A display list crosses a process or wire boundary** — a remote
   renderer, a replay file format, or display-list interchange between
   engines.
2. **A second in-repo consumer of a display list appears** — for
   example an exporter or hit-tester consuming the compiled list
   instead of the scene.
3. **Leaf-vocabulary divergence** — if the engines stop agreeing on the
   leaf pairing of geometry with paint-model values, the shared surface
   has moved and its boundary must be re-drawn.

If reopened, the spec's scope is the *serialized leaf stream and its
scoping brackets* for the crossing in question — not a unification of
the two engines' internal structures, which this study found to be the
expensive non-goal.

## Evidence

Non-normative provenance — where the two projections were read. Both
engines are cited by path in this repository (the v2 proof landed
in-tree with gridaco/nothing#5).

- Production engine (`main`): the retained layer tree, per-layer base
  record, mask groups, and effect surfaces in
  [`crates/grida/src/painter/layer.rs`](../../../crates/grida/src/painter/layer.rs).
- v2 engine (see
  [gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)):
  the flat item stream, paired scope commands, and per-item world
  transforms in
  [`crates/n0/src/drawlist.rs`](../../../crates/n0/src/drawlist.rs)
  (promoted from the `model-v2-anchor` branch's `model-v2/engine`).
