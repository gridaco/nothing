---
title: Paint Vocabulary Conformance Gap Report
description: "Phase 1 evidence: the ratified paint-model laws projected independently over cg and n0-model, with every non-equivalence named for D-C."
tags:
  - internal
  - wg
  - program
  - consolidation
  - painting
format: md
---

# Paint Vocabulary Conformance Gap Report

**Status:** Evidence complete, 2026-07-20. Decision **D-C is not taken**.
The two **AMD** amendments are drafted and re-pinned, not ratified. The
program owner remains the decision-maker for both gates.

This report is for the owner deciding D-C: whether each n0-model paint leaf
adopts a shared type or remains a separate type behind a permanent
law-equivalence mapping. It records what the
[ratified paint-model RFD](../feat-painting/paint-model.md) requires, what the
two current vocabularies can express, and what neither vocabulary proves.
Neither implementation is the oracle.

## Evidence boundary

The executable evidence is the test-local `PaintVocabulary` trait in
[`paint_rfd_conformance.rs`](../../../crates/n0-model/tests/paint_rfd_conformance.rs).
Each binding is checked independently against RFD observations. Native enum
matches and struct destructures are exhaustive, so a variant or field-set
change is compile-visible. Default and non-default sentinels exercise value
preservation. The suite is Skia-free and deterministic; it is a value-law
gate, not a pixel reftest.

Four evidence classes keep a green test honest:

| Class | Meaning |
|---|---|
| **Observed** | The native value can express the law and the harness executes it. |
| **Mapped** | Representations differ, but a lossless projection executes the same law. D-C still owns whether the mapping survives. |
| **Gap** | A required value or grouping is absent or has the wrong shape. It has a stable id below. |
| **External gate** | A leaf-value test cannot prove the claim. Importer behavior and pixels stay with their independent gates. |

No fixture file is appropriate for this suite: the subjects are finite value
vocabularies and algebraic laws. Renderer behavior remains subject to the
program's Chromium/consensus oracle discipline; this report never substitutes
legacy output for an oracle.

## Clause census

| RFD clause | cg | n0-model | Evidence or disposition |
|---|---|---|---|
| RGBA8, straight alpha | Mapped | Observed | Sentinel AARRGGBB words project to the same four channels; solid opacity is alpha. |
| Packed AARRGGBB canonical identity | Gap | Observed | cg stores four channel fields; the projection is bijective but storage is not canonical. `P1-CG-COLOR-CANONICAL`. |
| Byte → unit float → rounded byte identity | Observed | Observed | All 256 alpha values round-trip. The RFD now states recoverability rather than the false claim that every quotient is exactly representable in binary. |
| Transfer characteristic declared | External gate | External gate | The RFD's linear-vs-display contradiction remains a color-management obligation; neither RGBA8 type carries a declaration. |
| Six paint variants | Observed | Observed | Solid, linear, radial, sweep, diamond, image; exhaustive native matches. |
| Per-paint active, opacity, blend | Observed | Observed | All six variants retain the state. All sixteen native blend variants are projected one-way, then every normalized blend is independently constructed and read back. |
| Visible iff active and opacity > 0 | Observed | Observed | Patrol found both implementations used “nonzero”; both helpers now implement the literal law, including negative and NaN probes. Pixel equivalence after removal remains an external render gate. |
| Ordered stack, entry zero bottommost | Observed | Observed | Native construction order is retained and `push` appends the topmost entry. Empty remains representable. Actual draw sequence is a renderer gate. |
| Blend locality | Structural | Structural | Blend stays on each paint entry. Actual compositing is a renderer/pixel gate. |
| Node opacity is isolated, not distributed | External gate | External gate | Node opacity is deliberately outside these leaves. Isolation and valid opacity-folding optimizations belong to the stroke-fill-opacity render gate. |
| Unified, repeatable stroke application | Gap | Observed | Ordered native projections preserve every geometry enum and width payload; n0-model's native default supplies the miter default, one application retains a dash payload, and a node stores two distinct applications. cg exposes the primitives, including its dash wrapper, but no grouped application. `P1-CG-STROKE-APPLICATION`. |
| Stroke applications paint after children | External gate | External gate | List shape cannot prove node render order; the display-list/render gates own it. |
| Legacy flattened stroke maps to one application | External gate | Expressible | The target value is sufficient; the converter must prove the one-element, lossless mapping when it lands. |
| Run-fill absent / empty / nonempty | Observed | Observed | Both retain all three states. Owner ratification remains `AMD-RUN-FILL-TRISTATE`. |
| Decoration color value | Observed | Gap | cg carries the optional color; n0-model's paint-only text surface does not. Effective-text-color fallback remains a consumer gate. `P1-N0-DECORATION-COLOR`. |
| Text strokes reuse stroke applications | Gap | Gap | cg has a partial text-specific bundle; n0-model has no run-level stroke application. `P1-CG-TEXT-STROKE`, `P1-N0-TEXT-STROKE`. |
| Gradient field matrix | Observed | Observed | Exact native destructures, defaults, and non-default sentinels cover endpoints, tile-mode presence, all six transform coefficients, stop values, opacity, active, and blend. All four tile modes are present where allowed. |
| Gradient intrinsic sampling and transform composition | External gate | External gate | Field presence cannot prove the normalized-space intrinsics or `scale(width, height) × user-transform`; analytic/pixel gates own them. |
| Stop is offset + color | Observed | Observed | No parallel stop-opacity field exists. cg's helpers are gated for empty, one-, two-, and three-color ramps, including exact retained order and offsets. |
| Image field matrix | Mapped | Observed | Ordered native projections preserve both resource identities and payloads, all three fit forms (including transform/tile payloads), all four object fits, and all three repeat modes. Exact neutral and non-default image witnesses cover quarter turns, alignment, opacity, blend, and all seven filter scalars. |
| Image orientation, fitting, tiling, and filter execution | External gate | External gate | Modulo-four orientation, odd-turn intrinsic swaps, fitting/tiling order, filter ranges, and pixel effects require renderer gates. |
| Diamond beyond-unit behavior | Owner amendment | Owner amendment | Both renderers clamp today; the contract remains proposed as `AMD-DIAMOND-CLAMP`. |
| Stop-opacity folding, focal reporting, pattern rejection | External gate | External gate | These are importer obligations, not properties of the leaf types. No importer conformance is claimed here. |

