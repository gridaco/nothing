---
title: "The anchor Box Model"
description: "Open RFD for authored box intent, box sources, parent-relative bindings, layout participation, visual-only transforms, derived boxes, and pure document resolution."
tags:
  - internal
  - wg
  - layout
  - canvas
format: md
---

# The `anchor` Box Model

**Status:** Open RFD — graduation draft.

This document is written for an engine implementer who needs one
language-agnostic contract for authored box intent and resolved document
geometry. It brings the proven core of the archived `anchor` workbench
into the live WG tree. It does not ratify the unresolved parts of that
workbench, and it does not make either existing engine the oracle.

## Thesis

A scene node is an **anchored box with typed content**.

The box says where the node participates in its parent coordinate space.
The content says what is realized inside that box. The source of the box and
the realization of the content are independent choices. Authored state keeps
the intent needed to resolve them; derived geometry is output, never hidden
authored state.

This separation is what lets free-positioned graphics and structured layout
share one scene model:

- a freely positioned child retains parent-relative placement intent;
- an in-flow child contributes one box to its parent's layout;
- measured content feeds a natural size forward without writing it back;
- derived containers report boxes without compensating child writes; and
- visual transforms can change what is seen without changing what layout
  negotiates.

## Scope and ownership

This RFD owns:

- authored position and size intent;
- declared, measured, and derived box sources;
- the boundary between a node box and content realization;
- free-positioned and layout-owned participation;
- sizing boxes versus transformed visual bounds;
- structured rotation and flips;
- derived-box placement and union semantics;
- the pure resolution pipeline; and
- the semantic contract for materialized reads and intent-preserving writes.

It does not own:

- flex algorithm details, owned by the
  [Flex Layout Profile](./flex.md);
- grid algorithm details;
- leaf paint values, owned by
  [The Paint Model](../feat-painting/paint-model.md);
- node opacity and compositing, owned by
  [Stroke-Fill Opacity Compositing](../feat-2d/stroke-fill-opacity.md) and
  [Isolation Mode](../feat-2d/isolation-mode.md);
- shaping and text geometry, owned by
  [Universal Shaped Text Layout](../feat-paragraph/text-layout.md);
- vector-network topology and editing, owned by
  [Vector Network](../feat-vector-network/index.md);
- source syntax, default spelling, parsing, or canonical writing, owned by
  the [n0 XML RFD](../format/n0-xml.md);
- durable source addressing, owned by
  [n0 XML durable addressing](../format/n0-xml-addressing.md);
- grouping, auto-layout, scaling, or resize gestures, owned by the
  [canvas specification](../canvas/index.md);
- transactions and undo, owned by [History](../feat-history/index.md); or
- replication and merge policy, owned by [CRDT](../feat-crdt/index.md).

Those domains project into this model. They do not redefine it.

## Vocabulary

| Term | Meaning |
| --- | --- |
| **Anchored box** | One node's parent-relative sizing rectangle plus the intent that resolves it. |
| **Authored intent** | State chosen by an author or operation and retained across resolutions. |
| **Box source** | The rule that supplies one or both of a node's extents: declared, measured, or derived. |
| **Content realization** | The one-way projection that produces content inside a resolved box. |
| **Sizing box** | The untransformed box used by free positioning, layout, hug, and derived unions. |
| **Visual transform** | A post-layout transform that changes visual geometry without changing the sizing box. |
| **Visual bounds** | Conservative world-space bounds after visual transforms and every applicable content, stroke, and effect inflation. The owning content and painting specs define those inflations. |
| **Derived origin** | The stored local origin of a derived node's coordinate space. |
| **Active node** | A node whose effective activity value admits its subtree to resolution. An inactive node and its descendants contribute no sizing, layout, bounds, hit-testing, or painting output. |
| **Resolved document** | Immutable boxes, transforms, bounds, and content artifacts produced for one explicit environment. |
| **Error by rule** | An input combination whose meaning is deliberately refused and reported, rather than guessed. |

## The two independent axes

Every node kind declares one **box-source rule per axis** and one **content
realization**. A kind may use the same source on both axes or combine sources,
such as a declared width with a measured height. During one resolution, each
axis has exactly one effective extent owner.

### Box sources

| Source | Contract |
| --- | --- |
| **Declared** | Authored size intent supplies the box, subject to applicable constraints. |
| **Measured** | Content is measured under explicit constraints; its result supplies one or both axes. Measurement is one-way. |
| **Derived** | A kind-specific rule derives the box from other scene geometry. A group uses an active-child union; other kinds require their own adopted rule. The result is not stored. |

### Content realizations

Common realizations include:

- **parametric** — geometry is a function of the resolved box;
- **mapped** — stable source geometry is mapped from a reference space into
  the resolved box;
