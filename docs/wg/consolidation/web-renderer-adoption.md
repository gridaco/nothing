---
title: "Finding: how the mature Web renderer enters the chassis"
description: "Patrol and topology evidence for adopting the existing HTML/CSS/SVG capability without carrying its legacy engineering failures forward or growing a third engine."
tags:
  - internal
  - wg
  - program
format: md
---

# Finding: how the mature Web renderer enters the chassis

**Genre:** finding — a non-normative implementation inventory, patrol record,
and evidence memo. It is not a contract, a work plan, or an owner decision.

**Status:** open evidence for the staged **D-M** decision, 2026-07-22. The
[Web-First Amendment](./web-first.md) governs the source topology. The
[goal](./goal.md), [topology](./topology.md),
[glossary](./glossary.md), and ratified
[paint model](../feat-painting/paint-model.md) establish the shared leaf
vocabulary and engine-private drawlist. The
[display-list study](../feat-2d/display-list-contract.md) records supporting
evidence; it is not itself governing doctrine.

## The question

The repository already contains a substantial static HTML/CSS/SVG renderer.
The Web-first proving shell deliberately did not consume it and therefore
renders only its declared solid-rectangle slice. The question is not whether
the mature capability matters. It is **which parts are Web know-how, which
parts are failed legacy topology, and what evidence is still missing before
that know-how can enter the chassis without creating a third engine**.

Patrol supports migration by extraction, not relocation. The mature Web
semantics and their oracle fixtures are the estate to preserve. The direct
backend renderer, ambient process state, and legacy importer seam are not a
package boundary. The exact n0 join and the proving shell's disposition remain
an owner decision because the present n0 compiler has not yet proved that it
can consume a source-neutral resolved input.

## Three paths exist today

Capability claims must name the path they describe.

1. The mature direct renderer is
   [`crates/htmlcss`](../../../crates/htmlcss/src/lib.rs), extracted from the
   legacy crate behind a compatibility re-export. Its
   HTML path is parse and cascade → Web-private styled tree → layout → direct
   backend paint. Its SVG path is a broad direct DOM/resource/paint walk. The
   renderer itself does not consume the legacy node model; that coupling begins
   in the separate
   [HTML importer](../../../crates/grida/src/import/html/mod.rs).
2. The chassis is the n0 document/effective-values → resolve → private
   drawlist → paint path, with an immutable
   [frame product](../../../crates/n0/src/frame.rs). It is a real engine, but
   it is not an HTML/CSS semantic model or a general SVG renderer.
3. The Web-first proving shell is
   [`websem`](../../../crates/websem/src/lib.rs) →
   [`rframe`](../../../crates/rframe/src/lib.rs). It proves that one
   namespace-aware document can reach a source-neutral result and a direct
   painter for a solid rectangle. It did not adopt the mature renderer's
   feature estate.

The third path is therefore not evidence that the first path's capability was
lost. It is evidence that the proposed boundary works for one deliberately
small fact.

## Capability and gate census

The mature HTML estate has 139 committed fixtures. The
[exact manifest](../../../fixtures/test-html/suites/L0.exact.json) contains 65
entries and the
[coverage manifest](../../../fixtures/test-html/suites/L0.coverage.json) 68,
with 64 paths duplicated between them. Their union is only 69 fixtures: one
exact-only, four coverage-only, and 70 in neither manifest. The overlap also
violates the documented promotion rule, which says an exact fixture moves out
of coverage.

The exact manifest's `1.0` Chromium floor is not raw PNG byte identity. It uses
zero pixel threshold with anti-alias exclusion, hides text, makes the body
transparent, and sometimes supplies a fixture-specific viewport to match the
renderer cull. Those conditions are valid when named; they must not be
summarized as unconditional browser parity.

The direct SVG renderer contains 47 focused unit tests and broad feature code.
Its 1,679-case conformance corpus is machine-local by fixture policy. There is
no committed standalone/inline-SVG byte-sweep manifest, and
`svg-inline-basic.html` is one of the 70 unregistered HTML fixtures. The
focused SVG checkpoint has no independent oracle and proves only that expected
pixels are present. These facts establish valuable capability and a large
patrol surface. They do **not** establish a current general-SVG parity score.

## Patrol by stage

