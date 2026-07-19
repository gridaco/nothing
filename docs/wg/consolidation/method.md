---
title: The Method
description: "How one consolidation step is executed — the lifecycle, the three step shapes with worked examples, the gates, and where everything is recorded."
tags:
  - internal
  - wg
  - program
format: md
---

# The Method

**Genre:** operating procedure. The doctrine (patrol-before-drop,
migration by extraction, the two gate classes, oracle discipline,
frozen surfaces) is owned by the [charter](./charter.md) and is not
restated here — this doc is the *shape of one step*, so any session
can execute one without re-deriving the program.

## The step lifecycle

Every consolidation step — a crate cut, a capability grant, a
retirement — runs the same loop:

1. **Patrol.** Before touching the scope, a patrol agent triages
   everything that will be moved, replaced, or deleted, and produces a
   captured-essence ledger (below). No silent drops — both engines
   carry years of caveats, and the ledger is how they survive the
   merge.
2. **Name.** The naming exercise happens before the code moves (load
   the `naming` skill). Canonical names per end state; never
   plan-sequence codes or session shorthand.
3. **Cut or adopt along the contract.** A zero-behavior move is an
   extraction with a re-export — the consumer's source diff is empty.
   A capability grant is implemented against the agnostic contract
   (paint vocabulary, text artifact, layout model) — never by copying
   the other engine's internals. If the contract can't express the
   capability, that is a contract amendment to ratify first, not a
   code-side workaround.
4. **Gate.** Zero-behavior moves prove themselves with byte-identical
   sweeps. Capability grants land as scoreboard rows graded against
   the oracle. Nothing lands unmeasured.
5. **Record.** Tick the tracking issue; if a registry decision moved,
   update the charter; if essence was dropped, the ledger says what
   and why.
6. **Delete last.** Legacy code retires only after the gate holds —
   deletion is subtraction with a ledger, never cleanup. When in
   doubt, the standing rule: patrol first.

## What a patrol produces

The captured-essence ledger is a short structured document (scratch or
PR body — its *content* is what gets committed, re-homed into durable
places):

- **Scope inventory** — everything in the blast radius: code,
  fixtures, goldens, docs, baselines.
- **Caveats with provenance** — behaviors that exist for a reason the
  code alone doesn't show: measured performance numbers, standards
  citations, bug-driven special cases, platform differences. Each with
  where it came from.
- **Re-home destination per caveat** — a spec clause, a test, the
  receiving code, or the archive. Load-bearing caveats are re-homed
  *before* the deletion merges.
- **Deliberate drops, with reasons** — named in the commit or PR
  message. A drop that isn't named is a silent drop, and silent drops
  are the failure mode this rule exists to prevent.

## The three step shapes

The lifecycle specializes into three shapes. One worked example each —
illustrative, not work orders.

### Zero-behavior move — a vocabulary crate cut

*Example: the paint vocabulary becomes a shared crate (Phase 1).*

- **Patrol** the module and its consumers; the ledger notes the
  semantic subtleties the vocabulary carries (gradient stop
  interpolation, tile-mode edges, stroke ordering) that the
  conformance suite must later measure.
- **Name** the crate by the naming exercise — before the move.
- **Cut**: the module becomes a workspace crate; the legacy engine
  consumes it via re-export, so its own source diff is empty; the
  chassis becomes the second consumer.
- **Gate**: byte-identical sweeps (below) — pixels, goldens, encoded
  outputs all byte-equal; dependency lock additions-only; the wasm
  package still builds. No scoreboard involvement — a zero-behavior
  move never waits for scores.
- **Record and done** — nothing is deleted; the seam simply moved.

### Capability grant — a per-node paint capability

*Example: the image node's paint behavior reaches the chassis.*

- **Patrol** the legacy image-paint estate. The ledger captures the
  load-bearing spec: sampling quality keyed by render intent
  (export-grade filtering vs fast interactive sampling); image fills
  participating in ordered paint stacks like any other paint;
  image-specific filters. These are the caveats that must not
  silently drop.
- **Contract check**: anything the paint vocabulary can't express
  becomes a proposed RFD amendment with a named owner — ratified
  before code.
- **Adopt**: the chassis's painter implements the image leaf against
  the vocabulary; sampling policy keys off the render mode (accurate
  vs preview), honoring the mode rule — quality may differ per mode,
  meaning may not.
- **Gate**: image-paint conformance fixtures enter the scoreboard,
  graded against the oracle. The legacy engine stays the executable
  bar for this suite until the flip rule says otherwise.
- **Delete**: only at this suite's flip, with the ledger closed out.

### Retirement — a per-suite bar flip

*Example: one conformance suite's bar moves from legacy to chassis.*

- **Evidence**: the scoreboard meets the ratified flip rule for the
  suite — thresholds, coverage, and the oracle-discipline clause
  (where the two engines diverge and the chassis is closer to the
  oracle, the divergence is a legacy finding, not a chassis failure).
- **Flip** the suite's bar: the chassis becomes the graded reference
  for it; regressions now fail against the chassis's baselines.
- **Patrol** the legacy path serving that suite; re-home its essence.
- **Delete** the now-unreferenced legacy path — reversible per-suite,
  gated by the remaining byte gates and the wasm build staying green.

## Gates, precisely

Two gate classes, never mixed:

- **The byte-identical sweep** (zero-behavior moves): every shipping-
  surface output — rendered pixels across the declared corpora,
  committed goldens, encoded artifacts — is byte-equal before and
  after. Any diff fails; there are no thresholds and no similarity
  scores. Companions: the dependency lock changes additions-only, and
  the published wasm package still builds. CI runs the declared
  subset; the machine-local suites are named per-PR, not pretended.
- **The scoreboard** (capability grants and flips): per-fixture rows
  of three comparisons — legacy vs oracle, chassis vs oracle, legacy
  vs chassis — plus per-engine coverage counts over an enumerated
  corpus. Unsupported constructs are honest `UNSUPPORTED` rows; a gap
  measured is progress, a gap shimmed is debt. The flip rule is
  ratified *before* any score exists, so scores can never negotiate
  their own bar. The instrument itself is kept trustworthy: committed
  oracle bakes, regression-vs-baseline failure, a hard wall-clock
  budget — a slow or flaky scoreboard gets ignored, and an ignored
  scoreboard silently reverts the program to faith.

## Where everything is recorded

One home per artifact kind — a zero-context session should never have
to guess where something lives:

- **Decisions** → the [charter's registry](./charter.md), each with
  its evidence bar; amendments are dated in the charter.
- **Work items and sequencing** → issues
  ([gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43)
  is the umbrella); cross-repo references always in full
  `gridaco/<repo>#N` form.
- **Captured essence** → the ledger in the commit/PR message, with
  load-bearing items re-homed into specs, tests, or receiving code.
- **Findings** — spec conflicts, legacy-vs-oracle divergences, gaps
  discovered mid-step → issues, filed where the fix would land.
- **Knowledge** → this directory (program-level) or the sibling wg
  clusters (domain-level).
- **Scratch** → `*.plan.md`, gitignored by convention; never durable.

## Session etiquette

- Start at [index.md](./index.md); work from this directory plus the
  charter. Never re-derive the program from chat history or memory
  alone.
- Prefer one step per session, completed through **Record** — a step
  left half-recorded is worse than a step not started, because the
  next session can't trust the tree's state.
- Leave the tree green: checks passing, links resolving, no
  uncommitted debris.
- On conflict between this program and a domain spec, the domain spec
  wins — file the conflict as a finding.
- Merges to main and registry decisions wait for the owner's explicit
  GO; all-green CI is necessary, not sufficient.
