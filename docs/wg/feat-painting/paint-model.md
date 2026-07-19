---
title: The Paint Model
description: "Open RFD for the shared paint vocabulary — color, stroke, ordered paints, the text-style partition, and canonical gradient/image field sets — that both engines adopt at promotion."
tags:
  - internal
  - wg
  - canvas
  - painting
  - rendering
format: md
---

# The Paint Model

**Status:** Ratified — accepted via
[gridaco/nothing#33](https://github.com/gridaco/nothing/issues/33)
(closed by the owner, 2026-07-18). Two items flagged in the body remain
pinned as follow-up amendments: diamond-gradient extension behavior and
the tri-state run-fill verification.

This document is written for an engine developer deciding, at promotion
time, what the shared paint vocabulary is — the leaf-level value types
that a scene hands to a renderer. Two engines exist today: the production
engine and the v2 proof. Their paint vocabularies have the identical
variant set — solid, linear gradient, radial gradient, sweep gradient,
diamond gradient, image — but differ in representation at five points.
This RFD argues each of the five to a recommendation. It settles
vocabulary, not code: both implementations are cited as evidence only,
never as the norm. Ratification happens on the originating issue
([gridaco/nothing#33](https://github.com/gridaco/nothing/issues/33),
under the seam program
[gridaco/nothing#27](https://github.com/gridaco/nothing/issues/27)); the
authored-contract constraints it inherits are locked by
[gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9).

The companion study
[Display-List Contract Study](../feat-2d/display-list-contract.md)
concludes that the shared surface between the two engines stops at this
leaf vocabulary — the display list each engine compiles it into is
per-engine and deliberately unspecified. This RFD is therefore the whole
shared surface.

## Scope

In scope: the value vocabulary of painting — color, the paint variants,
the ordered paint stack, the stroke application, gradient and image
field sets, and the paint-only half of text style.

Out of scope, each with its owning home:

- **Compositing of node opacity against fill and stroke** — owned by
  [Stroke-Fill Opacity Compositing](../feat-2d/stroke-fill-opacity.md).
- **Layout-affecting text style** — owned by the
  [Universal Shaped Text Layout RFD](../feat-paragraph/text-layout.md)
  (decision 4 below states the boundary).
- **Per-side rectangular stroke geometry** — owned by
  [Rectangular Stroke Model](./stroke-rect.md).
- **Endpoint markers and curve decoration** — owned by
  [Curve Decoration](../feat-2d/curve-decoration.md).
- **Node-level blend and isolation** — compositing model, not paint
  vocabulary; see [Isolation Mode](../feat-2d/isolation-mode.md) and the
  stroke-fill opacity spec above.
- **Wide-gamut color management** — named as the successor to decision 1,
  deferred with its blocker stated there.

## Vocabulary

| Term                   | Meaning                                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **Paint**              | A self-contained recipe for producing color over a region: one of solid, linear, radial, sweep, diamond gradient, or image.          |
| **Paint stack**        | An ordered, finite list of paints composited in sequence; entry zero is bottommost.                                                  |
| **Stroke application** | One stroke geometry — width, align, cap, join, miter limit, dash pattern — carrying its own paint stack. Repeatable per node.        |
| **Unit gradient space**| The `[0,1] × [0,1]` box in which radial, sweep, and diamond gradients are defined, with implicit center `(0.5, 0.5)`.               |
| **Alignment point**    | A point in center-based normalized coordinates over the paint target box: `(-1,-1)` top-left, `(0,0)` center, `(1,1)` bottom-right. |
| **Stop**               | A pair of a scalar offset in `[0,1]` and a color.                                                                                    |
| **Quantization policy**| The declared rule a boundary applies when converting a unit-interval scalar channel to an 8-bit channel.                             |
| **Paint-only value**   | A value whose change cannot alter resolved geometry — only which ink fills already-resolved geometry.                                |

---

## Decision 1 — Color representation

### The question

Is the canonical color a packed 32-bit word, a struct of four 8-bit
channels, or four float channels with a color-space tag? And what are
the conversion laws between the forms that exist at the boundaries?

### Evidence

Three representations are in production use today:

1. The archive format stores color as **four 32-bit floats** in `[0,1]`
   (annotated in the schema as linear space).
2. The production engine's runtime value is a **struct of four 8-bit
   channels** (r, g, b, a), constructed from packed words in *both*
   `RRGGBBAA` and `AARRGGBB` orders depending on the constructor used.
3. The v2 proof's value is a **packed 32-bit word** in `AARRGGBB` order,
   with hex strings treated strictly as parse inputs and serialize
   outputs — never stored.

Two facts anchor the analysis:

- The archive decode boundary converts float channels to bytes by
  **round-to-nearest**; the HTML import boundary converts by
  **truncation**. The same authored channel value can therefore arrive
  in the engine as two different bytes depending on which door it came
  through — off by one, which is enough to break value equality, cache
  keys, and cross-engine pixel identity.
- Both runtimes agree that a **solid paint's opacity is its color's
  alpha channel** — deliberately not a second scalar field. Alpha is
  part of the color value, stored straight (unpremultiplied).

### Deciding table

| Candidate                          | Equality & hashing                        | Interchange form                    | Wide-gamut headroom | Where quantization lives                              |
| ---------------------------------- | ----------------------------------------- | ----------------------------------- | ------------------- | ----------------------------------------------------- |
| Packed 32-bit RGBA8 word           | Bitwise; one integer compare              | One number, one declared byte order | None                | Confined to authoring/import boundaries               |
| Four-byte channel struct           | Field-wise, equal to bitwise              | Four named fields                   | None                | Confined to authoring/import boundaries               |
| Four floats + color-space tag      | Epsilon or bit-pattern; NaN is a hazard   | Four floats plus a tag              | Yes                 | Moves inside the engine, paid at raster time          |

The first two rows are informationally identical: the packed word and
the channel struct are related by a bijection (byte placement, no
arithmetic), so choosing between them is not a semantic decision — it is
a choice of canonical identity and interchange form. The real semantic
decision is 8-bit quantized channels versus float channels, and on that
both engines already agree: the working value is RGBA8. The float form
appears only at the archive boundary, as an encoding of an RGBA8 value —
values off the 1/255 grid are not losslessly authorable today, so the
float spelling grants no extra gamut in practice.

### Recommendation

1. **The canonical color value is one RGBA8 quadruple with straight
   (unpremultiplied) alpha.** A solid paint's opacity *is* its alpha;
   no second opacity field exists on solid paints.
2. **The canonical identity and interchange form is the packed 32-bit
   word, in exactly one declared byte order.** Channel access is a
   projection of the word, not a second storage form. The vocabulary
   must name the order explicitly (most-significant byte first:
   `A, R, G, B` is the order the current packed implementation uses);
   the evidence shows two packed orders coexisting behind constructor
   names today, which is precisely the ambiguity a single declared
   order removes.
3. **The lossless conversion laws.**
   - *Word ↔ channel struct*: bijective byte placement; lossless in
     both directions.
   - *Byte → unit float*: `f = n / 255`; every byte value is exactly
     representable; lossless.
   - *Unit float → byte*: `n = round(clamp(f, 0, 1) × 255)`,
     round-to-nearest. Under this rule the byte → float → byte round
     trip is the identity. Any float off the 1/255 grid loses
     information — that loss is **quantization, and quantization is
     caller policy**: it is a property of the converting boundary, not
     of the color type, because the value carries no memory of how it
     was produced. Every boundary that accepts arbitrary floats must
     declare its rule. The vocabulary makes round-to-nearest the
     conformance default; the observed truncating boundary is, under
     this RFD, a deviation to be corrected at promotion.
4. **The transfer characteristic must be declared, and today it is
   contradicted.** The archive annotates its float channels as linear;
   both runtimes hand the same 8-bit channels to the rasterizer without
   a conversion step, which treats them as display-encoded. Both
   statements cannot be true. This RFD does not resolve color
   management; it flags the contradiction as a must-resolve conformance
   clause and defers the wide-gamut canonical form (float channels plus
   a color-space tag — the successor row of the table) to a dedicated
   color-management effort, which is its named blocker.

---

## Decision 2 — The stroke model

### The question

Is a stroke a *geometry carrying ordered paints*, or a *paint carrying
stroke parameters*?

### Constraint inherited

[gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9) locks
the authored contract: `stroke` is **repeatable**; each occurrence is an
independent geometry with its own ordered paint stack. Repeated strokes
paint in source order after the node's children; within one stroke the
paints composite bottom-to-top. The vocabulary must be able to express
what the format promises.

### Evidence

- The production engine's node records carry **one** stroke geometry —
  width, align, cap, join, miter limit, dash pattern, flattened onto the
  node — plus one ordered paint stack for strokes. Two strokes of
  different widths on one node are not expressible.
- The v2 proof carries a unified **stroke application**: a value
  bundling the six geometry properties with its own paint stack, held
  as a repeatable list per node. It is the element type of the
  repeatable authored `stroke`.
- Both sides use the *same* paint type for fills and for stroke paints;
  no paint variant carries stroke parameters anywhere in either engine.

### Deciding table

| Question                                                    | A — geometry carries ordered paints           | B — paint carries stroke parameters                                  | C — node carries one geometry + one stack (status quo)   |
| ----------------------------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------- |
| Expresses the authored repeatable-stroke contract           | Yes, structurally — one value per occurrence  | Only by repeating identical geometry on every paint; the grouping invariant is conventional, not structural | No — a single geometry cannot carry two widths           |
| Paint type stays reusable between fill and stroke           | Yes — paints stay geometry-free               | No — stroke fields are dead knobs in fill position                    | Yes                                                      |
| Multiple paints under one geometry                          | The paint stack, verbatim                     | Geometry duplicated per paint; equality of geometry becomes accidental | The paint stack, verbatim                                |
| Legacy corpus maps losslessly                               | Yes — as exactly one stroke application       | Yes, with n-fold geometry duplication                                 | Identity                                                 |

### Recommendation

**A.** The stroke is a geometry that carries an ordered paint stack —
the *stroke application* of the vocabulary table. Paints stay
geometry-free and identical between fill and stroke positions.

The stroke application's field set:

| Field        | Type                                                          | Notes                                                                                                     |
| ------------ | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| paints       | Paint stack                                                   | Ordered; decision 3 semantics                                                                             |
| width        | One of: none · uniform scalar · per-side rectangular widths   | The rectangular form is valid only for rectangular outlines; geometry owned by [stroke-rect](./stroke-rect.md) |
| align        | inside · center · outside                                     | Identical three-value enum on both sides                                                                   |
| cap          | butt · round · square                                         | Open paths only                                                                                            |
| join         | miter · round · bevel                                         |                                                                                                            |
| miter limit  | Scalar, default 4.0                                           | The cross-standard default (SVG, Canvas, PDF, and the raster backends)                                     |
| dash pattern | Optional list of scalars                                      | Dash phase is a tracked, currently-absent capability ([gridaco/nothing#15](https://github.com/gridaco/nothing/issues/15)) |

**Ordering laws.** Stroke applications paint in list order. Within one
application, paints composite bottom-to-top per decision 3. Strokes
paint after the node's children (the authored contract of
gridaco/nothing#9).

**Promotion law.** A legacy node record — one flattened stroke geometry
plus one stroke paint stack — maps to a list of exactly one stroke
application. The mapping is lossless and mechanical; no legacy document
can express anything the new form cannot.

**Named deferrals.** Variable-width stroke profiles (vector-network
strokes) and endpoint markers are node-capability extensions outside
this vocabulary; markers are owned by
[Curve Decoration](../feat-2d/curve-decoration.md).

---

## Decision 3 — Ordered paint-stack semantics

### The question

What exactly does an ordered paint stack mean? The production engine's
semantics are declared the source of truth by
[gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9); this
decision pins them in domain terms so a second implementer needs no
source access.

### The laws

1. **Order.** A paint stack is an ordered finite list. Entry zero is
   painted first and is therefore bottommost; each later entry
   composites over the accumulated result of the entries before it.
   Stored order is canonical engine order. A user interface may
   *display* the list top-first; that is presentation, never storage.
2. **Per-paint state.** Every paint carries: an active flag, an opacity
   in `[0,1]` (for solid paints, the color's alpha — decision 1), and a
   blend mode.
3. **Visibility.** A paint is visible iff it is active and its opacity
   is greater than zero. An invisible paint contributes no pixels
   regardless of blend mode and may be removed from a stack without any
   pixel change. Filtering invisible paints preserves the relative
   order of the survivors.
4. **Blend locality.** A paint's blend mode applies when that paint is
   composited over the accumulated result of the *earlier entries of the
   same stack*. It never retroactively affects paints already drawn,
   and it does not blend against content outside the stack's own
   compositing context.
5. **Node opacity is not distributive.** Node-level opacity is group
   isolation — the node's fill, strokes, and effects are composited at
   full opacity into an isolated group which is then composited at the
   node's opacity. Folding node opacity into per-paint alpha is an
   optimization with validity conditions, and both the model and those
   conditions are owned by
   [Stroke-Fill Opacity Compositing](../feat-2d/stroke-fill-opacity.md);
   this spec only pins that the paint stack itself carries no node
   opacity.
6. **Empty is not absent.** An empty stack means *explicitly no ink*.
   In inheriting contexts (per-run text fills, decision 4) the absent
   stack means *inherit*, and the two states are distinct and must not
   collapse into each other.

### Deciding table

The one representational choice inside these laws is the canonical
order:

| Question                                                      | First-is-bottommost (recommended)     | First-is-topmost                                            |
| ------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------- |
| Matches paint execution (draw order = list order)             | Yes — index equals draw sequence      | No — renderer iterates in reverse                           |
| Matches both engines' storage today                           | Yes                                   | No                                                          |
| Matches the authored fill channel of gridaco/nothing#9        | Yes — first child is bottommost       | No                                                          |
| Matches common editor list displays                           | No — UIs show topmost first           | Yes                                                         |

### Recommendation

Pin the laws above verbatim, with first-is-bottommost canonical order.
The editor-display mismatch is a view concern; storing display order
would make every renderer and every conversion layer pay a reversal to
serve one presentation.

---

## Decision 4 — The text-style partition

### The question

Text style contains both values that change geometry (font, size,
spacing) and values that only change ink (fill paints, decoration
color). Which half does this spec own?

### Constraint inherited

The [Universal Shaped Text Layout RFD](../feat-paragraph/text-layout.md)
owns text resolution. Its contract already draws the line this decision
needs: layout-affecting style participates in the resolution inputs and
the artifact's cache identity; *paint-only values may remain associated
with source ranges but do not change shaping or invalidate layout*.
Nothing may be decided twice, so this spec must not restate any of that
— it may only own what falls on the paint side of the line the
text-layout RFD draws.

### Deciding table

| Option                                                             | One concept, one home                                        | Invalidation semantics                                                       | Duplication risk                                        |
| ------------------------------------------------------------------ | ------------------------------------------------------------ | ---------------------------------------------------------------------------- | ------------------------------------------------------- |
| This spec owns all of text style                                   | Violated — shaping inputs specified outside the text contract | Paint spec would have to restate layout invalidation rules                    | High — two homes for font selection                     |
| The text-layout RFD owns all of text style, including paints       | Violated — a second paint model appears inside text           | Correct for layout, but paint changes would key layout caches unnecessarily   | High — a text-specific paint vocabulary emerges (an explicit non-goal of that RFD) |
| Partition: layout-affecting half there, paint-only half here       | Both homes honest                                            | Layout caches key on layout inputs; painted output keys on paint state separately | None — the boundary is one sentence                     |

### Recommendation

**Partition.** The boundary rule: *a text-style value is
layout-affecting iff changing it can alter the resolved text layout's
geometry; every layout-affecting value belongs to the text-layout RFD;
this spec owns only paint-only values.* Applied to the known property
classes:

| Property class                                                                     | Owner                                                              |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Font selection: family, weight, width, posture, optical sizing, features, variable axes | Text-layout RFD                                              |
| Size, letter spacing, word spacing, line height, text transform, kerning           | Text-layout RFD                                                    |
| Decoration geometry: which line, thickness, style                                  | Text-layout RFD (layout-owned decoration ink)                      |
| Decoration color                                                                   | **This spec** — paint-only; defaults to the run's effective text color |
| Per-run fill stacks and their inheritance                                          | **This spec** (below)                                              |
| Text strokes                                                                       | **This spec** — text strokes are stroke applications (decision 2)  |

The paint-only vocabulary this spec owns is small:

1. **Run fill ownership.** A text node carries a node-level fill stack.
   Each styled run may carry an *optional* fill-stack override with
   tri-state semantics: absent means inherit the node's fills; present
   and empty means explicitly no ink; present and non-empty replaces
   the node fills for that run. This is law 6 of decision 3 applied to
   text, and both engines implement it identically today.
2. **Decoration color** is a paint-only value riding on geometry the
   layout artifact owns.
3. **Text strokes** reuse the stroke application unchanged — no
   text-specific stroke or paint type exists.

The difference in field *coverage* between the two engines' text-style
records (the production record carries the full layout-affecting set;
the v2 proof deliberately carries a three-field subset) lies entirely in
the layout-affecting half, and is therefore the text-layout RFD's
environment-completeness concern, not a paint-model question.

---

## Decision 5 — Gradient and image canonical field sets

### The question

What exactly are the canonical fields of each non-solid paint variant?
The point of pinning them is that import losses become **conformance
statements** — declared, testable deviations — instead of silent drops.

### Evidence

- All three surfaces (production runtime, v2 proof, archive schema)
  agree on the field sets below with two known pressure points:
  per-stop opacity and the radial focal point.
- The SVG import path today drops a source gradient's per-stop opacity
  and focal point on the way to the engine model — the focal point
  survives into the import's intermediate representation and is
  discarded at the packing step; both drops are annotated in-source as
  model mismatches and are invisible to the user.
- Source SVG patterns are mapped to a transparent paint — a silent
  erasure; pattern paint servers are a tracked capability
  ([gridaco/nothing#14](https://github.com/gridaco/nothing/issues/14)).

### The canonical field sets

**Common to all six variants:** active flag, opacity, blend mode
(decision 3). For solid paints, opacity is the color alpha (decision 1).

**One parameterization rule for gradients.** Every gradient is defined
in a normalized space over the paint target box and positioned by an
affine transform composed as `scale(width, height) × user-transform`:
the gradient definition itself is resolution-independent, and all
rotation, skew, and offset live in the user transform — never baked
into intrinsic parameters.

| Variant | Intrinsic parameters                                                                 | Stops | Tile mode                     | Transform |
| ------- | ------------------------------------------------------------------------------------ | ----- | ----------------------------- | --------- |
| Linear  | Two endpoints as alignment points (defaults: center-left → center-right)              | Yes   | Yes                           | Yes       |
| Radial  | Implicit center `(0.5, 0.5)`, radius `0.5` in unit gradient space                     | Yes   | Yes                           | Yes       |
| Sweep   | Implicit center `(0.5, 0.5)`; angular domain one full turn, clockwise from 0°         | Yes   | No — the angular domain is closed; a full turn has no exterior to tile | Yes |
| Diamond | The radial field evaluated under the Manhattan distance metric, same unit space       | Yes   | No — no tile mode is carried on any surface today; behavior beyond the unit diamond must be pinned at ratification | Yes |

**Stop:** `(offset ∈ [0,1], color)` — recommended without a separate
opacity field; see the contested point below.

**Image paint:**

| Field         | Type / domain                                                                                   | Notes                                                                                       |
| ------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| image         | Resource reference (by content hash or by logical id)                                            | Resolution is the resource system's concern                                                 |
| quarter turns | Integer modulo 4                                                                                 | Discrete, lossless source-image orientation applied before fitting; odd turns swap intrinsic width/height |
| alignment     | Alignment point                                                                                  | Positions the fitted image in its box                                                       |
| fit           | One of: standard object-fit · explicit affine transform · tile (scale + repeat axes)             | Tiling is a distinct mode, composed pre-repeat, so dead combinations like cover-plus-repeat are unrepresentable |
| opacity       | `[0,1]`                                                                                          |                                                                                             |
| blend mode    | Per decision 3                                                                                   |                                                                                             |
| filters       | Seven scalar adjustments — exposure, contrast, saturation, temperature, tint, highlights, shadows | Each with a declared range and neutral zero; all-zero means no filtering                    |

### Contested point A — per-stop opacity

| Question                                              | Fold into stop-color alpha (recommended)                       | Separate per-stop opacity field                              |
| ----------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------ |
| Do the engines and archive carry it today             | Yes — all three have `(offset, color)` only                    | No — a three-surface addition (schema, both engines)         |
| Information difference                                | Effective alpha quantized to 8 bits at the fold                | Retains sub-8-bit precision of `alpha × opacity`             |
| Authoring compatibility                                | The authored grammar of gridaco/nothing#9 already spells stop opacity separately and folds it at the boundary | Same authoring surface either way        |
| Failure mode today                                    | Import currently *drops* stop opacity entirely — worse than folding | —                                                        |

**Recommendation:** no separate field. Authored or imported per-stop
opacity is folded into the stop color's alpha at the boundary, under the
declared quantization policy of decision 1. **Conformance statement:**
an importer that meets a source per-stop opacity must fold it; dropping
it is nonconformant. The sub-8-bit precision loss is bounded by half a
quantization step and is accepted until the wide-gamut successor of
decision 1 revisits channel depth.

### Contested point B — the radial focal point

A focal (two-point) radial gradient places the 0-offset point away from
the circle's center, producing isolines that are non-concentric circles.
No affine transform of a concentric radial can express this: affine maps
send concentric circles to concentric ellipses, so the focal family is
strictly outside the current parameterization. This is why the import
pipeline can only drop a source focal point today.

| Question                                        | Keep radial concentric-only                                     | Optional focal point in unit gradient space (recommended)         | Full two-point conical (two circles)                       |
| ----------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------- |
| Expressive coverage of the domain               | Loses SVG focal gradients permanently                            | Covers SVG focal semantics (focus + one circle)                   | Covers SVG and more (two radii)                            |
| Backend support                                 | Native everywhere                                                | Native two-point conical exists in the raster backends            | Same                                                       |
| Cost                                            | None                                                             | One optional field, defaulting to center (degenerate = today)     | Two extra parameters with no current producer              |
| Design-tool precedent                           | Matches Figma-style radial                                       | Superset; default behavior unchanged                              | No authoring surface wants the second radius today         |

**Recommendation:** extend the canonical radial with an *optional focal
point*, expressed in unit gradient space and defaulting to the center —
the default is byte-identical to today's behavior. Until the extension
is ratified and lands (a schema change, so promotion-program work — the
seam program forbids schema motion), the **conformance statement** is:
an importer meeting a source focal point must declare the deviation in
its import report; silent dropping is nonconformant.

### Conformance statements (summary)

1. Per-stop opacity: fold into stop alpha; never drop.
2. Radial focal point: represent once ratified; until then, a declared
   deviation, never a silent drop.
3. Source spread/tile methods on sweep or diamond gradients: not
   representable; declared deviation.
4. Source pattern paint servers: not representable
   ([gridaco/nothing#14](https://github.com/gridaco/nothing/issues/14));
   declared deviation, never a silent transparent substitution.
5. Any float-to-byte channel conversion: round-to-nearest unless the
   boundary declares otherwise (decision 1).

---

## What promotion does to legacy types

The seam program ships no type reshaping; this section states the
contract the promotion program implements, in vocabulary terms:

- **Color** — value semantics unchanged (RGBA8, straight alpha). The
  packed interchange order is pinned to the single declared order; the
  truncating import boundary adopts the round-to-nearest conformance
  default; the archive's float spelling remains a boundary encoding of
  the RGBA8 value.
- **Stroke** — every node's flattened single stroke geometry plus
  stroke paint stack becomes a list of exactly one stroke application.
  Lossless; the repeatable form is a strict superset.
- **Paint stack** — no change. The production semantics are the source
  of truth and are now pinned here as decision 3.
- **Text style** — the style record splits along the decision 4
  boundary: layout-affecting values remain with the text-resolution
  contract; run fills, decoration color, and text strokes are governed
  by this spec.
- **Gradients and images** — field sets unchanged except the ratified
  radial focal extension; importers upgrade every silent drop named in
  decision 5 to its conformance behavior.

## Evidence

Non-normative provenance — where the claims above were verified. Both
engines are cited by path in this repository (the v2 proof landed
in-tree with gridaco/nothing#5).

- Production engine (`main`): the paint vocabulary and color value in
  [`crates/grida/src/cg/types.rs`](../../../crates/grida/src/cg/types.rs)
  and [`crates/grida/src/cg/color.rs`](../../../crates/grida/src/cg/color.rs);
  the archive schema [`format/grida.fbs`](../../../format/grida.fbs)
  (float color struct, gradient tables); the rounding archive decode in
  [`crates/grida/src/io/io_grida_fbs.rs`](../../../crates/grida/src/io/io_grida_fbs.rs);
  the truncating conversion and the dropped stop-opacity / focal-point
  imports in [`crates/grida/src/import/html/mod.rs`](../../../crates/grida/src/import/html/mod.rs)
  and [`crates/grida/src/import/svg/from_usvg.rs`](../../../crates/grida/src/import/svg/from_usvg.rs)
  with [`crates/grida/src/import/svg/pack.rs`](../../../crates/grida/src/import/svg/pack.rs).
- v2 engine (see
  [gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)):
  the packed color, paint variants, paint stack, and unified stroke
  application in
  [`crates/n0-model/src/model.rs`](../../../crates/n0-model/src/model.rs)
  (promoted from the `model-v2-anchor` branch's `archive/model-v2/anchor/lab`).