- **flowed** — content is laid out under box constraints;
- **fitted** — content is fitted into the box by a declared fit rule;
- **children** — descendants retain their own boxes in the node's coordinate
  space; and
- **transformed children** — descendants are passed through an explicit
  post-resolution transform quarantine.

A realization may consume a resolved box. It must not rewrite that box or
feed derived realization output back into layout. A measured box source is
the deliberate exception to the first sentence: measurement produces a
natural size before layout, and the final realization then consumes the
resolved constraints. The text artifact and its one-resolution rule are
specified in the text-layout RFD.

## Authored and resolved tiers

The authored tier contains intent:

- axis bindings;
- size intent and applicable constraints;
- layout participation;
- structured rotation and flips;
- content parameters; and
- explicit references to external resources.

The resolved tier contains consequences:

- sizing boxes;
- local and world transforms;
- transformed visual bounds;
- shaped-text artifacts;
- materialized mapped geometry; and
- resolution diagnostics.

Resolved values are not ordinary authored fields. A cache may retain them
only as a marked derivative with enough identity to detect staleness; it
cannot become a second source of truth.

Resolution never writes resolved values back into authored state.

## Axis bindings

Each axis is independent. A free-positioned node uses one of four binding
forms against its direct parent's sizing extent:

| Binding | Authored values | Meaning for an ordinary box |
| --- | --- | --- |
| **Start pin** | offset `o` | near edge = `o` |
| **Center pin** | offset `o` | near edge = `(P - S) / 2 + o` |
| **End pin** | offset `o` | near edge = `P - o - S` |
| **Span** | start `a`, end `b` | near edge = `a`; extent = `P - a - b` |

`P` is the definite parent extent on the axis and `S` is the node's
resolved extent.

A span owns its base extent, so Fixed or Auto size intent is inapplicable
while the span is active. If `P - a - b` is negative, resolution clamps the
span extent to zero and reports the clamp. General min/max applicability is
not yet uniform across authoring surfaces: n0 XML rejects those constraints
on a spanned axis, while a surface that admits them applies them after the
span result.

Start pins do not need a parent extent. Center pins, end pins, and spans do.
If the direct parent has no definite extent on that axis, resolution must
surface an error by rule. It may provide a deterministic start-based fallback
inside a diagnostic result, but it must not present the fallback as successful
interpretation.

Bindings are parent-relative in this RFD. Passing a constraint through a
derived parent to a more distant ancestor is an unresolved extension, not
current semantics.

### Derived-node placement

A derived binding places the node's **origin**, never its union box. For a
definite parent extent:

| Binding | Derived origin |
| --- | --- |
| Start pin | `o` |
| Center pin | `P / 2 + o` |
| End pin | `P - o` |

The readable union-derived sizing box is the placed origin plus the local
child union. Moving one child may change that reported box, but it does not
rewrite the derived node's origin. In free positioning this also preserves
sibling positions; a parent layout may legitimately reflow its other children
when the derived box changes contribution.

Span is inapplicable to a derived box because the children, not the binding,
own its extent.

## Size intent

There are two size intents:

- **Fixed** — an authored, non-negative extent; and
- **Auto** — an extent supplied by the node's measured, derived, or
  container-hug rule.

There is no stored **Fill** size. Filling behavior belongs to one of three
places:

- a span, which owns the free-positioned axis;
- layout growth on the main axis; or
- layout stretch on the cross axis.

This keeps size state independent from the context that may impose a larger
resolved box.

An automatic extent on a kind with no natural, derived, or hug source is an
error by rule. Negative fixed extents are rejected at the authoring boundary.
Crossing zero during a resize is therefore a gesture policy over non-negative
extents and flip intent, not permission to store a negative size.

When aspect and min/max constraints apply, resolution follows one declared
order:

1. span extent, if any;
2. otherwise fixed or natural extent;
3. aspect derivation only for an otherwise unresolved axis; and
4. min/max clamping, with the minimum winning an inconsistent
   `minimum > maximum` pair.

For a free-positioned child, those steps produce the resolved extent. For an
in-flow child, they produce the basis supplied to layout. A layout profile
must separately state how its imposed growth or stretch interacts with the
same constraints; that interaction cannot be inferred from this pre-layout
order.

Measured content that depends on a constrained axis is remeasured after that
axis reaches its final constraint. Content-specific measurement rules remain
with the content's golden specification.

## Container hug

A container kind may declare that Auto derives an axis from its children.
For a free-positioned hug axis:

- the container's local start edge stays fixed;
- the content extent is the greatest positive far edge contributed by an
  active child, or zero when no child contributes;
- near and far padding are added around that content extent;
- children resolve inside the padding-inset content box; and
- child visual transforms and paint do not enlarge the hug result.

