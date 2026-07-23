---
title: End-State Topology
description: "Chassis and know-how: who absorbs whom across every domain, the render-mode taxonomy, and the end-state crate silhouette."
tags:
  - internal
  - wg
  - program
format: md
---

# End-State Topology

**Genre:** program record. This doc names current crates because the
program is about these crates; the domain models it leans on live in
the sibling wg clusters and are only referenced, never restated.

> **Amended (2026-07-21).** The **[Web-First
> Amendment](./web-first.md)** revises this end state: sources keep
> **source-native semantic models** and converge at **one provisional
> resolved render contract** (the engine kernel), rather than reducing to
> a single universal model. HTML/CSS + SVG lead as one Web semantic
> family. Read the amendment for the shared-boundary field discipline and
> the questions it defers to an evidence spike; the per-domain map below
> stands as the substance being consolidated.

## The two sources

**The v2 family is the chassis.** Its topology is adopted as-is:

- the pure pipeline — `(document + effective values) → resolve →
  drawlist → paint`, with an immutable frame product at the boundary,
  constructed through one entry no matter the host;
- **time as data** — a frame is requested at a value state: base, or
  sampled at an explicit time; nothing in the core owns a clock;
- the animation kernel — the chassis's most complete subsystem and the
  reference for how every time-varying input enters the engine;
- the resolver topology — measure → layout → transforms → bounds, with
  anchor bindings and declared size intents;
- the text-layout seam — a backend-neutral shaped-layout artifact
  behind a versioned oracle trait;
- the read and replay tiers — spatial query, the operation journal,
  the replay corpus, damage as data;
- the determinism doctrine — replayable sessions, versioned oracles,
  honesty rigs that fail loudly when goldens or budgets drift.

**The legacy family is the know-how.** Its substance is adopted
*through contracts*, never copied wholesale:

- the per-node paint specs and the effects estate;
- the measured performance-optimization estate;
- the import stacks (HTML/CSS and SVG) and the export subsystem;
- the paragraph feature depth and the text-editing stack;
- the editor-core vocabulary and the sync model;
- and, until the bar flips, the role of executable conformance
  reference for every suite it still wins.

**The bridge:** the chassis exposes **agnostic contracts** — the leaf
[paint vocabulary](../feat-painting/paint-model.md), the
[shaped-text artifact](../feat-paragraph/text-layout.md), the layout
model — and legacy capability migrates through those contracts, gated
per the [charter](./charter.md). Two things are deliberately *not*
shared: the display list (per-engine by settled ruling — the
[display-list contract study](../feat-2d/display-list-contract.md))
and each engine's compile policy above the leaf vocabulary.

## Per-domain adoption map

