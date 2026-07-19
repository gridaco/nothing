---
title: The Goal of n0
description: "What the consolidated engine becomes — the render ecosystem, the engine ecosystem, the dev editor, the render modes, the format posture — and its non-goals."
tags:
  - internal
  - wg
  - program
format: md
---

# The Goal of n0

**Genre:** doctrine — the destination the consolidation program drives
toward. The direction is stable; details are expected to change, and
every open detail is owned by a row in the
[charter's decision registry](./charter.md), not by prose here.

n0 ("nothing") is Grida's 2D graphics engine. The end state is **one
engine, one model, many hosts**: a single pure core — document model,
resolution, display-list build, paint — that every product surface
hosts thinly.

## Why one engine

Today two engines share this repository: the shipping legacy engine
and the v2 chassis proven in the lab. Carrying both indefinitely
doubles the cost of every capability, splits trust across two
implementations, and lets caveats hide in whichever engine isn't being
looked at. The program exists so that capability lands **once**, in
the chassis, measured against an external oracle — and so that the
legacy engine can retire by subtraction with nothing silently lost.

"Many hosts" is the payoff of purity: because the core is a pure
function from document and time to pixels, the CLI, the editor, the
server, and the wasm package are all thin hosts around the same core —
and a conformance result earned on one surface transfers to all of
them.

## The render ecosystem

Native rendering, each exposed as a CLI (and library) surface. All
four share one pipeline and one cascade discipline; determinism is a
product feature — the same input at the same declared time produces
the same bytes.

- **HTML/CSS rendering.** Browser-grade style resolution (a real
  browser cascade implementation, not a CSS-subset approximation),
  layout including grid, and paint. The conformance bar is the
  Chromium/consensus oracle, graded continuously by a
  web-platform-tests harness. The surface renders static styled
  content — it is not a user agent (see Non-goals).
- **SVG rendering.** SVG styled by the *same* browser-grade CSS
  resolution as the HTML path — one cascade, two grammars. The
  ambition is a best-in-class `svg in → pixels out` tool, distributed
  as CLI and library
  ([gridaco/nothing#13](https://github.com/gridaco/nothing/issues/13)),
  graded against the standard SVG conformance corpora.
- **SVG animation rendering.** Deterministic, explicit-time sampling
  of a declared SVG animation subset, with playback controls in
  interactive hosts and exact-time frame sequences in the export form
  — where dropped frames are structurally impossible, because every
  frame is an accurate render at its declared time.
- **n0 XML rendering.** The engine's own authored source language
  (`.n0.xml`, the [n0 XML RFD family](../format/n0-xml.md)):
  inspectable, diffable, responsive documents rendered natively.

The CLI is always a thin host: source and asset I/O, surface setup,
and encoding live in the host; meaning lives in the engine. A CLI must
never grow semantics of its own.

Relationship to the program: the program guarantees the **engine
capabilities**; CLI packaging and distribution are parallel product
tracks ([gridaco/nothing#13](https://github.com/gridaco/nothing/issues/13)
et al.) that consume capabilities as the charter's phases grant them.

## The engine ecosystem

The same engine backend serves both interactive and authoritative
hosts — one document semantics, two duty cycles:

- **The editor.** The interactive canvas: incremental frames at
  interactive latency, hit-testing and spatial query, invertible
  mutations with history, live preview of in-flight edits.
- **The server.** The document authority behind multiplayer — the
  CRDT server's document backend: headless and windowless, buildable
  with **no graphics backend at all**, deterministic, with the
  operation log as its wire-adjacent vocabulary and replay as its
  audit trail. Transport is a host concern per
  [feat-crdt](../feat-crdt/index.md); persistence stays host-owned (a
  program posture — feat-crdt does not address it); presence follows
  feat-crdt's ephemeral presence contract. The engine ships document
  semantics, not a server product.

Stated program default (owner may override before any server host
ships — the backend-optional build split is a demand-driven lane per
the [charter's amendments](./charter.md)): "same backend" means the
engine crate family itself runs server-side — not a separate server
engine, and not a fork.

## The dev editor

A dev editor with the full authoring and editing experience — the
reference host that proves the engine's editing contracts end to end:
authoring tools, direct manipulation, text editing, history,
multi-instance sync. It is where editing contracts become demos and
demos become regressions. The legacy editor core currently serves this
position; which crate owns editor-core in the end state is decision
**D6** in the registry.

## Render modes

Three modes, one time model — specified in the
[end-state topology](./topology.md):

- **realtime preview** — stable frames under camera movement and
  hot-loop editing; heuristics allowed, meaning never changed.
- **accurate static** — the export path: full quality, no heuristics,
  deterministic output.
- **accurate animation** — export at exact sampled times;
  frame-perfect by construction.

The mode rule, stated once and binding everywhere: modes may differ in
*when and at what quality* they paint — never in *what things mean*.

## The format posture

- The engine's canonical contract is the **in-memory model and its
  operation vocabulary** — not any file format. "Evolution-friendly"
  is achieved by keeping the durable contract at the model, where it
  can be versioned and tested, rather than in bytes on disk.
- `.n0.xml` is the authored, inspectable source form (its own RFD
  family under [format](../format/index.md)).
- Binary storage and interchange are **host-managed**: the engine
  provides conversion and packing tooling; the user-host owns the
  format contract. That layer stays deliberately simple — the engine
  never accretes format-compatibility burden, and hosts never depend
  on engine internals to read their own files. This posture is a
  stated program default (owner may override when it is decided as
  registry decision **D-J**, widening D-G(b), at the Phase 5
  boundary) — not yet a ratified fact.

## Non-goals

- **Native `grida.fbs` support.** The v1 format is legacy: read
  through an explicit converter only, never a native dialect of the
  engine. The drop is deferred (the freeze contract holds until
  Phase 6), but the format is eventually dropped and redesigned — see
  the charter's Phase 5–6 and decisions D-G(b)/D-J.
- **Being a browser.** The HTML/CSS surface is a rendering discipline
  with a conformance bar — static styled content in, pixels out.
  Scripting, navigation, interactivity, and the rest of the user-agent
  surface are out of scope.
- **A shared display-list contract between the two engines.** Settled
  by the
  [display-list contract study](../feat-2d/display-list-contract.md):
  the shared surface stops at the leaf paint vocabulary; display lists
  are per-engine projections.
- **Product and editor-UI concerns.** Those live in
  [gridaco/grida](https://github.com/gridaco/grida); this engine ships
  contracts and hosts, not product.