“Structural” is deliberately weaker than pixel conformance. It says the value
does not erase the state required to perform the operation; only the proper
render gate can establish the operation's result.

## Law-equivalent representation differences

These differences do not currently change a harness observation, but they are
the concrete cost surface for D-C.

| Leaf | cg | n0-model | Consequence |
|---|---|---|---|
| Color | Four `u8` channel fields; both RGBA and ARGB constructors | Packed AARRGGBB word | A test mapping is lossless. Adopting cg unchanged would contradict the RFD's canonical identity. |
| Paint stack | Private `Vec<Paint>` with serde | Private `Vec<Paint>` without serde | Order and mutation laws match; serialization policy is a boundary concern. |
| Gradient transform | math2 row-major 2×3 affine over `f32` | n0-model-local named affine coefficients over `f32` | All six coefficients map exactly; type identity and dependency direction remain distinct. |
| Run fills | `Option<Vec<Paint>>` | `Option<Paints>` | The three ownership states are identical; wrapper identity differs. |
| Image alignment field | Public field spelled `alignement` | Public field spelled `alignment` | Spelling is API debt, not a semantic difference. |
| Resource reference | Uppercase `HASH` / `RID` variants | Rust-style `Hash` / `Rid` variants | The two-way value set is identical. |

## Exact gaps

### P1-CG-COLOR-CANONICAL

cg's channel struct is losslessly related to AARRGGBB, but the RFD chose a
packed word as canonical identity and interchange form. Equality is presently
law-equivalent, not representation-identical. D-C must either preserve a
mapping deliberately or select a shared packed leaf; the report does not take
that decision.

### P1-CG-STROKE-APPLICATION

cg owns `StrokeWidth`, alignment, cap, join, miter-limit, dash, and paint-stack
values, but it has no value grouping one geometry with one paint stack and no
repeatable application list. The legacy node estate flattens one geometry and
one stack. The RFD requires the grouped, repeatable value already present in
n0-model.

### P1-CG-TEXT-STROKE

cg's styled run carries stroke paints plus optional width and alignment. It
does not carry cap, join, miter limit, or dash as one stroke application, and
therefore creates a partial text-specific stroke shape which the RFD forbids.
This partial coverage is retained as evidence; it must not disappear when the
text surface is consolidated.

### P1-N0-DECORATION-COLOR

n0-model's current `TextStyleRec` is deliberately a three-field shaping
subset. It has no decoration color even though the paint RFD assigns that
paint-only value to this vocabulary. The broader decoration geometry remains
owned by the text-layout contract; this gap is only the color value and its
fallback to the run's effective text color.

### P1-N0-TEXT-STROKE

n0-model can stroke a whole text node through `Node::strokes`, but a styled run
cannot own a stroke application. The paint RFD's text partition therefore is
not complete. This gap and the cg partial shape mean that choosing either text
record wholesale cannot close the contract.

## Declared external obligations

The following findings are real but are not D-C type-choice evidence:

- **Color transfer:** the archive's “linear” annotation and both renderers'
  display-encoded treatment remain contradictory. A later color-management
  contract must name the transfer characteristic.
- **Radial focal point:** neither type carries one. The RFD proposes the
  extension but explicitly requires declared importer deviation until it is
  ratified. The earlier sentence calling it ratified was a document defect and
  is corrected with this report.