| Stage | Captured essence | Topology that must not cross |
| --- | --- | --- |
| Document and cascade | Namespace-aware DOM, Stylo selector and cascade behavior, an actual UA-origin sheet, author-sheet handling, and computed values | Process-global leaked DOM storage; context-free node handles that manufacture global references; an external single-document-at-a-time requirement; fixed viewport, color scheme, time, and placeholder font metrics disguised as universal defaults |
| Web-private style | Broad computed-CSS mappings and plain value records for backgrounds, borders, gradients, filters, transforms, fonts, layout, lists, and widgets | The existing `StyledElement` as a shared contract: it contains HTML tags, ordered DOM children, replaced-element attributes, widget/list semantics, and serialized SVG XML. Only individual computed-value leaves may later prove reusable |
| HTML layout | Taffy mappings for block approximations, flex, grid, intrinsic replaced sizing, and accumulated layout caveats | Backend paragraph objects used for measurement and line layout; table and inline-formatting approximations treated as standards; source tags leaking into a source-neutral contract |
| SVG semantics | Element classification, lengths, viewports, transforms, paths, bounds, paint servers, markers, use trees, clips, masks, filters, and many bug-driven edge cases | The temporary SVG-only cascade; backend paths, colors, matrices, shaders, or canvas operations inside semantic contracts; inline SVG represented as an out-of-band serialized document in the new path |
| Resources and text | Existing image, CSS, font, fallback, text, and SVG-resource behavior; host machinery in uneven path-specific forms | Decoded backend images and typefaces in public provider traits; ambient system-font discovery; shaping performed during paint; an assumption that the thin HTML host already resolves external resources—it currently supplies no images |
| Paint and host | Existing paint-order behavior, backend realizations, and thin CLI/reftest host experience | Direct canvas traversal, nested backend pictures, source parsing, PNG/oracle utilities, I/O, or ambient clock in the engine core. Every behavior must be re-derived against Chromium or consensus, never accepted because legacy emits it |

A bulk move would preserve pixels by renaming the legacy renderer as the new
architecture. It would also export the very engineering failures the rewrite
exists to remove. The adoption unit is a proved semantic fact or utility, not a
directory.

## Why the high join is not proved yet

The proving shell makes a high join look cheaper than it is. `rframe` owns a
resolved frame, private drawlist, painter, and raster/PNG helpers, while n0
still owns its resolver, private drawlist, frame product, damage, caches,
resource checks, and painter. Growing both complete downstreams would create a
third engine regardless of whether a trait gave them the same method names.

The solid-rectangle canary also skips an important dependency. n0's current
drawlist compiler reads both the resolved tier and the authored/effective model:

| Current compiler read | Present purpose | Boundary question |
| --- | --- | --- |
| resolved world transform and resolved box | visibility pruning, geometry, and placement | plausible source-neutral facts, subject to coordinate-space and bounds laws |
| node identity carried into every draw item | drawlist identity and later frame comparison/damage behavior | n0's arena/generation identity is runtime-specific, while the current proving identity is frame-local; a high join needs stable cross-frame identity and provenance laws |
| document root, child order, and payload kind | traversal, backdrop exclusion, derived-node exclusion, and primitive selection | mixes source semantic topology with private compile policy; a high join must locate each fact explicitly |
| effective opacity and clip flag | emission and placement of composition scopes | visually neutral concepts, but their structural encoding is private compile policy |
| effective corner radius and corner smoothing | rectangle geometry | radius may be neutral; smoothing must become resolved geometry or remain below the join |
| effective fills and strokes | visible paint stacks and stroke items | blocked on the leaf-vocabulary seat and D-C equivalence evidence |
| resolved path artifact, including commands, fill rule, bounds, and contour closure | path fills/strokes and their geometry/renderability | plausible normalized geometry, but it needs a neutral path seat and explicit fill-rule/bounds laws |
| payload-dependent stroke filtering | decides whether a stroke is visible or meaningful for each primitive kind | private compile policy that needs a normalized primitive classification; copying the n0 payload enum would fail source neutrality |
| text payload, resolved text layout, run paints, and exact font registry | text item materialization and replay validity | deliberately excluded from the vector decision; requires the later two-producer text evidence |

This is not proof that a source-neutral compiler input is impossible. It is
proof that changing the compiler's dependency from n0 model state to a common
resolved contract is a semantic refactor, not a crate move. The smallest useful
high-join spike is private and independently constructed. Its normalized-input
and equivalence arm is glyphless; its orchestration harness also contains real
still-private text so mixed composition cannot be assumed. It must:

- express visual composition and painter order rather than copy an n0 document
  hierarchy into a differently named record;
- cover rect/ellipse/path/line geometry, fill rule, fill, stroke, opacity,
  clip, corners, and the payload-dependent admissibility rules above;
- compare the existing n0 input with an independently built equivalent
  normalized input, requiring both private-drawlist and raster equality;
- exercise at least two frames with stable identity/provenance and a property
  transition, requiring equivalent damage and cache behavior; and
- mix the admitted vector facts with still-private text in one ordered frame.
  If no one private orchestration seam can preserve that composition without a
  source-specific or opaque contract field, the vector stage is not
  independent and cannot be taken before text.

