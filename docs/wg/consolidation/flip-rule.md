---
title: Conformance-Bar Flip Rule Proposal
description: "Unratified FLIP proposal: the per-suite threshold, coverage, and oracle-discipline rule drafted before the first scoreboard score exists."
tags:
  - internal
  - wg
  - program
format: md
---

# Conformance-Bar Flip Rule Proposal

**Status:** Unratified decision proposal. This document is not yet **the flip
rule**. Registry decision **FLIP** remains open in
[gridaco/nothing#49](https://github.com/gridaco/nothing/issues/49), and no
score may be produced or inspected until the owner explicitly approves a
version of this rule.

## Decision scope

The flip rule answers one question per conformance suite: when is that suite
eligible for its **bar** to move from the legacy engine to the chassis?

Eligibility is evidence for a later owner decision. It does not itself flip a
bar, grant a capability, retire legacy code, or authorize a merge.

## Proposed rule

A suite is eligible only when every condition below holds in one complete
scoreboard run:

1. **Fixed denominator.** The run uses the enumerated included corpus named by
   the rule. Every included row is present exactly once. Changing that set
   creates a new corpus version and invalidates eligibility under the old
   version. The manifest's excluded-family ledger preserves patrol findings;
   those family globs are not denominator membership and do not claim to
   enumerate every file outside the corpus.
2. **Complete coverage.** Chassis coverage is 100% of included rows. No
   included chassis row is `UNSUPPORTED` or an error. Excluded source families
   remain visible with their reasons, but only an explicit corpus-version
   change can move a source into or out of the denominator.
3. **Per-row threshold.** Every included chassis-vs-oracle comparison meets
   the suite threshold. An aggregate cannot rescue a failing row.
4. **Determinism.** Repeating each chassis render from the same source and
   declared inputs produces byte-identical decoded pixels before either render
   is compared with the oracle.
5. **Current evidence.** The run matches the committed corpus identity,
   oracle-bake identity, scoring-method identity, and rule version. It has no
   per-row regression against the committed scoreboard baseline.
6. **Operational trust.** The complete run finishes within its declared hard
   wall-clock budget. An incomplete, over-budget, or provenance-invalid run is
   ineligible and cannot produce a baseline candidate.

## Proposed threshold table

| Suite | Corpus | Oracle | Chassis coverage | Per-row threshold |
|---|---|---|---:|---|
| Direct SVG rect/path | `svg-rect-path-v0` | Committed Chromium bake of the unchanged source | 100% | Zero non-antialiased differing pixels under a threshold-zero comparison |

The Direct SVG rect/path corpus is deliberately constrained to deterministic,
opaque-background solid-shape fixtures. Its comparison ignores pixels
identified as rasterizer edge antialiasing, but applies no color-distance
tolerance to the remaining pixels. Therefore the proposed threshold is exact
after the declared antialiasing classification, rather than a percentage
selected after seeing results.

The proposed hard wall-clock budget for the complete v0 instrumented work is
120 seconds, beginning before corpus, oracle, and parser validation and ending
after per-row comparisons and the prior-baseline regression check. Build time
is outside that interval. The budget is part of the rule identity and has no
operator override.

Before any other suite is scored, this table must be amended with that suite's
enumerated corpus, oracle, coverage requirement, and per-row threshold through
the same owner-gated process. A result cannot choose its own threshold.

## Oracle discipline

The oracle is Chromium or the declared consensus, never the legacy engine.
The legacy-vs-oracle and legacy-vs-chassis comparisons preserve migration
context; neither can overrule the chassis-vs-oracle result.

Where the two engines diverge and the chassis is closer to the oracle, the
divergence is a legacy finding, not a chassis failure. Where the chassis misses
the oracle, agreement with the legacy engine does not excuse the miss. Where
the oracle is disputed, the row is ineligible until the suite's declared
consensus procedure resolves it.

## Baseline changes

The committed scoreboard baseline records trend; it does not redefine the
threshold. A normal run never changes that baseline. A bless flow may produce
a complete review candidate from the same valid run, but it may not change the
corpus, oracle bake, scoring method, wall-clock budget, or rule version.

Accepting a regression, changing a threshold, or changing coverage after a
score exists requires a new explicit owner decision and a new rule version.

## Ratification record

No ratification has occurred. Owner GO, the accepted rule version, and the
date belong here only after they are recorded on
[gridaco/nothing#49](https://github.com/gridaco/nothing/issues/49).