- **Importer losses:** per-stop opacity must fold into stop alpha; a focal point
  and unsupported spread/pattern constructs must be reported, never silently
  dropped or replaced with transparency. Importer conformance belongs to the
  import-IR and scoreboard gates.
- **Quarter-turn execution and paint-box transforms:** the fields exist on both
  sides, but modulo-four orientation and gradient sampling are renderer
  behavior. Existing implementation agreement is evidence, not an oracle.
- **Stack and node compositing:** pixel-invariant removal of invisible paints,
  blend locality, first-entry-first drawing, stroke-after-children order, and
  node-opacity isolation require render/display-list gates. The value suite
  proves only that the state and canonical order survive to those gates.
- **Text consumers:** decoration-color fallback and the rule that ownership is
  resolved before invisible-paint filtering require targeted consumer tests.
  The value suite proves only the present/empty/absent states and exact record
  shapes.

## D-C decision surface

D-C remains an owner gate. The evidence supports leaf-by-leaf decisions; it
does not support one wholesale type swap.

| Leaf group | If one shared type is selected | If separate types remain |
|---|---|---|
| Color | The shared leaf must preserve packed AARRGGBB canonical identity. | The checked AARRGGBB projection becomes a permanent law gate. |
| Blend, tile, alignment, paint stack | Current value sets and laws are equal; dependency and serialization policy still need an explicit home. | Exhaustive mappings remain small but permanent. |
| Gradient and image paints | Transform type, math dependency, serde policy, and the `alignement` API debt must be settled without changing field laws. | The full field-matrix mapping remains a permanent gate. |
| Stroke application | A shared surface must express n0-model's grouped, repeatable application without flattening it. | cg remains nonconformant until its consumer boundary maps the legacy singleton into the canonical application. |
| Text paint partition | Neither record is complete; decoration color and full run stroke applications must survive whichever ownership choice is made. | Both named gaps remain capability work; mapping alone cannot create absent values. |

No production adapter lands with this report. The test-local bindings are
evidence for D-C. If D-C chooses shared types, a binding is deleted only after
the same RFD assertions run directly on the selected type. If D-C chooses
separate types, the corresponding binding becomes the permanent
law-equivalence gate. This is the deletion gate for the only mapping introduced
by this step.

## Patrol and captured-essence ledger

| Patrolled estate | Captured essence | Disposition |
|---|---|---|
| cg color, paint variants, gradient/image leaves, ordered stack | Straight RGBA8, six variants, per-entry state, bottom-to-top order, field matrices | Executed in the trait harness; no drop. |
| cg gradient color helpers | Even stop distribution and retained input order | Singleton NaN was a contract violation; corrected to a finite offset of zero; empty through three-color cases are gated. |
| Legacy flattened stroke vocabulary | One geometry plus one ordered stack; rich geometry primitives | Preserved as `P1-CG-STROKE-APPLICATION`, including the future lossless singleton mapping. |
| cg styled-run paint fields | Tri-state fills, decoration color, partial stroke bundle | Tri-state and color executed; partial stroke shape preserved as `P1-CG-TEXT-STROKE`. |
| n0-model paint leaves and stack | Packed color, same six variants and order | Executed independently; no cg-as-oracle comparison. |
| n0-model stroke application | Grouped geometry + paints; repeatable node list | Executed as the RFD-conformant value shape. |
| n0-model styled-run paint fields | Tri-state fills; sparse paint-only coverage | Tri-state executed; absent decoration and run strokes named separately. |
| Both visibility helpers | Active plus nonzero-opacity test | Etiology traced to a duplicated predicate; corrected to the literal `opacity > 0` law and gated for zero, negative, NaN, and positive values. |
| Both diamond render paths and Draft-0 n0 XML | L1 ramp with clamp at the unit boundary | Re-homed as evidence for proposed `AMD-DIAMOND-CLAMP`; no ratification claimed. |
| Importer deviations named by the RFD | Stop opacity, focal point, spread methods, patterns | Quarantined as external gates; no silent claim that a type test covers them. |

This step deletes and replaces no engine estate. The ledger nevertheless
records every caveat encountered so D-C cannot turn representation cleanup into
a silent capability drop.

## Gate

The report is current only while all of these hold:

```sh
cargo test --locked -p n0-model --test paint_rfd_conformance
cargo test --locked -p cg -p n0-model
cargo test --locked -p grida -p n0
cargo check --locked -p grida -p grida-canvas-wasm -p grida_dev -p n0 -p n0-model
cargo clippy --locked -p cg -p n0-model --no-deps -- -D warnings
cargo fmt --all -- --check
```

The first command requires every stable gap and pinned amendment id to occur in
this report. It does not produce or inspect a scoreboard score.
