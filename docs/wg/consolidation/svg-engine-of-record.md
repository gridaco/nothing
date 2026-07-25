---
title: "Finding: the SVG engine of record"
description: "The owner-taken decision that the n0 path — one document, one cascade, one effective-value view, one compiler, one contract, one kernel — is the engine of record for SVG, static and animated; the mature Web renderer is demoted to a frozen semantics donor."
tags:
  - internal
  - wg
  - program
format: md
---

# Finding: the SVG engine of record

**Genre:** finding and decision evidence. Not a spec and not a plan. It records
the owner-directed resolution of the two-paths question that the
[Web-first prototype](./web-first.md) left open, and the scope of the
resulting routing decision.

**Status:** **D-N taken 2026-07-24** in the
[charter's registry](./charter.md), owner-directed in session. The n0 path
(`websem → rframe → n0`) is the engine of record for SVG — static and
animated — in this repository's product host. The mature static Web renderer
(`crates/htmlcss/src/svg/`) is demoted to a frozen in-repo semantics donor:
no new behavior lands there; evolution rungs port its semantics onto the
engine of record. This settles routing and succession, not capability: no
conformance claim is made or implied.

## The crux

Two SVG render paths existed and both were positioned to grow:

- the **mature static route** — `htmlcss::svg`, wide coverage, Blink-shaped,
  but transitional by its own documentation: a direct-Skia paint walk (a
  second painter beside the n0 kernel), an in-tree CSS matcher that "must not
  become the cascade of record" ([web-first amendment](./web-first.md)), and
  a deliberately static value model ("we collapse the animVal/baseVal split
  since Grida is static");
- the **n0 path** — the amendment's topology made real: one document
  (csscascade), one Stylo cascade, one effective-value view (Base |
  Sample t), one SVG compiler (websem), one resolved contract (rframe), one
  kernel (n0) — narrow coverage, but pixel-exact against Chromium on its
  admitted corpus, with exact-time sampling and seek-order determinism
  law-tested.

A previously drafted resolution — transplanting the time axis *into* the
mature renderer — was rejected by the owner on review: it would grow the
transitional painter and matcher, and it removed the n0 kernel from the
explicit render loop. The owner's direction, quoted for provenance:

> while doing the work, we will only focus to the new engine (n0) and not
> care about the legacy grida depending on htmlcss or other modules AT ALL.
> translation: you can leave grida broken, or even simply drop the htmlcss
> render capability from it. we can wire that back after work is done and
> proven, if that makes things simpler and more focused.
> TL;DR we are only interested in n0 path, as it is meant to be a legacy
> grida successor afterall.

## Evidence

- **E1.** The n0 path renders its enumerated corpus pixel-exact against
  independent Chromium bakes: the ten `fixtures/web-first/` primitives and
  the five `fixtures/web-first/animation/` frames (Base plus four exact
  signed-nanosecond samples), all through the one compiler and the one
  kernel, with byte-deterministic re-renders. Provenance is pinned in the
  committed `oracle-bake.json` manifests and re-verified by law tests.
- **E2.** Exact-time sampling on the n0 path is deterministic and
  seek-order independent: the retained-session laws
  (`crates/websem/tests/svg_animation_x.rs`) pin shuffled seeks, pre-roll,
  the freeze boundary at exact nanosecond granularity, and that sampling
  never mutates retained state. Kernel-side identity and damage laws live in
  `crates/n0` and `crates/n0-model`.
- **E3.** Beyond-slice constructs refuse loudly with the construct named
  (compile errors and sample-time animation refusals), never wrong pixels —
  the refusal discipline is itself law-tested (`typed_fill.rs`,
  `svg_animation_x.rs`, `standalone_xml_entry.rs`). The HTML entry has one
  scoped boundary of a different shape: it compiles exactly the document's
  **first inline SVG** — when that subtree is admitted the render succeeds
  and the surrounding page (layout, text, later SVGs) contributes nothing.
  That first-SVG-only contract is pinned by a host law, not left silent.
- **E4.** The mature renderer's own documentation marks it transitional:
  `crates/htmlcss/src/svg/README.md` scopes it static-only (SMIL out of
  scope), records the in-tree matcher as temporary ("Replacing the in-tree
  matcher with Stylo remains future work"), and reserves the animVal seam as
  unbuilt. Its pipeline is single-shot with no retained session.
- **E5.** The amendment's ruled-out constructions
  ([web-first](./web-first.md)) forbid the shapes that keeping two growing
  routes would produce: no temporary matcher as the cascade of record, no
  three renderers behind one trait, one shared downstream.
- **E6.** Time on the n0 path already satisfies the chassis invariants —
  explicit time, immutable frame products, no ambient clock: *time changes
  effective values; it selects no renderer, compiler, or painter.* The
  "static render evolves to take time t" property holds by construction:
  Base is the absent-time view of the same compile.

## The options

| # | Option | What it buys | What it costs | Verdict |
|---|---|---|---|---|
| 1 | Transplant the time axis into `htmlcss::svg` (retained session + animVal seam in the mature renderer) | immediate static+animated unification over the full mature static surface | grows the transitional direct-Skia painter and in-tree matcher; drops the n0 kernel from the explicit route; deepens what the Stylo and kernel swaps must later undo | **Rejected by the owner** |
| 2 | Keep both routes and grow both | no regression anywhere | the two-paths problem this decision exists to end; permanent double maintenance and divergent semantics | Not viable |
| 3 | Cut the product host over to the n0 path now, with loud refusals outside the admitted slice; demote `htmlcss::svg` to a frozen semantics donor mined by evolution rungs | one engine of record; the succession the owner directs; refusal-gated honesty; mature semantics still harvested, not re-derived | an explicit, enumerated capability regression at the host until rungs land | **Chosen** |

## Decision and scope

> The n0 path is the SVG engine of record. The product host (`n0_cli`)
> routes standalone SVG and HTML documents through the retained websem
> session — static renders are Base; `--time-ns` renders are exact-time
> Samples — painted only by the n0 kernel. The mature `htmlcss::svg` route
> retires from the host. Outside the admitted slice, the host refuses
> loudly with the unsupported construct named.

Scope and consequences, pinned:

- **Capability regression, stated honestly.** SVG constructs the mature
  route rendered — basic shapes beyond rect, paths, strokes, gradients,
  text, filters — now refuse at the host with named errors until their
  evolution rungs land; the refusals are the capability statement. General
  HTML pages regress along the HTML entry's contract instead: the host
  renders the page's first inline SVG when it is admitted (and refuses by
  name when it is not); everything else on the page contributes nothing.
  Both boundaries are pinned as host laws — the refusal pins and the
  first-SVG-only pin — so the regression is observable, never silent.
  Wire-back of any legacy capability is deferred, per the owner's
  dispensation quoted above, until the work is done and proven.
- **`htmlcss::svg` is a frozen donor.** Its Blink-anchored modules (typed
  attribute grammars, path data, viewport mathematics, resource resolution,
  paint order) are the reference and source material that evolution rungs
  port onto the engine of record — onto the cascade of record (Stylo), not
  the in-tree matcher. No new behavior lands in the donor.
- **Legacy `grida` may break.** This decision does not touch it; future
  rungs that extract donor modules may leave `crates/grida` (and its
  htmlcss integration) broken or drop its render capability outright. The
  engine-of-record gates exclude legacy crates.
- **Evolution program.** Per-feature and per-attribute rungs, each
  Chromium-baked and law-pinned, tracked the way CSS properties are
  tracked — admitted-set code, refusal-law tables, and the fixture corpus
  are the tracker. Static rung order: basic shapes (circle/ellipse/line) →
  full viewport semantics (preserveAspectRatio) → paths → strokes →
  translucency and opacity scopes → gradients and paint servers → text.
  Animation rungs (further attributes, elements, and the CSS×SMIL
  precedence surface) ride the `animation-sampling` kernel, which already
  models more than the SVG front-end admits.
- **What this does not supersede.** Patrol-before-drop, oracle discipline,
  the frozen surfaces (v1 schema; the published wasm freeze contract), and
  the two gate classes all stand. [FLIP](./flip-rule.md) remains
  unratified: this is a routing and succession decision with an explicit
  regression, not a conformance or parity claim, and no scoreboard artifact
  accompanies it. A capability is still not "landed" until its required
  gate is legally available and passes.
- **Naming note.** After the cutover the `n0` binary finally *is* the n0
  engine end to end. The `D-K` time-model decision (realtime preview) is
  untouched; this record is exact-time file rendering only, and D-K keeps
  its own trigger and evidence bar.

## Subsequent status (2026-07-24): best-effort becomes the host default

Owner-directed, same day, after the cutover landed. The host's default
admission flips from refuse-loud to **best-effort with declared
degradations**; `--strict` keeps the refusal harness. The owner's direction,
quoted for provenance:

> make best effort by default, and have a flag to noisy fail (the current
> default) to be explicit - for dev purposes, our own harness and TODO
> reminder.
> why: things will grow slowly, and the practical default is to "just
> render"

What this amends and what it preserves:

- **The invariant is unchanged, restated precisely: never *silent* wrong
  pixels.** Under the default, beyond-slice subtree constructs are skipped
  and a beyond-inventory dynamic surface samples as the Base view — each
  declared on stderr with its stable node path and reason
  (`degraded: skipped svg/circle[1]: unsupported element <circle>`), and
  the render line carries the degraded count. A declared hole is not a
  guessed pixel; nothing renders *wrong*, some things render *absent* — by
  name.
- **The patrol is per attribute and per cascaded property, not just per
  element.** An admitted element carrying a known rendering-relevant
  attribute the slice does not consume (`opacity`, `transform`, `rx`,
  `stroke…`, the enumerated set in the compiler) skips-and-declares by
  default and refuses under `--strict` — it is never painted wrong; only
  attributes outside the SVG rendering vocabulary are ignored, exactly as
  Chromium ignores them. The cascaded surface is patrolled for the
  enumerated properties (`opacity`, `display: none`, `visibility`, shape
  `stroke`, beside the typed `fill`/`fill-opacity` reads); cascaded
  properties beyond that enumeration remain a **named open boundary** of
  the slice, recorded in the compiler's module doc — not a coverage claim.
  `<script>` in a standalone document refuses in both admissions: the XML
  parse suspends at the element and content after it would be a silent,
  undeclared hole. A script inside the compiled inline SVG of an HTML page
  refuses likewise, at any nesting depth — a load-time script can rewrite
  the authored state the Base view renders; scripts elsewhere on the page
  stay under the pinned first-SVG-only entry contract.
- **`--strict` is the dev harness and the TODO surface.** It refuses on the
  first beyond-slice construct exactly as the original decision text
  states. The refusal pins live on under strict; the degradation
  declarations are the same capability edge worn product-side. Both
  admissions are law-tested (`crates/websem/tests/best_effort.rs`, the host
  pins), and where nothing degrades the two admissions are identical —
  gated frame-for-frame across the full oracle corpus and byte-for-byte at
  the host's spot checks.
- **Document-level contracts do not degrade.** No `<svg>` root, malformed
  standalone XML, and the outer viewport sizing/mapping checks refuse
  identically in both admissions: best-effort degrades subtree content, it
  never invents the canvas.
- **The capability regression bullet above now reads through this lens:**
  the constructs listed there degrade by name at the default admission and
  refuse by name under `--strict`. Everything else in the decision —
  donor status, gate classes, the FLIP posture, the rung order — stands
  unchanged.

## Subsequent status (2026-07-25): the viewport rung lands

The first evolution rung, taken out of the written static order — viewport
semantics before basic shapes — because the best-effort default changed the
payoff: root sizing is document-level, the one gap best-effort cannot
soften, and viewBox-only documents (the most common real-world SVG shape)
refused outright in both admissions. What landed:

- **Root sizing follows SVG2 §8.2.** The host's requested raster is the
  **initial viewport** — the window a standalone document loads into.
  Explicit root `width`/`height` win; a missing or `auto` dimension
  resolves to 100% of it; the inline HTML entry keeps refusing until CSS
  replaced-element sizing (the `auto → 300×150` and aspect-ratio rules) is
  its own rung. Percentage root dimensions refuse by name until the
  percentage basis chain is consumed.
- **The full `preserveAspectRatio` grammar** — nine case-sensitive
  alignments plus `none`, `meet`/`slice` — applied through a near-literal
  transplant of the frozen donor's `compute_viewbox_matrix`
  (`crates/htmlcss/src/svg/layout/viewport.rs`, Blink lineage
  `svg_svg_element.cc` ViewBoxToViewTransform). Unequal-aspect viewBoxes
  letterbox under the default instead of refusing. The transplant was the
  rung's method test and it passed at first contact: all seven new
  Chromium bakes rendered pixel-exact with no math corrections. Malformed
  grammar — including the SVG2-dropped `defer` prefix, which Chromium
  treats as unparseable and silently defaults — refuses as
  `BadPreserveAspectRatio` in both admissions; this slice refuses rather
  than silently defaulting.
- **The adversarial round closed a silent-wrong-pixels class the rung
  would have widened**: cascaded CSS `width`/`height` (a `<style>` rule or
  `style` attribute beats attributes and the auto default in Chromium) is
  now patrolled at the computed level on the root and on rects — the flex
  probe's refusal now names its true reason. Rust-superset number tokens
  (`width="32."`, valid f32 but not an SVG number, dropped by Chromium)
  refuse as bad numbers. The bare geometry longhands `x`/`y`/`rx`/`ry` do
  not exist in the pinned Stylo build and stay inside the named open
  boundary.
- **The oracle corpus grew 10 → 17** (viewBox-only sizing, no-sizing auto,
  letterbox, stretch, slice clip, alignment offset, explicit-PAR
  admission), each Chromium-baked at a window sized to its declared
  dimensions — the window *is* the initial viewport — with the ten prior
  oracles verified byte-identical under the resized windows.
  `unsupported/` holds the malformed-grammar and percentage cells with
  typed-refusal laws. rframe and the n0 kernel needed no changes: the
  per-node affine carried everything.

Remaining static rungs are unchanged in content: basic shapes
(circle/ellipse/line), paths + groups/transforms, strokes — the tiger
milestone — then the rest of the written order.
