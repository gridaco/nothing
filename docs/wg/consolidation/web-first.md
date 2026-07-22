---
title: The Web-First Amendment
description: "Owner amendment: the topology becomes one engine kernel, source-native semantic models, one provisional resolved render contract, many hosts — and HTML/CSS + SVG lead. What it supersedes, and what it does not."
tags:
  - internal
  - wg
  - program
format: md
---

# The Web-First Amendment

**Genre:** program amendment — a ratified owner direction that revises
the program's topology and sequencing. Not a spec and not a plan: it
records *what changed* and *why*, defers the unchanged model to the
sibling docs, and leaves implementation shape to the PR that carries it.

**Status:** Ratified by the owner, 2026-07-21. **Owner:**
universe@grida.co. Position tracked on the program umbrella
[gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43).

**What it touches.** It supersedes the "one **model**" reading of
[goal.md](./goal.md) and the phase *ordering* of the
[charter](./charter.md) where the two conflict. It does **not** touch the
standing rules (see [What it does not supersede](#what-it-does-not-supersede)).

## The revised topology

The end state was stated as *one engine, one model, many hosts*. It
becomes:

> **One engine kernel, source-native semantic models, one provisional
> resolved render contract, many hosts.**

The single word that changes is **model**. There is not one universal
document model that HTML, SVG, and n0 all reduce to. Each source keeps
the semantic model its grammar demands; they converge **downstream**, at
a shared render contract, not upstream at a shared authored model.

- **One engine kernel** — the shared downstream: bounds, culling, damage,
  drawlist construction, raster caches, batching, export. Convergence
  begins only where the inputs genuinely match.
- **Source-native semantic models** — HTML/CSS, SVG, and n0 each retain
  their own semantic model, cascade/resolution, and editing semantics.
  n0 is **not** promoted to the universal HTML/SVG model.
- **One provisional resolved render contract** — a single source-neutral
  description of derived frame data (see
  [The shared boundary](#the-shared-boundary)), shaped by real producers,
  internal and breakable.
- **Many hosts** — unchanged.

## The Web semantic family leads

HTML/CSS and SVG are prioritized now, ahead of the charter's original
phase order, and are treated as **one Web semantic family**:

- HTML and inline SVG are **one namespace-aware document** sharing **one**
  browser-grade cascade (inheritance, resources, and layout integration
  included). Descendant style inside inline SVG comes from the surrounding
  document cascade.
- Standalone SVG is a **different grammar entry into the same** SVG/Web
  machinery — the same semantic compiler, the same cascade behavior — not
  a separate renderer.

Four constructions are ruled out because each fractures that family:

- **No serialize-and-reparse.** Inline SVG must not be flattened to a
  string and re-parsed through a nested renderer; it already lives in the
  one document.
- **No temporary SVG-only matcher as the final cascade.** A bespoke
  SVG-only property matcher may exist as scaffolding; it must not become
  the cascade of record. The cascade of record is the shared browser-grade
  (Stylo) cascade.
- **No universal `n0` model.** n0 must not become the model HTML/SVG are
  expressed in.
- **No three renderers behind one trait.** The goal is one shared
  downstream, not three complete renderers hidden behind a `Render`
  abstraction.

## The shared boundary

The provisional common product is **derived frame data** — never an
authored source of truth, never a file format, never a round-trip
promise. It carries only normalized visual facts:

- stable source-neutral identity and provenance;
- geometry and resolved bounds;
- transforms, clips, masks, effects, isolation, and painter order;
- ordered paint stacks;
- shaped-text artifacts;
- resource references and exact environment revisions.

It must **not** carry: HTML tags, selectors, or CSS syntax; SVG element
or attribute syntax; n0 bindings or operations; raw embedded XML; parser
ASTs; filesystem or network policy; backend objects, opaque callbacks,
`Any`, or nested pictures; serialization or round-trip guarantees.

**Invalidation stays source-specific.** Each source owns how its inputs
change; sharing begins only where the inputs genuinely match — bounds,
culling, damage, drawlist construction, raster caches, batching, export.

**The chassis invariants hold**, even though n0 is not the current
product target — because they are what makes the boundary a *contract*
rather than a dump: explicit time; stable identity; immutable frame
products; declared font/image/resource environments; damage as data; no
I/O or ambient clock in the core.

**The contract yields, not the fields.** It is internal and breakable by
design. If honoring it would force a Web-specific field into the shared
representation, the sharing boundary moves **downward** (toward the
drawlist) rather than the contract admitting the field.

## Forbidden in the new path

New edges this amendment draws, beyond the standing non-goals it inherits
([goal.md](./goal.md) already forbids native `.grida`/schema support; the
[frozen surfaces](./charter.md) and patrol-before-drop rules still bind):

- **No render through legacy import adapters.** Web HTML/SVG must not be
  routed through the legacy SVG-import-to-document converter; that path
  collapses Web semantics into the design-tool model and loses the shared
  cascade.
- **No legacy node model in the new path.** The legacy geometry-first
  node model / scene graph does not appear downstream of the Web sources.
- **No backend escape hatch around the kernel.** The render product is the
  resolved contract, not an opaque backend picture handed past the shared
  kernel; nested backend pictures are not a substitute for expressing
  isolation and effects as data.
- **Adopt by contract, not by copy.** Legacy know-how (SVG element→paint
  semantics, the painter's no-escape-hatch discipline) is patrolled and
  re-derived against the contract, with a captured-essence ledger — never
  copied wholesale.

## Deferred by evidence, not by prose

Three questions are explicitly left to a later evidence spike, not
settled here:

- **D-M: n0's join point** — whether n0 emits the common resolved contract,
  or
  joins only at the drawlist boundary. A deliberately tiny n0 canary
  exercises the shared downstream to keep it source-neutral; that canary
  is an invariant probe, **not** an n0 product milestone, and n0 XML
  capability work stays parked (kept building and tested, not expanded).
  A gap analysis against n0's real downstream facts — the
  [n0-join-point finding](./n0-join-point.md) — reframes this as a
  *per-fact* boundary (visual primitives converge high; shaped text is the
  deciding fact) and names the spike that would settle it. D-M is coupled to
  D-C's paint/stroke gap report and cannot be inferred from the canary.
- **A generic frontend trait** — none is published or stabilized yet.
  Concrete data must be shaped by at least two real producers first.
- **D-M: the leaf-vocabulary seat** — where the neutral value vocabulary
  ultimately lives is the coupled half of D-M, decided by the same evidence,
  not assumed.

These belong in the [charter's decision registry](./charter.md) as they
are filed; this amendment names them so no session settles them by
default.

A fourth, concrete open decision the first prototype surfaced with
evidence: **D-L, how SVG paint enters the shared cascade.** The Stylo build the
workspace compiles omits the SVG paint properties, so `fill`/`stroke`
cannot come from the shared cascade today — see the
[SVG-paint-cascade finding](./svg-paint-cascade.md) for the enumerated
evidence, the options, and the decision it gates.

## What it does not supersede

Unchanged and still binding: **patrol-before-drop**, **oracle
discipline** (the bar is the Chromium/consensus oracle, never a legacy
engine), **frozen surfaces** (the v1 schema and the published wasm freeze
contract), and the **two gate classes**. A capability is not "landed"
until its required scoreboard gate is legally available and passes; while
[FLIP](./flip-rule.md) is unratified, a capability that needs it is
parked honestly rather than backed by manufactured evidence.

The revised topology changes *what converges where*; it does not relax
*how anything is proven*.
