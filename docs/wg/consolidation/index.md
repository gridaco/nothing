---
title: The Consolidation Program
description: "Program home: one engine by extraction. The destination, the topology, the method, and the ratified charter — start here."
tags:
  - internal
  - wg
  - program
format: md
---

# The Consolidation Program

One engine. The v2 family (`crates/n0`, `crates/n0-model`,
`crates/n0_dev`) is the **chassis**; the legacy family (`crates/grida`
and its satellites) is the **know-how**. The program ends when the
chassis carries all the know-how, the conformance bar has flipped, and
the legacy engine has contracted to nothing.

**Status:** Active. **Owner:** universe@grida.co.
**Tracking:** [gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43)
(umbrella), [gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)
(the v2 model program), [gridaco/nothing#1](https://github.com/gridaco/nothing/issues/1)
(the migration anchor).

**Genre:** program record — doctrine, destination, and method for the
people and agents doing the work. Not a domain spec. The domain specs
live in the sibling wg clusters (`canvas/`, `format/`, `feat-*`) and
**always win on conflict**; a conflict with one of them is a finding to
file, never something to paper over here.

## Read in this order

A zero-context session becomes a working session by reading four short
docs, in order:

1. **[The goal of n0](./goal.md)** — what the consolidated engine
   becomes, and what it refuses to be.
2. **[End-state topology](./topology.md)** — chassis vs know-how: who
   absorbs whom, per domain, and the render-mode taxonomy.
3. **[The method](./method.md)** — how one consolidation step is
   executed, with a worked example.
4. **[The charter](./charter.md)** — the ratified route: phases with
   entry/exit gates, the first PRs, the owner decision registry, and the
   known unknowns.

**Amendment (ratified 2026-07-21):** the **[Web-First
Amendment](./web-first.md)** revises the topology to *one engine kernel,
source-native semantic models, one provisional resolved render contract,
many hosts*, and moves HTML/CSS + SVG to the front. Read it with the goal
and topology — it supersedes their "one **model**" reading (the engine
kernel converges downstream, not at one authored model) and reorders the
charter's phases where they conflict. It leaves every standing rule
below intact.

Reference, consulted as needed: **[the glossary](./glossary.md)** — the
program vocabulary. Use its terms; do not mint synonyms.

Decision proposal, not yet ratified: **[the conformance-bar flip
rule](./flip-rule.md)** — the pre-score threshold, coverage, and
oracle-discipline proposal tracked by
[gridaco/nothing#49](https://github.com/gridaco/nothing/issues/49).

The charter is the only doc of the four that sequences work — and it
records the *route*; the current *position* (active phase, landed PRs,
taken decisions) is tracked on
[gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43),
which a session reads before starting work. The other three docs are
direction: stable in shape, expected to gain detail as decisions in the
registry land.

## Standing rules (bind every session)

Stated once, owned by the [charter's doctrine section](./charter.md):

- **Patrol-before-drop.** No deletion or replacement without a patrol
  triage and a captured-essence ledger first.
- **Migration by extraction.** A module becomes a shared crate when its
  second consumer appears in the workspace — never before.
- **Absorption direction.** The chassis absorbs the engine role by
  consuming extracted crates — never by copying; the legacy engine is
  decomposed, not absorbed.
- **Two gate classes.** Zero-behavior moves gate on byte-identical
  sweeps; capability grants gate on the scoreboard. Nothing lands
  unmeasured.
- **Oracle discipline.** The conformance bar is the Chromium/consensus
  oracle — never the legacy engine itself.
- **Frozen surfaces.** The v1 schema (`format/grida.fbs`) and the
  published wasm freeze contract stay untouched until Phase 6 says
  otherwise.
- **Owner gates.** Merges to main and every registry decision need the
  owner's explicit GO. All-green CI is necessary, not sufficient.

## Working conventions

- Plans are `*.plan.md` files — gitignored working scratch, never
  committed knowledge. Durable knowledge lands in this directory or the
  sibling wg clusters; work items land in issues.
- Skills to load per task: `docs-wg` and `naming` before authoring or
  renaming anything; `links` for cross-repo references; `fixtures`,
  `render-reftest`, `render-perf`, `io-svg`, `io-grida` for their
  engine loops.
- Concluded programs are archived under repo-root `archive/` — the
  frozen v2 workbench record at
  [`archive/model-v2/`](../../../archive/model-v2/README.md) is the
  precedent, and this directory retires the same way when the program
  concludes.