A child with a negative start may therefore overflow the container's start
edge rather than shifting the container origin. Because an Auto hug axis has
no definite extent until its children are measured, a free child on that axis
must use a Start pin. Center, End, and Span are errors by rule there; allowing
them would create a cycle.

The [Flex Layout Profile](./flex.md) owns automatic container extent when the
container participates as flex rather than free positioning.

## Layout participation

A child participates either as:

- **free-positioned**, where its bindings place it in the parent's local
  space; or
- **in flow**, where the parent's layout algorithm owns its slot and
  position.

When layout owns an axis, authored free-position bindings on that axis are
retained intent but are not effective inputs to the current resolution. A
read reports the materialized box. A write that cannot honestly retarget the
active layout intent is rejected or must explicitly change participation; it
must not silently overwrite an ineffective coordinate.

The sizing box is the only negotiation surface between a child and parent
layout. Content realization, visual transforms, and paint never alter a
layout contribution. Growth and stretch may change the resolved sizing box;
they do not introduce a second stored size mode.

The [Flex Layout Profile](./flex.md) owns the bounded flow algorithm and its
Chromium-grounded conformance. Grid remains a separate extension. Neither
existing Grida engine is an oracle.

## Visual transforms and the two read tiers

Rotation and flips are post-layout visual transforms.

The **sizing tier** reads untransformed boxes:

- free positioning;
- flex contributions and slots;
- hug measurement; and
- derived unions.

The **read tier** applies transforms and downstream inflation:

- transformed bounds;
- spatial queries and hit testing;
- transformed geometry consumed by selection systems; and
- painting.

Consequently, a rotated child may visually overlap a sibling or escape a
hugging parent. That is the chosen visual-only transform contract, not a
layout failure. Callers that need containment compare visual bounds, including
applicable content and paint inflation, not the sizing box.

Ordinary boxed and measured nodes rotate and flip about the center of their
sizing box. Derived nodes rotate and flip about their stored origin. A local
mirror is applied before rotation; placement composes outside both
(`T · R · F`).

A rotation read returns the effective structured scalar used by resolution;
it is not recovered by decomposing a resolved matrix. Source access remains a
separate read of authored intent. A conforming rotation-write boundary
canonicalizes negative zero to positive zero so two spellings of the same
rotation do not survive as distinct intent.

Ordinary nodes expose structured rotation plus two flip booleans. They do not
store a general affine matrix. Arbitrary ordered transforms belong to an
explicit transform-quarantine node whose box is derived before those
operations. Its scene-model name and several edge semantics remain open.
The n0 XML RFD owns `<lens>` as a source spelling; that spelling does not
settle the generic scene-model name.

Resize and scale remain three different operations at this boundary:

- ordinary resize changes the sizing box and keeps absolute style parameters
  stable;
- [parameter scale](../canvas/parametric-scaling.md) is an explicit authored
  bake over geometry and style parameters; and
- retained picture scale is a post-resolution transform in the quarantine.

## Union-derived boxes and groups

A union-derived sizing box is the union of active children's
**untransformed sizing boxes at their pins**, expressed in the derived node's
local coordinate space. Child rotation and flips do not enter this union.
Hidden children do not contribute. An empty union-derived node has a
zero-size union at its stored origin.

A group is the minimal derived kind:

- it is a named set with a coordinate space;
- it owns no paint, effects, or inherited style;
- it imposes no child layout;
- it stores an origin rather than a compensating fitted rectangle; and
- uniform node properties do not change its box-source semantics.

Editing a child changes the materialized union. It does not require a write to
the group. A group-resize gesture may deliberately fan out writes, but that is
an editor operation and remains outside this box-source rule.

Boolean-result bounds, organizational trays, and other derived or
container-like kinds require explicit adoption rows. They must not be inferred
from group behavior.

## Resolution

Resolution is one pure projection:

```text
document
+ effective values
+ declared environment
    -> resolved document + diagnostics
```

It has four semantic phases:

1. **Measure** — obtain natural sizes and immutable content artifacts.
2. **Layout** — resolve sizing boxes and parent-relative placement.
3. **Transform** — compose local and world visual transforms.
4. **Bounds** — derive transformed visual bounds, including applicable
   content and paint inflation, for downstream reads.

The phases are an ordering contract, not a mandate for a particular internal
data structure or layout library. Resolution is unquantized. Raster
quantization and display-scale policy belong downstream.

The environment is explicit. A result obtained with a different viewport,
font manifest, resource set, or oracle version is a different resolution, not
another view of the old one.

## Reads and writes

Reads materialize:

- `x`, `y`, `width`, and `height` report the resolved sizing box;
- transformed bounds are a separate read;
- mapped geometry reports real coordinates rather than storage-normalized
  coordinates; and
