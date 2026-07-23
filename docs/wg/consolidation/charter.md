---
title: The Consolidation Charter
description: "The ratified route of the consolidation program — phases with entry/exit gates, the doctrine, the owner decision registry, and the known unknowns."
tags:
  - internal
  - wg
  - program
format: md
---

# The Consolidation Program — one engine by extraction

**Status:** Active — governs the era after the v2 landing
([gridaco/nothing#5](https://github.com/gridaco/nothing/pull/5)).
Ratified by the owner's review of the panel synthesis (2026-07-19).
Tracking: [gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43)
(the program umbrella) and
[gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)
(the v2 model program). The program's destination and method live in the
sibling docs of this directory — start at [index.md](./index.md).
Predecessor: the legacy seam program
([gridaco/nothing#27](https://github.com/gridaco/nothing/issues/27) —
work complete; tracker open pending owner close).

**Genre:** ratified charter — the program's sequencing and decision
record, and the only doc in this cluster that orders work. It is
deliberately concrete: it names real crates, modules, and suites so
that cut targets are unambiguous; the sibling docs stay at domain
altitude.

**Route vs position:** this charter records the *route*. The current
*position* — active phase, landed PRs, taken decisions — is tracked on
[gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43);
read it before starting work.

## Doctrine (carried, not renegotiated)

- **Migration by extraction.** A module becomes a workspace crate when its
  second consumer appears in the workspace — and not before. The landing of
  [gridaco/nothing#5](https://github.com/gridaco/nothing/pull/5) put the second consumer (n0) in-tree, arming every seam certified by
  the M4 extraction-readiness review.
- **Absorption direction.** n0 absorbs the engine *role* by consuming
  crates extracted from `crates/grida` — never by copying. `crates/grida`
  is decomposed, not absorbed: its agnostic modules become shared crates;
  its adapters remain the v1 compat layer; its render estate remains the
  executable conformance bar until the scoreboard flips it.
- **Two gate classes.** *Zero-behavior* moves (crate cuts, re-exports) are
  gated by byte-identical sweeps — strictly stronger than any similarity
  score; they never wait for the scoreboard. *Capability-granting* moves
  (n0 gaining an importer) are gated by the scoreboard — no capability
  lands unmeasured.
- **Oracle discipline.** The conformance bar is the Chromium/consensus
  oracle, never v1 itself. Where v1 and n0 diverge and n0 is closer to the
  oracle, the divergence is a v1 finding.
- **Patrol-before-drop.** Before every deletion, replacement, or
  conflict resolution, a patrol triages the affected content and produces
  a captured-essence ledger; load-bearing caveats are re-homed, the rest
  named in the commit message. Applies to every absorption step.
- **Frozen surfaces.** `format/grida.fbs` (v1, SCHEMA_VERSION lockstep)
  and the `@grida/canvas-wasm@0.91.0-canary.22` freeze contract are
  untouched until Phase 6 says otherwise.
- **Owner gates.** Merges to main and every registry decision need the
  owner's explicit GO. All-green CI is necessary, not sufficient.

## Phases

| # | Phase | Goal | Entry gate | Exit gate |
|---|---|---|---|---|
| 0 | **Land + arm** | n0 on main; [gridaco/nothing#5](https://github.com/gridaco/nothing/pull/5)'s one-time proofs become permanent required CI checks; sequencing declared where eager extractions will see it | Owner GO on gridaco/nothing#5; CI green on the tip | gridaco/nothing#5 merged `--no-ff`; CI arming PR landed (pixel-sweep subset declared, seam arch tests, lock-additions-only, wasm build, CI-hosted v2 gate baselines); every decision below filed as a tracked issue; the two long-pole docs lanes started (anchor-spec WG graduation; htmlcss font-provider study) |
| 1 | **Vocabulary lane** (∥ 2) | One paint vocabulary | Phase 0 exit (does NOT wait for the scoreboard) | `cg` is a workspace crate (legacy consumes via re-export; sweeps byte-identical; skia-free lock becomes the crate boundary); paint-RFD conformance suite passes against BOTH cg and n0-model types via a trait harness, yielding a gap report; the two pinned RFD amendments ratified or re-pinned with named owners; D-C decided on the gap report; any adapter is empty or amendment-pinned with a deletion gate |
| 2 | **Instrument lane** (∥ 1) | The v1-vs-n0 scoreboard, CI-hosted, with flip rules written before any score exists | Phase 0 exit; zero-bridge constraint (only existing entry points: grida_dev's render seam for v1; a callable seam generalized from n0's render bins) | Scoreboard v0 report on main: per-fixture triples (v1-vs-oracle, n0-vs-oracle, v1-vs-n0) + per-engine coverage counts over an ENUMERATED intersection corpus; CI job with regression-vs-baseline failure, bless flow, committed Chrome bakes, hard wall-clock budget; the flip rule ratified as a short WG doc (incl. the oracle-discipline clause) |
| 3 | **SVG — first capability grant** | The SVG import IR becomes a model-agnostic crate with two real consumers; n0 gains SVG import via its own packer | Phase 1 crate cut landed AND Phase 2 exit; SVG IR crate name resolved (deferred once, not again); the crate's math vocabulary (math2 vs kurbo/n0-model-math) decided at cut | Legacy: pack.rs+grida.rs consume the crate; resvg / W3C 1.1 / oxygen-icons byte-identical. n0: packer with the dependency-direction lock (adapter depends on IR + n0-model; n0-model depends on NEITHER, arch-tested); unmappable constructs are UNSUPPORTED scoreboard rows, never shims; n0 SVG entry scores recorded |
| 4 | **HTML lane + editor lane** (concurrent) | HTML import shared via the styled-tree front-end (Stylo); D6 (editor-core ownership) decided before double-accretion | HTML: htmlcss closed-set arch test landed; D-D (font-provider) decided. Editor: anchor spec graduated; timeboxed D6 evidence spike done | HTML: frontend.rs + StyledElement extracted; legacy byte-identical behind the htmlcss golden gate (139 goldens today); n0 `from_styled` adapter scored; text stays on the text-layout RFD's artifact both sides. Editor: D6 executed |
| 5 | **Format + text oracle** | Every v1 `.grida` opens in n0; archive and oracle decided at a safe boundary | Anchor spec ratified; n0 XML RFD ratification pass complete (root-element identity settled); D-H evidence in hand | `.grida`→n0 converter (frozen fbs is read-only input; converter-shaped forever), scored against v1's rendering of a pinned real-document corpus; D-G(b) decided (n0 XML vs future v2 binary archive); D-H decided at this boundary only — an oracle flip re-blesses both engines atomically |
| 6 | **Flip + retirement** | One engine by subtraction | Scoreboard meets the Phase-2 flip criterion (not renegotiated); D6 executed; converter shipped; baselines are trend, not noise | Per-suite conformance-bar flips; `crates/grida` contracts monotonically to v1 adapters + `painter/compile.rs` + fbs io + wasm publisher (each deletion gated by wasm build + remaining byte gates; reversible per-suite); D-I: wasm switch, soak window, grida-side unpin coordination, freeze-contract retirement, archive branch |

## Web-first capability sequence (ratified amendment)

The [Web-First Amendment](./web-first.md) moves the Web semantic family ahead
of the phase ordering above and supersedes the legacy-adapter routes in Phases
3–4 where they conflict. The phases retain the program obligations that do not
conflict: owner gates, FLIP, oracle discipline, the format lane, editor lane,
and retirement by subtraction.

The Web-first path advances one capability at a time. A row does not start
until the prior row has its contract, gate, and record; a capability does not
land merely because its implementation exists.

| order | Capability outcome | Entry obligation | Exit evidence |
| --- | --- | --- | --- |
| 0 | **Proving shell only** — one namespace-aware document, one Stylo cascade, one provisional source-neutral resolved contract, one private drawlist, one painter | Amendment ratified | Solid-fill rectangle fixtures are fully enumerated and exact against committed Chromium oracles; dependency direction and the tiny n0 canary are locked. This is infrastructure, not an SVG capability claim |
| 1 | **SVG vector** — paths, basic shapes, paint/stroke, viewport mapping, groups/transforms, opacity, clips, gradients, and patterns | D-L decided; a conforming SVG/XML grammar entry, SVG-namespace stylesheet intake, presentation-hint precedence, and host-declared semantic environment exist; D-M/D-C decided before the provisional contract expands beyond its quarantined solid-fill leaf into stroke/gradient vocabulary | Probe-friendly primitives pass their Chromium/consensus gate; unsupported constructs are explicit; FLIP is ratified and the applicable capability gate passes |
| 2 | **Shaped text** — SVG text/tspan, then HTML inline text, through a neutral shaped-text artifact and font-key/resource-environment boundary | SVG vector exit; the two-producer text spike required by D-M exists; the relevant text/font oracle decision is registered before choosing a bar | Structure/geometry/computed-style assertions and the legally available text pixel gate pass for both producers |
| 3 | **Images and resources** — SVG image, HTML img, and CSS background images through the declared resource environment | Shaped-text exit; the image resource-environment contract and any newly forced registry decision are recorded | Resource fixtures are deterministic with no ambient I/O; exact or consensus gates pass |
| 4 | **CSS box and layout** — block flow, flexbox, grid, backgrounds, borders, radius, shadows, gradients, and the CSS visual long tail | Resource exit; host inputs reach semantic resolution before frame production | Primitive and resolved-frame gates pass against Chromium; every admitted shared field remains a normalized visual fact |
| 5 | **Page composition** — the article, landing, and docs fixtures render as complete documents | CSS box/layout exit | All three page fixtures meet their declared Chromium reftest floors, with content masks and font-fragile data assertions recorded rather than hidden by thresholds |

## First three PRs after merge

1. **CI arming** — promote gridaco/nothing#5's proof machinery into required main checks;
   host the v2 gate baselines in CI (retiring the machine-local baselines
   whose environmental variance gridaco/nothing#5's A/B documented). Carries the non-PR
   obligations in its wake: the issue-filing sweep and the gridaco/nothing#9 sequencing
   declaration.
2. **cg crate cut** — naming exercise, then `crates/grida/src/cg` becomes a
   workspace crate; legacy consumes via re-export. Gates: pixel sweeps and
   goldens byte-identical, lock additions-only, wasm green. Contains NO n0
   changes.
3. **Scoreboard v0** — a `grida_dev scoreboard` subcommand over the
   zero-bridge intersection corpus; first deliverable is the corpus
   *enumeration*; ships with the draft flip rule. (PR 4, right behind: the
   paint-RFD conformance suite + cg-vs-n0-model gap report.)

## Decision registry (owner decisions, each with its evidence bar)

| id | Decision | When | Evidence required before deciding |
|---|---|---|---|
| GO | Merge gridaco/nothing#5 | **taken** 2026-07-19 (gridaco/nothing#5 merged as `a2d7c290`) | CI green on the tip; sweep evidence current |
| AMD | Paint-RFD amendments (diamond-gradient extension; tri-state run-fill) | Phase 1 | named owner + drafted amendment text; gates D-C and adapter deletion |
| D-C | n0-model adopts extracted cg types per-leaf vs keeps its own behind a law-equivalence mapping test | Phase 1 exit | the [paint-vocabulary conformance gap report](./paint-vocabulary-gap.md) |
| FLIP | [FLIP proposal](./flip-rule.md) (unratified: per-suite thresholds, coverage requirements, oracle-discipline clause) | Phase 2 — before any score exists | scoreboard v0 design + corpus enumeration |
| NAME | SVG IR crate name + its math vocabulary (math2 vs kurbo); also confirms the two-surface reading (import-to-document vs render-to-pixels) stated in the [topology](./topology.md) | Phase 3 entry | naming exercise per doctrine |
| D-L | SVG paint enters the shared cascade through Servo-capable support maintained in official upstream Stylo. Prefer the first published release containing [servo/stylo#383](https://github.com/servo/stylo/pull/383); until one exists, use immutable official-upstream revision `a64923b5d5c67313c81c5056f5e30ec0babb04d6`. A floating branch and private source fork are outside the decision. This settles paint dependency provenance, not ingress or capability | **taken** 2026-07-23 | the [SVG-paint-cascade finding](./svg-paint-cascade.md) plus a bounded feasibility bundle covering the real SVG/XML grammar entry, SVG-namespace stylesheet intake, presentation-hint precedence, minimal paint-longhand computation, and precedence/currentColor/invalid-value behavior |
| D-M | Staged n0 join per fact kind. **Vector stage (open):** the leaf-vocabulary seat, glyphless semantic level, and proving-shell disposition. **Text stage (not yet ripe):** the shaped-text/font/resource join. Coupled to D-C per admitted fact rather than inferred from the rectangle canary | Vector stage before stroke/gradient enlarges the provisional contract; text stage before any shared text promotion. Vector may leave text open only if one private orchestration seam proves the two outcomes composable | Vector: the [n0-join-point finding](./n0-join-point.md), [Web renderer adoption patrol](./web-renderer-adoption.md), D-C paint/stroke gap report, complete n0 compiler-read inventory, and independently constructed normalized-input spike proving drawlist/raster equality plus multi-frame identity, damage, cache, and mixed-fact composition. Text: a two-producer shaped-text/font-key/resource-environment spike |
| D-D | htmlcss font-provider seam (flagged open at M4) | Phase 4 HTML entry (study starts Phase 0) | the WG study |
| D6 | Editor-core ownership: grida_editor vs n0 journal/ops ([gridaco/nothing#1](https://github.com/gridaco/nothing/issues/1) is the migration-anchor context; a dedicated D6 issue is filed in Phase 0's registry sweep) | Phase 4, concurrent lane | timeboxed spike mapping the legacy editor core's operation catalog against the graduated spec (scoped subset, not the full catalog) |
| D-H | Text-oracle identity: stay on `skparagraph@skia-0.93.1` vs fonts-backed production oracle | Phase 5 boundary only | crates/fonts contract + a differential run |
| D-G(b) | v2 archive story: ratified n0 XML vs a future v2 binary | Phase 5 | n0 XML RFD ratification pass |
| D-J | Format stewardship: binary storage is host-managed with engine-provided tooling (widens D-G(b) — the engine's canonical contract is the in-memory model + ops) | Phase 5 boundary, with D-G(b) | converter experience + n0 XML ratification pass |
| D-K | The unified time model for realtime preview (camera and hot-loop edits as sampled inputs under the animation kernel) | when the preview lane starts | a render-modes design doc against the [end-state topology](./topology.md) |
| D-E/D-I | D-E: the per-domain bar flips; D-I: the wasm switch (package identity, soak, grida-side unpin, freeze-contract retirement) | Phase 6 | the Phase-2 criterion read off the board |

## Amendments (end-state sync, 2026-07-19)

Ratified when the owner's end-state overview was reconciled against both
engine families (see [goal.md](./goal.md) and
[topology.md](./topology.md)):

- **Export lane.** The legacy export subsystem (raster formats, PDF, SVG
  at full render intent) is the accurate-static render mode and must be
  granted to n0 before Phase 6's contraction — the original contraction
  list did not name it. Scoped into Phase 5 alongside the format
  converter; Phase 6's per-suite flips include the export surfaces.
- **Engine-ecosystem pricing.** Two priced workstreams gate the
  engine-ecosystem goal and are not free: (a) a graphics-backend-optional
  build of the engine crate (the raster backend is confined by design but
  unconditional in the build today — a feature split must exist before a
  backend-free server build does); (b) the v2 wasm target (nothing
  exists; the D-I switch assumes a port that must be priced as work).
  Both are demand-driven lanes: entered when their consumer appears (a
  server host; the wasm switch), after Phase 2's instruments exist and
  before anything in Phase 6 depends on them.
- **Product surfaces.** The render-CLI products (the render ecosystem in
  [goal.md](./goal.md)) are parallel product tracks
  ([gridaco/nothing#13](https://github.com/gridaco/nothing/issues/13)
  et al.) that consume capabilities as phases grant them; the program
  guarantees engine capability, not CLI packaging.
- **Registry additions.** D-J (format stewardship) and D-K (the unified
  time model for realtime preview) entered the decision registry above.

## Known unknowns (flagged honestly, priced into the phases)

- The intersection beyond scoreboard v0's enumerated direct rect/path proving
  shell remains unknown; the first corpus makes no general-SVG coverage claim.
- "n0 adopts cg types" is mechanical only if resolve/paint semantics are
  bit-identical — the conformance suite exists to measure exactly that
  (gradient-stop interpolation, tile-mode edges, stroke ordering).
- The callable chassis render seam remains bounded to that proving shell; a
  general SVG entry point is Phase 3 capability-grant work.
- The full byte-identical sweeps cannot run at fidelity in CI (untracked
  `fixtures/local` corpora); the CI-arming PR must encode a *declared
  subset* and name the local-only suites per-PR, not pretend.
- A slow or flaky scoreboard gets ignored and the program silently reverts
  to faith — the hard wall-clock budget and committed Chrome bakes are the
  countermeasure, not niceties.
- Phase 6's grida-side unpin is coordination this repo cannot green-light
  alone; the freeze contract retires only with the product side.

## Provenance

Synthesized from a three-lens design panel (extraction-first,
verification-first, product-arc) plus adversarial judge, run 2026-07-19
(the panel transcript is unarchived; the ratified text herein supersedes
it); grounded in the seam program's M4 extraction-readiness review
(recorded on
[gridaco/nothing#27](https://github.com/gridaco/nothing/issues/27)), the
ratified [paint-model RFD](../feat-painting/paint-model.md), the
[display-list contract study](../feat-2d/display-list-contract.md) (no
shared display list — the leaf vocabulary is the whole shared surface),
and the [text-layout RFD](../feat-paragraph/text-layout.md).