The index; each row is expanded in [The domains](#the-domains) below.

| Domain | From the chassis (v2) | From the know-how (legacy) | Status / decision |
| --- | --- | --- | --- |
| Pipeline & frame | resolve → drawlist → paint; frame product; time-as-data request seam | frame-scheduling experience informs preview-mode policy | landed |
| Animation & time | the whole kernel: sampling, the SVG animation profiles, playback clock, offline exact-time export | — (no animation system exists in legacy) | chassis-only domain |
| Layout | resolver topology; flex via the shared layout library; anchor bindings | block flow; the CSS-grid mapping; virtual-node subtree handling | capability grants through the resolver |
| Text | shaped-layout artifact + oracle seam (shape once, replay glyphs) | paragraph depth: decorations, features, RTL runs, fallback, editing stack | seam from chassis, coverage from know-how; D-H at Phase 5 |
| Painting | the agnostic leaf paint contract (vocabulary crate cut, Phase 1) | the 19 node types' paint specs and the effects estate | the program's center of gravity |
| Realtime perf | damage-as-data sockets; the scene raster cache | the optimization estate as preview-mode *policy* over the pure core | D-K (unified time model) |
| SVG import | — | the import-to-document pipeline (IR crate at Phase 3) | Phase 3 |
| SVG render | — | the browser-grade-cascade lineage | stated program default: distinct surface from import; confirmed at Phase 3 entry |
| HTML/CSS | — | the htmlcss engine, cascade front-end, WPT harness | Phase 4; D-D (font provider) |
| Export | — | raster/PDF/SVG export at full render intent | amendment: granted before Phase 6 contraction |
| Editor core | operation journal + replay | mutations, history, commands, sync vocabulary | D6 |
| Server & headless | the skia-free model family runs server-side today | — | needs the graphics-backend-optional engine build (priced) |
| Wasm | — (no v2 wasm exists) | the published v1 wasm package | port is priced work; the switch is Phase 6 (D-I) |
| Format & storage | model + ops as the canonical contract; `.n0.xml` as source | `.grida` (fbs) as legacy input via the converter | D-G(b) + D-J |
| Foundations | — | math, font introspection, the cascade engine as shared satellite crates | grows as vocabulary crates are cut |
| Verification estate | replay corpus; honesty rigs | reftest suites; the WPT harness; bench tooling | shared instrument tier; scoreboard is Phase 2 |
| Dev shells | the windowed dev shell, live player, headless shots | the dev CLI: bench, reftest, export tooling | converge on the dev editor as capability flips |

## The domains

### Pipeline & frame

The chassis pipeline is the end-state pipeline: resolution is a pure
function of the document and its effective values; the display-list
build is a pure projection of resolved geometry and authored paint
intent; painting executes that projection against a backend. A frame
is an immutable product with timings, and every host — CLI, editor,
server, test rig — reaches it through the same entry. Legacy
contributes experience rather than structure here: its frame-scheduling
and stability policies inform the preview mode (see Realtime
performance). Status: landed with the v2 promotion.

### Animation & time

Chassis-only — the legacy engine has no animation system, so nothing
migrates. The kernel provides format-neutral, explicit-time sampling:
exact time arithmetic, typed curves over scalars, colors, transforms,
and path geometry, keyframe timing with easing, and additive/
accumulative composition — compiled from authored sources into a
source-neutral animation program that maps one declared time onto
plain property values without mutating the document. The first
authored frontend is a strict SVG animation subset (the cumulative
profiles under [feat-svg](../feat-svg/animation.md)); native n0 XML
animation syntax is deliberately deferred until a second, materially
different producer exercises the program
([n0 XML animation RFD](../format/n0-xml-animation.md)). Playback is a
caller-owned clock mapping host time to sample time; frame scheduling
is forever a host concern — the core never owns a wall clock.

### Layout

Not a rivalry: both engines delegate flex to the same shared layout
library, so the contest was never *whose flexbox*. The chassis's
resolver topology is the end-state frame — measure, layout,
transforms, bounds, with anchor-based positioning and declared size
intents. The know-how to grant through it: block flow, the CSS-grid
mapping proven in the HTML renderer, and the handling of virtual
grouping nodes that appear in trees without owning layout. The
resolver's semantics — rotation-in-flow, and size intents that are
fixed or hug with deliberately no fill (growth is by grow and
self-align) — are anchor-model doctrine; the anchor spec graduates
from the
[archived paper](../../../archive/model-v2/models/anchor.md) into the
wg tree in Phase 0's docs lane, with its home cluster chosen by the
docs-placement doctrine at graduation.

### Text

The chassis owns the seam: a backend-neutral shaped-layout artifact
behind a versioned oracle trait — shape once, record glyph placements,
replay them downstream, never reshape. The know-how owns the coverage:
decorations, font features, letter/word spacing, RTL run handling,
fallback management, ellipsis and line clamping, stroked text, and the
full text-editing stack (sessions, selection, caret metrics, history).
All of it migrates through the artifact, not around it. Known finding:
paragraph base direction is latin-first on the legacy side today; the
end state resolves base direction from content and authored intent.
The text-oracle identity — stay on the lab shaper vs move to a
production font stack — is decision **D-H**, taken only at the Phase 5
boundary, because flipping the oracle re-blesses both engines
atomically.

### Painting

The program's center of gravity. The shared surface is the leaf paint
vocabulary (the ratified
[paint-model RFD](../feat-painting/paint-model.md)): ordered paint
stacks, stroke applications, and tri-state text-run paint ownership.
The know-how is everything the legacy engine knows *above* that
vocabulary: how each of its 19 node types compiles to paint — shape
derivation, boolean flattening, virtual grouping, mask groups — and
the effects estate: gradient semantics, render-intent-keyed image
sampling, image filters, drop and inner shadows, noise, blur. The
migration order: the vocabulary becomes a shared crate (Phase 1); a
conformance suite runs the paint RFD against both engines' types and
yields a gap report; per-node capabilities then land in the chassis
gated by the scoreboard. Display lists stay per-engine; so does
compile policy.

### Realtime performance

The chassis brings the sockets: damage as data (frame products diff
into precise change sets), stable node identity for cache keys, and a
scene raster cache that re-composites whole frames under camera
motion. The know-how brings a *measured* estate: cache promotion with
heuristics tuned by benchmarks (including refusing to cache
actively-edited nodes), whole-frame reuse under pan and zoom,
input-cadence-aware frame stabilization (interaction-quality frames
while the user moves, full-quality on settle), reduced effect quality
during interaction with cache-variant hygiene, budgeted progressive
re-rasterization after zoom, and camera quantization for pixel-stable
blits. In the end state that estate returns as **preview-mode policy**
layered above the pure core — never as semantics inside resolve or the
display-list build. Whether camera and hot-loop edits become sampled
inputs under the same time model as animation is decision **D-K**.

### SVG — two surfaces

Two distinct product surfaces share the name:

- **Import-to-document.** The existing import pipeline's intermediate
  representation becomes a model-agnostic crate (Phase 3); the legacy
  engine keeps consuming it unchanged, and the chassis gains its own
  packer against it. Constructs the model cannot express become honest
  `UNSUPPORTED` scoreboard rows — never silent shims.
- **Render-to-pixels.** The SVG render CLI builds on the
  browser-grade-cascade lineage — SVG styled by real CSS resolution —
  aiming at the `svg in → pixels out` product
  ([gridaco/nothing#13](https://github.com/gridaco/nothing/issues/13)).

Stated program default (owner may override at Phase 3 entry, where the
charter's **NAME** decision also confirms this reading): the two remain
distinct surfaces — a document import and a rendering product. SVG
*animation* is already chassis territory (see Animation & time).

### HTML/CSS

Wholly know-how today, and mature (the owning domain doc:
[htmlcss](../feat-2d/htmlcss.md)): a real browser cascade
implementation resolving computed styles, a layout mapping that
includes CSS grid, paint with the long tail of CSS visuals, and a
web-platform-tests harness grading it against the Chromium bar
continuously. End state: the styled-tree front-end is extracted and
shared; the chassis consumes it through an adapter that maps styled
elements onto the model; text on both sides flows through the
shaped-text artifact of the
[text-layout RFD](../feat-paragraph/text-layout.md). Gated by the
font-provider decision (**D-D**) and the closed-set architecture test
(Phase 4).

### Export

The accurate-static render mode made concrete: raster formats, PDF
including multi-page documents, and SVG out — always at full render
intent, camera aimed at a node or scene, resolution-scaled without
resampling artifacts. Wholly know-how today. Granted to the chassis
before Phase 6's contraction (a charter amendment — the original
contraction list omitted it). Export doubles as the conformance path:
what the scoreboard grades is what export produces, so the export
surface inherits every conformance result for free.

### Editor core

The chassis carries the operation journal and replay — editing as
time-as-data, sessions as corpora that serve repro, bench, fuzz, and
conformance at once. The know-how carries the editing vocabulary: a
working copy over the document, invertible serializable mutations,
history with origins, a closed command registry with keybinding
chains, intent interpretation (gesture → mutation), and the sync model
— authority-ordered optimistic replication per
[feat-crdt](../feat-crdt/index.md). Whether the end-state editor core
extends the chassis's ops or graduates the legacy editor core onto the
chassis is decision **D6**, decided by a timeboxed mapping spike
against the graduated anchor spec — before the two accrete
independently.

### Server & headless

The end state runs the engine as the document authority behind the
CRDT server: fully headless, deterministic, and buildable with **no
graphics backend at all**. The model family already meets this
requirement — model, operations, resolution, picking, and animation
sampling are backend-free. The engine crate does not yet: its raster backend is
confined by design but unconditional in the build, so a build split
must exist before a backend-free server build does (priced in the
charter's amendments). Deterministic text exists via the lab oracle;
production-grade server text hangs on **D-H**. Transport, persistence,
and presence are host concerns — the engine ships document semantics,
not a server.

### Wasm

The published v1 wasm package is the freeze-contract surface and the
product's only consumption point today; its publishing obligations
continue unchanged through the program. The chassis has no wasm target
yet — the port is priced work, not an assumption — and the switch
(package identity, soak window, product-side unpin, freeze-contract
retirement) is Phase 6's **D-I**, coordinated with the product side.
Until then, no new capability accretes to the v1 package.

### Format & storage

The canonical engine contract is the in-memory model and its operation
vocabulary. `.n0.xml` is the authored source form (the
[n0 XML RFD family](../format/n0-xml.md); root-element identity is
still open in its ratification pass). The v1 binary (`.grida` /
FlatBuffers) is legacy input through an explicit converter —
converter-shaped forever, never a native dialect. What replaces it as
the archive/interchange form — ratified n0 XML, a future v2 binary, or
a host-owned format with engine-provided tooling — is **D-G(b)**
widened by **D-J** (the host-managed posture in
[goal.md](./goal.md)). The v1 schema is a frozen surface until
Phase 6.

### Foundations

The model-agnostic satellite crates both engines stand on: math
(geometry primitives and transforms), font introspection (parsing,
feature enumeration, family selection, webfont metadata — distinct
from shaping, which lives behind the text oracle), and the cascade
engine. The end state keeps this tier as shared foundations consumed
by contracts and engines alike; vocabulary crates cut during the
program (paint first, the SVG IR next, the styled-tree front-end
after) join it. Each cut settles its own math vocabulary at the moment
of cutting, per the method's naming step — never before; the SVG IR's
cut carries the charter's **NAME** decision.

### Verification estate

The program's instrument tier — shared, not owned by either engine:
the reftest tooling and its scored suites, the web-platform-tests
harness, the replay corpus (one corpus serving repro, bench, fuzz, and
conformance), the honesty rigs (byte-identical goldens, double-run
determinism checks, budgeted benches that fail on regression), and —
the program's spine — the v1-vs-v2 **scoreboard** with a flip rule
ratified before any score exists. Corpora live under the repo-root
fixtures tree; the large suites are machine-local by convention, so CI
hosts a declared subset plus the baselines, and pretends nothing.

### Dev shells & observability

Each engine keeps its dev shell for the program's duration: the legacy
dev CLI (benchmarks, reftests, export tooling, the render seam the
scoreboard will call) and the v2 windowed shell (direct-manipulation
feel spikes, the live animation player, headless screenshot and bench
modes). They converge on the dev editor as capability flips. Tracing
stays feature-gated and zero-cost when off. Shells are hosts: nothing
in a shell may become load-bearing engine semantics, and anything a
shell proves must graduate into the engine or a contract to count.

## Render modes — one time model

The chassis already makes time an explicit input. All three render
modes are **policies over the same pure core**; they may differ in
*when and at what quality* they paint, and must never differ in *what
things mean*. Meaning lives in the resolved document and the paint
vocabulary — a mode cannot change it.

- **realtime preview** — the interactive canvas. Heuristics for
  stable frames under camera movement and hot-loop editing (frame
  reuse, quality reduction, budgeted re-rasterization) are welcome,
  and the legacy estate is the reference know-how — but they live at
  the host/compositor tier, never inside resolve or the display-list
  build. The candidate design treats camera and in-progress edits as
  sampled inputs under the same animation kernel; that unification is
  decision **D-K** — a design to be specified, not an existing fact.
- **accurate static** — the export path: full quality, no
  heuristics, deterministic output. The legacy export subsystem
  defines today's bar.
- **accurate animation** — offline export at exact sampled times;
  every frame is an accurate-static render at its declared time, so
  frame drops are structurally impossible. This mode already exists
  in the chassis for the SVG animation profiles.

## End-state crate silhouette

Deliberately coarse — each concrete cut gets its own naming exercise
at the moment doctrine arms it (second consumer), never before:

- the v2 engine family as landed: the backend-free model crate, the
  engine crate (gaining a graphics-backend-optional build to earn the
  server role), the dev shell;
- shared vocabulary and foundation crates, growing as cuts land — the
  paint vocabulary first, the SVG IR next, the styled-tree front-end
  after, alongside the existing math/fonts/cascade tier;
- the legacy family contracting by subtraction until only its Phase 6
  residue remains — the v1 compat adapters, the legacy paint compiler,
  the v1 format io, and the wasm publisher — and then, at Phase 6,
  retiring entirely;
- `archive/` holding each concluded program's frozen record — this
  program's eventual home too.