Every model read that cannot be represented without leaking n0 semantics
becomes evidence for a lower join. This spike proves feasibility; it does not
create the public contract.

## The D-M alternatives remain real

The evidence now leaves two coherent join shapes and rejects one incoherent
shape. The owner must choose per fact family.

### High join

Web and n0 independently lower equivalent visual facts into one provisional
source-neutral resolved contract. One engine-private compiler then projects
those facts into one private drawlist and executor with one set of frame,
damage, and cache policies. Under this option, the proving `Frame` may supply
contract evidence, but its downstream renderer does not survive beside the
chassis downstream.

This option best matches the one-engine destination. It is only valid if the
vector-input equivalence arm accounts for n0's current model reads without
moving source semantics or unresolved authoring intent into the common
contract.

### Low join

Web and n0 retain separate resolved forms, compilers, private drawlists, and
private orchestration executors. They share only the leaf-paint vocabulary and
realization utilities plus the raster backend. Under this option, the Web
proving shell may remain a Web-private projection, but each drawlist remains
paired with its own executor; no common painter consumes two unrelated private
drawlist types.

This option preserves the display-list study's per-engine reading and may be
required for facts that do not normalize honestly. It also leaves more than
one compile policy in the final topology, so D-M must explicitly judge whether
that is compatible with the program's one-engine requirement rather than
letting it happen by inertia.

### Rejected: parallel kernels

Keeping the mature direct painter, widening `rframe` into a complete renderer,
and retaining the n0 chassis as three peer engines is not a D-M option. It
violates the amendment before any question of implementation quality arises.

The finding recommends the **high join for glyphless visual facts if and only
if the full equivalence and mixed-composition spike passes**, with a lower join
for any fact that fails the source-neutrality test. That is evidence offered
for owner GO, not a taken decision. Text remains separately open only if the
mixed-composition condition proves that the stages are separable.

## Constraints that hold under either join

- HTML and inline SVG remain one namespace-aware Web document under one Stylo
  cascade. Standalone SVG is another grammar entry into that same Web semantic
  family. The new path never serializes and reparses inline SVG.
- The mature path currently does serialize inline SVG, skips its descendants,
  and reparses it during paint. During extraction, any byte-preserving legacy
  renderer must receive that string through an explicitly legacy-only
  compatibility projection or sidecar. It may not contaminate the new
  Web-private tree or the source-neutral contract.
- Web-private style may contain tags, text, DOM order, replaced-element
  semantics, and layout intent. The shared contract may not. Computed CSS leaf
  records earn a more general home only after a second independent producer
  actually needs the same meaning.
- A document-scoped cascade is not a wrapper around the current global slot.
  The Stylo handles themselves need session context and bounded lifetimes.
  Making the existing semantic environment explicit and replacing the global
  DOM/session are distinct zero-behavior changes with distinct gates.
- The process-wide `layout.grid.enabled` preference affects every Stylo caller,
  including the importer. It must become a declared process policy or a proved
  session policy; it is not renderer-local state.
- No legacy node model, SceneGraph, `.grida`, `grida.fbs`, backend object,
  picture, callback, I/O policy, or serialization promise enters a new Web or
  shared semantic contract.
- Source-specific lowering stays with its producer. A common contract, if
  admitted, owns only normalized visual facts with proved identity,
  coordinate-space, bounds, ordering, and resource-environment laws.
- Resource-environment identity is required only for facts that reference a
  resource. Geometry-only products remain valid without inventing one.

## Existing evidence and missing gates

| Surface | Durable evidence today | What is still missing before adoption |
| --- | --- | --- |
| Legacy HTML render stability | The consolidation A/B sweep enumerates the 65-item exact manifest | A deduplicated renderable manifest and recorded disposition for all 139 fixtures; overlap repair; byte-identical old/new evidence over that declared set |
| Chromium HTML oracle | The 65-item exact manifest declares threshold, AA handling, helper CSS, viewport, and `1.0` floor | A durable producer-and-diff gate and honest registration of the other 74 paths; no threshold may hide a divergence |
| HTML importer | The snapshot suite exercises all 139 fixtures; the architecture suite separately scans dependency direction | Unchanged snapshots through any shared cascade/style extraction and explicit tests for cross-caller process state |
| SVG and inline SVG | 47 focused unit tests, a presence checkpoint, and a reproducible local 1,679-case corpus | A new committed standalone/inline-SVG manifest with an independent Chromium/consensus oracle; the local corpus remains supplemental unless fixture policy changes |
| n0 chassis | `cargo test -p n0-model -p n0` plus the n0 gate's four shots and three replay scenes | For compiler generalization, drawlist equivalence on the normalized-input spike and unchanged existing render/replay bytes; do not claim a broader n0 XML/animation byte corpus until one is declared |
| Architecture | Existing source scans lock several dependency directions | Locks appropriate to the chosen join, plus proof that no new Web/shared contract depends on legacy nodes, a backend, host I/O, or ambient process state |

