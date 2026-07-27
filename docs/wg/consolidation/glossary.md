---
title: Program Glossary
description: "The consolidation program's vocabulary — one term per concept, used consistently across sessions and agents. Use these terms; do not mint synonyms."
tags:
  - internal
  - wg
  - program
format: md
---

# Program Glossary

**Genre:** reference. One term per concept. If a session needs a
concept this glossary lacks, add it here in the same change that first
uses it — two names for one thing across sessions is how programs
drift.

## The program

- **the chassis** — the v2 engine family (`crates/n0`,
  `crates/n0-model`, `crates/n0_dev`): the adopted topology. The end
  state is built *on* it. The charter names it concretely: "n0".
- **the know-how** — the legacy family's accumulated substance
  (per-node paint specs, optimizations, import/export stacks,
  paragraph depth, editor vocabulary): adopted *through contracts*,
  never copied wholesale.
- **the legacy engine** — the shipping v1 engine (`crates/grida` and
  satellites). Not a pejorative: it is the executable conformance
  reference until each suite's bar flips. The charter names it
  concretely: "v1".
- **consolidation step** — one unit of program work: a zero-behavior
  move, a capability grant, or a retirement. Runs the
  [method](./method.md) lifecycle.
- **patrol** — the triage pass that must precede any deletion or
  replacement; produces the captured-essence ledger.
- **captured-essence ledger** — the patrol's output: scope inventory,
  caveats with provenance, re-home destinations, and deliberate drops
  with reasons.
- **frozen surface** — a contract the program must not touch until
  Phase 6: the v1 schema (`format/grida.fbs`) and the published wasm
  package's freeze contract.
- **the freeze contract** — the standing obligation that the published
  v1 wasm artifact is never unpublished or deprecated; emergency
  patch releases only.

## Moves and gates

- **zero-behavior move** — a restructuring with provably no observable
  change: crate cuts, re-exports, renames. Gated by the byte-identical
  sweep; never waits for the scoreboard.
- **capability grant** — the chassis gaining an ability it lacked
  (an importer, a node's paint, an export form). Gated by the
  scoreboard; lands measured or not at all.
- **retirement** — deleting legacy capability after its suite's bar
  has flipped; subtraction with a ledger, reversible per-suite.
- **the extraction rule** — a module becomes a shared workspace crate
  when its second consumer appears in the workspace, and not before.
  Stated in the doctrine sections as "migration by extraction" — the
  same rule.
- **byte-identical sweep** — the zero-behavior gate: every shipping-
  surface output byte-equal before and after; no thresholds, no
  similarity scores.
- **contraction** — the legacy engine shrinking by subtraction toward
  its Phase 6 residue (the v1 compat adapters, the legacy paint
  compiler, the v1 format io, and the wasm publisher), then to
  nothing.

## Verification

- **the oracle** — the external truth conformance is graded against:
  Chromium/consensus for web-rendering domains, the versioned text
  oracle for shaping. Never the legacy engine itself.
- **the bar** — which engine a conformance suite currently holds as
  its executable reference. Starts legacy everywhere; flips per-suite.
- **the scoreboard** — the program's instrument: per-fixture rows of
  legacy-vs-oracle, chassis-vs-oracle, legacy-vs-chassis, plus
  coverage, over an enumerated corpus, CI-hosted with committed oracle
  bakes.
- **the flip rule** — the ratified criterion (thresholds, coverage,
  oracle-discipline clause) a suite must meet for its bar to flip.
  Written before any score exists.
- **`UNSUPPORTED` row** — the honest scoreboard entry for a construct
  the model deliberately cannot express; a measured gap, never a shim.
- **golden** — a committed reference output (usually pixels) that a
  rig compares against byte-exactly.
- **honesty rig** — the chassis's self-check harness: byte goldens,
  double-run determinism, budget benches; designed to fail loudly.
- **replay corpus** — recorded sessions (canonical document + ordered
  ops) serving repro, bench, fuzz, and conformance from one artifact.
- **oracle discipline** — the standing clause: where the two engines
  diverge and the chassis is closer to the oracle, the divergence is a
  legacy finding, not a chassis failure.

## Engine vocabulary

- **effective values** — the immutable property values a frame is
  resolved from: authored values, or authored values overridden by a
  sampled animation program. Resolution never sees where they came
  from.
- **resolved document** — the pure product of resolution: geometry,
  transforms, bounds. Never serialized; recomputed, not stored.
- **frame product** — the immutable output of one frame construction:
  the resolved tier, the display list, and timings.
- **display list / drawlist** — an engine's private compiled form
  between resolution and paint. Per-engine by settled ruling; not a
  contract.
- **leaf paint vocabulary** — the shared paint contract (ordered paint
  stacks, stroke applications, text-run paint ownership) — the whole
  shared surface between the engines' renderers.
- **shaped-text artifact** — the backend-neutral text layout (lines,
  glyph runs, positions) produced once by a text oracle and replayed
  by painters; never reshaped downstream.
- **sample time / explicit time** — the declared instant an animation
  program is evaluated at. Time is always an input, never ambient.
- **render mode** — a policy over the pure core: realtime preview,
  accurate static, or accurate animation. Modes may differ in when and
  at what quality they paint — never in what things mean.

## Formats

- **n0 XML (`.n0.xml`)** — the engine's authored, inspectable,
  diffable source language (its own RFD family; formerly "Grida XML").
- **`.grida`** — the v1 packed binary (FlatBuffers). Legacy;
  converter-input only in the end state.
- **the converter** — the explicit `.grida` → model bridge; the only
  way v1 documents enter the chassis. Converter-shaped forever.

## Governance

- **the charter** — [charter.md](./charter.md): phases 0–6 with
  entry/exit gates, the doctrine, the decision registry, the known
  unknowns.
- **D-\* / registry decision** — an owner decision with a stated
  evidence bar and a phase where it's taken (D-C, D-D, D6, D-E, D-H,
  D-G(b), D-I, D-J, D-K, D-L, D-M, AMD, FLIP, NAME). Decisions are not taken
  early and not renegotiated late.
- **GO** — the owner's explicit approval, required for merges to main
  and registry decisions. All-green CI is necessary, not sufficient.
- **phase** — one of the charter's six program stages; phases gate on
  evidence, not on dates.

## Charter concretes

Shorthand the charter uses at its deliberately concrete register:

- **cg** — the legacy engine's paint-vocabulary module; the Phase 1
  crate cut that makes the leaf paint vocabulary a shared crate.
- **the zero-bridge constraint** — the scoreboard may only call the
  engines through their existing render entry points; no new bridges
  are built for the sake of measurement.
- **the skia-free lock** — the architectural rule that the model tier
  carries no graphics-backend dependency; at the Phase 1 cut it
  becomes the crate boundary itself.