- layout-owned values report their effective result.

Writes target authored intent. Every write is in one declared regime:

- **retargeted** — update the active intent so the requested materialized
  value becomes effective, and report what was retargeted; or
- **rejected** — return a typed reason because no single honest intent update
  exists.

Examples of rejection include writing an axis owned by span, writing a
position owned by layout without changing participation, writing a declared
size on a derived kind, or introducing a non-finite or negative extent.

A rejected operation must leave authored state unchanged. This is a semantic
requirement for the operation as a whole, not merely for each field setter.
Compound editor operations and journal ownership remain with
[Canvas](../canvas/index.md) and [History](../feat-history/index.md).

## Conformance contracts

| ID | Contract | Gate |
| --- | --- | --- |
| **ANCHOR-1** | Authored state contains intent; ordinary resolution never writes derived geometry back. | Resolve the same authored document under two environments and verify that only resolved output changes. |
| **ANCHOR-2** | Every node kind declares one box-source rule per axis and one content realization; each resolved axis has one effective extent owner. | Axis-aware kind-by-mechanism census; no silent cell. |
| **ANCHOR-3** | Parent-relative pin and span arithmetic follows the axis tables. | Analytic geometry cases, including zero parent extent. |
| **ANCHOR-4** | Span owns its axis; negative span extent clamps to zero with a diagnostic. | Geometry and diagnostic assertions. |
| **ANCHOR-5** | Size is Fixed or Auto; filling is expressed by span or layout participation. | Applicability matrix and round-trip intent checks. |
| **ANCHOR-6** | The sizing box is layout's only child negotiation surface. | Layout cases compared with the applicable Chromium behavior. |
| **ANCHOR-7** | Rotation and flips never affect sizing, hug, flow contribution, or derived union. | Paired zero/nonzero-transform geometry cases. |
| **ANCHOR-8** | Visual bounds and spatial queries include structured transforms and every applicable content, stroke, and effect inflation. | Independent transformed-geometry, paint-inflation, and hit-test probes. |
| **ANCHOR-9** | Derived bindings place the origin; child edits do not rewrite the parent, and free-positioned siblings remain stable. Parent layout may reflow from the changed contribution. | Document-diff and free-context world-stability cases. |
| **ANCHOR-10** | Union-derived boxes use active children's untransformed sizing boxes; empty is zero at origin. | Nested, hidden, empty, rotated, and flipped cases. |
| **ANCHOR-11** | Resolution follows measure → layout → transform → bounds from explicit inputs. | Phase-boundary and repeat-resolution cases. |
| **ANCHOR-12** | Reads materialize; writes retarget or reject explicitly. Rejection is atomic. | Read/write matrix plus whole-operation unchanged-state checks. |
| **ANCHOR-13** | An import or export mapping preserves every applicable authored intent or returns an explicit degradation or unsupported result. | Mapping cases evidenced against the source standard or another independent oracle. |

## Unresolved model semantics

The graduation draft deliberately leaves these box-model questions open:

- the numeric coordinate budget and resolved-overflow policy;
- boolean box source and operand semantics;
- the authored write contract, reference rectangle, and zero-extent behavior
  for mapped vector geometry;
- the box source and root behavior of organizational trays;
- constraint pass-through across derived parents;
- the scene-model name, operation origin, singular-transform behavior, and
  read surface of the transform quarantine;
- layout-imposed size on derived boxes;
- min/max applicability on a spanned axis outside n0 XML; and
- percent bindings, arbitrary-node anchors, and grid.

An unresolved row is not license for a silent default. A consumer must reject,
quarantine, or retain the source intent until the semantics are adopted with
their conformance gate.

## Evidence and precedence

The workbench is preserved as evidence:

- [consolidated workbench statement](../../../archive/model-v2/anchor/MODEL.md);
- [DEC-0 visual-only transform ruling](../../../archive/model-v2/anchor/dec0-visual-only.md);
- [experiment report](../../../archive/model-v2/anchor/REPORT.md); and
- [earlier phase-2 draft](../../../archive/model-v2/models/anchor.md).

The DEC-0 ruling controls where those papers disagree about
rotation-in-flow or derived unions. Layout-visible rotation remains a measured
alternative, not the current model. Experiment metrics, implementation
bindings, schema sketches, and peer tables remain archive evidence; they are
not normative clauses here.

The workbench also contradicts itself about constraint pass-through across a
derived parent. The dedicated group paper labels E-A13 a proposal, while
later summaries list it among settled amendments. It has neither a completed
conformance gate nor a proving implementation. This graduation draft
therefore preserves direct-parent bindings and records pass-through as
unresolved rather than silently promoting it.

This RFD carries the proven behavioral core and names its unproven surface.
Its status remains open until the owner gate.