No row above is a FLIP score. A capability is not landed until its applicable
independent-oracle gate is legally available and passes.

## Captured-essence ledger

The following behavior must be classified and re-homed or deliberately dropped
before any responsible path is replaced. Broad labels such as “layout caveats”
are not sufficient patrol evidence.

**Cascade and collection**

- computed-property mappings, including the renderer's round versus importer's
  truncate color-quantization behavior;
- the real UA-origin sheet separately from the Grida fallback **author** sheet
  injected when no HTML-namespace style block exists;
- the process-global Grid preference and the fixed 1280×720, DPR 1, light,
  time-zero, placeholder-font-metrics environment;
- root-margin stripping, list counters and HTML list attributes, widget
  synthesis and intrinsic defaults, inline grouping, and importer-versus-render
  collection differences;
- inline SVG descendant skipping plus serialize/reparse behavior as legacy-only
  compatibility evidence, never as the new representation.

**Layout, paint, text, and resources**

- disabled Taffy rounding; inline-block-as-conditional-flex and
  table-as-flex approximations; intrinsic image sizing and missing-image
  placeholders;
- requested-height behavior and content-height culling—the current HTML render
  entry ignores its height argument;
- SVG presentation precedence and silent-drop behavior; viewport and
  preserve-aspect rules; duplicate-ID, cycle, fallback, partial-path, paint
  server, marker, clip, mask, filter, and resource behavior;
- text shaping, fallback, decoration, positioning, and ambient system-font
  behavior;
- path-specific CSS/image/font host behavior, including the HTML WPT host's
  current no-image policy and the SVG reftest host's preload behavior.

**Fixtures and oracle conditions**

- the [HTML/CSS capability tracker](../feat-2d/htmlcss.md), all 139 HTML
  fixture dispositions, the exact/coverage overlap debt, and explicit
  registration of inline SVG;
- exact-suite AA exclusion, hidden-text and transparent-body helper CSS,
  per-fixture viewport/cull rules, deterministic-run requirements, and oracle
  provenance;
- SVG unit fixtures and the local-corpus acquisition/filter recipe; a declared
  committed SVG/inline-SVG subset for byte-identical old/new evidence in any
  zero-behavior move that claims that surface;
- independent Chromium/consensus references for that subset at the separate
  capability-grant gate, subject to FLIP;
- n0's time, identity, immutable-frame, resource, damage, and deterministic
  replay invariants.

**Do not adopt**

- the process-global leaking cascade document or context-free Stylo handles;
- fixed environment values disguised as universal defaults;
- inline SVG serialize-and-reparse in the new path;
- the temporary SVG-only matcher as the cascade of record;
- backend types in semantic, resource, geometry, or text contracts;
- direct backend-picture output or nested-picture escape hatches;
- legacy node/import types in the Web path;
- a third drawlist, painter, cache, damage system, or export kernel.

**Deliberate drops in this finding:** none. No production path is replaced or
deleted here. The mature renderer remains executable evidence and the proving
shell remains bounded while D-M and the missing gates are prepared.

## Decision state

- **D-L is taken (2026-07-23).** SVG paint uses Servo-capable support
  maintained in official upstream Stylo, preferring the first published
  release containing [servo/stylo#383](https://github.com/servo/stylo/pull/383),
  with the tested
  immutable official revision used until then. A floating branch and private
  source fork are excluded. This settles dependency provenance only; the
  production ingress, remaining property breadth, consumption, and capability
  gates stay open. The mature SVG matcher's breadth remains evidence, not
  permission to promote that matcher.
- **D-M vector stage**, coupled to D-C, decides the leaf-vocabulary seat, the
  glyphless join per fact family, and the proving shell's disposition. It needs
  the compiler-read inventory above, D-C's gap report, and the vector-input
  equivalence/mixed-composition spike before stroke or gradient enlarges
  the provisional contract.
- **D-M text stage** remains open until two real shapers exercise a candidate
  shaped-text/font-key/resource-environment boundary. A vector-stage decision
  can leave that later stage open only if the mixed-composition spike proves
  one private orchestration seam can interleave both outcomes.
- **FLIP** still gates every capability-landed claim. This finding reports no
  score and makes no bar-flip claim.

The implementation sequence and candidate cascade cuts are intentionally kept
out of this finding. The charter owns outcome sequencing; the local working
plan owns disposable implementation ordering.
