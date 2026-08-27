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

## Current state

The sections below are the decision and its dated record, in the order things
happened. This is what is true at the tip, so a reader need not reconstruct it
from the dated addenda below:

- **The host renders best-effort by default.** The admitted subset renders and
  every construct outside it is declared on stderr with a node path and a
  reason. `--strict` refuses on the first one instead, and is the harness that
  names the slice's edge. Document-level contracts refuse in both — including
  a load-active animation element that cannot be attributed to one skippable
  element (an `href` retarget, a root-`<svg>` target). A beyond-inventory
  animation element with an attributable target skips that target in every
  view, declared; it never renders the authored state Chromium overrides at
  load (the addendum below closed this — it was the register's one recorded
  silent wrong pixel).
- **The admitted slice** is `<rect>`, `<circle>`, `<ellipse>`, `<path>`,
  `<line>`, `<polygon>` and `<polyline>`, filled and stroked — solid or
  gradient paint (`<linearGradient>`/`<radialGradient>` paint servers, the
  gradient rung), including `context-fill`/`context-stroke` selected through
  same-document use instances and fully resolved before the frame; with
  centred stroke geometry, the closed cap/join family, opacity, and resolved
  dash patterns with a checked cycle, signed local-space phase, and
  `pathLength` source calibration;
  `<g>` and `<a>` containers, visibility, isolated element/group/root opacity,
  and HTML-ancestor opacity around the selected inline SVG; the whole
  `transform` grammar in both spellings (the attribute is a
  presentation hint of the CSS `transform` property, and `gradientTransform`
  is that attribute on gradient elements);
  `<use>`/`<defs>` same-document references (the id-resolution table);
  geometric same-document `clip-path` resources in the bounded path strategy
  (direct geometry/`<use>` unions, chained intersections, both
  `clipPathUnits`, inherited `clip-rule`, and resolved effect scopes);
  same-document SVG image masks on non-root admitted targets (one isolated
  alpha/luminance source image, both `maskUnits` and `maskContentUnits`, hard
  object-box/user-space regions, admitted graphics/gradient/`<use>` sources,
  clips, nesting, transforms, and effect ordering);
  same-document SVG filters on non-root admitted targets through a checked
  resolved graph of safe-kernel `feGaussianBlur`, integer `feOffset`,
  zero-input `feFlood`, all `feComposite` operators, all sixteen `feBlend`
  modes, ordered `feMerge`/`feMergeNode`, native one-input `feDropShadow`,
  one-input `feColorMatrix`, one-input `feComponentTransfer` with its direct
  `feFuncR`/`feFuncG`/`feFuncB`/`feFuncA` children, and one-input
  `feMorphology`, `feConvolveMatrix`, and `feDiffuseLighting` with one direct
  `feDistantLight`, `fePointLight`, or `feSpotLight` child, plus zero-input
  `feTurbulence` and two-input `feDisplacementMap`; graph inputs resolve from
  `SourceGraphic`/`SourceAlpha`/prior and named results, with both filter
  coordinate systems and color spaces, hard regions, nesting, admitted
  transforms, `<use>`, and the established effect order;
  one declared-font, single-run `<text>` profile; viewBox-only root sizing with
  the full `preserveAspectRatio` grammar; and one exact-time
  `<animate attributeName="x">` on a top-level `<rect>`.
  `crates/n0_cli/README.md` is the statement of record.
- **The corpus** is 812 Chromium-baked primitive cells plus 10 sampled frames.
  All byte-exact except seven curved cells carrying a declared, geometrically
  confined tolerance (the native-oval/conic boundary) and four gradient cells
  carrying a declared one-code-value ramp-quantization tolerance (one pixel
  against Chromium's Skia; 18 knife-edge pixels between this engine's own
  macOS and Linux Skia builds; 336 ramp pixels under an isolated layer's
  restore; 576 after a masked ramp becomes luminance alpha). The named refusal
  register has 152 rows.
- **Not claimed:** no conformance score exists or may be computed — FLIP is
  unratified. The FLIP record and identity-changing review are prepared, but
  only the owner act on gridaco/nothing#49 may authorize them and the first
  run. The Web checklist, not a score, remains the work queue.

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

## Subsequent status (2026-07-26): the basic-shapes rung lands

The second evolution rung, and the first to add geometry rather than
mapping: `<circle>` and `<ellipse>` are admitted. `<line>` is deliberately
left out of it — fill never paints on a line (SVG2 §10.5: line elements
have no interior), so before the strokes rung it cannot produce a pixel;
admitting it would grow the vocabulary without moving one.

- **The kernel again needed nothing.** `rframe::Geometry` grows
  `Ellipse(Rectangle)` — the axis-aligned ellipse inscribed in a
  local-space box — and n0's glyphless compiler lowers it to
  `ItemKind::OvalFill`, which the drawlist and painter already carried for
  the n0-XML path. The exact-bounds law and the per-node affine were
  unchanged: a scaling `viewBox` carries an ellipse exactly as it carries
  a rect.
- **Degenerate radii are admitted nothings, not refusals.** SVG2 says an
  invalid radius must be ignored and a zero radius disables rendering, and
  Chromium implements it as a used-value clamp (`LayoutSVGEllipse`), so a
  missing, zero, or negative `r` resolves to zero extent and paints
  nothing. The `rx`/`ry` `auto` matrix follows: absent adopts the other
  axis, a *negative* radius is that same `auto` (the frozen donor's
  Chrome-confirmed reading, re-proved against the pinned bake version),
  both-auto and either-zero disable rendering.
- **The `auto` keyword is a CSS value, not an attribute value.** The
  adversarial round refuted the first implementation here: Blink parses
  geometry presentation attributes with the SVGLength grammar, where
  `auto` is invalid and maps an explicit `0px` — Chromium renders
  *nothing*, the opposite of the absent attribute's adopting `auto`.
  Reading the keyword would have painted an ellipse the browser does not,
  so it refuses as a bad number instead. The root `width`/`height` keyword
  read is not the analogous case: there the CSS sizing properties
  genuinely take `auto`.
- **Three silent-divergence classes closed with the rung**, each live-
  probed rather than reasoned: attribute lookup ignored namespaces, so a
  prefixed `foo:r` was consumed as geometry (it now requires the
  no-namespace attribute every SVG rendering attribute lives in); numeric
  attributes were trimmed with Rust's Unicode `str::trim`, admitting
  NBSP-padded numbers Chromium rejects (now exactly the five ASCII
  characters the SVG grammar calls whitespace); and CSS
  `transform`/`clip-path`/`filter`/`mix-blend-mode` — all `engine =
  "gecko"`-gated in the pinned servo-mode Stylo, so the cascade drops the
  declaration and no computed value survives to patrol — rendered as if
  absent. The authored text is now patrolled at both ingresses: the
  `style` attribute per element, a `<style>` sheet at the document level,
  since a stylesheet is not attributable to one element without selector
  matching.
- **The pixel gate learned a declared tolerance, and only for curves.**
  Chromium reaches a filled ellipse through the same `SkCanvas::drawOval`
  this engine calls, but through its own build of Skia; the two
  analytic-AA scan-converters disagree on fractional coverage along the
  curve, identically across every available construction (`draw_oval`,
  `draw_circle`, `PathBuilder::add_oval`/`add_circle`, an oval `RRect`).
  Byte exactness is therefore not reachable for a curved edge by any
  choice of call. The gate drops that one property and keeps the rest: a
  differing pixel must lie within a pixel of the fixture's declared ideal
  boundary, within a declared per-channel delta, and under a declared
  count — all three pinned at the *measured* values (worst cell: 6 pixels
  at delta 3; farthest differing pixel: 0.51px from its boundary). A
  misplaced, mis-sized, or miscolored shape still fails loudly, and
  `svg-circle-defaults-clip` bakes byte-exact with no tolerance at all.
  `primitives.json` goes to schema_version 1 because a passing gate now
  means something an old reader would misread.
- **The oracle corpus grew 17 → 24.** Sampling stays the rect-x proving
  slice: an `<animate>` under a materialized circle is a declared blocker,
  not a silent admission.

Remaining static rungs: paths + groups/transforms, then strokes — the
tiger milestone, at which `<line>` joins — then the rest of the written
order.

## Subsequent status (2026-07-26): the container rung lands

`<g>` and the `transform` grammar are admitted — the rung that unblocks
what the previous two already built. In the legacy L0 corpus 101 of the
declared skips were `<g>`, and because a group skip drops its whole
subtree, the shapes inside were unreachable: admitting circles and
ellipses had bought fewer pixels than it looked like. All 37 of those
documents now render and `<g>` is gone from the skip list entirely.

- **Containers are flattened, not represented.** A `<g>` contributes a
  transform and a place in paint order; the resolved contract already
  carries both per node, so rframe and the n0 kernel changed *again* not at
  all — three rungs now with no kernel change. The equivalence is exact
  only while every construct needing a real group scope (`opacity`,
  `clip-path`, `mask`, `filter`, `mix-blend-mode`, `isolation`, `display`)
  still refuses; when a scope-bearing rung admits one, the contract grows a
  scope then, driven by that producer, rather than speculatively now.
- **The walk became recursive**, with nested degradation paths
  (`svg/g[1]/path[1]`) and per-parent ordinals. A failure *on a container*
  fails its subtree, because nothing inside can be placed without it; a
  failure on one *descendant* is that descendant's own hole. Without that
  split, best-effort would drop a whole illustration for one unsupported
  child — the L0 corpus is mostly groups full of paths.
- **The transform grammar is the donor's tokenizer with its leniencies
  removed.** The donor filters unparseable arguments out of its list, so
  `translate(10,abc)` silently becomes `translate(10,0)`; here one bad
  number invalidates the list, arity is exact, and the `comma-wsp`
  separator rules refuse a leading, trailing, or doubled comma — each of
  which Chromium rejects outright, painting the element untransformed.
  Quarter turns come from integer matrices (f32 cosine of a right angle is
  `-4.37e-8`, not zero), and that shortcut is bounded: past `90 * 2^23`
  every quotient is integral, so an unbounded test would snap arbitrary
  angles onto an exact matrix.
- **The AA boundary is narrower than the previous rung claimed.** All eight
  new cells bake byte-exact, including a 45° rotation whose edges land off
  the pixel grid. Straight edges agree between Chromium's Skia and the
  pinned skia-safe; the declared tolerance is specific to **conic** edges,
  not to anti-aliasing. Corpus 24 → 32.
- **Two cleanups.** `<title>`/`<desc>`/`<metadata>` join `<style>` as
  non-rendering — no geometry *and* no hole, where they had accounted for 72
  skips reporting differences that do not exist. And a stylesheet declaring
  a property the cascade cannot represent is now strict-refused but
  best-effort-declared-and-rendered, restoring a document the previous
  rung's document-level refusal had regressed. That required an honest third
  degradation action: `DeclarationIgnored` — the element rendered without a
  value Chromium honors, which is neither a skip nor a sampling policy.

The adversarial round was the most productive yet, and six of its findings
were defects in this rung's own work: the root `<svg>`'s transform was
silently dropped (Chromium applies it to the CSS box, outside the viewBox
mapping, so it refuses by name until that rung); `skewY` was a copy of
`skewX`; a composed transform could overflow to a non-finite matrix from
in-grammar input and kill the whole frame with nothing named;
`display: contents` was unpatrolled; only the first stylesheet finding was
reported, and on the inline-HTML entry its path was fabricated; and the
depth bound could not prevent the crash it existed to prevent, because two
other walks over the same tree recursed unbounded before the compiler ran.

One process note worth recording: a verifier agent mutated engine source to
test whether a law would catch a transposed matrix slot, and left the
mutation in place. It was caught by the very law the round asked for, but
the lesson generalizes — an adversarial round with write access must be
treated as a source of edits to audit, not only of findings to read.

Remaining static rungs: paths, then strokes — the tiger milestone, at which
`<line>` joins — then translucency and opacity scopes, paint servers, text.
Text is now the largest single block in the L0 corpus (167 declared skips,
visible only because containers stopped masking it).

## Addendum — the paths rung (2026-07-26)

`<path>` is admitted: the SVG path-data grammar compiles to a resolved command
stream, `fill-rule` is consumed from the one cascade, and the elliptical arc
refuses by name. Thirteen new Chromium cells bake **byte-exact** (corpus
32 → 46), and the contract grew its second geometry kind — the first growth in
four rungs, and still no change to the n0 kernel.

- **The contract grew minimally, and the growth is checked.**
  `rframe::Geometry::Path` carries a `PathData`: canonical absolute commands,
  the fill rule, the tight local bounds, and whether every contour closed. It
  has **no arc command** — an arc is authored syntax whose lowering decides the
  pixels, so that choice belongs to the producer, the only party that can
  verify it against the browser it is matching — and **no rational conic**,
  because the one producer that needs one is not admitted yet. Bounds are
  solved once, by the contract, because the producer needs them for the node's
  `bounds` and the consumer for coverage, and two implementations would have to
  agree bit-for-bit.
- **Canonical form is the producer's job, and each normalization was measured
  first.** SVG allows move-only contours, redundant `Z`s and an implicit move
  after a close; the contract does not. Each removal was verified pixel-neutral
  in Chromium *on anti-aliased geometry* before being applied — see the
  adversarial finding below for why that qualifier is the whole point.
- **Two deliberate divergences, both declared.** A malformed `d` refuses the
  whole path at its byte offset rather than rendering Chromium's valid prefix
  (SVG2 §9.3.9); where the prefix is empty the two agree exactly, because both
  paint nothing. And the arc refuses, for a measured reason: Blink's path
  *normalizer* decomposes an arc into cubics, and those same cubics — authored
  explicitly and rendered **by Chromium** — differ from Chromium's own `A` by
  77 pixels at up to a 170-per-channel delta. What Chromium actually paints is
  identifiable: the half-ellipse arc `M8 28 A24 20 0 0 1 56 28 Z` is
  **byte-identical** to `<ellipse cx="32" cy="28" rx="24" ry="20">` over every
  row they share. An arc reaches the rasterizer as the ellipse's conics. The
  arc rung must emit conics, and it inherits the oval departure below.
- **The AA boundary narrowed a third time.** The basic-shapes rung declared a
  tolerance for "curved" edges; the container rung showed straight edges agree
  even rotated 45°; this rung shows the **cubic, smooth-cubic and quadratic**
  cells bake byte-exact. Curves as such are not the boundary — the weighted
  **rational conic** is. No path cell carries a tolerance.
- **Six CSS patrol leaks, each measured, each closed.** The `d` *property*
  (Chromium paints a stylesheet's `d: path(…)` in place of the attribute); the
  `all` shorthand (`all: initial` makes Chromium paint nothing where this
  engine painted attribute geometry); vendor aliases of names already on the
  list (`-webkit-clip-path`, `-webkit-transform`, `-webkit-filter` all paint);
  CSS motion path (`offset-path`/`offset` move a shape, and a whole subtree on
  a container); a CSS comment or an ident escape adjacent to a property name
  (`d/**/:`, `\000064:`), which leaves the declaration valid for Chromium while
  the scanned text no longer names a listed property; and a `<style>` whose CSS
  is split across text nodes by a comment node, which the cascade concatenates
  and the patrol scanned separately. The scan now strips comments and vendor
  prefixes, refuses any escaped property name outright, and reads the sheet the
  cascade actually compiles. It is still not a CSS tokenizer, and the honest fix
  — tokenizing with the parser the document already carries — is recorded as
  such in the code.
- **A false explanation in earlier work, corrected.** The
  `CASCADE_PROPERTIES_NOT_REPRESENTED` doc claimed every listed property is
  `engine = "gecko"`-gated in Stylo. Checked name by name against the pinned
  revision, that is true for `clip-rule`, `paint-order`, `transform-box` and
  the `marker`/`offset` families; false for `transform`, `translate`, `rotate`,
  `scale`, `transform-origin`, `clip-path`, `filter`, `mix-blend-mode` and
  `isolation`, which the build represents and this compiler simply does not
  read; and wrong a third way for `backdrop-filter`, `mask-image`, `mask` and
  `offset-path`, which are `servo_pref = "layout.unimplemented"`-gated. The
  list's *behaviour* was right — the authored-text scan is a superset of what a
  computed-level patrol would catch — but the stated mechanism is what a next
  author would rely on when deciding a name is safe to read.

**The adversarial round found the rung's sharpest defect, and it was a defect in
a claim.** `M x y Z` — a contour that only moves and then closes — was dropped
as "a contour that draws nothing", with both the module doc and a law asserting
Chromium paints the authored and normalized forms identically, "measured, not
assumed". It is not neutral: dropping it moves up to 96 pixels of the
*surviving* geometry, because an extra contour is an extra contour to the scan
converter. The law passed only because its coordinates were integer and
axis-aligned, where no edge has fractional coverage to perturb — a fuzz of 40
randomized fractional-coordinate triangles diverged 40 out of 40. The construct
also strokes as a cap-shaped dot, so the contract's stated reason for dropping
it ("no fill area and no stroke geometry") was false in its second half and
would have misled the strokes rung directly. The fix is exact rather than a
tolerance: Chromium renders `M x y Z` byte-identically to `M x y L x y Z`, a
form the contract already carries, so the producer resolves it into that
spelling. Its corpus cell uses fractional coordinates on purpose.

Two smaller claim defects from the same round: the module doc grounded the
number grammar on a spec-versus-browser disagreement it could not verify from
this machine (the code now states only what was measured), and both a law and a
fixtures README called the `marker-*` patrol "provably inert" when nothing else
reads a marker property — the property *is* the paint trigger, so that patrol is
the load-bearing one. `pathLength` was the over-refusal at this rung; the
source path-distance amendment below later graduates it.

One finding is recorded rather than fixed, because it is systemic and predates
this rung: a SMIL `<animate>`/`<set>` targeting a *consumed* attribute is active
at load in Chromium, which paints the animated value, while a Base render paints
the authored attribute and declares nothing. Admitting `d` and `fill-rule`
widened the surface, but the same silence is measurable on `x` and `fill` from
earlier rungs. It belongs to its own rung: declare it, keeping Base as the
authored state while telling the caller the browser would paint something else.

**Measured payoff.** The L0 corpus renders 37/37 documents, with its 38 `<path>`
elements compiled instead of declared; text (167 skips) and strokes (66) are the
two remaining blocks. The Ghostscript tiger — 240 paths, 241 groups, no arcs —
now renders at exit 0 with **78 declared degradations, every one of them a
`stroke` or `stroke-width` attribute on a `<g>`, and nothing else.** Strokes are
the last rung between the ladder and the tiger.

## Addendum — the strokes rung, and the tiger (2026-07-26)

Strokes are admitted, `<line>` joins, and **the tiger renders**: 240 paths, 241
groups, `--strict`, exit 0, **zero declared degradations**, both admissions
byte-identical, and 96.27% of its pixels byte-identical to Chromium's render of
the same file. Twenty-four new cells (corpus 46 → 70), 23 of them byte-exact.
(The stroke family has since grown to 31 cells, 30 byte-exact, when the
closed-contour cap was finished across every painter arm; `primitives.json` is
the count of record.)

- **The contract grew a stroke, not a paint mode.** `FrameNode.stroke:
  Option<Stroke>` carries paints, a width in the node's local space, a cap, a
  join and a miter limit. Two absences are deliberate. There is **no alignment
  field**: a Web stroke straddles its geometry, that is the only alignment any
  Web source can express, and an inside- or outside-aligned stroke grows the
  type when a producer needs one. And there is **no invisible stroke**:
  construction returns `None` for a zero width or an empty paint stack, so no
  consumer re-derives "is this visible".
- **The width is a length, not a number.** Reading it as a typed cascaded value
  buys `8px`, `0.5em`, inheritance through a `<g>`, a stylesheet override, CSS
  keyword case-insensitivity on the cap and join, and the *correct* fallback for
  a negative width — the declaration fails the property's non-negative grammar,
  so the cascade drops it and the inherited or initial value stands, which is
  what Chromium paints. None of that is this compiler's code.
- **`bounds` stayed the geometry's.** A stroke paints outside it, so the field's
  doc now says so and the consumer inflates it by the stroke's own reach
  (`Stroke::outset`, which accounts for a miter's extra length) when it needs
  the covered area for damage. Keeping `bounds` exact is what lets the
  exact-bounds law stay a law.
- **`<line>` needed no geometry kind.** It compiles to a two-command path.
  Chromium's `<line>` is byte-identical to the equivalent `<path>` (measured), a
  line has no interior for a fill to cover, and a two-point path has zero area —
  so the caps, the joins and the zero-length rules all come out identical for
  free. Its endpoints default to zero, which makes a bare `<line>` a zero-length
  segment: nothing under a butt cap, a dot under a round one.
- **The zero-extent rule is an element rule, not a fill rule.** SVG2 §10.1
  disables rendering of a `<rect>` with a zero `width`/`height`, or a
  `<circle>`/`<ellipse>` with a zero radius — *including its stroke*. Chromium
  paints nothing for a zero-extent stroked rect (measured), while a naive stroke
  of a zero-extent box would draw a line. A `<path>` is deliberately exempt: a
  zero-extent path is a zero-length segment, which strokes as a cap-shaped dot.
- **A dash array that paints nothing is admitted.** `stroke-dasharray: none`,
  an all-zero array, and an invalid value all render solid in Chromium
  (measured), so the refusal tests for a dash that would *paint* rather than for
  a non-empty list. Refusing on non-empty would have dropped documents the
  browser renders solid — and `stroke-dasharray="0"` is something authoring
  tools actually emit.
- **Two patrols stopped being over-refusals.** `vector-effect` and `paint-order`
  were provably inert while strokes refused; consuming strokes made both
  load-bearing. That is exactly the trap the earlier rungs kept them for, and
  the reason `pathLength` entered the patrol at this rung even though nothing
  read it yet. The source path-distance amendment below later graduates it.
- **Strokes land inside the declared conic class, mostly below it.** 23 of 24
  cells bake byte-exact — including every cap, every join, the miter-limit
  bevel, the elliptical pen from `scale(2,1)`, and the stroked circle and
  ellipse. The one exception is the round join at a *closed* contour's closing
  corner: 4 pixels at delta 3, declared with that join's arc as its boundary.

**The tiger.** The Ghostscript tiger is the de-facto SVG smoke test, and it is
now a milestone rather than a target. It needs exactly what the last four rungs
admitted and nothing else: a `viewBox`-only root (the viewport rung), one
`matrix()` on a group (the container rung), 240 `<path>` elements with relative
`s`/`c`/`m`/`l`/`v`/`z` data and no arcs (the paths rung), and `stroke` +
`stroke-width` inherited through `<g>` (this one). Before this rung it rendered
with 78 declared degradations, every one of them a stroke attribute on a group;
now it renders with none.

The remaining 3.7% of differing pixels are curve boundaries — the tiger is
almost entirely conics and cubic outlines — and they sit where the corpus's
declared departure says they will: 3042 of the 9788 differ by a single level,
and only 25 pixels of 262144 differ by more than 64.

*The tiger file is local-only and deliberately not committed.* It is AGPL
(Ghostscript authors, derived from `tiger.eps`) and this repository is dual
Apache-2.0 / MIT, which is precisely the license-restricted case `fixtures/local/`
exists for, beside the W3C SVG 1.1 suite and resvg's tests. Provenance, the
fetch command and the sha256 live in `fixtures/local/tiger/PROVENANCE.txt` — a
file that exists only on a machine that has already fetched the tiger, so
**this measurement is not reproducible from the repository alone**, and nothing
committed here carries the URL or the hash to make it so. It is a reported
observation, not a gate. **The engine's capability is gated by the committed
byte-exact corpus**, which is reproducible from a clean checkout; the tiger says
only that the ladder holds on a real-world drawing of that size.

**The adversarial round found three unit-basis defects, all of the same
shape.** The rung consumed its first cascaded *length*, and a length is only as
good as its basis — which this build does not always have. A viewport-relative
`stroke-width` was the sharp one: Chromium resolves `1vw` against the SVG
viewport (0.64 units on a 64x64 document, byte-identical to an authored `0.64`),
while the cascade's device is pinned at 1280x720 and computed 12.8 — a
twentyfold error, painted silently. `ex`/`ch` resolve from placeholder font
metrics rather than measured ones. Both families now refuse by name; threading
the document's real viewport into the cascade's device is the honest fix and its
own rung, since it moves media queries too. The third was the opposite problem:
`em` was *correct* from a stylesheet and wrong from a presentation attribute,
because a bare `font-size="32"` is invalid to the CSS grammar and was dropped —
Blink parses SVG presentation attributes in a mode where a unitless length is
user units, so csscascade now retries a bare number as `px` and admits
`font-size` for its basis alone. My own law had tested only unitless, `px` and
`em`-at-the-default, which is exactly why all three passed.

The round also found that `Stroke::outset()` did not bound what its doc claimed:
it inspected only the *join*, and a square cap's corners sit at `radius·√2` from
the endpoint, so a round-joined square-capped diagonal inked outside the coverage
box two consumers were told to trust. It was accidentally true in the common
case, because the default miter join at limit 4 swallows the cap — which is why
no law caught it. And a negative `<rect>` extent aborted the whole render with an
internal message naming a `VisualRef`, where Chromium disables that one element
and paints the rest: the shapes rung's defect, adjacent enough to this one's
zero-extent rule that leaving it would have made the rule's doc a lie.

**Where the ladder stands.** The L0 corpus renders 37/37 documents, and its
declared skips are down to 351 with a clear shape: `<text>` is 167 of them
(48%), then `<defs>`/`<use>`/`<filter>` and the paint-server fills, then
`stroke-dasharray` (17), `<polygon>`/`<polyline>` (12), and rounded rects (8).
Text is the next rung by a wide margin. Also queued, each with its brief already
written by measurement: the arc (emit conics, inherit the oval tolerance), the
`points` shapes, dashing, opacity and translucency, paint servers, and the
declaration of a SMIL animation that targets a consumed attribute.

## Addendum — the cub: a committed scene, static and sampled (2026-07-26)

The corpus gained `svg-scene-cub` — an **original** composition that covers the
tiger's feature set in 17 materialized nodes instead of 240, committed as a
fixture pair so the whole rung ladder is gated both statically and under
exact-time sampling. Five frames per pair (Base plus four samples), ten across
the two sampling fixtures, every one byte-identical to Chromium.

- **It exists because the tiger cannot be committed.** The tiger proved the
  ladder but stays untracked under `fixtures/local/` for its AGPL provenance, so
  the capability it demonstrated had no committed cell. The cub is drawn to the
  same *feature* list — viewBox-only root sizing, a container with a transform
  and a nested `<g>`, `fill`/`stroke`/`stroke-width`/`stroke-linejoin`/
  `stroke-linecap` inherited through both, cubics and quadratics, relative
  command runs, multi-subpath paths, round caps and joins, `fill="none"` and
  `stroke="none"`, `<rect>`s in a group, a `<line>` — and renders `--strict`
  with zero declared degradations at zero differing pixels.
- **A composition is a different gate from the single-feature cells.** Each
  primitive cell isolates one construct; the cub is the only cell where
  inheritance, container nesting, paint order, curve rasterization and stroking
  must all be right *simultaneously* for the pixels to land. At its gate size it
  passed on the first probe, before any harness code existed. At a *second* size
  it found a defect five rungs of single-feature cells had missed — recorded
  below.
- **The sampling laws now hold over a scene, not just a shape.** The host law
  states the property a composition makes checkable: every pixel that differs
  between the Base frame and a sample lies in the animated rect's own rows.
  Sixteen nodes of curves and strokes render identically at every time, and the
  frozen pair is the same frame. The frame-side laws read the animated node by a
  *declared* index and node count (`cases.json`'s `frame` block), so a fixture
  that silently stops materializing an element fails there instead of quietly
  weakening every law below it.
- **Two absences in the drawing are the slice speaking.** The animated element
  must be a top-level `<rect>` (the inventory admits an `<animate>` only on a
  materialized direct child of the root), so the sliding block sits beside the
  figure rather than inside its group. And the scene carries no `<style>`, no
  `style=` and no `color=`: the first two are dynamic-inventory blockers, and
  `color` is a presentation attribute Chromium honors that this engine declares
  rather than paints — so `currentColor`, which the corpus reaches today only
  through a `style` attribute, is unreachable in a *sampled* document by either
  door until its own rung lands. Writing the fixture is what made that
  intersection visible.
- **No `<circle>` or `<ellipse>`, on purpose.** The muzzle, eyes and ear tips are
  cubic approximations of ellipses. Filled *and stroked* cubics bake byte-exact
  while a true rational conic does not, so the scene gates at zero difference
  with no tolerance — the declared AA departure stays exactly where the strokes
  rung left it, on the weighted conic alone.
- **The sampling corpus is now plural.** `cases.json` carries a `fixtures` array
  (schema 1), the baker loops it with a per-fixture initial viewport and
  validates each declaration structurally — including that a fixture's authored
  value differs from its first sample, so Base and `Sample(0ns)` can never
  silently coincide — and each fixture's oracles live under
  `chromium/<id>/`. The rect-x pixels moved directories and were re-verified
  byte-identical by the bake, which is the proof the rename lost nothing.

**Verifying the cub at a second size found a defect the five rungs never
showed.** The scene gates byte-exact at 96x96. Rendered at 48x48 — the same
document, a 1x viewport mapping — 242 of 2304 pixels differ from Chromium. That
is not the declared conic departure. Bisecting it by measurement:

| case (closed cubic contour, `fill` + `stroke`) | stroke width, at 1x | differing |
| --- | --- | --- |
| `stroke-linecap="butt"` | 1 device px | **0** |
| `stroke-linecap="round"` | 1 device px | 185 of 2304, worst channel delta 100 |
| `stroke-linecap="square"` | 1 device px | 185, worst delta 128 |
| `stroke-linecap="round"` | 2 or 3 device px | **0** |

**A cap cannot exist on a closed contour** — and Chromium agrees: its butt and
round captures of that document are byte-identical to each other (0 differing
pixels), so its raster is cap-invariant there. This engine's is not. The cap is
carried correctly all the way down; the divergence is in the *consumer*, which
paints a stroke by filling the outline `StrokeRec::apply_to_path` returns, and
that outline is not cap-invariant for a closed contour at a stroke width near
one device pixel. Butt is byte-exact, so butt is the right answer and the other
two caps are silently wrong pixels — the invariant this program does not allow.

Two things kept it hidden. No committed cell combines *all three* triggers (a
closed contour, a non-butt cap, and a ~1-device-pixel stroke width): the cap
cells stroke open contours, and the closed-contour cell uses the default butt.
And the tiger never declares a `stroke-linecap` at all, so its 96.27% agreement
was never going to show this. The cub is the first fixture that inherits a round
cap onto closed contours — it just does so at a size where the artifact
vanishes.

The fix is its own rung, not a patch here: caps are inert per *contour*, so a
path mixing open and closed contours (the cub has both) cannot be normalized
wholesale — it needs per-contour cap handling in the stroke lowering, plus
corpus cells at a one-device-pixel stroke width, which the corpus lacks
entirely. Recorded rather than fixed, with the reproduction above.

## Addendum — the cap defect, closed (2026-07-27)

Fixed. Re-measuring first moved every part of the diagnosis above, which is
worth recording as much as the fix:

- **The attribution was wrong.** A centred SVG stroke does not fill
  `StrokeRec::apply_to_path`'s outline; it sets `PaintStyle::Stroke` and hands
  Skia the cap — the same stroker Blink drives. Two consumers of one stroker
  were disagreeing.
- **The fill is not involved.** A closed contour with `fill="none"` diverges
  identically, so the earlier "fill + stroke" framing was incidental.
- **It tracks the *device* width, not the authored one.** The same document
  rendered at 2x diverges at an authored `0.5` and agrees at an authored `1`;
  the threshold sits between 1 and 1.25 device pixels. The register's table read
  as a property of the authored value, which it is not.
- **An open contour is byte-exact at every width and every cap** — measured
  across `butt`/`round`/`square` at 0.5, 0.75, 1, 1.25, 1.5, 2 and 3 device
  pixels. So this was never thin-stroke rasterization in general. It is a cap
  appearing where a closed contour rejoins.

The fix is the semantics: a closed contour has no ends, Chromium's raster is
cap-invariant on one, so n0 normalizes the cap to butt when every contour is
closed. One draw, one composite pass, byte-exact at every width probed.

**The exception the corpus caught.** A contour with no extent is a point, and
SVG2 §13.2 makes the cap the *only* thing that renders it: `M44 32 Z` under a
square cap is a dot Chromium paints, and normalizing that cap away erased it.
`svg-stroke-zero-length-dot` failed within minutes of the change. The predicate
now declines to normalize anything that closes on the point it opened at,
comparing on-curve endpoints only — a false positive merely keeps the authored
cap, a false negative would delete a dot.

**The mixed case refuses instead.** A path carrying both open and closed
contours needs two caps at once and one paint carries one. Splitting the draw
was implemented and measured, and it is worse: byte-exact below a device pixel,
then 32 to 47 differing pixels at 1.25 and 2 units, because the two runs'
anti-aliased edges composite twice where they overlap. So websem refuses a
non-butt `stroke-linecap` on a mixed path by name. It over-refuses — the error
needs a device width the compiler cannot know — and serving the case properly
means stroking each contour to an outline and unioning them into one filled
path, which is its own rung.

**The corpus gained the intersection it lacked**: a closed contour, a non-butt
cap and a one-device-pixel width, one cell per cap, all three byte-identical to
Chromium. The old cap cells stroke an open line sixteen units wide and the old
closed-contour cell takes the default butt, so between them they covered each
half of the trigger and never both at once.

`svg-scene-cub` is now byte-exact at 48x48 as well as at 96x96, and the declared
AA departure stays exactly where the strokes rung left it — the weighted
rational conic alone.

## Addendum — the load-active SMIL hole, closed (2026-07-30)

The finding recorded at the paths rung — *a SMIL `<animate>`/`<set>` targeting
a consumed attribute is active at load in Chromium, which paints the animated
value, while a Base render paints the authored attribute and declares
nothing* — is closed. Measured before the fix, the hole was wider than the
record stated: the host's Base-time degradation filter also swallowed the
`SamplesAsBase` declaration the library did emit, so
`<set attributeName="fill" to="red"/>` rendered the authored fill at exit 0
with **zero declarations in both admissions**, `--strict` included, while
Chromium paints red. The refusal corpus had no SMIL row, so no gate could
notice.

The fix is a classification, not a patch over the filter. The sampling
inventory's findings now split by what they distort:

- **Sampling-only blockers** — dynamic surfaces that leave the Base view
  honest: event handlers, CSS animation carriers, `<style>` sheets, the
  inline-HTML entry block. Unchanged: Base renders, strict refuses the
  sample request, best-effort declares `SamplesAsBase` and resolves samples
  to Base. Everything the host's Base-time filter hides is now genuinely
  Base-inert, so the filter stands as written.
- **Authored-state overrides** — beyond-inventory animation elements. SMIL
  defaults `begin` to offset `0s`, so each is active the moment Chromium
  loads the document: the target's authored state never honestly renders.
  Strict refuses at construction, like any beyond-slice construct.
  Best-effort recompiles with the SMIL default target (the parent) left
  out of the frame — a declared hole at the target's stable path, in every
  view, never a wrong pixel in any. An override that cannot be attributed
  to one skippable element — an `href` retarget (id resolution is not
  owned), a root-`<svg>` target (the override reaches the whole canvas) —
  refuses in both admissions, exactly as `<script>` does.

**A deliberate departure from the recorded remedy.** The paths-rung note
sketched "declare it, keeping Base as the authored state while telling the
caller the browser would paint something else." That would render a value
Chromium does not paint, annotated — a declared *wrong pixel*, which the
first law does not recognize as a category. The attribute patrol set the
precedent: an admitted element carrying an unconsumed rendering attribute is
skipped, not painted-with-a-note; painting-with-a-note is reserved for the
one construct that is genuinely unattributable (a stylesheet, absent
selector matching). An attributable SMIL override follows the attribute
precedent.

**Named over-refusals, kept.** The inventory owns no per-element
applicability model, so `<animate attributeName="x">` under a `<circle>` —
inert in Chromium, since `x` does not apply — still skips the circle: a
declared hole where Chromium paints, preferred to the model the slice does
not own. Likewise a `begin`-conditioned animation (`begin="click"`,
`begin="2s"`) skips its target although Chromium's load-time picture shows
the authored state; SMIL timing beyond the admitted `dur`/`fill=freeze`
shape is the future animation rungs' business, and `animation-sampling`
already models more than this front-end admits.

**What remains open, now named.** The admitted `<animate attributeName="x">`
keeps its corpus-pinned Base semantics: Base is the static *projection*
(the animation contributes nothing), baked as `base-static-projection`
cells against a Chromium document with the animation stripped, and "Base is
not shorthand for Sample(0)" stays a law. The divergence between that
projection and Chromium's load-time picture of the *animated* document is
a documented, gated semantic for the one admitted element — no longer a
silent one for every other.

Three refusal-corpus rows gate the closure (`svg-smil-set-load-active`,
`svg-smil-animate-transform`, `svg-smil-retarget-href`), and the laws that
pinned the defective behavior moved with it
(`crates/websem/tests/best_effort.rs`,
`crates/websem/tests/svg_animation_x.rs`, the groups and shapes contracts).

## Addendum — the points rung (2026-07-30)

`<polygon>` and `<polyline>` are admitted. Both lower to the line-segment
path the contract already carries — `MoveTo` + `LineTo`\* (+ `Close` for a
polygon) — exactly as `<line>` does, so the rung cost the contract nothing.
Closure is the one semantic difference between the two elements, and the
`points` grammar runs through the same number scanner as path data, so the
two grammars cannot drift.

**Measured before written.** The grammar's edges were probed against
Chromium 149 before the parser existed, and the probe moved the design in
one place: a trailing separator after the last complete pair is *accepted*
in `points` (unlike the `viewBox` grammar, whose trailing comma stays a
refusal), so the slice admits it, Chromium-baked. The rest confirmed the
plan: a leading or doubled comma, a trailing dot, and a percent are errors
whose valid *pair prefix* Chromium renders — this slice refuses the whole
element by name instead, the paths rung's declared divergence restated
(`svg-points-odd-coordinate` is its refusal-corpus row); a filled polyline
paints as if closed; and a single point splits by closure — the polygon is
the zero-length **closed** contour whose cap paints a dot, resolved into
the contract's canonical `M x y L x y Z` spelling (the cap-normalization
exception from the cap-defect addendum fires for it unchanged), while the
polyline is a neutral move-only contour that paints nothing under any cap
and is admitted as not-a-node.

**Eight cells, byte-exact.** Fill with mixed separators, the trailing
separator, an evenodd self-intersecting star (the cascaded `fill-rule`
read, shared with `<path>`), the stroked closure split (closing segment
and joins on the polygon, caps and no closing edge on the polyline), the
implicit-close fill equivalence, and the two single-point cells. All eight
bake byte-exact — no new tolerance; the declared AA departure stays the
weighted rational conic alone.

**The register moved with the slice.** The `svg-polygon`/`svg-polyline`
refusal rows graduated (the enumeration gate forces the move), the L0
`basic-shapes` host pin lost its three points-shape holes, and the
`polygon-fill-probe` strict pin flipped from the capability edge to an
admitted probe. At this rung the points shapes inherited the path patrols, so
`pathLength` and the marker properties remained refusals. The later
path-distance amendment graduates `pathLength`; the marker boundary remains.

## Addendum — the visibility rung (2026-07-31)

`display: none` and `visibility` are consumed — the first rung that turns
over-refusals into the correct nothing rather than admitting new paint.
Both enter as presentation hints through the one cascade (csscascade's
admitted set grew its ninth and tenth properties, precedence-law-gated),
so the attribute and every CSS spelling resolve identically and an author
rule beats the attribute — measured, and baked as the un-hide cell.

**The split is semantic, and each half is measured.** `display: none`
generates no box: the subtree is pruned and a `visibility: visible`
descendant stays gone. `visibility: hidden` and `collapse` (identical for
shapes) turn off one element's *own* paint; the property inherits, and a
descendant whose computed value is `visible` un-hides itself — the walk
therefore still descends through hidden containers, and each element's own
computed value decides its node. Neither is a hole: nothing is declared,
because Chromium also paints nothing — `r="0"`'s admitted nothing,
restated. A pruned or hidden element's other unconsumed properties stay
silent too: a refusal there would turn a correct nothing into a false
alarm. `display: contents` stays a named refusal (its own corpus row): it
paints children in the parent's place, which the flattened walk cannot
express without dropping a transform silently.

**The oracle corrected the probe.** An embedded-context probe suggested a
root `display: none` never paints; the bake of `svg-display-none-root`
showed a **standalone** document's outermost `<svg>` ignores the property
and paints normally — only an embedded (inline-HTML) root generates no
box. The compiler now splits by entry, the law file pins both halves, and
the divergence between probe context and document context is exactly why
cells are baked from the entry they gate.

**Seven cells, byte-exact** (92 total): the shape and container prunes,
the standalone-root proof, hidden and collapse, the descendant un-hide,
and the author-rule-beats-attribute cell. `visibility_contract.rs` is the
rung's law file; the smuggle law's display/visibility rows graduated into
it, and `svg-display-contents` replaced the pair in the refusal corpus.

## Addendum — the translucency rung (2026-07-31)

`fill-opacity`, `stroke-opacity`, and translucent sRGB paint are consumed.
One rule generates the rung: paint alpha is the product of the colour's own
alpha and the paint-level opacity, multiplied in float and quantized
**once** — Chromium composites the product, not the quantized factors, and
the multiplied cell (`svg-fill-opacity-times-alpha`) pins the rounding
byte-exactly. Both properties enter as presentation hints (csscascade's
admitted set grows to twelve) and fold at the two typed paint reads; the
fill and stroke stacks are separate, so the fold can never composite the
wrong paint, and a zero opacity resolves to the same admitted nothing as
`fill: none`.

**Seven cells, byte-exact** (99 total): the overlap composite in both
spellings of the one `<alpha-value>` grammar, the rgba equivalence, the
multiplied quantization cell, inheritance through a container, the
stroke-over-own-fill compositing split, and the miter join's single-pass
self-overlap. The first quantization guess (round, once) matched Chromium
on every cell — no tolerance entered.

**Element `opacity` stays refused, by design.** It composites fill and
stroke through one layer; folding it into per-paint alpha would
double-blend where they overlap. It is the group-scope rung's first
producer, and the refusal-law table says so at its row. The strokes-rung
laws that pinned `stroke-opacity` and translucent paint as refusals
graduated into the translucency contract, and the beyond-surface paint
laws keep their point through a colour space the slice still refuses.

## Addendum — the percentages rung (2026-07-31)

Shape-geometry and `stroke-width` percentages are consumed. The bases are
SVG2 §7.10's, threaded once from the root: the viewport's user-unit extent
— the `viewBox` when one maps the viewport, the root's own extent
otherwise — with x-axis lengths against its width, y-axis against its
height, and the "other" lengths (a radius, a stroke width) against the
normalized diagonal `sqrt(w² + h²)/√2`. The `10%`-of-64x64-is-6.4-units
measurement recorded when this was a refusal became the resolution's first
assertion, and all six cells (both bases, the non-square axis split, the
line endpoints, the stroke width) baked byte-exact.

**Scoped away from the root, deliberately.** Root percentage sizing keeps
its document-level refusal: its basis is the host window itself, which the
element-capture baker cannot express as a cell (the window is the basis,
so any declared fixture dimension is circular). It graduates only with a
host-level oracle. The CSS-spelled geometry surface likewise keeps its
`Size::Auto` refusal — the compiler reads geometry from attributes, and a
cascaded width on a `<rect>` stays a named skip.

**A refusal variant retired.** `UnsupportedLength` existed to name the
percentage refusal; with the resolution landed nothing constructs it, and
its essence lives on as the `PercentBases` resolver and its laws. The
malformed spellings (`5 0%`, junk digits) stay `BadNumber` refusals, the
same posture as every other invalid number.

## Addendum — the anchor rung (2026-07-31)

`<a>` is admitted as a container: SVG2 §16.2 makes its `href` interaction,
not paint, so it shares `<g>`'s compiler, patrols, transform composition,
and flattening — one law asserts the equivalent `<g>` resolves to the
identical frame, and one Chromium-baked cell (`svg-anchor-container`)
proves the composed transform and a translucent overlap through it. The
container dispatch is now parameterized by element name; nothing else
moved.

## Addendum — the transform rung (2026-07-31)

The CSS `transform` property is consumed, and consuming it dissolved the
attribute as a separate concept: CSS Transforms L1 §7 makes the SVG
`transform` attribute a presentation attribute of the one property, so the
rung moved the attribute grammar into csscascade, which rewrites a valid
list into equivalent CSS text (§7.3's unit assignment; the 3-argument
`rotate` expands to its defining translate·rotate·translate sandwich) and
injects it at presentation-hint level. Precedence is therefore the
cascade's, not reimplemented: any author rule beats the attribute —
`transform: none` included — the style attribute beats the rule, and an
invalid CSS declaration never enters, so the attribute stands. websem reads
only the computed operation list, converting it per-op to one affine with
the exact quarter-turn matrices the attribute path always had.

**A 40-measurement probe matrix decided the semantics before any code.**
The load-bearing verdicts, each now a law or a cell: SVG elements pivot on
used `transform-origin 0 0` — the local user-space origin, which Chromium
keeps even under a negative-min `viewBox` (spec letter says the reference
box moves with `viewBox` min; the oracle says it does not); percentage
translations resolve against the viewport's user-unit extent; a malformed
attribute list **drops whole and renders untransformed** (all 23
previously-refused lists re-baked as drops, so the old
refuse-by-name posture flipped to Chromium-exact silence — the pixels
agree, which is the law); and the grammar carries the measured leniency no
browser ever tightened (csswg-drafts#2623): numbers may run together
(`translate(10-10)` is (10, −10)) and functions need no separator, while
every comma strictness (leading, doubled, trailing — list-level trailing
included, a tightening this rung added) stays enforced.

**Thirteen cells baked byte-exact** (corpus 106 → 119): the graduated
refusal fixture, both precedence directions, `none`-restores,
invalid-falls-back, compound composition, the percentage basis, a
container's cascaded transform, the negative-quadrant rotation, the
`-webkit-transform` alias (the pinned Stylo implements it — verified, not
assumed, when the name left the scan denylist), the two leniency forms,
and the malformed-drop. The refusal register moved one row out and four
in: `transform-origin` and `transform-box` (the knobs that move every
pixel a transform touches — the second does not exist in the servo-mode
build at all), the beyond-2D function family (Chromium composes
`translate3d` on SVG — measured — so it refuses by function name rather
than flattening under-measured), and the individual `rotate` property
(Chromium composes the individual properties *with* `transform`;
consuming one without the others would compose a different matrix). The
root `<svg>`'s transform now refuses in both spellings — the computed
patrol closed the stylesheet route to the root that the scan's graduation
would have opened. `BadTransform` retired with the drop-semantics flip;
its overflow half lives on as `NonFiniteTransform`, still named at the
element.

## Addendum — the use/defs rung (2026-07-31)

The engine's first id-resolution table, and with it `<use>` and `<defs>`.
The architecture is the one the probe matrix forced: the referenced
subtree is **physically cloned under the `<use>` before the one cascade
runs** (csscascade's `svg_use`, at DOM freeze), so the instance is styled
by the same single pass — presentation attributes and `style` clone with
it, and inheritance flows from the use site, which is exactly Chromium's
measured behavior (`fill` on the use colors a clone that authors none;
the clone's own attribute beats it; `currentColor` resolves against the
use site's `color` — and `color` joined the admitted hint set for it).
websem renders `<use>` as a container of its shadow content and skips
`<defs>` by name; the walk, the paths, the patrols and the animation
inventory all see the expanded tree, so a beyond-slice construct inside
an instance is a declared hole at the clone's real path, and a cloned
animation element classifies against the clone it targets.

**The 31-measurement probe matrix's load-bearing verdict is the styling
boundary.** Selector matching against an instance is *totally* scoped to
the cloned subtree: `#id`, class, type and clone-internal structural
selectors match; NO selector involving any ancestor outside the clone
does — not `defs > rect` (the original's position does not carry), not
`use > rect`, not even descendant combinators like `svg rect`. A clone
parented in the one flattened tree cannot reproduce that boundary in
either direction, so **author CSS and `<use>` refuse together, by name**,
until a shadow-matching rung earns it with Stylo's shadow machinery. The
2018-era svgwg#504 claim that Blink copies the original's computed style
is measurably no longer true of Chromium 149.

Everything else measured landed as law and cell: `x`/`y` translate
appended inside the use's own transform; whole-document first-id-wins
resolution with forward references; plain `href` over `xlink:href`;
`width`/`height` inert for admitted targets; the correct nothings (an
unresolved reference, a mutual cycle, an ancestor circle — each baked as
pixels, each rendering exactly Chromium's nothing with nothing declared);
a light-tree target painting in place and as an instance; `display: none`
cloning onto the instance. Chains expand through with cycle guards on the
expansion chain (use ids and target ids alike, push/pop so siblings never
see each other's history); indirect cycles beyond the measured shapes hit
a depth budget and refuse loudly as expansion overflow, as do external
references and authored element children — each a register row with a
fixture. `<symbol>` targets surface the symbol element at the clone's own
path and refuse like the nested viewport they are.

Twenty cells baked byte-exact (corpus 119 → 139, the largest single rung
yet). The refusal register moved one row out (`svg-use` graduates) and
four in; `svg-path-marker-end`'s defs half stopped declaring the moment
defs was consumed, leaving the marker attribute itself as the named hole
— the row now names it directly.

## Addendum — the gradient rung (2026-08-01)

The engine's first non-solid paint, and the first contract amendment of the
rung ladder: `rframe`'s `SolidPaintStack` became `PaintStack`, admitting
linear and radial gradients from the shared `cg` leaf vocabulary alongside
solids (visible, normal-blend; sweep, diamond, image, and non-normal blends
stay construction rejections). The repo law that read "`rframe` cannot
express a gradient" was falsified by design and re-stated: rframe cannot
express a paint that *references* a resource. A gradient's geometry is
stated in the unit square of the item's paint box; the producer folds every
SVG coordinate system into that fact, and no `gradientUnits`, `href`, or
spread keyword crosses the contract.

A 72-probe Chromium matrix decided the semantics before code. The
load-bearing verdicts:

- **`gradientTransform` is the transform property's presentation attribute
  on gradient elements** — the attribute and an author `transform`
  declaration are byte-identical through non-quarter rotations and scales,
  the plain `transform` attribute is inert there, `transform: none` disarms
  the attribute with ordinary cascade precedence, and the value applies
  about the *raw origin* of gradient space (a scale-2 probe discriminated;
  the CSS-origin hypothesis died). csscascade hints `gradientTransform`
  through the same measured rewrite the transform rung built.
- **The fallback fires only on an invalid reference.** A valid gradient
  with zero stops — including a self-cycle that composes to zero — paints
  nothing and leaves the authored fallback unfired; a missing id or a
  non-gradient target is invalid and the fallback paints. Baked as cells.
- **At this rung the backend's degenerate rules were resolved by the
  producer** so the engine's preflight never met them: one stop and the baked
  `r = 0` `pad` case became the last-stop solid; coincident linear endpoints
  became the last stop under `pad` and the ramp's *integral average* under
  `reflect`/`repeat` (measured 128,0,128 — Chromium shares the backend's rule).
  That evidence did not establish non-`pad` radial degeneracy. The opacity
  closure addendum below records the later measurement and correction, and
  also preserves one-stop gradient rasterization.
- **Ramps dither.** Chromium's rasterizer dithers gradient ramps with the
  backend's ordered matrix; the painter now sets the same flag and 26 of 27
  gradient cells bake byte-exact — including the dither pattern itself.
  `fill-opacity` over a gradient multiplies at the backend's 8-bit alpha
  step (measured ×128/255, not ×0.5), pinned in the painter and its cell.
- **Stops are attribute reads** — the pinned servo Stylo has no
  `stop-color`/`stop-opacity` longhands, so author CSS on stops refuses by
  name (a sheet at document level, a stop's style attribute at the paint),
  closing what was a silent drop. `currentColor` in a stop resolves against
  the gradient's own ancestor chain through the one cascade — never the
  referencing element (measured).
- **The id table is the document's**: whole-document, first-id-wins,
  `<use>` shadow clones excluded (a clone earlier in expanded order does
  not shadow the original — measured), gradients referencable outside
  `<defs>`, `href` beating `xlink:href`, chains crossing gradient types for
  stops and common attributes but never geometry, cycles killing only the
  edge.

The glyphless engine seam took its once-deferred decision: a path's paint
box anchors at the tight-bounds origin (a unit-space pre-translate on the
gradient's transform), and `preflight_gradients` — generic over the
drawlist owner now — runs inside the glyphless compile, so a gradient the
backend cannot shade is a named `BuildError` before any product exists,
never a painter panic.

What refuses by name: focal radials (the shared radial leaf is concentric —
`fx`/`fy`/`fr` wait on the paint RFD's focal amendment),
`color-interpolation: linearRGB` (honored by Chromium, measured, and
inexpressible in one backend ramp), font-relative units in gradient
geometry, percentages in a gradient's computed transform (Chromium
resolves them against mismatched spaces — measured and declined), external
references, and `<pattern>`.

Twenty-seven cells baked (corpus 139 → 166), 26 byte-exact on the first
gate run. Two carry the corpus's new `ramp-quantization` tolerance, both
at one code value with measured counts: the off-center radial differs
from Chromium's Skia in one pixel (an ulp at a ramp knife-edge), and the
non-monotonic-stops cell — byte-exact on macOS — differs in 18 clamp-edge
pixels under the Linux Skia build's SIMD path, the corpus's first
measured cross-platform departure. Mapping it taught the gate to sweep
the whole suite before failing, so a platform difference is now one CI
round-trip. The refusal register moved `svg-gradient-paint-server` out
and five named rows in.

## Addendum — the group-scope rung (2026-08-04)

Element `opacity` is consumed, and consuming it grew the contract's first
**structural** amendment — the second amendment overall, and the one the
container rung's flattening promise waited on. `rframe`'s flat node list
became a checked painter-ordered item stream: a compositing `Scope`
encloses a contiguous span as one isolated group, with balance,
non-emptiness, and bounded nesting proven at construction (`PathData`'s
posture), and the new item enum forcing every consumer match site at
compile time — the shape was chosen precisely so a consumer *cannot*
read the nodes and silently skip the scopes. The effect vocabulary opens
with `Opacity` over the open unit interval (identity and zero are
producer resolutions, not scope facts) and grows per producer —
`clip-path` is the named next effect. What a scope refuses is what the
crate refuses: an effect that references a resource (mask, filter,
pattern) has no representation.

**A 49-document probe matrix decided the semantics before code**, and its
load-bearing verdict reframed the rung: Chromium's element opacity is not
one route but two, one code value apart, and both are meaning.

- **The fold.** Over content that is a single un-transformed, un-folded
  draw, element opacity is byte-identical to the `fill-opacity` fold — it
  joins the translucency rung's one float product (colour alpha ×
  paint-level opacity × element factor), quantized once; the multiplied
  probes pinned the product in both stacking orders. The fold reaches
  through plain containers, past zero-draw and hidden siblings, and holds
  on stroke-only shapes (ink beyond the geometry bounds included).
- **The layer.** Everything else composites through a real isolated
  layer, whose quantization sits one code value below the fold:
  fill+stroke on one shape (the double-blend fact that motivated the
  refusal — 57 code values from the per-paint spelling at the overlap),
  sibling overlap (topmost child at the group alpha, composited once),
  and nesting, which quantizes **per layer** and never flattens to a
  scalar product (`g(.5)›g(.5)` differs from `opacity=.25` by exactly one
  code value across the whole fill).
- **The boundary between them is structural.** The fold fires at most
  once per draw — a group over an already-folded draw runs a layer — and
  any non-`none` computed transform strictly *below* the scope element
  breaks it, while transform and opacity on the same element still fold.
  The measured rule websem implements: *fold iff the span is exactly one
  draw, un-folded, un-scoped, and un-transformed below the scope element;
  layer otherwise* — a fold replays the span with the factor threaded
  into the paint resolve so the product still quantizes once, identically
  in both admissions.

**The painter's existing opacity route was measured and found to be a
different meaning.** n0's arithmetic-blender `BeginOpacity` (the model's
backdrop-initialized group) produces the *fold* bytes — one code value
from Chromium's layer. Rather than change the model's meaning, the
drawlist vocabulary grew `BeginIsolatedOpacity`: a plain Skia layer
restored source-over at the group alpha, which matches the oracle
byte-for-byte, with two in-crate laws pinning the layer and nested-layer
rasters against the measured bytes. Scope owners carry identity and
provenance like nodes; damage treats a scope-opacity edit as the union of
its span.

**Fourteen cells baked (corpus 167 → 181), thirteen byte-exact on the
first gate run** — the rotated translucent group's straight-edge AA and
the per-layer nesting quantization included. The fourteenth, the dithered
ramp under a real layer, carries the corpus's third `ramp-quantization`
tolerance at its measured bounds (336 of 2304 pixels, one code value):
the layer restore halves every ramp value, and the two Skia builds round
one code value apart across the ramp — the radial knife-edge's physics,
multiplied by the layer.

At that rung the refusal register moved one row out and two in.
`svg-element-opacity` graduated (the lone-fill fold, byte-exact), while
`svg-element-opacity-gradient` and `svg-root-opacity` became load-bearing
patrols. The gradient patrol named the missing post-materialization alpha
factor; the root patrol named the then-missing transparent whole-frame scope.
Those are historical rung facts, not the current boundary: the opacity-closure
addendum below graduates both after supplying exactly those two facts.

`opacity` joined csscascade's admitted presentation hints (one
<alpha-value> grammar; percentage spelling, clamping, hint precedence,
and invalid-drops all measured and baked). `<use>` and `<a>` scope
exactly as `<g>`; opacity through a translucent target compounds
per-layer, byte-identical to the nested-group cell. `opacity: 0` renders
the correct nothing for shapes and containers alike. The scoreboard
suite's element-`opacity` rows are now within reach; `rx`/`ry` is the
remaining row-1 construct.

## Addendum — the conic rung (2026-08-06)

The elliptical arc and the rounded rect are consumed, and consuming them
grew the contract's **third amendment**: `rframe::PathCommand` gained
`ConicTo { x1, y1, x, y, weight }` — the rational quadratic, the curve
class Chromium's rasterizer draws arcs and rounded corners through. The
weight has its own checked domain (positive and finite,
`BadConicWeight` by index), a conic's tight bounds are solved from the
rational derivative in f64 (the quadratic `N'D − ND'`, whose cubic terms
cancel), and the amendment deliberately stops there: **no arc command**
remains the crate's refusal. The arc parameterization is authored
syntax, and which curve sequence it becomes decides pixels, so the
resolution stays with the producer — exactly as the module doc's
reserved slot said it would.

The repo law that read "the contract carries no conic command yet" was
repaid rather than falsified; but one measured *prediction* fell: the
corpus README reasoned that admitting arcs "inherits exactly this
departure" — the oval cells' declared conic scan-converter divergence.
Measured, it does not. All eleven new cells — the half-ellipse identity
arc, all four flag combinations, a rotated elliptical sweep, both
degenerate correct-nothings, uniform and elliptical rounded rects, the
measured clamp order, and both strokes — bake **byte-exact** against
Chromium 149.0.7827.55 with no tolerance blocks. The departure class
belongs to the `drawOval` construction, not to the conic as a curve.

**A 24-probe Chromium matrix decided the semantics before code:**

- **A rounded rect is a conic path.** `<rect rx="8">` and the equivalent
  explicit `A`-command contour render byte-identically in Chromium
  itself (elliptical corners: 1 pixel at delta 2). The rect therefore
  lowers at the producer to four quarter-turn conics of weight cos 45°,
  and `rframe::Geometry` needed no rounded-rect variant.
- **The rect's resolution order is measured, not assumed.** `auto`
  adopts the other axis's *authored* value first, then each axis clamps
  to half its own extent independently — `rx="30"` on a 40×48 rect is
  (20, 24), not (20, 20). Negative is invalid-to-auto; a used zero on
  either axis squares every corner; percentages base on width and
  height respectively.
- **Arc corrections are the spec's, byte-for-byte.** Too-small radii
  scale up uniformly (identical to authoring the scaled radii);
  negative radii take absolute value; zero radius is the authored
  straight line; coincident endpoints elide; a smooth cubic after an
  arc reflects about the current point.
- **Nothing canonicalizes, and nothing reduces.** Chromium paints a
  rotated circular arc 2 pixels from its unrotated spelling and `390°`
  51 pixels from `30°` — its internal float noise, which no external
  construction can or should chase. The producer feeds the authored
  angle through plain f64 arithmetic: a circle's rotation then cancels
  algebraically, the unreduced angle's residue sits below f32, and each
  cell gates against its own oracle. n0-model's native-route
  canonicalizations (circle rotation to zero) were deliberately **not**
  copied — the Web producer's conversion is its own, verified against
  the browser it matches.

Consuming `rx`/`ry` closed a latent silent-divergence class: Chromium
honors the CSS `rx`/`ry` spellings over the attributes (measured), and
the pinned cascade cannot represent those longhands — so both names
joined the `d`-property stylesheet patrol, declared by name instead of
silently painting attribute geometry where the browser paints the
sheet's. The `websem` parser's arc arm keeps the malformed-before-
unsupported ordering it always had, now with nothing left to be
unsupported: `PathDataError::UnsupportedCommand` and
`CompileError::UnsupportedPathCommand` are deleted — the `d` grammar's
every command letter emits, and the sole path refusal is a value that
stops being path data. The whole-path refusal on erroneous data (vs
Chromium's valid-prefix rendering) stands unchanged; admitting the
prefix rule remains a rung of its own.

Graduated: `svg-path-arc` and `svg-rect-rounded` left the refusal
register for the admitted corpus (the identity-measurement geometry is
now its own cell), and the checklist's `rx`/`ry` presentation-attribute
rows tick. `d` stays unchecked while the prefix rule is refused, and
the CSS-property twins stay unchecked at the pin. With `rx`/`ry`
landed, the row-1 scoreboard construct the register named after the
gradient rung is in hand: the corpus-growth step toward FLIP
eligibility no longer waits on capability.

## Ratified amendment — resolved stroke dash intervals (2026-08-13)

The resolved render contract may now carry an optional stroke-dash interval
cycle. This is the fourth contract amendment of the Web-first ladder, ratified
before producer or consumer code under the contract-first precedent of
gridaco/nothing#75.

The fact is deliberately narrower than the source grammar. It is an immutable,
even-length sequence of local-space distances, alternating painted and
unpainted intervals and beginning with paint. A present cycle is non-empty;
each distance is finite and non-negative; and the finite sum of the cycle is
strictly positive. The phase is exactly zero and the cycle restarts at the
beginning of every contour. Nothing in the fact names a source unit,
percentage, cascade rule, authored list length, or path calibration.

The producer resolves the source language into that canonical form. An odd
source list repeats once to become even. A source `none` or an all-zero list
reaches the same solid-stroke dash absence. An invalid declaration contributes
nothing, so the cascade's surviving winner — a lower declaration, an inherited
cycle, or the solid initial value — is what resolves. A positive cycle with
zero-length painted intervals remains meaningful under round or square caps,
which paint at those interval endpoints; under a butt cap, a cycle whose every
painted interval is zero paints nothing and the stroke itself resolves away.
Dashes do not alter the stroke's conservative reach beyond its geometry.

Two nearby facts remained expressly outside this amendment when it landed. A
non-zero dash offset is phase, and `pathLength` is authored calibration rather
than a resolved render fact. Both were load-bearing refusals once dashing
existed: the probe matrix measured offset moving the pattern, and measured
`pathLength` scaling dash distances on paths, rectangles, circles, and
ellipses. The latter patrol therefore belonged on every admitted geometry
element, not only path-like elements. The later signed-phase and source
path-distance amendments resolve those ordinary cases without changing the
cycle contract stated here.

The same matrix fixed the producer boundary before code: numbers and lengths,
comma and whitespace separators, attribute and CSS spellings, inheritance,
invalid-declaration fallback, and odd-list repetition all converge on the one
cycle; percentages and mixed length-percentage arithmetic resolve against the
normalized viewport diagonal. As earned in gridaco/nothing#80, unit classes
with their own checklist rows may remain refused only when every silently
divergent ingress is named, registered, and guarded. Viewport-, container-, and
unavailable font-metric bases, variable indirection, poisoned font-relative
bases, and escaped spellings retain that refusal burden here. The standard
grammar remains the bar, following gridaco/nothing#77; a partial admission
would be the unticked split established by gridaco/nothing#81, not grounds to
widen this contract.

### Dasharray rung verdict — cells landed, rows split

This subsection records the gridaco/nothing#83 verdict as it landed. Its
cycle-sum premise is superseded by the 2026-08-14 ratified correction below:
the apparent authored-magnitude remainder was Chromium's fixed-length used
clamp, not an unrepresentable resolved-frame fact.

The amendment is exercised by 25 Chromium-149-baked cells. Together they cover
the attribute and CSS spellings; number, length, percentage, exponent, comma
and whitespace forms; CSS math and an authored font-size basis; odd-list
repetition; invalid-declaration fallback; author-over-hint precedence;
inheritance through containers and use-site instances; the `none`, all-zero,
negative-invalid, and zero-painted cap edges; all admitted geometry kinds;
closed and mixed contours; per-contour restart; and local-space scaling. Every
new cell is byte-exact through the shared Web producer, resolved contract, and
kernel. The corpus therefore moves from 230 to 255 gated cells. The final cell
pins Chromium's renderer-level saturation of a cycle dense enough to exceed
the painter's bounded dash expansion: line, cubic, rect, ellipse, and round-cap
routes do not share one coarse fallback, so the cell discriminates both an
all-solid and an all-absent implementation. This is measured renderer behavior,
not a source normalization or a resolved-contract limit.

The probe also turned two prepared over-refusals into load-bearing patrols.
Dash offset changes phase for positive, negative, and percentage values.
`pathLength` calibrates dashes on paths and the basic geometry shapes, and
calibrates dash offset on paths; at this rung its patrol stated one law across
all seven admitted geometry elements. Neither fact was smuggled into the
zero-phase, uncalibrated contract. Both ordinary cases graduate in the later
signed-phase and source path-distance amendments.
The solid-stroke mixed-contour cap refusal narrows in the other direction:
dashed segments have ends on closed contours, so one authored cap is correct
for every contour and the dashed case is admitted.

At this rung six new residual classes held named refusal rows: dash offset;
untrustworthy unit bases; variable indirection; a poisoned font-relative basis;
escaped spelling; and a cycle whose individually finite intervals summed
beyond the representable contract range. The existing `pathLength` patrol was
the seventh load-bearing class. The first five new classes and `pathLength`
carried their gaps in their own checklist rows or the unit rows, under the
gridaco/nothing#80 precedent. The later path-distance amendment graduates that
seventh refusal without replacement.
The cycle-sum class does not:
Chromium honors the standard-track dasharray grammar for that magnitude class,
and a zero-painted round-cap probe proves it cannot be normalized wholesale to
a solid stroke. That is gridaco/nothing#81's exact split condition. The cells
land, but both `stroke-dasharray` checklist rows remain unchecked until a
representability rung closes the measured, registered remainder. No record
claims the rows ahead of that evidence.

## Ratified amendment — context paint resolves before the frame (2026-08-13)

The resolved render contract does not gain a context-paint value. The owner
ratified this fifth contract amendment by authorizing the full context-paint
arc after its recursion and reference-box scope was stated. A context paint is
a source relationship, not a visual fact: it selects another element's fill or
stroke, while the frame states only the eventual no-paint, solid, linear-ramp,
or radial-ramp result. The relationship must therefore finish before the
resolved boundary, and no context element, use chain, paint-server reference,
or reference-box provenance may cross it.

The selection rule is exact. Without a context element, either context keyword
selects no paint. Inside an instantiated subtree, the immediate use element is
the next context: `context-fill` selects its computed fill and
`context-stroke` selects its computed stroke. A selected context keyword
repeats that step. Selection ends at the first ordinary paint value or at no
paint. The paint's own colour alpha belongs to the selected paint; fill and
stroke opacity remain separately inherited properties and are not copied as
part of context selection.

A selected paint server keeps the coordinate space and reference box of the
context element that authored that eventual server value. An intermediate use
whose own paint is another context keyword does not take ownership of the
server's box. Before the resolved frame is stated, the selected ramp is rebased
from that context space into each destination geometry's self-contained paint
facts. Object-bounding-box and user-space ramps therefore remain continuous
across differently transformed descendants without asking the consumer to
recover an instance relation. The context box contains geometry hidden by
visibility or zero opacity, but excludes a display-pruned subtree.

The amendment is deliberately no wider than the resolved paint vocabulary.
Patterns, images, and external resources remain inexpressible even when a
context relation selects them; source handling must refuse them by their own
names rather than disguise them as context-paint failures. Marker context is a
separate applicability rung. A source parser's extension that permits a
fallback after a context keyword is likewise not admitted: the standard-track
paint grammar permits a fallback only after a URL, and the Chromium oracle
drops the extended declaration.

This was ratified as a contract-first amendment: by itself it authorized a
producer to emit an already selected and rebased paint through the existing
resolved vocabulary and made no claim that the producer, cells, or checklist
rows had landed. The rung verdict below records that subsequent evidence.

### Context-paint rung verdict — four rows close

A 102-capture Chromium 149 matrix fixed the source semantics before the
capability landed. It measured both context keywords in both destination
properties and both source spellings; the no-context no-paint result; host
`none`, `currentColor`, colour alpha, inheritance and CSS-wide values; all
four property crossings; nearest-context recursion and independent instances;
URL fallback; linear and radial gradients in object-bounding-box and user
space; the eventual outer URL owner's box through nested context references;
and the box contribution of hidden, zero-opacity, and display-pruned geometry.
Paint opacity is not part of selection: it remains an independently inherited
property.

Twenty-two of those cases are now committed Chromium cells. Eight are the
atomic product of destination fill/stroke, selected fill/stroke, and
attribute/CSS spelling. Four cells isolate no context, host no-paint,
`currentColor`/alpha, and inherited/CSS-wide values. Two establish recursive
selection, nearest-owner precedence, light-tree absence, and independent
instances. The remaining eight establish missing-URL fallback, the inert
fallback behind a valid stopless gradient, both gradient kinds in both unit
systems, eventual-owner anchoring, and the three box-participation cases.
Every new cell crosses the same resolved-frame and kernel boundary as the
existing corpus; no context relation or reference-box provenance crosses it.

Two scratch follow-ups remain explicitly measured, not celled. Every `<use>`
`x`/`y` translation on a nested consumption chain moves the selected paint,
while the eventual ordinary paint owner remains the owner of its URL and box.
Chromium constructs a context object bounding box from each descendant's
transformed *local axis-aligned box*, not from exact post-transform curve
extrema; rotated and skewed controls discriminate the two. A singular
destination transform paints nothing across the admitted filled and stroked
shape classes. These verdicts close the coordinate forks without widening the
resolved contract.

The standard-track boundary stays sharp. Chromium drops a fallback following
`context-fill` or `context-stroke`, because `<paint>` permits that tail only
after a URL; the pinned source parser accepts the extension, so one new named
registered refusal and guards cover attribute, inline-style, and stylesheet
ingresses. The former load-bearing context-paint refusal graduates, leaving the
register at 56 rows at that rung (and still 56 through the stroke-width
replacement below).
Context-selected patterns and external paint resources
were also measured to propagate in Chromium, but remain refused by their own
resource names. Marker context and author stylesheets across a use-shadow
boundary likewise retain their own rows. Under the own-row precedent of
gridaco/nothing#75 and gridaco/nothing#80, none of those gaps belongs to the
four paint rows; under gridaco/nothing#77, the non-standard fallback extension
is outside their grammar bar.

The CSS SVG-presentation `fill` and `stroke` rows and their SVG
presentation-attribute twins therefore tick together. This is a capability
verdict only. It produces no conformance score and takes no FLIP action.

## Ratified correction — stroke used-value range (2026-08-14)

The broad width-refusal conclusion recorded here is historical rung evidence.
The 2026-08-20 amendment below supersedes it: general positive percentage
saturation is now carried and Chromium-celled, while a narrower source-precision
alias remains refused.

The browser's used-value range, not the resolved render contract, closes the
dasharray remainder recorded by gridaco/nothing#83. Chromium clamps each pure
fixed stroke length before using it. The authored ceiling is 33,554,429
in its fixed-point CSS length range, represented in the resolved scalar
vocabulary as 33,554,428. For a dash list, that clamp happens member by member
before an odd list is repeated. The existing contract's finite, non-negative,
positive-sum cycle therefore already states the browser's resolved fact; no
contract or consumer amendment is warranted.

Percentages follow a distinct used-value rule. A moderate resolved percentage
may exceed the fixed-length ceiling and remains unclamped. At the extreme, an
overflow during percentage resolution causes Chromium to drop the dash effect,
so the stroke becomes solid while retaining its authored cap. The same extreme
percentage used as stroke width is not governed by that dash-effect result:
under a discriminating transform, Chromium paints a butt-capped round or bevel
join but not the default miter or round/square-cap variants. It therefore
entered as a named capability at this correction instead of being normalized
to one universal no-stroke result.
These are property-specific outcomes, not a shared normalization of every
large length.

The earlier short-path probe did not discriminate authored `3.4e38` from the
browser's used clamp: either interval stayed longer than the entire path, so
both painted the same apparent solid or initial cap dot. The corrected matrix
put the transition inside large user-space geometry and located the fixed
ceiling directly: 33,000,000 differs, while 34,000,000, 40,000,000, and
`3.4e38` converge. It separately measured percentages above that ceiling and
the extreme percentage outcomes for dasharray and width.

Two byte-identical Chromium cells now cover the presentation-attribute and CSS
dasharray spellings. Together they discriminate the fixed used clamp, odd-list
repetition, restart at each subpath, and the cap-preserving solid result for an
extreme percentage. A third cell repairs the fixed-length evidence of the
previously closed width family by covering the clamp in both spellings. At this
correction, the extreme percentage width class was measured, not celled, and its
presentation-attribute, CSS, and
percentage-only arithmetic ingresses were guarded by one named refusal.
The corpus moves from 277 to 280 cells; 97 of its 98 `svg-stroke-*` cells are
byte-exact, with only the previously declared closed-path tolerance. At this
correction, the cycle-overflow refusal left as the percentage-overflow width
refusal entered, so the register remained at 56 rows. Dashoffset, `pathLength`,
unit-basis, variable-indirection, escape, and font-basis patrols were unchanged.

The earned precedents apply without revision: gridaco/nothing#77 supplies the
standard-track grammar bar, gridaco/nothing#80 assigns independent unit and
spelling gaps to their own guarded rows, and gridaco/nothing#81 remains the rule
for a genuine measured split. Here the corrected evidence removes that split's
premise for dasharray, so both `stroke-dasharray` checklist rows tick. It also
corrects gridaco/nothing#80's width evidence: the valid extreme-percentage
class has no independent row and Chromium paints some of it, so both
`stroke-width` rows reopened under gridaco/nothing#81 pending that named
capability. This is a capability verdict only; it produces no conformance score
and takes no FLIP action.

## Ratified amendment — finite wide stroke reach (2026-08-20)

Every accepted resolved stroke now has a finite, conservative, direction-free
reach outside its geometry. This is the sixth contract amendment of the
Web-first ladder. The carried width, cap, join, miter limit, paint, and optional
dash cycle do not change. Only the arithmetic domain of the reach derived from
those facts widens before any operation, so independently valid finite members
cannot become an unrepresentable stroke merely because their aggregate bound
exceeds the carried scalar range.

The bound still means exactly what it did. A round or bevel join and a butt or
round cap reach one half-width. A miter is bounded by the larger of one and its
miter limit, in half-widths. A square cap reaches a half-width along and across
the endpoint, so its direction-free bound is the half-width times the square
root of two; the represented irrational result rounds outward. The result is
finite for every combination of finite carried members, including the largest
width and miter limit the contract can state.

A consumer that needs a finite damage envelope projects this wide reach through
the node transform before narrowing it. It intersects the projection with the
finite frame clip and only then encodes a frame-bounded envelope; a fully
clipped node has no pixel envelope but retains its material attribution. Scope
unions obey the same frame bound. None of this changes the exact stroke sent to
the painter, the geometry's own bounds, or a gradient's geometry reference box.
It widens conservative bookkeeping, not paint semantics.

### Stroke-width saturation rung verdict — cells landed, rows split

The Chromium 149 matrix fixed the percentage operation order and its renderer
edges before the amendment. For a pure percentage, the authored percentage is
multiplied by the normalized-diagonal basis before division by 100. Positive
overflow in that intermediate resolves to the maximum finite width. It does not
mean one universal no-stroke result: under discriminating transforms, round and
bevel joins paint where the default miter does not; a miter limit of exactly 2
paints while the next representable value does not; an ordinary dash cycle can
survive; closed and cornered topologies can drop; and round and square caps that
drop under normal-axis-only compression paint when both axes are compressed.

Four Chromium-baked cells carry that evidence in presentation-attribute and CSS
twins. Two cover an unambiguous direct finite `5e36%` control, direct positive
saturation, miter/join, dash, and topology branches. Two cover
the round- and square-cap split with independently discriminating alpha passes.
All four are byte-exact. The primitive corpus moves from 280 to 284 cells; 101
of its 102 `svg-stroke-*` cells are byte-exact, with only the already-declared
closed-path tolerance.

The same probe found a narrower remainder that is not saturation-specific. On a
64×32 user space, the valid adjacent values `100.00000762939453%` and
`100.00001525878906%` collapse to the same percentage bucket in the pinned
cascade (`0x3f800001`), while Chromium retains distinct used widths: their
amplified rasters differ by 16 pixels. The last-finite and first-overflow pairs
at 64- and 32-unit normalized-diagonal bases expose the same loss at the
saturation boundary. A non-f32 decimal pair around `57384.265625%` differs by
864 pixels despite reaching the same computed bucket, while non-identity
percentage math reaches further rasters after the cascade has erased its
operation history — including authored length terms that cancel to zero before
the pure computed percentage arrives. These cases are measured, not celled.
Once distinct source values with different oracle results have become one
computed fact, a producer cannot
select any result without being silently wrong for another. Direct ambiguous
values and folded percentage math therefore refuse by name across the
attribute, inline-style, stylesheet, and inherited ingresses.

The former broad percentage-overflow refusal graduates into the four cells and
the percentage-precision-alias refusal enters in its place, so the refusal
register remains at 56 rows. Unit-basis, mixed arithmetic, variable-indirection,
escape, and font-basis patrols remain unchanged. The standard-track bar from
gridaco/nothing#77 admits both aliased spellings; the own-row rule from
gridaco/nothing#80 cannot move this gap elsewhere because it has no independent
row. It is therefore the gridaco/nothing#81 split condition that
gridaco/nothing#86 reopened the width family to resolve: cells land, but the CSS
`stroke-width` row and its SVG presentation-attribute twin remain unchecked.
This is a capability verdict only. It produces no conformance score and takes
no FLIP action.

## Ratified amendment — signed stroke dash phase (2026-08-21)

A resolved dash pattern now pairs its checked interval cycle with one finite
phase in the same local path-distance space. The phase is not source syntax:
units, percentages, cascade, authored signs, and odd-list repetition have
already resolved before the boundary. Construction canonicalizes the signed
value modulo the positive repeated cycle. Positive phase advances into that
cycle, equivalent signed and multi-cycle values have one representation, and
the same canonical phase restarts at the beginning of every contour.

Phase cannot exist without a positive cycle. An absent cycle, `none`, an
all-zero cycle, or a cycle dropped by the browser's extreme-percentage rule
remains the single solid-stroke state, and dash offset is inert there. Moving a
live cycle changes along-path paint placement but cannot change the stroke's
direction-free reach outside its geometry. The phase, intervals, width, and
geometry remain local under the node transform. Path-length calibration was
not part of this amendment when it landed; the following source path-distance
amendment resolves it into these same interval and phase facts and graduates
the separate refusal.

The fixed used-value range is signed and asymmetric before phase
canonicalization. Blink's positive fixed ceiling is authored 33,554,429 and is
carried as f32 33,554,428; its negative floor is -33,554,430 exactly. On an
`8 4` cycle, extreme positive and negative fixed offsets therefore canonicalize
to phases 4 and 6. Percentage offsets follow the percentage used-value route,
including finite saturation at the extreme, rather than either fixed bound.

### Dashoffset rung verdict — cells landed, rows split

Chromium 149 fixed the ordinary phase semantics in a 91-source matrix captured
twice. Its 69 exact pair verdicts covered the attribute and CSS spellings;
positive, negative, zero, unitless, px, exponent, and percentage values;
normalized-diagonal basis in a non-square viewBox; signed and multi-cycle
modulo after odd-list doubling; zero painted slots under all caps; open,
closed, and multi-contour restart; every admitted geometry; uniform and
non-uniform transforms; inheritance, use-site inheritance, and CSS-wide
values; precedence and invalid-declaration fallback; solid-stroke inertness;
and path-length calibration. A separate numeric matrix established both fixed
bounds, direct percentage saturation, and the source-precision boundary. A
focused residual matrix then established the guarded viewport-unit, variable,
escape, and poisoned-font-basis ingresses with exact numeric controls.

Thirteen of those cases are committed Chromium cells; the complete cell ledger
and its measured-not-celled mutation controls live in the
[Web-first evidence table](../../../fixtures/web-first/README.md). Two cells
cover the base attribute/CSS grammar, two cover signed percentages and their
viewBox basis, two cover odd-cycle modulo and zero-length cap slots, two cover
contours and every geometry/transform route, three cover inheritance,
`<use>`, CSS-wide values, precedence, and invalid fallback, and two cover the
fixed and percentage used ranges. Every cell is byte-exact; the three
attribute/CSS twin matrices and the two phase-four cascade outcomes are
byte-identical.
Scratch zero-phase mutation controls *(measured, not celled)* change every
oracle by the exact counts recorded in the Web-first evidence table.

Measured facts deliberately not assigned their own cells include omitted,
explicit zero, and negative-zero identity; further equivalent signed modulo
pairs; duplicated transform/topology/cap cross-products; phase inertness when
no live cycle exists; and path-length calibration, which was measured here but
not celled by this rung. The following path-distance amendment gives it its own
exact cells and removes that refusal. The residual unit, variable, escaped
spelling, and poisoned-font-basis probes remain registered guards rather than
capability cells. The unit and variable families retain their independent
checklist rows under the gridaco/nothing#80 precedent; the Chromium-invalid
comment-split property spelling may be conservatively over-refused under
gridaco/nothing#77.

The source-precision blocker is decisive. The valid authored percentages
`57384.265625%` and `57384.267578125007%` collapse into one pinned-cascade f32
bucket while Chromium retains distinct phases: their rasters differ by 120
pixels in both source spellings. Their negative mirrors differ by 142 pixels.
The last-finite and first-overflow percentage sources likewise collapse while
Chromium differs by 688 pixels. An adjacent pair around 100% was identical on
the same sensitive geometry, so this is a narrower provenance gap rather than
a rejection of percentage precision in general. Tested non-identity percentage
math happened to agree with its direct high control on that matrix, but the
cascade erases its operation history; one bank cannot establish every such
expression, and re-evaluating CSS math after the cascade would be a second
matcher.

The broad dashoffset refusal therefore graduates, while a stable
percentage-precision-alias refusal and four source/basis guards replace it.
The primitive corpus moves from 284 to 297 cells. Its stroke inventory moves
from 102 to 115 cells, 114 byte-exact, with only the existing closed-path
tolerance. The refusal register moves from 56 to 60 rows. The valid
Chromium-honored precision class has no independent checklist row, so this is
gridaco/nothing#81's SPLIT condition: the CSS `stroke-dashoffset` row and its
SVG presentation-attribute twin both remain unchecked. This is a capability
verdict only. It produces no conformance score and takes no FLIP action.

## Ratified amendment — source path-distance calibration (2026-08-21)

An SVG geometry may now calibrate its dashed stroke from one authored
`pathLength` number without adding source syntax to the resolved render
contract. Let _A_ be the browser-compatible total of the geometry's non-empty
local contours and _L_ a positive authored length. Every already-resolved dash
member and the signed raw phase receive the common factor _A / L_. An odd list
still repeats into one even cycle, and phase canonicalization still happens
against the resulting positive cycle. The frame boundary remains unchanged:
it carries only those final local-space intervals and the canonical phase, not
the authored number or a second calibration fact.

Calibration is a geometry-local operation. It precedes a node transform and
the root viewBox mapping, and every contour restarts with the same calibrated
phase. The attribute is non-inherited and applies only to path, line, rect,
circle, ellipse, polyline, and polygon. On an instance, the referenced
geometry's value participates; a spelling on the use site or a container does
not. With no live dash cycle the attribute is inert. Chromium's legacy value
branches are preserved: absence and a negative number leave distances
uncalibrated, while zero, negative zero, and a malformed present value select
the authored-zero branch. On geometry with a non-zero local metric, the
resulting scale saturates rather than becoming zero; a zero local metric instead
yields a zero calibration factor. At the tested numeric edges this makes the
resulting effect solid or densely cap-painted according to the ordinary dash
and cap rules; it is not a new stroke state. A saturated scale can also leave a
tiny interval cycle finite while overflowing a large finite authored phase.
Chromium then rejects the dash effect and retains the solid stroke.
Malformed-present values reach the same outcome through the authored-zero
branch, and both admissions carry it without a refusal.

The metric and the eventual dash traversal must observe one consistent set of
local coordinates. Chromium's native oval is four rational conics measured and
traversed in f32, and that arithmetic is observably translation-sensitive.
Moving an oval's box origin into a transform is equivalent over real numbers
but can move antialiased dash endpoints. A calibrated dashed oval therefore
retains its absolute local bounds through painting. This is a precision
invariant, not a new geometry kind or a change to solid-oval semantics.

The current standard also defines the CSS property
`path-length: none | <length [0,∞]>`, non-inherited on the same seven geometry
elements, and maps the attribute as a pixel-length presentation hint. The
pinned Chromium 149 build has that experimental property disabled. Valid
inline and stylesheet declarations therefore drop wholesale and do not
override the active legacy attribute. Matching that drop requires no parallel
matcher and follows the gridaco/nothing#77 precedent: a valid listed member the
browser itself drops does not keep a row open once the drop is celled.

### Path-length rung verdict — nine cells landed, both rows tick

A twice-deterministic 96-pair attribute matrix established a broad source
branch before admission: standard number spellings; negative, zero, malformed,
underflow, overflow, and the probed large-finite behavior; positive and
negative phase; modulo after odd-list repetition; zero-length painted slots
under every cap; solid-cycle inertness; all seven geometry routes; straight,
quadratic, cubic, conic, rounded, open, closed, mixed, and move-only contour
metrics; transforms, viewBox, instances, group inapplicability, and XML case
sensitivity. Chromium also scales fixed and percentage members alike after
percentage resolution on the normalized diagonal, including a mixed list.
That measured behavior disagrees with the current SVG2 statement that
`pathLength` does not affect percentage distance-along-path calculations; the
Chromium-gated cells record the disagreement rather than silently substituting
the standard's operation order.

A supplemental numeric black-box bank then exercised 269 authored cases in
five composites, each captured twice in Chromium for determinism and rendered
once through the engine for comparison.
After correcting a raster-layout confound, the invalid, rounding, range, and
deterministic composites were exact, and 45 of the 48 grammar cases were exact.
The three remaining spellings — `5.`, `5.e0`, and `125.e-3` — are
invalid-present in Chromium rather than valid trailing-dot numbers. Their
interaction with a saturated scale, a tiny finite cycle, and a large phase
exposed the solid-fallback edge now carried by the ninth cell and the corrected
admission. The post-correction rerender makes all five composites exact, with
zero differing pixels, zero maximum channel delta, and no degradation.

Three DOM metric pins close the native-oval precision question at the exact
tested positions: a radius-12 circle centered at (40, 30) is
74.91236114501953 (`0x4295d321`); radii 16×10 centered at (80, 30) are
82.2452621459961 (`0x42a47d93`); and radii 24×12 centered at (140, 30) are
115.52963256835938 (`0x42e70f2c`). Adjacent raw dash values discriminate every
pin. Source-number measurements separately pin the long-decimal results
`123456789.123456789 → 0x4ceb79a2` and
`1.654435761 → 0x3fd3c48e`, the smallest-subnormal and underflow boundary,
exact overflow rejection, and negative-zero bits.

Nine committed Chromium cells carry the admission. They separate attribute
grammar; phase/list/cap algebra; fixed and percentage scaling; every geometry;
curves and summed contours; transforms, viewBox, and instance ownership;
numeric extremes; the finite-cycle/non-finite-phase fallback; and the disabled
CSS property in inline and stylesheet spellings. All nine engine renders have
zero differing pixels and zero maximum channel delta; representative strict
renders are declaration-free and byte-identical to best effort. Their 50
scratch mutation controls comprise 44
pixel-discriminating differences and six exact equivalence controls. The
per-cell mutation counts and the measured-not-celled remainder are enumerated
in the [Web-first evidence table](../../../fixtures/web-first/README.md).

Facts measured but deliberately not assigned further cells include the
remaining equivalent number spellings and invalid-present forms, duplicated
transform and topology cross-products, further modulo identities, zero-length
geometry, inapplicable group and wrong-case spellings, and the exact numeric
and oval metric pins above. The CSS cell carries representative `none` and
non-negative pixel-length members through both source ingresses; the disabled
property registration makes the remainder a full-grammar audit rather than a
second collection of identical drop cells.

The primitive corpus moves from 297 to 306 cells. The path-length refusal
graduates without replacement, so the named register moves from 60 to 59. The
stroke-prefixed inventory remains 115 cells, 114 byte-exact, with only the
existing closed-path tolerance. The previously missing CSS `path-length` row
is added and ticked, and the SVG `pathLength` attribute row ticks. Path
consumers not admitted here — including markers and text-on-path — retain their
own checklist rows and named capability boundaries; they do not reopen this
attribute's carried grammar. This is a capability verdict only. It produces no
conformance score and takes no FLIP action.

## Addendum — opacity closure (2026-08-21)

The two opacity remainders left by the group-scope rung are now resolved. A
valid gradient paint can carry element opacity without flattening it into the
gradient's intrinsic opacity, and opacity on the root `<svg>` can enclose the
complete transparent SVG-local frame. The same outer-scope meaning extends to
each non-identity HTML ancestor of the selected inline SVG. These are three
source spellings of two source-neutral render facts: a factor on one paint
pass, or an isolated scope around a composite.

`rframe::PaintStack` now carries one checked `PaintAlphaFactor` in the closed
unit interval. It is neither intrinsic paint opacity nor group opacity. Each
paint entry materializes its own alpha first; the factor then multiplies that
float alpha before coverage and source-over compositing, without a second
8-bit quantization. On a multi-paint stack it applies independently to each
entry and changes no paint order. Identity is the ordinary producer default,
and zero normalizes the whole stack to no paint. The factor is part of the
resolved stack's equality, so a factor-only change is also a raster-identity
and damage change rather than an invisible producer annotation.

The n0 boundary copies that bit-exact value into its private drawlist as a
post-paint opacity. Every fill and stroke route applies the same float fold
after its existing solid/gradient alpha materialization. Native n0 producers
state identity, so no n0-model public API grew. Preflight sees the same drawlist
fact as rasterization, and the factor participates in item equality and damage.
This is the measured one-draw fold of Chromium's alpha-layer effect; it is not
a replacement for a group scope. Fill plus stroke, overlapping descendants,
nested opacity, and the existing structural fold boundaries continue to use
the isolated `Scope` fact and retain their per-layer quantization.

The producer keeps the alpha routes distinct:

- A direct solid combines colour alpha, fill/stroke opacity, and the eligible
  element factor in its existing one-product fold.
- A valid paint server keeps fill/stroke opacity intrinsic to its paint and
  attaches element opacity as `PaintAlphaFactor`. A one-stop server becomes a
  two-identical-stop constant gradient, retaining Chromium's gradient and
  dither route. An exact geometric degeneracy may resolve to a solid only when
  the staged alpha remains representable, and it still retains the later
  element factor.
- An invalid URL's authored fallback is an ordinary direct solid. Its factors
  fold as solid alpha; it does not acquire paint-server staging merely because
  its source mentioned `url()`.
- Root opacity is an isolated scope around the complete item stream in both
  standalone and inline entries. Each HTML ancestor contributes its own outer
  scope in tree order. Opacity remains non-inherited by default; an explicit
  `inherit` on the SVG adds the root scope beneath the ancestor scopes. A zero
  root or host scope resolves to the transparent empty frame before an
  unsupported descendant can leak a false refusal. Every scope consumes the
  same checked nesting budget and retains ordinary scope identity, provenance,
  and damage semantics.

### Evidence and the retained stop-precision boundary

Eight committed Chromium-149 cells carry the closure, all byte-exact:
`svg-element-opacity-gradient`, `svg-element-opacity-gradient-css`, and
`svg-element-opacity-gradient-stroke` cover gradient fill/stroke, both source
spellings, intrinsic paint opacity, and a non-half element factor;
`svg-opacity-grammar-attr` and `svg-opacity-grammar-css` cover the twin grammar
and cascade surface; `svg-root-opacity` and `svg-root-opacity-zero` cover the
transparent whole-frame scope and its zero; and
`html-inline-svg-ancestor-opacity` covers three distinct non-half host/root
layers, precedence, and explicit inheritance.

The scratch evidence remains explicitly outside the committed corpus. A
twice-repeated 15-pair standalone bank produced six identities and nine
differences; a twice-repeated eight-pair HTML bank produced two identities and
six differences. The discriminating mutations include flattened gradient
factors, all-identity grammar controls, per-child substitution for root/group
scopes, visible substitution for zero, identity substitution for each host
layer, and flattening two or three host scopes. Every source was captured twice
per run and every output was byte-identical across runs. The exact pair names,
pixel counts, and maximum channel deltas are enumerated in the
[Web-first evidence table](../../../fixtures/web-first/README.md), not promoted
to additional cells.

A separate 32-case grammar composite is exact against Chromium. Focused
transparent-root probes measured attribute/CSS, root/whole-content-group,
zero/empty, above-one/ordinary, and direct/math identities. Twelve adjacent
number, percentage, presentation-attribute, and `calc()` decimal pairs around
the tested 127.5/255 and 64.5/255 alpha thresholds were also identical. Those
precision aliases are harmless at the final element-opacity raster step, so
opacity has no stroke-width-style authored-provenance blocker on the measured
surface.

The same investigation found a different, own-row gap in gradient stops.
Chromium resolves a stop colour's base alpha byte, multiplies `stop-opacity` in
float, and carries the effective alpha into the shader. A non-byte product
cannot cross the current RGBA8 gradient-stop contract honestly. A valid
degenerate server also cannot collapse an exact-byte translucent stop with a
non-endpoint fill/stroke opacity, or with a later non-identity post-paint
factor, into one RGBA8 solid. The exact stop-77, paint-identity,
element-`.6` witness for the latter differs by 2,304 pixels at one code value.
Finally, the integral average of a degenerate `reflect`/`repeat` ramp can
synthesize either a nonintegral alpha or a nonintegral colour channel: opaque
`#000000`→`#010000` produces the decisive later-opacity red-half witness, while
exact-byte-alpha `#00000080`→`#01000080` exposes the same rounding at identity.
Twice-captured zero- and negative-radius radial banks additionally fix the
tile-specific degenerate result: `pad` selects the last stop, while `repeat`
and `reflect` select the integral ramp average. This is therefore a
component-precision boundary, not alpha provenance alone.

One stable `svg-gradient-stop-precision` refusal guards those five semantic
loss classes. Live and constant exact-byte gradients and endpoint-safe `pad`
degenerates remain admitted. The alpha-average guard conservatively
over-refuses both the measured 77→128 ramp and an endpoint 0→1 ramp when the
derived alpha is non-byte. A fractional RGB average remains admitted only when
its own alpha is opaque and both later opacity stages are identity, where
measured bytes agree; a non-endpoint average alpha or later paint/post-paint
opacity exposes its rounding and refuses. That deliberate over-refusal belongs
to the independent `stop-color`/`stop-opacity` capability family. Flattening
the refused witnesses changes up to 2,304 pixels by one code value. Those two
presentation-property rows and both presentation-attribute rows remain open;
their author-CSS ingresses remain unimplemented.
Under the gridaco/nothing#75/#80 own-row precedent, this discovered and
quarantined gap does not hold the CSS `opacity` property or SVG
presentation-attribute `opacity` row open.

The primitive corpus moves from 306 to 314 cells. The two broad opacity
refusals graduate and the narrower stop-precision refusal enters, so the named
register moves from 59 to 58. The CSS `opacity` property and its SVG
presentation-attribute twin tick; `stop-color` and `stop-opacity` do not. This
is a capability verdict only. It produces no conformance score and takes no
FLIP action.

## Rung: the SVG `<stop>` presentation attributes (2026-08-22)

`stop-color` and `stop-opacity` are read directly off the `<stop>` element,
because the pinned Stylo build has no such longhand. That is what made the
previous rung's account of them incomplete: nothing cascades over these two
values, so nothing resolves them either, and four spellings that need a
resolver were falling back to the initial instead of refusing. All four were
measured painting a wrong pixel silently. `stop-color="inherit"` under an
ancestor gradient carrying `stop-color="red"` paints red in Chromium and
painted the initial black here — 4,096 pixels at Δ253; the `stop-opacity`
twin is Δ190. `var()` is substituted inside a presentation attribute
(Δ190). A CSS math function is valid in `stop-opacity`, whose SVG 2 value is
the `opacity` property's own, and Chromium evaluates it: `calc(1 / 3)` is
byte-identical to the literal third, against Δ169 here. A non-legacy sRGB
stop colour — `color(srgb …)` — parses into the same colour space as hex and
was admitted, but it changes how Chromium interpolates the *whole* ramp
rather than only its endpoint: 4,080 pixels at Δ26, where the identical
value as a solid is byte-identical to `#010000`. Each now refuses by name,
and each names a construct carrying its own checklist row.

The previous rung's `svg-gradient-stop-precision` refusal split in two, and
the split is the substance of this rung. Its live-ramp half was a contract
limit, not a browser fact: Chromium hands the ramp the float product of a
stop colour's alpha byte and `stop-opacity`, and `stop-opacity="0.5"` is
distinguishable from *both* 127/255 (992 pixels) and 128/255 (2,424). The
resolved leaf now carries that product unquantized — `cg::GradientStop` and
the model tier's `GradientStop` hold checked unit-sRGB float components,
which is the width `format/grida.fbs` already declared and the decoder was
narrowing. Byte RGB is enough: every path that could have carried sub-byte
RGB is either quantized by Chromium itself (measured — a solid, a degenerate
substitution, `rgb()`, and a colour's own alpha all land on bytes) or is a
colour function with its own row.

Its degenerate half survives, renamed to what it actually is. Chromium does
not paint a degenerate paint server as a flat colour; it keeps a shader, and
that shader dithers. Where every stage lands on a byte there is nothing to
dither and the flat solid is byte-identical, and those cases are now celled:
a `repeat` average of 77/255 and 129/255 landing exactly on 103/255, a
`reflect` colour average of 0 and 3 rounding up to 2, `pad` with a byte-exact
translucent last stop, and the zero-radius radial route. Where a stage does
not, Chromium's output matched no flat solid and no constant ramp the probe
could construct — a degenerate `pad` at stop alpha `0.5` under
`fill-opacity=".7"` sat 2,560–4,096 pixels from every candidate at Δ1, the
signature of a dither tied to the degenerate shader's own geometry. Guessing
that rule would be a silent wrong pixel, so `svg-gradient-degenerate-precision`
refuses it. **The refusal fires on the collapsed value, not on how it arose**:
two byte-exact `stop-color`s whose ramp average lands between codes trip it
with no `stop-opacity` present at all. It is therefore a gradient-geometry
gap belonging to the already-ticked `<linearGradient>`/`<radialGradient>`
rows, not a stop-grammar gap, and it does not hold the two attribute rows
open. It is filed as gridaco/nothing#93.

The primitive corpus moves from 314 to 319 cells; the named register moves
from 58 to 62. The SVG presentation-**attribute** rows for `stop-color` and
`stop-opacity` tick. Their CSS presentation-**property** twins stay open and
unchanged: the pinned cascade has no longhand for either, so a sheet or a
stop's style attribute remains a document-level refusal, and closing those
rows needs the longhands, never a second matcher around Stylo. This is a
capability verdict only. It produces no conformance score and takes no FLIP
action.

## Rung: SVG geometry presentation attributes `cx`/`cy`/`r` (2026-08-22)

The rung began at the raw-number boundary, before any admission work. These
three attributes never meet the cascade: the pinned Stylo build has no
`cx`/`cy`/`r` longhands, so the compiler reads the presentation-attribute text
directly. That ownership remains correct. Routing the values through a new
matcher around Stylo would create a second cascade. The unresolved question
was narrower and numeric: whether Rust's direct f32 parse and arithmetic retain
the same used value as Blink's CSS-number path.

They do not. Four amplified, twice-deterministic Chromium-149 probe families
establish the boundary:

- For the stroke-rung source alias `57384.267578125007%`, Chromium selects the
  lower normalized control for each of `cx`, `cy`, and `r`. The higher control
  differs by 88 pixels at maximum channel delta 240, 89 pixels at delta 236,
  and 80 pixels at delta 255 respectively. The previous geometry route selected
  that higher neighbour;
  direct Chromium-versus-engine source renders differed by 88, 89, and 144
  pixels.
- A decimal just above the exact midpoint between f32 1 and its successor is a
  second divergence class. Chromium's decimal-to-f64-to-f32 path selects the
  lower neighbour, while the direct decimal-to-f32 route selected the higher.
  The three higher controls differ from Chromium by 88 pixels at delta 242,
  88 pixels at delta 236, and 80 pixels at delta 255.
- Ordinary percentage arithmetic has an independently observable order. On a
  ten-unit basis, `r=".5%"` is byte-identical to
  `0.05000000074505806` in Chromium and differs from the
  division-before-multiplication neighbour `0.04999999701976776` by 2,176
  pixels at delta 255. The producer now multiplies by the basis before dividing
  by 100. The extreme oval amplification needed to reveal one ULP also exposes
  a separate 144-pixel Blink/Skia raster difference even when both receive the
  same numeric control; that arithmetic fact is therefore measured and
  bit-lawed, not promoted into an inexact corpus cell.
- A clamp-sensitive range matrix exposed a separate silent boundary. On a
  64-unit basis Chromium drops `3.4e38%` for each attribute exactly to empty,
  while finite direct `2.176e38` controls survive its used-value clamp and,
  under `scale(.000001)`, paint 230 pixels for `cx`, 227 for `cy`, and 3,520
  for `r`, all at delta 255. The old percentage route leaked infinity and made
  n0 fail later with generic invalid frame bounds. The direct-center controls
  returned success with no declaration but painted empty, by those same exact
  differences; direct `r` also produced invalid bounds.

The first two valid classes now trip one stable
`unsupported SVG geometry: <attribute> numeric precision alias loses Chromium
used-value provenance` guard for `cx`, `cy`, and `r`. Its f64 route is a
one-way classifier only: agreement leaves the raw parser in charge;
disagreement refuses, and the shadow value is never substituted as a second
parser. A nine-row refusal addition records this precision class, the
percentage-overflow and fixed-used-range classes, the absent CSS longhands,
unit-bearing values, CSS math, `var()`, CSS-wide keywords, and CSS comments.
The first four value families each retain their own unchecked checklist row.
Comments and the range classes are no-own-row valid gaps: Chromium strips the
comments and renders the ordinary number exactly, drops the overflowing
percentage, and clamps the direct extreme; this producer refuses all three
boundaries rather than guessing or leaking a backend-only drop.

The semantic repair is otherwise local. Negative `cx` and `cy` remain valid
coordinates. Missing and explicit-zero centers coincide at zero. A missing,
zero, or negative circle radius now produces no frame node: negative `r` is an
invalid element geometry, not a magnitude to clamp, and all three cases render
the same honest nothing. Percentages keep the existing `PercentBases` contract:
the x and y axes for `cx`/`cy`, and `sqrt(width² + height²) / sqrt(2)` for `r`,
using the mapped `viewBox` extent when present and initial-viewport user units
otherwise. `<use>`, transforms, and strokes consume those resolved facts
without changing their meaning. A resolved non-finite value now refuses at its
attribute, and `cx`/`cy` plus positive `r` values outside the established Web
fixed-length range refuse until the geometry used-value clamp is represented;
negative `r` keeps its invalid no-node meaning. Circle box
construction checks the radius, origin, diameter, and both expanded corners
for finiteness before any rectangle fact crosses the resolved-frame seam; the
shadow checks never supply a replacement geometry.

No cross-crate seam changed. `rframe` already states circles and ellipses as
an ellipse over a local rectangle; n0 already lowers that fact to its native
oval fill/stroke routes. The compiler's correction stops before that boundary.
CSS declarations are quarantined at their authored ingresses: a stylesheet is
named at the sheet and a style declaration at its element. The CSS
presentation-property twins remain open at the pinned-cascade cap, with no
matcher layered around Stylo.

Five committed Chromium cells carry the admitted subset. The grammar cell
pins absent/explicit-zero centers, negative centers, number spellings, and the
three non-rendering radius cases. A 64×32 unmapped initial viewport and a
70×10 `viewBox` separately pin axis and normalized-diagonal bases. The final
two cells carry `<use>` instances and transform-plus-stroke composition. Their
exact controls reproduce each oracle. The wrong-center and positive-radius
mutations differ by 132 and 672 pixels; the unmapped and `viewBox` wrong-basis
mutations differ by 778 and 1,492; deleting the instances differs by 97; and
the composed wrong-basis mutation differs by 480. Every maximum delta is 255.

The gate's sensitivity was tested, not assumed. Temporarily routing `cx`
percentages to the y axis made `just gate` fail on four cells, including the
new unmapped and `viewBox` witnesses at 796 and 2,378 differing pixels. Restoring
the axis table returned the complete gate to green. All candidate sources were
also rendered through the actual `n0` CLI path; the five admitted cells were
declaration-free and decoded-pixel exact to their Chromium probes.

The review-time range probes were also run through the actual CLI before and
after the patrol. Before it, the overflowing-percentage composite failed only
downstream as an invalid frame and the finite direct-center controls exited
cleanly with wrong empty pixels. Afterwards best-effort succeeds while naming
all three skipped attributes, and strict refuses on the first stable
`unsupported SVG geometry` reason; no non-finite frame reaches rframe or n0.
Temporarily lowering the new used-range ceiling to one made `just gate` fail
immediately on `svg-circle-defaults-clip`; restoring the measured ceiling
returned the full gate to green.

The primitive corpus moves from 319 to 324 cells; the ten exact-time sampled
frames are unchanged. The named refusal register moves from 62 to 71 rows.
Neither the SVG presentation-attribute nor CSS presentation-property rows for
`cx`, `cy`, or `r` tick: the valid numeric-precision, range, and comment classes
have no independent row, while the CSS twins remain structurally unavailable
at this Stylo pin. This is a measured SPLIT verdict under the
gridaco/nothing#81/#89/#90 precedent, not a capability closure. It produces no
conformance score and takes no FLIP action.

## Rung: SVG geometry presentation attributes `x`/`y`/`width`/`height` (2026-08-22)

This follow-on began with the same source-number question as the preceding
circle rung. Rect coordinates and extents take the same raw finite
number/percentage route, while Chromium reaches their presentation values
through its CSS-number path. Root sizing, CSS property spellings, and the
attributes' applications to resource-bearing elements remain separate
contracts; the probe first asked only whether the shared raw route preserved
Chromium's used geometry.

It did not. Twice-deterministic Chromium-149 probes reproduce both known alias
classes on every attribute (measured, not celled):

- `57384.267578125007%` selects the lower normalized control in Chromium and
  the higher neighbour on the former producer route. The higher controls
  differ by 32 pixels for `x` and `y`, and 16 for `width` and `height`, all at
  maximum channel delta 238.
- A decimal just above the exact midpoint between f32 1 and its successor
  selects the lower neighbour in Chromium and the higher neighbour on the
  former producer route. The four differences have the same 32/32/16/16
  pixel shape at maximum delta 218. For both classes, the producer source was
  pixel-identical to Chromium's higher control while both explicit controls
  were cross-engine exact.

A second range matrix exposed a distinct silent boundary (measured, not
celled). Chromium carries finite direct `2.176e38` values through its fixed Web
used-length clamp:
positive `x`/`y` equal the upper-bound control, negative `x`/`y` equal the
lower-bound control, and positive `width`/`height` equal the upper-bound
control. Each clamped position paints 144 pixels beyond empty (delta 218 for
positive and 233 for negative); each clamped extent paints 544 pixels beyond
empty at delta 217. The former producer positions exited cleanly and painted
empty. Its huge extents painted 896 pixels from the origin instead of the
clamped 544, leaving 368 wrong pixels. Negative extents are different: every
magnitude is invalid element geometry, and both engines paint the same honest
nothing. A finite `3.4e38%` source overflows only when the viewport basis is
applied; Chromium leaves no visible rect for all four attributes — off-canvas
coordinates and disabled extents — and the attributable overflow refusal is
pixel-equivalent in best-effort.

The cascade boundary had one further leak (measured, not celled). Chromium
honors inline and stylesheet `x`/`y` declarations over the presentation
attributes. The pinned Stylo build has neither longhand, so both declarations
formerly painted the attribute position with no departure; each measured 512
pixels from Chromium. They now refuse at their authored ingress. CSS
`width`/`height` are represented at the pin but remain unconsumed for SVG
geometry, so their existing computed-style refusal continues to guard both
ingresses. No matcher was added around Stylo.

The admitted rect semantics are otherwise unchanged and now explicit.
Coordinates default to zero and accept signs, leading-dot and exponent number
forms; negative `x`/`y` remain valid. Missing, zero, or negative extents disable
the whole element, including its stroke. Percentages use the independent
viewport axes in unmapped root units and in mapped `viewBox` units, and retain
those values through same-document `<use>`, transforms, and centered stroke.
The source-provenance classifier is one-way: a disagreement refuses by
attribute and never substitutes its shadow value. Coordinates refuse outside
both measured Web boundaries; positive extents refuse above the upper boundary;
negative extents keep their invalid no-paint meaning.

The broader presentation-value grammar remains quarantined, not silently
defaulted (measured, not celled). Chromium makes `px`, `calc(16px + 16px)`, and
`var(--v)` exact to a literal `32` for all four attributes; strips comments
around ordinary values; resolves `initial`, `unset`, `revert`, `revert-layer`,
and unoverridden `inherit` to coordinate zero or `auto`; and makes rect
`width="auto"` and `height="auto"` exact no-paint geometry. This producer
refuses every one by its exact attribute. Units, CSS math, custom properties,
and CSS-wide values retain their own unchecked rows. Numeric provenance, the
unimplemented used clamp, comments, and rect `auto` are valid no-own-row gaps
and therefore hold the four attribute rows open.

Three new Chromium cells carry only new evidence. The grammar cell pins
defaults, number forms, negative coordinates, and the six disabled extent
branches; its canonical control is exact and the wrong mutation changes 792
pixels at delta 255. The percentages rung's existing
`svg-percent-rect-root-units` and `svg-percent-rect-in-viewbox` cells already
pin the two basis contexts and were reused rather than duplicated. A new use
cell is exact to its expanded numeric light tree and differs from no instances
by 512 pixels at delta 238. A new 64×32-user-space transform-and-stroke cell is
exact to its numeric control and differs from the swapped-axis mutation by
1,121 pixels at delta 238. All candidate sources and controls were also
rendered through the actual CLI path; each admitted source was declaration-free
and decoded-pixel exact to Chromium.

The gate's sensitivity was proved by temporarily routing y-axis percentages
through the width basis. `just gate` failed loudly, including the retained new
transform-and-stroke cell at 1,094 differing pixels and maximum delta 238.
Restoring the axis mapping returned the complete gate to green. Ten registered
refusal rows cover numeric provenance, percentage overflow, fixed used range,
CSS properties, units, CSS math, custom properties, CSS-wide values, comments,
and rect `auto`; strict and best-effort name the same attributable reason for
every skipped element.

No cross-crate seam changed. Resolved rectangles, instance transforms, and
stroke facts already cross the frame contract and lower through the one n0
kernel; the correction stops at source admission. Root `auto` remains the
admitted absent-dimension value, while root percentage sizing and cascaded CSS
sizing remain document-level refusals. `<image>`, `<pattern>`, and `<mask>` keep
their own element/resource refusals, so this rect evidence does not claim their
geometry.

The primitive corpus moves from 324 to 327 cells; the ten exact-time sampled
frames are unchanged. The named refusal register moves from 71 to 81 rows.
Neither `x`/`y` CSS presentation-property row nor any of the four SVG
presentation-attribute rows ticks. This is a measured SPLIT verdict under the
gridaco/nothing#81/#89/#90 precedent, not capability closure. It produces no
conformance score and takes no FLIP action.

## Rung: SVG path-data presentation attribute `d` (2026-08-23)

The `d` attribute closes. Its complete standard-track grammar is
`none | <path-data>`: every path command was already admitted, including arcs;
this rung repays the remaining valid-prefix divergence and the numeric faults
that became visible while measuring that boundary. The CSS property twin does
not close. Chromium honors `d: path(…)`, while the pinned cascade has no `d`
longhand; both authored CSS ingresses remain named by the existing patrol, with
no second matcher around the cascade.

The first defect was the known prefix rule. Chromium retains every complete
segment before a path-data error, including complete implicit repeats, and
never emits part of an incomplete compound command. A trailing move-only
contour contributes no visual segment; a failure before a complete leading
moveto is empty geometry. Unknown commands, incomplete line/cubic/arc repeats,
bad arc flags, errors after close, overflowing exponents, and the decimal at
the exact finite range boundary were all measured against explicit prefix
controls. The three refusal rows that formerly turned those cases into skipped
elements therefore graduate.

The prefix probe exposed a separate silent pixel class in valid input. Blink's
SVG number parser accumulates decimal digits in ordered float operations;
parsing the same token as an ideal decimal and rounding once can select the
other neighbouring float. Both directions reproduce: `1188.679260273` selects
the lower neighbour in Chromium, while `5186.454833937` selects the upper. The
former producer selected the opposite result for each. Amplified path probes
move 96 pixels at maximum delta 255 per full-height witness. Polygon points use
the same source-number grammar and reproduced the fault at 48 pixels per
witness. The repair is consequently shared by path data, polygon/polyline
points, and the already-established path-distance number route; feature-local
rounding rules would have left one shipped consumer silently wrong.

A second range class belongs to path construction rather than source parsing.
Finite authored numbers can produce a non-finite derived coordinate. An
ordinary line or reflected curve then invalidates the whole browser path,
erasing earlier ink. An endpoint arc can instead abandon construction before
appending a path segment: the earlier prefix survives, the logical current
point still advances, and following relative commands use that point. Huge
equal radii and subnormal non-zero radii take this no-segment branch; a huge
finite rotation remains a live authored angle. These outcomes follow the
float conic construction of Chromium's pinned path builder, including its
maximum of three conics, rather than the earlier four-quarter, double-precision
model. The resolved contract already carries rational conics, so no contract
or painter seam changes.

Six new `d` cells and one companion `points` cell carry the evidence. The
empty-prefix and retained-prefix controls differ by 2,304 pixels at delta 237
and 1,178 at delta 233. Substituting the two wrong numeric neighbours changes
96 path pixels at delta 233 and 48 polygon pixels at delta 225. Swapping the
ordinary-poison and arc-no-segment outcomes changes 768 pixels in either
direction at delta 232. Replacing the huge/subnormal arcs with lines changes
192 pixels at delta 232; reducing the huge angle changes 150 at delta 86. The
dedicated no-segment/current-point witness differs by 446 pixels at delta 218
from either wrong model: keeping the stale point or appending an implicit move.
Every candidate was rendered through the actual CLI path with no declaration,
and every decoded raster was exact to its Chromium probe.

The byte gate's sensitivity was proved by temporarily restoring the former
one-shot float parse. `just gate` failed on the new path-number cell by 96
pixels and on the shared-points cell by 48, both at maximum delta 233. Restoring
the ordered SVG evaluator returned all cells to green. The primitive corpus
moves from 327 to 334 cells; the ten exact-time sampled frames are unchanged.
The refusal register moves from 81 to 78 rows. The SVG presentation-attribute
`d` row ticks; the CSS presentation-property row stays open at the pinned
cascade boundary. This records no conformance score and takes no FLIP action.

## Rung: SVG geometric `clip-path` path strategy (2026-08-24)

The verdict is SPLIT. Same-document geometric clip resources now carry a large
and useful rendering slice, but `clip-path` as a whole does not close. The
standard property also contains CSS basic shapes, geometry boxes, a root
CSS-layer route, and raster-mask strategies. Those branches remain explicit
work. The independently listed `clipPathUnits` attribute does close: its
complete `userSpaceOnUse | objectBoundingBox` grammar is carried by committed
Chromium evidence.

`clip-path` enters through the pinned cascade rather than a parallel matcher.
The presentation attribute is a hint; inline style and stylesheet declarations
beat it, `none` removes it, and the shipped `-webkit-` alias and custom-property
substitution reach the same typed computed value. A malformed reference value
computes to no clip. Same-document fragments use document-order, first-id
lookup: a missing id or a non-`<clipPath>` target is likewise no clip, while an
external resource refuses because this product has no resource environment.
The CSS basic-shape and geometry-box variants are represented by the cascade
but refuse at the source boundary instead of being mistaken for `none`.

One resource layer is the union of its visible direct geometry contributors.
All seven admitted geometry kinds contribute, as does a direct `<use>` whose
expanded target is one of those shapes. Their authored fill, stroke, and
opacity do not paint into the clip; a direct container is not descended; hidden
or pruned contributors add no path. A valid resource with no contributing path
is meaningful and clips every pixel. Missing or wrong-kind resources are
different: they install no clip. The cells distinguish those outcomes.

The admitted fill rule is the inherited `clip-rule` presentation attribute.
Default and explicit `nonzero`, `evenodd`, inheritance, and the CSS-wide
behavior were measured, with the discriminating compound contours committed.
Its CSS property twin is absent from this Servo-mode Stylo pin, so author CSS
is quarantined and no second matcher is grown around the cascade. Valid
presentation spellings containing CSS comments, an escape, or `var()` also
remain named over-refusals: the direct inherited reader cannot tokenize them
without becoming that second matcher. These lexical gaps keep the attribute
row open too.

The coordinate-system measurement fixes the order. Missing
`clipPathUnits` and explicit `userSpaceOnUse` use target user space;
`objectBoundingBox` first maps the unit square through the target's fill
geometry box, then applies the resource's own transform. Percentages inside
that resource retain the current viewport basis before the object-box map.
The box excludes stroke, and a zero-area target produces the valid empty clip.
An invalid case-sensitive units token falls back to user space. Object-box
group bounds as the union of descendant fill geometry and an object-box clip
on a `<use>` target applying instance `x`/`y` exactly once were also confirmed
(measured, not celled).

Child, resource, and target transforms stay distinct through outer `viewBox`
mapping. A clip on a clip resource becomes another intersection layer; a clip
on already clipped target content is another nested effect scope. Resource
opacity is inert, while target opacity encloses the clipped result. The
resolved normal form therefore needs only unions of geometry and intersections
of layers. It carries no URL, DOM identity, paint, image, or backend mask, and
cannot silently widen into one.

The line between path and mask was established by measurement. Chromium keeps
the path-union strategy through 42 visible contributors and switches at 43.
Visible text and a contributor carrying its own clip use the mask strategy at
any count. Those three branches refuse under one registered raster-strategy
row. A root `<svg>` takes a different host CSS-layer route and refuses in both
admissions. Cyclic chains, resource animation, external URLs, basic shapes,
and geometry boxes have their own stable rows. A load-active animation inside
clip geometry skips the referencing target in best-effort and refuses in
strict mode; stale authored resource geometry is never rendered as though it
were the browser's Base value.

Eight new Chromium cells carry the admitted slice. Seven are byte-exact. The
graduated circle resource differs at six native-oval boundary pixels with
maximum channel delta 3, exactly the existing bounded oval class; every
straight-edge, transformed, chained, rule, units, target, and opacity control
is exact. The cells cover all source ingresses, first-id and invalid-reference
semantics, geometric unions and inert paint, both fill rules, both unit modes,
percentage/object-box order, `viewBox` and all three transform seats, groups,
target and contributor `<use>`, stroke-box separation, chain intersection, and
effect ordering. Every candidate was also rendered through the actual product
command, not merely captured in Chromium.

The gate's sensitivity was proved by temporarily changing clip intersection
to subtraction. `just gate` rejected the graduated circle cell as a geometry
defect, with the mismatch 17.23402 pixels outside its declared boundary ring.
Restoring intersection returned the complete gate to green. The contract also
bounds each union at 42 contributors and each chain at 64 layers, checks all
geometry and transforms before they cross the frame boundary, and preflights
backend path operations so failure cannot become a silent unclip.

The primitive corpus moves from 334 to 342 cells; the ten exact-time sampled
frames are unchanged. The former broad clip-path refusal graduates, and nine
narrower strategy, source, and cascade rows replace it, moving the named
register from 78 to 86 rows. Only `clipPathUnits` ticks. The `<clipPath>`
element, both `clip-path` rows, and both `clip-rule` rows stay open with their
remaining work named above. This records no conformance score and takes no
FLIP action.

## Rung: SVG image masks (2026-08-24)

The verdict is SPLIT. Same-document image masks now carry a substantial
rendering slice on admitted non-root SVG targets. The independently listed
`maskUnits`, `maskContentUnits`, and `mask-type` presentation-attribute rows
close. The `<mask>` element and `mask` presentation-attribute row stay open for
valid source, cycle, layer, host, and precision branches named below. Every CSS
mask-family row also stays open: this Servo-mode Stylo pin furnishes no
computed mask route this producer can consume, so authored CSS is quarantined
at ingress rather than matched by a second cascade. The audit also restores
the missing unchecked `mask-border-mode` row to the checklist.

The reference semantics are document ordered and source local. One direct
`url(#…)` resolves through whole-document, first-id-wins lookup; CSS comments
around it are accepted. A missing id, a wrong-kind id, malformed syntax, or
explicit `none` installs no mask. A valid empty source is different: it always
hides the target. An opaque black source hides it in luminance mode but reveals
it under `mask-type="alpha"`. External resources stay
outside this self-contained command, and an active mask on the root `<svg>`
stays outside the SVG-local frame because Chromium applies it through the host
CSS layer. Full mask layers, multiple layers, and custom-property substitution
likewise remain with their independently listed property/value routes.

One source image composites all of its admitted children before interpretation.
Missing `mask-type` and explicit `luminance` use Chromium's luminance weights;
explicit `alpha` uses source alpha. Two overlapping half-alpha source shapes
therefore produce three-quarter coverage, not a per-child mask operation.
Paths, stroke ink, groups, transforms, gradients, clips, `<use>`, opacity, and
nested masks all participate in that source image. The mask element's own
display, opacity, transform, mask, and `clip-path` are inert; the last is
measured in both attribute and CSS spellings. A resource-own CSS `filter` is
also inert, while its attribute twin remains an over-refusal under the filter
row (measured, not celled). The resource's inline style is not generally
inert: Chromium inherits `shape-rendering: crispEdges` into the source exactly
like the same child declaration, 96 pixels at maximum delta 63 from the
default, while the former n0 route emitted that default byte-identically.
Resource-own `color-interpolation: linearRGB` also moved 30 pixels at delta 1
from the default. Unrepresented source-side cascade effects now refuse by one
focused row (measured, not celled). Any unsupported source child
invalidates source construction transactionally: strict admission refuses and
best-effort skips the whole referencing target. Partial source paint can never
escape as a plausible but wrong mask.

The coordinate measurement fixes both spaces and the region. Missing
`maskUnits` means `objectBoundingBox`; missing `maskContentUnits` means
`userSpaceOnUse`. Their explicit opposite and same-as-default spellings are all
carried, as are invalid-value fallbacks. The default region is
`-10% -10% 120% 120%` in the target's fill-geometry box, excluding stroke.
User-space percentages use the current viewport or mapped `viewBox`.
Object-box numbers and percentages resolve through the target box, while
object-box content maps its unit square through that box. The region is a hard
boundary with no antialias fringe; zero or negative extents make an empty mask.
CSS-wide and invalid region spellings take the per-field default; `inherit`
does so even under a parent carrying an explicit width (measured, not celled).
Inline and stylesheet CSS `x`/`width` declarations on `<mask>` are inert; only
the corresponding SVG attributes moved the measured region (measured, not
celled).
Target transforms carry the region, source transforms remain in source space,
a target clip encloses the mask, and target opacity encloses the masked result.
The measured same-element order is therefore clip outside opacity outside the
mask image.

The proposed source-parsing precision crux did not reproduce on the new
CSS-token route (measured, not celled). Direct-number and `px` midpoint sources
selected the lower adjacent control in Chromium and n0 under an admitted pure
translation. With the independent upscale patrol temporarily bypassed, the
percentage source selected the upper control in both. Each opposite control
differed by 96 pixels. There is therefore no source-provenance refusal in this
rung. Ordinary percentages still retain Blink's observable multiply-then-
divide order, `basis × percentage ÷ 100`, rather than normalizing the
percentage first.

The raw route also exposed the established fixed Web used-length boundary
(measured, not celled). Chromium made `x="1000000000"`,
`x="100000000000000000000"`, the valid beyond-f32 exponent `x="1e100"`, and
the adjacent 33,554,430/33,554,432 controls identical to 33,554,428. Before the
patrol, each huge source lost 1,728 pixels and the adjacent controls lost
96/192, all at maximum delta 255. Region fields now refuse outside that
unimplemented clamp; the x witness is exact and the sibling fields
conservatively share its named range boundary.

A separate precision class did reproduce after source parsing. Translation and
sampled positive axis-aligned downscales through identity were exact. At
x-scale 1.01 the threshold-aligned lower and upper controls differed from
Chromium by 96 and 48 pixels respectively, both at maximum delta 255 (measured,
not celled). The mask route refuses upscales and conservatively over-refuses
rotations, reflections, and shears through the same named boundary. This
boundary is deliberately local to mask-region hard-edge rasterization; it does
not reopen the already landed general transform grammar or claim a generic
damage-envelope result.

Nineteen new Chromium cells carry the admitted slice. Eighteen are byte-exact.
The luminance-gradient cell differs at 576 pixels by one code value and carries
that exact cell-local `ramp-quantization` bound; every solid, region, transform,
clip, nesting, and source-geometry cell is exact. The final six-panel grammar
cell explicitly distinguishes both coordinate-system enums and
`luminance`/`alpha`: replacing the two object-box branches changes 639 pixels
at maximum delta 255, and replacing luminance with alpha changes 768 at delta
201. Every scratch candidate was captured through the shared hash-pinned
Chromium posture and rendered through the actual product command.

The byte gate's sensitivity was proved by temporarily disabling luminance
conversion in the mask painter. It rejected the default-luminance and
first-id cells at 2,304 differing pixels (maximum deltas 201 and 233) and the
gradient cell at 2,304 pixels / delta 253, far outside its 576/1 bound.
Restoring luminance conversion returned the complete gate to green.

The former broad mask refusal graduates. Sixteen narrower CSS-ingress,
resource, source, value-family, cycle, and precision rows replace it, moving
the named register from 86 to 101. The primitive corpus moves from 342 to 361
cells; the ten exact-time sampled frames are unchanged. Only `mask-type`,
`maskUnits`, and `maskContentUnits` tick. `<mask>`, both `mask` rows, and every
other CSS mask-family row remain open with their work named above. This records
no conformance score and takes no FLIP action.

## Rung: SVG filter chassis and `feGaussianBlur` (2026-08-25)

The verdict is SPLIT. A resolved image-filter graph now crosses the shared
frame boundary and the first static primitive paints, but neither the
`<filter>` nor `<feGaussianBlur>` element row closes. The independently listed
`filterUnits` and `primitiveUnits` attributes do close at their complete
case-sensitive enum grammars. The `filter` presentation attribute, its CSS
property twin, `color-interpolation-filters`, `in`, `result`, `stdDeviation`,
and `edgeMode` all stay open for their wider applicability or named remainder.

The resolved contract carries one checked operation space, a positive hard
effect region, and a bounded acyclic list whose inputs are the source image,
source alpha, or an earlier node. Authored URLs, DOM identity, result strings,
unit spellings, parser state, and painter objects do not cross it. The consumer
preflights construction before painting and applies the result to one isolated
target image. A construction failure therefore cannot restore the target
unfiltered. This is the chassis later primitives extend; it is not an SVG-shaped
second renderer.

The source route is deliberately split from CSS. One direct presentation
attribute accepts `none`, the reset keywords, or one same-document URL token,
with either quoted or unquoted `url()` content, on admitted non-root SVG
targets. CSS comments around the URL are accepted;
comments inside it make an invalid hint. Whole-document, first-id-wins lookup
resolves the resource. A missing id, wrong-kind id, malformed hint, or explicit
`none` installs no filter; a valid empty graph instead produces an empty image
and hides the target. External resources, root-host filtering, filter lists and
functions, `var()`, inheritance, and `<filter href>` remain named boundaries.
Author CSS is quarantined separately because the pinned Servo-mode cascade has
filter functions but not the URL computed variant needed by SVG resources. No
matcher was added around it.

The first graph operation is `feGaussianBlur`. Missing input means
`SourceGraphic`; later missing or unknown inputs select the previous result.
`SourceAlpha` is carried distinctly. The optional Background, FillPaint, and
StrokePaint inputs are unavailable in Chromium's current SVG paint route and
take the same previous-or-source fallback. Result names resolve before the
frame, later duplicate names replace earlier bindings, and reserved built-in
names never enter that table. Unsupported primitives invalidate construction
transactionally, so best effort skips the whole affected target rather than
painting a believable prefix.

The measured `stdDeviation` grammar accepts one or two numbers, whitespace or
a comma, signs, leading dots, and exponents. One value expands to both axes.
Missing, malformed, extra-member, and zero values pass the input through;
negative axes independently become zero in current Chromium, so `3 -1` equals
`3 0` rather than disabling the x blur. Object-box primitive units scale each
axis through the target's fill-geometry box. A sampled decimal immediately
above an f32 midpoint remained exact to Chromium's `3` control; the geometry
raw-reader alias did not reproduce at that witness (measured, not celled).

The region measurement pins the default `-10% -10% 120% 120%` filter region,
explicit user space, object-box mapping, `viewBox` bases, and hard primitive
subregions. Numbers, percentages, and `px` are carried, retaining Blink's
observable basis-times-percentage-before-division order. Non-`px` units, CSS
math, `var()`, and the unimplemented Web used-length range refuse by stable
field name. A non-positive outer filter region is the correct empty image; a
non-positive or disjoint primitive subregion still needs a transparent graph
result and remains a named over-refusal. The complete missing/explicit/invalid
enum matrices for `filterUnits` and `primitiveUnits` are committed and close
those two rows.

The used-range patrol is independently measured. With `x=-33554396`, Chromium
clamps widths `33554432` and `33554436` to `33554428`; both match the ceiling
control exactly and differ from an unclamped crop by 96 pixels at maximum
channel delta 233. The overflowing values now have their own registered
refusal row.

Missing `color-interpolation-filters` is linearRGB in current Chromium.
Explicit `linearRGB` is identical, while explicit `sRGB` differs by 636 pixels
at maximum channel delta 73; explicit `auto` is byte-identical to sRGB, and an
invalid value or `initial` returns to linearRGB (measured, not all separately
celled). The admitted direct inherited attribute preserves conversions at each
operation. CSS ingress, comments, escapes, and `var()` remain named gaps at the
Stylo boundary. All three valid `edgeMode` values and the missing value were
byte-identical even on a boundary-sensitive source; current Blink has no blur
edge-mode field and uses transparent decal. At this rung that browser-dropped
behavior was measured but not celled, so the global attribute row stayed open
for `feConvolveMatrix`. The convolution rung below commits the blur drop and
closes the shared row.

Composition stays one meaning across effects. Filter isolation encloses fill,
stroke, descendants, and overlaps; target transforms carry its operation
space; filters nest; `<use>` applies the effect at the instance; and non-uniform
`viewBox` mapping preserves independent x/y sigma. On one target the measured
operation order is filter, then mask, then opacity, then clip. Dedicated cells
make the clip and mask boundaries hard after the blur, while the group-opacity
cell prevents per-child filtering or alpha folding.

The first backend precision audit found a 680-pixel / maximum-delta-3
difference in a `1 → 2 → 1` blur chain and initially attributed it to graph
depth. The later shadow-graph audit below corrected that diagnosis: a single
effective sigma of `1` already differs, while three chained safe-sigma kernels
are exact (measured, not celled). The focused refusal now names the measured
small-kernel boundary; the resolved contract has no two-operation graph limit.

Twenty-five new Chromium cells carry the admitted slice, all exact. They cover
blur grammar and axis behavior, source alpha, named and previous results,
empty graphs, reference lookup and both URL-token spellings, both regions and
unit systems, both color spaces, transforms, `viewBox`, stroke, groups,
opacity, clip, mask, nesting, and `<use>`. Every candidate also rendered
through the product command. Gate
sensitivity was proved by adding one unit to both resolved sigmas: `just gate`
failed 19 filter cells, with maximum channel deltas up to 75. Restoring the
resolved sigmas returned the complete gate to green.

The former broad filter refusal graduates. Sixteen narrower source, cascade,
resource, primitive, value-family, and precision rows replace it, moving the
named register from 101 to 116. The primitive corpus moves from 361 to 386
cells; the ten exact-time sampled frames are unchanged. Only `filterUnits` and
`primitiveUnits` tick. This records no conformance score and takes no FLIP
action.

## Rung: SVG shadow graph primitives (2026-08-25)

The verdict is SPLIT. `feFlood`, `feComposite`, `feMerge`, and `feMergeNode`
close for their complete static primitive behavior, and the four arithmetic
coefficient rows `k1`–`k4` close at their complete number grammar. `feOffset`
does not close: its identity-mapped integer subset is exact, but three valid
backend-precision classes remain named refusals. The wider `filter`, `<filter>`,
`feGaussianBlur`, `in`, `in2`, `operator`, `result`, `dx`, `dy`,
`flood-color`, and `flood-opacity` rows remain open for their independently
named applicability, cascade, value, resource, or precision remainder.

The resolved graph now states zero-, one-, two-, and ordered N-input image
operations. A node may offset one input, supply one bounded solid source,
composite two inputs by one resolved rule, or merge an ordered input list.
Inputs refer only to the original source, its alpha, or an earlier node; source
names and resource identity still stop before the frame. The checked arity and
backward-reference rules keep the graph acyclic, while an empty merge remains
a real transparent output rather than an absent operation.

Offset uses the measured SVG number behavior. Missing and invalid fields use
zero; signs, leading plus, exponents, and a lone trailing comma prefix are
accepted; unit, percentage, CSS-math, custom-property, and extra-member forms
fall back to zero in current Chromium. User-space and object-box primitive
units, source alpha, previous and named inputs, hard primitive crops, and the
input's unshifted default subregion are carried. Three integer offsets compose
exactly, so the earlier two-operation bound was never a graph law.

Flood is a zero-input sRGB source. Its direct attributes carry initial
black/one, admitted sRGB color spellings, transparent and `currentColor`,
number or percentage opacity with both clamps, invalid and reset fallback,
non-inheritance, and a hard primitive region. A color's alpha is first resolved
to its byte value and then multiplied by flood opacity in float; the result is
not flattened into a second byte before composition. Percentage opacity keeps
the CSS token's parse, divide, then binary32 narrowing order. Parsing to
binary32 before dividing selected the lower neighbour for
`57.384267578125007%`; an arithmetic amplifier changed all 4,096 pixels at
maximum delta 16 until the operation order was corrected. Authored CSS, explicit
inheritance, custom-property substitution, the wider color-function grammar,
and CSS math remain focused refusals under their own checklist rows. The
pinned cascade has no flood longhands, and no second matcher was added around
it.

Composite carries `over`, `in`, `out`, `atop`, `xor`, `lighter`, and
`arithmetic`. `in` is the foreground and `in2` the background. Missing and
unknown inputs follow the measured first-or-previous fallback; an invalid or
wrong-case operator takes `over`. Arithmetic carries four independent signed
number coefficients, initial zero, leading-plus/decimal/exponent forms, and
unit-interval channel clamping. Its default primitive region is the union of
its input regions. These facts close `k1`–`k4`; `operator`, `in`, and `in2`
remain open because the names apply to later primitives too.

Merge reads only direct `feMergeNode` children, in document order. Empty,
singleton, two-input, and longer lists; omitted, unknown, source, previous, and
named inputs; ignored non-node and nested-node children; result reuse; and the
input-union default region are all carried. The crisp compositional shadow —
offset source alpha, flood a color, composite the color into that alpha, then
merge it below source graphic — therefore exercises every admitted arity in
one exact graph.

The precision patrol found five silent classes before these rows closed. Four
remain quarantined; the flood-opacity normalization class above was corrected
and celled. A fractional offset differs from Chromium at 48 pixels with
maximum channel delta 128. Sampled graphs combining blur and offset differ at
540–643 pixels, up to delta 11. An eleven-candidate transform follow-up found
that an integer local offset also differs when scale, rotation, or `viewBox`
mapping makes its device displacement fractional: sampled cases changed 12–97
pixels, up to delta 122, while integer-effective controls were pixel-exact.
All three offset classes now refuse before a partial graph can paint. The blur
audit then contradicted the previous depth premise. Across a
91-candidate follow-up, identity-mapped effective sigmas `0`, `.125`, `.25`,
and every sampled value from `2` through `6` were exact; sampled `.5` through
`1.875` differed. Sigma `1` changes 556 pixels at maximum delta 12. The class
is conservatively patrolled across the open interval between the exact `.25`
and `2` endpoints and follows the target mapping: local sigma `1` scaled to effective `2` is exact,
local `3` scaled to `1.5` changes 312 pixels at delta 4, and local `4` scaled
to `2` is exact. The old depth refusal is replaced by this effective-sigma
boundary. Three safe-sigma chains, the same chains through identity merges,
and parallel safe branches are exact (measured, not celled).

The fully blurred five-node hand shadow is byte-identical to native
`feDropShadow` in Chromium at the sampled sRGB parameters (measured, not
celled). It is not admitted through the hand graph because blur plus offset is
one of the named boundaries; native `feDropShadow` was left for the next
primitive rung at that checkpoint and lands below. Under the default linear
interpolation route the hand graph and native
primitive differ by 135 pixels at maximum delta 2 (measured, not celled), so
that later rung must retain its own color-placement evidence.

Sixty shadow-graph Chromium cells carry this rung, all exact. Eleven cover offset,
fifteen flood, twenty-two composite, and twelve merge plus the crisp shadow.
The extra composite cell pins the admitted default-linear conversion path;
the precision flood cell pins CSS percentage normalization. The other graph
cells use explicit sRGB. Every scratch candidate rendered through the product
command. Gate sensitivity was proved by swapping composite foreground and background:
thirteen cells failed, with as many as 1,776 differing pixels and maximum
channel delta 157. Temporarily restoring parse-before-divide opacity made only
the precision cell fail, at 4,096 pixels and delta 16. Restoring both routes
returned the complete gate to green.

Review added one further blur cell: a zero-sigma primitive remains an operation,
so its explicit primitive region still crops the input. Bypassing that operation
changes 1,160 pixels at maximum delta 218. Hosted x86 verification also exposed
seven otherwise exact composite and merge cells with 2–384 one-code-value
departures. The cause was a CPU-family split in low-precision Porter-Duff
division: generated filter sources require the exact byte-domain divide-by-255
rounding Chromium exhibits, while graphs sampling the source image retain
floating source coverage. Six cells returned to exactness after separating
those graph domains. The remaining case was a one-input merge, which performs
no internal composition and therefore reached Skia's platform-specific final
SrcOver unchanged. The layer restore now carries the same split as the graph:
generated-only results use exact byte-domain SrcOver, while source-derived
coverage stays floating. Both processor families then reach the same exact
oracles without tolerance.

Eight focused offset/flood rows join the refusal register. The former graph-
depth row is corrected to the small-kernel row without changing the count, and
the broad unsupported-primitive witness moves from the now-admitted offset to
`feBlend`. The primitive corpus moves from 386 to 447 cells; the ten exact-time
sampled frames are unchanged. The named register moves from 116 to 124. This
records no conformance score and takes no FLIP action.

## Rung: native `feDropShadow` (2026-08-25)

The verdict is CLOSE/SPLIT. The `<feDropShadow>` element closes for its static
primitive behavior through the existing filter graph. Its shared `dx`, `dy`,
`stdDeviation`, `flood-color`, `flood-opacity`,
`color-interpolation-filters`, `filter`, and `<filter>` rows remain open for
their wider applicability, cascade, value-family, resource, dynamics, or
precision surface. No CSS property row closes.

The resolved graph carries drop shadow as one checked one-input operation. It
states two offsets, two non-negative blur axes, one resolved color with float
alpha, one operation color space, and one hard primitive region. The operation
includes the input as foreground. It is not lowered into blur, offset, flood,
composite, and merge: that decomposition crosses the already named
blur-plus-offset boundary, and a sampled fractional native shadow differs from
the hand graph at 579 pixels with maximum channel delta 13 (measured, not
celled). Authored element identity, input and result names, parser state, and a
backend object still stop before the frame.

Hosted processor-family verification exposed a second backend boundary after
the semantic operation was already exact locally. The direct shadow helper
departed in all twenty-eight cells, by as much as eight code values. Making the
shadow's byte-domain internal compositions exact narrowed the set to twenty-five
source-derived sRGB cells at one code value. Replacing colorization and changing
offset sampling left that same set unchanged, and the zero-blur cell still
departed, ruling out the kernel and offset as the shared cause. The remaining
boundary was the filtered result's restore onto the backdrop. Exact byte-domain
restore for sRGB native-shadow descendants makes both processor families exact
without tolerance. A blanket source-derived rule is incorrect: it changes three
unrelated floating-path cells, so a color-space conversion clears the rule and
the default-linear endpoint remains floating. None of this changes the resolved
vocabulary: the frame still states one drop-shadow operation, and its backend
realization remains painter-owned.

Missing `dx`, `dy`, and `stdDeviation` use Chromium's measured initial `2`.
Signs, fractions, leading plus, exponents, comma-separated axes, and a lone
trailing comma prefix are accepted. One sigma expands to both axes; each
negative sigma independently becomes zero. Invalid, unit-bearing, percentage,
CSS-math, and custom-property forms return the affected field to its initial
in current Chromium. First, previous, source-alpha, named, and unknown input
routing and `result` reuse remain the graph's common rules. No second
raw-number normalization divergence reproduced across the midpoint and
percentage aliases tested for all four scalars (measured, not celled).

Both primitive coordinate systems, hard primitive regions, anisotropic blur,
safe prior and following operations, and direct flood color/opacity values are
carried. The opacity percentage cell reuses the CSS parse/divide/narrow witness
that caught the earlier flood alias. Direct sRGB, `currentColor`, embedded color
alpha times float opacity are carried. A committed endpoint-channel control
proves the default linearRGB route exact while consuming an earlier blur;
missing and explicit linearRGB are byte-identical on that source. Authored flood
CSS, explicit inheritance, wider color functions, CSS math, and `var()` continue
through their existing stable refusal rows; the pinned cascade has no flood
longhands and no matcher was added around it.

Composition stays in the one effect route. Exact cells cover an exact quarter
turn, integer axis scaling, integer `viewBox` mapping, `<use>`, centered stroke,
group source content, target opacity, clip order, a blurred input, and a later
native-shadow result. The native operation therefore neither bypasses the
checked graph nor introduces a second effect-order path.

The backend audit found four additional silent classes and registered each
before the element row closed. Pure translations, exact quarter turns, and
integer axis maps are exact, while a 19-degree rotation differs even when
offset or blur is zero and sampled fractional maps also differ. Solid
fill/stroke source layers are exact; gradients and a non-target descendant
opacity cross a distinct source-layer precision split. Under linearRGB, a
sampled interior-channel shadow color differs at 194 pixels by one code value,
while the committed sRGB control is exact. The Web used-length ceiling control
at 33,554,428 is exact, but larger finite and non-finite-producing parameters
cannot safely cross the checked frame. These range, transform, source-layer,
and color-conversion classes now refuse by stable native-shadow names in both
admission policies (measured, not celled).

The shared blur patrol also applies to native shadow. Identity-mapped effective
sigmas in the sampled open interval between exact `.25` and `2` endpoints
differ from Chromium, while the endpoints and safe anisotropic controls are
exact (measured, not celled). The patrol remains one operation-independent
kernel boundary rather than a second drop-shadow approximation.

Twenty-eight Chromium cells carry the admitted slice, all byte-exact. They
cover defaults, number spellings, independent axes, zero blur, fractional
offset, input and result routing, regions and units, direct color and opacity,
the default-linear endpoint route, the safe transform envelope, source
composition, and neighboring graph nodes. Every scratch and committed source
also rendered through the product command. Gate sensitivity was proved by
temporarily adding one unit to the native operation's horizontal offset: `just
gate` rejected twenty-six drop-shadow cells. Restoring the resolved offset
returned all 475 cells to green.

The same complete 475-cell gate passes on hosted x86 after the scoped restore
rule. The earlier all-cell, internal-composition, colorization, sampling, and
blanket-restore classifiers are retained as measured negative evidence; no
tolerance was introduced.

Four focused rows join the refusal register, moving it from 124 to 128. The
primitive corpus moves from 447 to 475 cells; the ten exact-time sampled frames
are unchanged. Only `<feDropShadow>` ticks. This records no conformance score
and takes no FLIP action.

## Rung: `feColorMatrix` (2026-08-25)

The verdict is CLOSE/SPLIT. The `<feColorMatrix>` element closes for its
complete static primitive behavior through the existing checked filter graph.
The shared `type`, `values`, `in`, `result`, primitive-region,
`color-interpolation-filters`, `filter`, and `<filter>` rows remain open for
their wider element applicability, cascade, resource, host, or dynamics
surface. No CSS property row closes.

The resolved graph carries one row-major 4×5 matrix over non-premultiplied
RGBA. The source conveniences are resolved before that boundary: no authored
type, value list, result name, document node, or backend object crosses it.
The checked fact accepts exactly one input and twenty finite coefficients. The
same graph rules continue to resolve SourceGraphic, SourceAlpha, the previous
result, and an earlier named result, and the ordinary primitive region remains
a hard output crop.

Chromium 149.0.7827.55 establishes the source grammar. Missing or invalid
`type` selects `matrix`; the four valid case-sensitive members are `matrix`,
`saturate`, `hueRotate`, and `luminanceToAlpha`. A matrix is active only with
twenty numbers. Saturation and hue rotation are active only with one number;
their missing or empty list uses one and zero respectively, while a wrong
count passes through. Luminance-to-alpha ignores `values`, including malformed
text. Saturation is not clamped. Hue rotation keeps Blink's float
degree-to-radian and trigonometric operation order rather than reducing the
source angle modulo 360: `360000090` differs from `90` across all 2,304 source
pixels at maximum channel delta 17 (measured, not separately celled).

The number-list audit carries leading signs and dots, exponents, SVG
whitespace, comma-only and mixed separators, one trailing comma, adjacent
signed numbers, and a second dot beginning the next number. A leading or
doubled comma, CSS comment, unit, percentage, CSS function, custom property,
CSS-wide keyword, trailing dot, malformed or overflowing exponent, or
non-ASCII whitespace clears the complete list and takes the measured
pass-through branch. A large negative exponent underflows to zero. The direct
committed grammar probe corrected a stale note from the earlier group probe:
wrong-case and surrounding-whitespace enum spellings are invalid. They select
the default matrix type; surrounding whitespace around `hueRotate` therefore
produces matrix-count pass-through rather than a hue operation.

The arithmetic audit proves straight-channel matrix semantics rather than a
premultiplied approximation. RGB and alpha feed each other; additive terms can
create visible alpha throughout the hard primitive region; outputs clamp to
the unit interval. A positive alpha offset and a visually similar authored
half-opacity source differ by one byte in the sampled control, proving that
the two constructions cannot be folded together (measured, not celled).
Default/missing interpolation is linearRGB. Explicit sRGB differs on the
sampled source in 2,304 pixels at maximum delta 55, and both routes have exact
cells. Generated filter input, SourceAlpha, target opacity, a target clip,
fractional axis mapping, reflection, exact quarter turns, circles, and paths
remain in the one graph and effect-order route.

The pixel patrol found two distinct restore classes and three source-side
precision boundaries before close. Source-derived sRGB matrix output requires
the floating final composition that matches Chromium, while generated-only
matrix output retains the backend's ordinary final composition. A
source-derived matrix also crosses one extra isolated-source boundary unless
it creates alpha from transparent input. Keeping those policies private to
paint preserves the source-neutral resolved matrix.

Even with that realization, curved strokes, translucent anti-aliased fills,
paint-server fills, descendant opacity, and an overlapping source group still
produce wrong edge pixels. The admitted source profile is therefore
conservatively one direct admitted geometry with one opaque solid fill, no
stroke, and no children; generated-only graph input is independent of that
profile. Fractional axis maps, reflections, and exact quarter turns are exact,
while sampled non-quarter rotations differ at 8–303 pixels by one channel
value. A source-dependent matrix combined with blur changes 23 circle pixels
at maximum delta 1; combined with native shadow it changes 15–16 pixels at
maximum delta 3. Three stable source-layer, transform, and spatial-composition
refusals guard these classes in both admissions. The patrol deliberately
over-refuses additional exact controls until a broader stable boundary is
known (measured, not celled).

Twenty-seven Chromium cells carry the admitted slice, all byte-exact. Eight
multi-panel cells cover enum, count, separator, invalid-list, saturation, hue,
luminance, and underflow grammar. The remaining cells cover identity and
non-identity matrices, RGB and alpha scaling, alpha creation, SourceAlpha,
negative and above-one saturation, ordinary and large hue angles,
luminance-to-alpha, both color spaces, generated and composite input, the safe
transform envelope, opacity, clip, and path geometry. Every committed source
also rendered through the product command. Gate sensitivity was proved by
temporarily adding `0.25` to the first matrix coefficient: `just gate` rejected
twenty-five of the twenty-seven cells, with as many as 2,304 differing pixels
and maximum channel delta 61. Restoring the coefficient returned all 502 cells
to green.

Three focused rows join the refusal register, moving it from 128 to 131. The
primitive corpus moves from 475 to 502 cells; the ten exact-time sampled frames
are unchanged. Only `<feColorMatrix>` ticks. This records no conformance score
and takes no FLIP action.

## Rung: `feComponentTransfer` and `feFunc*` (2026-08-25)

The verdict is CLOSE/SPLIT. `<feComponentTransfer>` and all four channel
function elements close for their complete static direct-child vocabulary.
The function-only `amplitude`, `exponent`, `intercept`, `slope`, and
`tableValues` attributes close with them. The shared `type`, `offset`, `in`,
`result`, primitive-region, `color-interpolation-filters`, `filter`, and
`<filter>` rows remain open for their wider applicability, cascade, resource,
host, or dynamics surface. Transfer-function animation remains outside this
static rung. No CSS property row closes.

The resolved graph carries four independent 256-byte lookup tables over
straight RGBA and exactly one input. No authored function element, type name,
number list, result name, document node, or backend object crosses that
boundary. The existing graph continues to resolve SourceGraphic, SourceAlpha,
the previous result, and an earlier named result, and the primitive region
continues to hard-crop the output. A table that creates nonzero alpha from zero
is explicitly visible to the graph's transparent-input accounting.

Chromium 149.0.7827.55 establishes the source vocabulary. Only direct
`feFuncR`, `feFuncG`, `feFuncB`, and `feFuncA` children participate; a nested
function is inert, and the last direct function for a repeated channel wins.
A missing channel is identity. The case-sensitive types are `identity`,
`table`, `discrete`, `linear`, and `gamma`; missing and invalid `type` use
identity. Linear defaults to slope one and intercept zero. Gamma defaults to
amplitude one, exponent one, and offset zero. Invalid scalar text uses that
parameter's initial value. Empty table/discrete lists are identity, a singleton
is active, and longer lists use the complete ordered SVG number-list grammar.

The numeric audit carries signs, leading dots, exponents, SVG whitespace,
comma-only and mixed separators, one trailing comma, adjacent signed numbers,
and a second dot beginning another number. Malformed separators, CSS tokens,
parsed-number overflow, and non-ASCII whitespace invalidate the complete list.
Values beyond the unit interval remain active before final clamping. Negative
gamma exponents are likewise active, including their non-finite operation
result at source zero.

Blink's source normalization order is observable. The linear source
`slope="1.654435761" intercept=".18682"` selects the upper binary32 control;
parsing the same text through the lower adjacent float changes all 2,304 source
pixels by one code value. A `.9` table member separately exposes three lookup
bytes at that normalization boundary. Table, discrete, and gamma evaluation
then follow the measured double-precision route, while linear uses the measured
float products and sum. Results clamp to the byte range and truncate. All 256
input byte values were swept twice for every function kind (measured, not
celled).

The committed cells carry identity and every active type; defaults and invalid
fallbacks; list interpolation, indexing, clamping, and truncation; alpha
creation and removal; SourceAlpha, generated and named inputs; both filter
color spaces; regions and both primitive unit systems; direct shapes, paths,
stroke, `<use>`, transforms, and `viewBox`; and ordering with blur, offset,
color matrix, native shadow, composite, and merge. Keeping a geometric clip
and partial opacity on separate enclosing elements is exact.

Three silent raster classes were quarantined before close. A source-derived
transfer over a paint-server fill or stroke differs from Chromium: the sampled
linear-gradient identity case changes 848 pixels at maximum channel delta 2,
an active transfer changes 434 at delta 3, a radial source reaches delta 115,
and a gradient stroke reaches delta 6. Fractional target translation changes
two pixels at delta 7, and a 17-degree rotation changes 58 at delta 12; the
admitted mapping envelope retains integer translation, axis maps, reflection,
and exact quarter turns. Generated-only inputs do not inherit either
source-dependent patrol (measured, not celled).

The third class is operation-independent. When one enclosing element owns both
a geometric clip and partial opacity, identity component transfer, identity
color matrix, zero blur, zero offset, one-input merge, and a transparent
zero-shadow each reproduce the same 1,154-pixel restore difference at maximum
delta 2; an active transfer reaches 1,162 pixels at delta 3. The unfiltered
control and the split-scope clip/opacity control are exact. One generic
effect-stack refusal therefore guards every active filter rather than
mislabeling this as component-transfer grammar (measured, not celled).

A separate direct-circle edge probe also differed without any filter. The
matching unfiltered controls place it in the already tracked fill-only
ellipse/box-world boundary, so this rung neither changes nor hides that issue
(measured, not celled).

Thirty-two Chromium cells carry the admitted slice, all byte-exact. Every
scratch and committed source rendered through both actual command admissions,
and strict and best-effort output agree for every admitted cell. Gate
sensitivity was proved by temporarily swapping the red and blue painter
tables: `just gate` rejected twenty-eight component-transfer cells, with up to
all 4,096 pixels differing and maximum channel delta 255. Restoring the channel
order returned all 534 cells to green.

Three focused rows join the refusal register, moving it from 131 to 134. The
primitive corpus moves from 502 to 534 cells; the ten exact-time sampled frames
are unchanged. Exactly ten checklist rows tick: the five elements and five
function-parameter attributes named above. This records no conformance score
and takes no FLIP action.

## Rung: `feBlend` (2026-08-26)

The verdict is CLOSE/SPLIT. `<feBlend>` closes for its complete static
Compositing Level 1 behavior, and the element-specific `mode` attribute closes
with it. The shared `in`, `in2`, `result`, primitive-region,
`color-interpolation-filters`, `filter`, and `<filter>` rows remain open for
their wider primitive applicability, cascade, resource, host, or dynamics
surface. CSS `mix-blend-mode`, filter-function syntax, animation, and later
draft additions are separate surfaces. No CSS property row closes.

The resolved graph carries one checked two-input blend operation and one
source-neutral sixteen-member mode. Its first input is the foreground and its
second is the backdrop. Authored element names, mode text, input names, result
names, document nodes, and backend objects are all resolved before that
boundary. The filter blend vocabulary remains distinct from paint-stack or
layer blending even where their mode names coincide.

Chromium 149.0.7827.55 establishes the source grammar. The complete
case-sensitive set is `normal`, `multiply`, `screen`, `overlay`, `darken`,
`lighten`, `color-dodge`, `color-burn`, `hard-light`, `soft-light`,
`difference`, `exclusion`, `hue`, `saturation`, `color`, and `luminosity`.
Missing, empty, invalid, wrong-case, whitespace-padded, legacy camelCase,
`plus-lighter`, and CSS-wide spellings all select the initial `normal`.
Sampled empty and valued `no-composite` spellings leave a valid `multiply`
unchanged. The last two findings are Chromium drops carried by committed
controls under the valid-drop precedent; neither draft spelling expands the
resolved vocabulary.

The arithmetic audit uses asymmetric opaque and translucent mode atlases. All
fifteen non-normal modes differ from normal on the translucent generated-input
control across all 4,096 pixels, with maximum channel deltas from 11 through
73. Exact center values distinguish every mode, including non-separable hue,
saturation, color, and luminosity behavior.

Local ARM initially made the native backend route look byte-exact, but the
first hosted x86 gate contradicted it in eight blend cells: four input/default
controls, both mode atlases, the grammar control, and the region crop differed
by one to three code values. The pinned backend explains the CPU-family split:
its N32 low-precision blend path performs exact divide-by-255 rounding on NEON
but intentionally approximates the same step as `(value + 255) / 256` on x86.
Nine modes use that path: `normal`, `multiply`, `screen`, `overlay`, `darken`,
`lighten`, `hard-light`, `difference`, and `exclusion`. They now run over
explicit byte-domain operands with exact divide-by-255 rounding. The seven
modes on the backend's high-precision path remain native.

That arithmetic repair removed seven fixture failures and every opaque
mismatch on the second hosted x86 run. Only the translucent atlas remained:
2,816 pixels at delta 1 across eleven mode tiles, with first engine/oracle
bytes `[109,160,200,255]` and `[109,160,199,255]`. The mode-independent shape
located a second CPU-family split at the final sRGB layer restore, not in
`color-dodge`, `color-burn`, `soft-light`, or the non-separable formulas. A
blend-scoped exact restore closes that boundary; a later color-space conversion
clears the policy before its own floating arithmetic. The complete 572-cell
gate is byte-exact on ARM and hosted x86. No tolerance was introduced.

Graph routing remains common to the checked filter program. `in` is the
foreground and `in2` the backdrop: swapping an overlay changes all 4,096
pixels at maximum delta 38. A missing, empty, or unknown input name selects
the previous result, or `SourceGraphic` when the blend is first. The last
duplicate `result` name wins and a blend result can feed later operations.
`SourceAlpha`, generated flood inputs, hard primitive crops, primitive-unit
mapping, and explicit result reuse all have exact committed evidence.

Missing `color-interpolation-filters` equals explicit linearRGB. Explicit sRGB
changes the sampled screen blend across all 4,096 pixels at maximum delta 13,
and a primitive-level value overrides the filter ancestor. Exact cells also
cover direct solid source, path, stroke, gradient, group, `<use>`, non-uniform
`viewBox`, fractional axis translation, fractional axis scale, exact quarter
turn, target opacity, target mask, and blend ordering before and after blur,
offset, component transfer, generated composite, and generated merge.

The precision audit found three silent classes and registered each before the
rows closed. Axis maps, fractional translations and scales, and exact quarter
turns are exact. A sampled 17-degree source-derived blend differs by 358
pixels at maximum delta 212; generated-only controls differ by 104–108 pixels
at maximum delta 179/153. The mapping patrol therefore belongs to the filtered
blend output, not source rasterization, and conservatively admits only axis
maps and exact quarter turns (measured, not separately celled).

A circular geometric clip changes 92 generated-only pixels and 100
source-derived pixels, both at maximum delta 85. Moving the same clip to an
ancestor is identical in Chromium. Axis-aligned rectangular clip controls are
exact, but the current clip patrol deliberately over-refuses the whole
geometric-clip class until a stable narrower boundary is known (measured, not
celled). The pre-existing same-scope clip-plus-partial-opacity patrol remains
operation-independent and is not relabeled as blend behavior.

The third class is independently generic. A translucent `SourceGraphic`
entering a later two-input composite or multi-input merge can differ even with
no blend at all: the no-blend `atop` control changes 2,147 pixels at maximum
delta 3. Blend→`atop` changes 1,925 at delta 3 and blend→merge changes nine at
delta 1, while generated-only controls are exact. One translucent-source
multi-input refusal therefore guards the underlying composition boundary for
every filter graph rather than installing a blend-shaped exception (measured,
not celled).

Thirty-eight Chromium cells carry the admitted slice, all byte-exact. They
cover the complete mode vocabulary and fallback grammar, both operand orders,
input defaults and unknown-name fallback, SourceAlpha, result shadowing and
reuse, both color spaces and local override, region union and crop, both
primitive unit systems, the source and composition profiles above, and safe
neighboring operations. Every scratch and committed candidate rendered
through both actual command admissions. Gate sensitivity was proved by
temporarily mapping `multiply` to `normal`: `just gate` rejected four named
cells, with 256–512 differing pixels and maximum channel delta 202. Restoring
the mode returned all 572 cells to green. The architecture repair has its own
control: replacing exact divide-by-255 rounding with the measured x86
approximation made twelve blend cells fail, with up to 4,096 differing pixels
and maximum channel delta 3. Restoring exact arithmetic returned the complete
gate to green. The second and third hosted x86 runs independently make the
scoped restore load-bearing: before it, the translucent atlas alone differed;
after it, the full workspace and all 572 oracle cells pass.

Three focused rows join the refusal register, moving it from 134 to 137. The
primitive corpus moves from 534 to 572 cells; the ten exact-time sampled frames
are unchanged. Exactly two checklist rows tick: `<feBlend>` and `mode`. This
records no conformance score and takes no FLIP action.

## Rung: `feMorphology` (2026-08-26)

The verdict is CLOSE/SPLIT. `<feMorphology>` closes for its complete static
Chromium behavior. The shared `operator`, `radius`, `in`, `result`, primitive
region, color-interpolation, filter-resource, and dynamics rows remain open for
their wider primitive applicability. CSS filter functions and animation are
separate surfaces. No CSS property row closes.

The resolved graph carries one checked one-input operation: erosion or
dilation, with two finite non-negative local-space radii. Authored text,
parser state, input and result names, document nodes, and backend objects are
resolved before that boundary. Erosion and dilation stay filter-image
operations; they are not shape inset/outset vocabulary.

Chromium 149.0.7827.55 establishes the source grammar. `operator` is the
case-sensitive `erode | dilate` enumeration with initial `erode`; missing,
empty, invalid, wrong-case, whitespace-padded, CSS-wide, and comment-bearing
spellings select that initial. `radius` is SVG
`<number-optional-number>` with initial zero. One number supplies both axes;
two numbers stay independent. Leading plus, exponent, comma-wsp, one trailing
comma, an adjacent sign, and a second dot that starts a second number are
accepted. Missing, empty, malformed, unit-bearing, percentage, CSS-math,
custom-property, CSS-wide, overflowing, non-ASCII-whitespace, extra-member,
and trailing-dot forms select zero. Numeric underflow also reaches zero.

Pinned Chromium clamps negative members independently before painting. A
negative or zero horizontal member therefore leaves a positive vertical
member active, and conversely; only two effective zero axes make the operation
an identity. This differs from the older whole-operation wording but is the
twice-measured browser behavior. A zero operation still applies its primitive
subregion as a hard output crop.

Mapped positive radii round at half-pixel boundaries after the target mapping,
independently on each axis. Exact probes pin `.49 → 0`, `.5 → 1`, `1.49 → 1`,
`1.5 → 2`, `2.49 → 2`, and `2.5 → 3`, including non-uniform `viewBox` and
object-box mappings. A 600×32 scratch bank places the device-radius ceiling at
256: `255.49` remains 255, `255.5` becomes 256, and `256.5`, `257`, and `1000`
all equal 256 (measured, not celled). A finite `3e38` source remains active,
while positive overflow is invalid and selects zero.

The source-number normalization order is observable. Under an amplifying
mapping, the valid source
`1.000000059604644775390625000000000000000000000001` equals the upper
binary32 neighbor and differs from the lower control by 32 pixels at maximum
delta 162. The committed alias cell carries that branch exactly; a raw lexical
parse would select the wrong radius.

Morphology takes channel-wise extrema over premultiplied filter pixels. Exact
channel cells distinguish sRGB `[171,0,255,191]`, linearRGB
`[214,0,255,191]`, and SourceAlpha `[0,0,0,191]` at the measured overlap.
Graph evidence covers first and prior inputs, SourceAlpha, generated floods,
duplicate-result shadowing, hard regions, both primitive unit systems, paths,
strokes, rounded rectangles, `<use>`, fractional axis maps, exact quarter
turns, target opacity, clips, masks, and ordering around blur, component
transfer, color matrix, blend, merge, and native shadow.

The region probe exposed one filter-chassis fault before close. The source
image was being clipped to the filter's output region before a spatial kernel
could read it. A filter region is an output crop; source pixels outside it may
still contribute to an output pixel inside it. Removing that premature source
clip corrected 16 pixels at maximum delta 136 in the discriminating
morphology crop and preserved the complete earlier filter corpus. The hard
crop remains carried by every graph result, where it belongs.

Three measured raster boundaries are quarantined. First, axis maps and exact
quarter turns are exact, while a sampled 17-degree source mapping differs by
171 pixels at maximum delta 12 and a generated-only mapping differs by 142 at
delta 9; shears reproduce the class. Second, paint-server source images differ
even through a zero morphology: sampled linear gradients differ by 125 pixels
at delta 1 at zero and 191 at delta 1 when active; radial and gradient-stroke
controls reproduce it. Third, active filled native circles and ellipses expose
the retained fill-coverage boundary: the circle differs by three pixels at
delta 9 and a fractional ellipse by eight at delta 6. Rounded rectangles,
curved paths, circle strokes, and round path strokes are exact. The third
patrol deliberately leaves the older fill-only ellipse work to
[gridaco/nothing#88](https://github.com/gridaco/nothing/issues/88); this rung
does not alter that issue or its evidence. These are measured, not separately
celled precision classes.

The broad scratch replay contains 187 twice-deterministic Chromium sources.
After the patrols, all 162 admitted sources are pixel-exact and strict and
best-effort agree; the other 25 reach one of the three new stable names or an
older operation-specific patrol. Five compact grammar/source candidates were
then captured twice and are exact in both admissions. Thirty-seven committed
Chromium cells carry the admitted slice without a new tolerance. Gate
sensitivity was proved by temporarily swapping erosion and dilation:
`just gate` rejected 35 of the 37 cells, with up to 1,560 differing pixels and
a maximum channel delta of 255. Restoring the operation returned all 609 cells
to green.

The first full-workspace hosted-x86 run then contradicted the ARM-local result
in nine active sRGB morphology cells: `axis-fractional`, `blur-after`,
`blur-before`, `matrix-before`, `path`, `quarter-turn`, `rounded-rect`,
`source-use`, and `stroke`. They differed in 1,633 pixels altogether, every
one at maximum channel delta 1. The common boundary was not morphology's
channel-extrema operation: it was the final low-precision sRGB filter-layer
restore, where the pinned backend performs exact divide-by-255 rounding on
NEON and its approximate division on x86. Active sRGB morphology now requests
the architecture-neutral byte-domain restore already established for that
boundary. A zero-radius operation retains its earlier pass-through policy, and
a later color-space conversion clears the restore policy before floating
arithmetic. The same hosted workspace test now passes the complete 609-cell
gate, as does ARM; no tolerance was introduced. The separate hosted n0 rig is
not treated as Web-first corpus evidence because it does not enumerate these
cells.

Three focused rows join the refusal register, moving it from 137 to 140. The
primitive corpus moves from 572 to 609 cells; the ten exact-time sampled frames
are unchanged. Exactly one checklist row ticks: `<feMorphology>`. This records
no conformance score and takes no FLIP action.

## Rung: `feTurbulence` + `feDisplacementMap` (2026-08-26)

The verdict is CLOSE/SPLIT. `<feTurbulence>` and `<feDisplacementMap>` close
for their complete static Chromium behavior. Their seven element-specific
attributes close with them: `baseFrequency`, `numOctaves`, `seed`,
`stitchTiles`, `scale`, `xChannelSelector`, and `yChannelSelector`. The shared
`type`, `in`, `in2`, `result`, primitive-region,
`color-interpolation-filters`, filter-resource, and dynamics rows remain open
for their wider applicability. CSS filter functions, animation, and external
resources are separate surfaces. No CSS property row closes.

Chromium 149.0.7827.55 establishes the turbulence grammar. `type` is the
case-sensitive `turbulence | fractalNoise` enumeration with initial
`turbulence`; missing, empty, invalid, wrong-case, whitespace-padded, and
CSS-wide spellings select that initial. `baseFrequency` is one or two SVG
numbers with initial zero. One member supplies both axes. Leading plus,
exponent, comma-wsp, and one trailing comma are accepted. Missing, empty,
malformed, unit-bearing, percentage, CSS-function, comment-bearing,
extra-member, and overflowing spellings select the initial pair. If either
parsed member is negative, Chromium resets both axes to that pair rather than
clamping one independently.

`numOctaves` is an integer with initial one. Leading plus and surrounding SVG
whitespace are accepted; decimal, exponent, trailing-comma, and integer-
overflow spellings select one. Positive values cap at nine in the rendered
formula. Zero remains a real formula input: zero-octave turbulence is
transparent, while zero-octave fractal noise is the neutral-half field. A
negative integer produces a transparent result. `seed` is a signed SVG number
with initial zero. Fractional values are accepted and the procedural formula
truncates them toward zero; negative seeds remain active. `stitchTiles` is the
case-sensitive `stitch | noStitch` enumeration with initial `noStitch` and the
same exact-text fallback discipline as `type`.

The resolved procedural source carries only its formula, two finite
non-negative frequencies, capped octave count, finite seed, stitch decision,
hard region, and operating color space. It has no image input. The
`baseFrequency` pair is not scaled by `primitiveUnits`; object-box units still
govern authored primitive regions. For a stitched source, Chromium truncates
the finite primitive-region width and height to integer tile lengths before
constructing the repeating field. Fractional stitched-region controls and the
corresponding non-stitched field differ and are both exact.

Chromium establishes a distinct two-input displacement operation. `scale` is
a signed SVG number with initial zero and the same one-member grammar as
`seed`. Each selector is the case-sensitive `R | G | B | A` enumeration with
initial `A`; padded, wrong-case, invalid, and CSS-wide spellings select alpha.
The operation reads non-premultiplied channels from its second image and moves
the first image by `(channel - 0.5) × scale` on each axis. A transparent map
therefore selects the negative half-scale vector rather than zero movement;
the half-alpha control lies near the zero vector at the byte-normalized alpha.
Both source images enter the primitive's declared filter color space before
sampling. This is observable even with alpha selectors because the color input
conversion also participates in the result.

Object-box displacement follows Blink's one-scalar native route: the authored
scale is multiplied by the target width for both x and y displacement, not by
independent axis bases. A non-square target discriminates that rule from both
the height basis and separate-axis scaling. Hard filter and primitive regions,
percentage regions, bounded displacement maps, SourceGraphic, SourceAlpha,
source-as-map, generated color/map inputs, named/previous input fallback,
procedural maps, and result reuse all have exact evidence.

The source audit exposed a filter-chassis error rather than a procedural-noise
error. A user-space filter can have a positive region even when the target
contributes no source pixels; a generated primitive must still paint that
region. The prior route discarded every zero-area target before filter-unit
resolution, and the resolved item stream rejected the resulting empty scope.
The corrected contract distinguishes a fully transparent source invocation
from an absent filter: source references become an explicit bounded
transparent image, and a source-generating graph makes the otherwise empty
filter scope meaningful. The same empty target under object-box filter units
has no positive filter region and correctly paints nothing. User-space filter
units with object-box primitive units still paint the unscaled turbulence
field. Empty-source turbulence, turbulence blended with SourceGraphic, and a
generated displacement graph are committed exact controls. That generated-only
scope also owns the transformed filter-region coverage used for frame damage;
changing its seed damages that region even though no source draw contributed a
box.

The color audit found a second narrow chassis boundary. Missing
`color-interpolation-filters` equals explicit linearRGB; explicit sRGB changes
both noise formulas and a procedural displacement map. Procedural pixels stay
floating through blend arithmetic and color conversion; treating them as an
ordinary generated byte image changes 349 to 490 pixels at maximum deltas 1
to 6 in three measured blend cases. One narrower distinction remains inside
that statement. A direct sRGB procedural input reaches the pinned backend's
byte-domain product rounding for `difference` and `exclusion`, while a linear
input or an intervening component transfer promotes the blend to floating
arithmetic. An sRGB blend computes in its selected domain and then materializes
before a later blend; linear blend output remains floating, and a later transfer
promotes either route again. Four sixteen-mode atlases covering both color
spaces and direct or transferred alpha discriminate the first split. A
thirty-four-source chain matrix covers offset, blur, matrix, transfer,
morphology, prior blend, displacement, merge, and composite placement around
the affected modes: thirty admitted sources are exact and four reach the
existing source-derived multi-input patrol (measured, not celled). Eight
committed controls retain the two affected modes and the two-blend transition.
Active sRGB morphology directly over a procedural image
establishes a byte-domain boundary, while blend followed by morphology retains
the stronger procedural-composition provenance. The opposite operation order
is an exact control. The final procedural layer quantizes only the composed
result with explicit half-up byte rounding before the N32 boundary; sRGB
displacement uses the established exact byte restore. Generated solids and
earlier byte-domain filter outputs retain their existing policies. Both
turbulence formulas in both color spaces, both operation orders, the full
blend-mode atlases, and the noise-to-displacement chain are exact on the current
ARM host without a tolerance.

Three remaining silent classes are now stable refusals. Axis maps, fractional
translation and scale, reflection, and exact quarter turns are exact. A
sampled 17-degree turbulence mapping differs by 3,173 pixels at maximum
channel delta 7 and a shear by 3,110 at delta 6. The corresponding displacement
controls differ by 280 pixels at delta 13 and 360 at delta 18. Separate mapping
patrols therefore guard each operation before paint. A geometric clip around
displacement differs by 35 pixels at maximum delta 2; the opacity control
without that clip is exact. One displacement-specific clip patrol quarantines
that class. Existing small-blur and translucent-source multi-input patrols
continue to own their independent graph boundaries (measured, not celled).

The numeric crux did not reproduce the earlier stroke/geometry provenance
alias in a visible procedural field. Seed values bracketing sampled binary32
midpoints and adjacent tiny base frequencies produced deterministic equal
pixels at 64×64, so no second numeric refusal was invented from an
undiscriminating raster. The admitted route nevertheless uses the shared
ordered SVG-number evaluator rather than Rust's raw lexical `f32` parser. This
is a negative probe verdict, not a claim that every possible numeric alias is
impossible (measured, not celled).

Two broad matrices contain 164 twice-deterministic Chromium sources, each also
rendered through both actual command admissions. All 156 admitted sources are
pixel-exact and strict and best-effort agree; eight reach one of the three new
stable names or an older operation-specific patrol. A review-triggered matrix
adds eighteen procedural-operation sources across blend, morphology, matrix,
transfer, displacement, and both color-space directions. Fifteen are admitted
and exact after repair; three reach older shadow or composition patrols. The
former restore policy changes five measured cases: the three blend cases above,
sRGB-to-linear morphology by 2,365 pixels at maximum delta 1, and blend followed
by morphology by 393 pixels at delta 1. The four blend atlases add sixty-four
exact procedural mode/space/alpha combinations, and the operation-chain matrix
adds thirty exact admitted sources (measured, not celled). Ninety-one committed
Chromium cells carry the admitted slice without a new
tolerance. They cover
the complete parameter grammars, both formulas, all selectors, color spaces,
regions and units, source and map profiles, empty targets, graph routing,
safe mappings, source shapes, opacity, `<use>`, `viewBox`, and the combined
procedural-warp graph.

Gate sensitivity was proved by temporarily mapping turbulence to fractal noise
and the red displacement selector to alpha. `just gate` rejected fifty-five
new cells: turbulence controls changed up to all 4,096 pixels and displacement
controls reached maximum channel delta 202. Restoring both mappings returned
the gate to green. Independently restoring the former procedural-image policy
makes the five dedicated operation-order cells fail, up to 2,365 pixels and
maximum delta 6. Restoring the measured provenance rule returns the complete
700-cell gate to green. Replacing composed-result half-up quantization with
floor made forty-one cells fail, up to 3,648 pixels, all at delta 1; replacing
procedural multiply with normal made five cells fail, up to 1,362 pixels at
delta 116. Finally, forcing the promoted floating route on direct sRGB
`difference`/`exclusion` makes the two dedicated cells fail by 3,497 and 3,532
pixels, while forcing byte-domain products on the promoted routes makes four
cells fail by 3,287–3,697 pixels. Carrying a floating result across the measured
sRGB blend-output boundary makes the two chain controls fail by 3,468 and 3,700
pixels. Every restoration returns the complete gate to green.

The first hosted-x86 workspace run rejected twenty-two rung cells: eighteen
sRGB displacement cases and four procedural outputs, totalling 294 pixels, all
at delta 1. Scoped exact displacement restore cleared all eighteen displacement
failures on the second run. That run left four procedural cells and 729 pixels,
all at delta 1: one pixel each in the default-color, linear-color, and stitched
controls, plus 726 pixels in the procedural blend. Explicit floating mode
arithmetic, the direct-sRGB product exception above, and composed-result-only
half-up quantization cleared the blend control on the third hosted run. Three
singleton delta-1 pixels remained: default-color and linear-color produced
`[203, 190, 203, 255]` where Chromium produced `[203, 189, 203, 255]`, and the
stitched control produced `[173, 159, 171, 255]` where Chromium produced
`[174, 159, 171, 255]`.

Pinned Skia source located that last direct-noise split in process startup.
The painter had never initialized Skia's runtime-selected raster pipeline, so
hosted x86 retained the baseline non-fused Perlin path while ARM used its fused
NEON path. `skia_safe::graphics::init()` now runs before any drawlist replay;
on hosted x86 it selects the AVX2 pipeline whose fused multiply-add ordering
matches the admitted procedural values. The fourth hosted-x86 gate and the ARM
gate are byte-exact across all 700 cells. No tolerance or pixel exception was
introduced.

Three focused rows join the refusal register, moving it from 140 to 143. The
primitive corpus moves from 609 to 700 cells; the ten exact-time sampled frames
are unchanged. Exactly nine checklist rows tick: the two elements and seven
element-specific attributes named above. This records no conformance score and
takes no FLIP action.

## Rung: `feConvolveMatrix` (2026-08-27)

The verdict is CLOSE/SPLIT. `<feConvolveMatrix>` closes for its complete static
Chromium behavior. Eight attribute rows close with it: `bias`, `divisor`,
`edgeMode`, `kernelMatrix`, convolution `order`, `preserveAlpha`, `targetX`,
and `targetY`. `kernelUnitLength` remains open because it also applies to the
still-open lighting primitives. The shared `in`, `result`, primitive-region,
`color-interpolation-filters`, filter-resource, and dynamics rows remain open
for their wider applicability. No CSS property row closes.

Chromium 149.0.7827.55 establishes one rectangular convolution. `order` takes
one or two values, with one supplying both axes; Blink normalizes finite values
by truncating toward zero. Missing, empty, malformed, wrong-count,
unit-bearing, function-valued, and CSS-wide spellings retain the initial 3×3
order. A parsed non-positive axis produces a transparent operation result. The
wider invalid-spelling matrix is measured, not all separately celled.
`kernelMatrix` is an SVG number list whose length must equal the product of the
two order axes. Missing, malformed, non-finite, and wrong-count lists likewise
produce transparent. The authored matrix is reversed once so the operation is
convolution rather than correlation. One-hot left/right and asymmetric target
cells distinguish that direction from a backend correlation.

The pinned operation accepts at most 256 coefficients. Chromium executes the
measured strategy boundaries at 28, 29, 64, 65, and 256 coefficients, while a
257-coefficient kernel produces transparent. Those accepted boundaries and
the browser drop are committed exact evidence. This follows the browser-drop
precedent established by
[gridaco/nothing#77](https://github.com/gridaco/nothing/pull/77): a listed
value the oracle itself drops does not require this engine to invent pixels.

`divisor` carries one signed SVG number. Missing, an exactly empty attribute,
and either signed zero select the kernel's ordered binary32 sum; a zero sum
becomes one. A present, nonempty malformed value selects one rather than the
sum. Positive and negative nonzero values remain active. The ordered sum is
observable: cancellation in `1e20 1 -1e20` is not interchangeable with a
reassociated sum. A default sum that overflows produces Chromium's measured
transparent output. `bias` carries one signed SVG number with initial zero;
missing and malformed values select that initial, while large finite values
clamp only at output.

`targetX` and `targetY` use the signed SVG-integer grammar. When absent, each
defaults independently to the floor of half its order axis. An authored
fraction, exponent, malformed token, or integer-storage overflow selects zero.
A valid negative target or a value at or beyond its order axis produces
transparent. `edgeMode` is the case-sensitive `duplicate | wrap | none`
enumeration with initial `duplicate`; all three differ at an actual input-image
boundary. `preserveAlpha` is the case-sensitive `false | true` enumeration with
initial `false`. SourceAlpha and positive-bias controls prove that false
convolves alpha and may create coverage over a transparent source, while true
retains the input alpha.

The other listed `edgeMode` applicability is `feGaussianBlur`. Current Chromium
ignores every blur spelling; a dedicated committed cell now carries the drop
that the chassis rung had only measured. Current Chromium likewise ignores
every sampled `kernelUnitLength` spelling on convolution: positive one- and
two-axis values, zero, negative, malformed, units, percentages, functions,
custom properties, and CSS-wide values all leave a discriminating kernel
unchanged (measured, not all separately celled). The drop is celled, but the row
cannot close before its lighting applicability is earned.

The operation participates in the established filter graph and effect order.
Committed exact cells cover SourceGraphic, SourceAlpha, previous and named
results, generated input, result reuse, hard primitive crops, both primitive
unit systems, default linearRGB, explicit linearRGB and sRGB, Chromium's
`auto`-to-sRGB behavior, paths, strokes, groups, `<use>`, non-uniform `viewBox`,
fractional axis mapping, exact quarter turns, target opacity, clip, and mask,
and ordering beside blur and morphology. Stroke geometry enters the isolated
source before convolution; target effects remain outside the filter scope in
their established order.

The numeric probe found the source-parser crux rather than assuming it away.
An amplified decimal just above the midpoint between adjacent binary32 values
selects the upper neighbor in Chromium and is exact through the shared ordered
SVG-number evaluator. Kernel-size strategy boundaries, ordered divisor sums,
sum overflow, large bias, and accepted finite arithmetic all reproduce without
a tolerance. Unit, percentage, CSS-function, custom-property, and CSS-wide
spellings take their measured fallback or transparent-error branch; none
silently becomes a different valid kernel.

Three remaining silent classes are stable refusals. Fractional translation and
scale, reflections, axis maps, and exact quarter turns are exact, but a sampled
17-degree target rotation differs by 462 pixels at maximum channel delta 15
and a shear by 632 at delta 13. Arbitrarily small sampled rotations and shears
reproduce the class. A source-dependent linear-gradient fill differs by 1,425
pixels, a radial fill by 1,658, and a gradient stroke by 609, each at maximum
delta 7; generated-only input stays exact. Finally, a valid nonzero divisor of
`1e-45` has a reciprocal outside the finite resolved arithmetic domain.
Chromium executes it and the sampled pixels equal a nearby finite-gain control,
so the engine refuses that narrow arithmetic range rather than emit a
non-finite operation. These pixel verdicts are measured, not celled; three
focused refusal fixtures guard their stable names in strict and best-effort
admission.

Four twice-deterministic scratch matrices exercised grammar, error states,
operation semantics, precision boundaries, graph routing, composition, and
source classes. Every candidate also rendered through both actual command
admissions; every admitted result was compared by pixels, not merely by process
success. Three dense grammar atlases and thirty-seven focused convolution cells
carry the accepted surface. One further cell commits the blur `edgeMode` drop.
All forty-one are byte-exact without a new tolerance.

Gate sensitivity was proved by temporarily removing the required matrix
reversal. `just gate` rejected eleven convolution cells. The dedicated reversal
cell changed 230 pixels at maximum channel delta 202; the broad failures reached
2,814 pixels and maximum delta 250. Restoring the reversal returned the complete
741-cell gate to green.

Three focused rows join the refusal register, moving it from 143 to 146. The
primitive corpus moves from 700 to 741 cells; the ten exact-time sampled frames
are unchanged. Exactly nine checklist rows tick: the element and eight
attributes named above. This records no conformance score and takes no FLIP
action.

## Rung: `feDiffuseLighting` and the light-source family (2026-08-27)

The verdict is CLOSE/SPLIT. `<feDiffuseLighting>`, `<feDistantLight>`,
`<fePointLight>`, and `<feSpotLight>` close for their complete static
Chromium behavior. Eight element-specific attribute rows close with them:
`azimuth`, `diffuseConstant`, `elevation`, `limitingConeAngle`, `pointsAtX`,
`pointsAtY`, `pointsAtZ`, and `surfaceScale`. The shared `x`, `y`, `z`,
`specularExponent`, `kernelUnitLength`, `lighting-color`, `in`, `result`,
primitive-region, color-space, filter-resource, and dynamics rows remain open
for their wider applicability or independently named value/cascade remainder.
`<feSpecularLighting>` remains unsupported; no CSS property row closes.

Chromium 149.0.7827.55 establishes diffuse illumination from one input image's
alpha field. Missing `in` follows the established first/previous graph rule,
and SourceGraphic and SourceAlpha are identical because source RGB does not
enter the operation. The result is opaque throughout its primitive subregion,
including opaque black where the diffuse coefficient is zero; a primitive
without a recognized direct light child instead contributes transparent
black. Non-light children are skipped, nested lights do not participate, and
the first recognized direct `feDistantLight`, `fePointLight`, or `feSpotLight`
child wins.

`surfaceScale` and `diffuseConstant` carry one signed SVG number with initial
one. For this animated-number family an exactly empty attribute becomes zero,
while whitespace-only and malformed text use the initial; leading plus,
exponent, and the measured trailing-comma prefix are accepted.
`surfaceScale` remains signed. A negative diffuse constant clamps to zero.
The three light-source coordinates and the three spot targets carry the same
one-number grammar with initial zero. Distant azimuth and elevation are signed
and periodic. Spot exponent defaults to one and clamps to the inclusive
1–128 range; its shared attribute row stays open because the same name also
applies to specular lighting. A missing or zero cone angle, or one outside the
inclusive -90–90 range, selects the backend's 90-degree behavior. Within that
range a negative angle is pixel-identical to its positive magnitude. A spot
whose position equals its target retains Chromium's native degenerate result.

User-space point coordinates pass through directly. Under object-box primitive
units, x and y map against the target box's independent axes and z maps against
the normalized diagonal, `sqrt((width² + height²) / 2)`. Spot positions and
targets use that same three-dimensional map. Exact controls cover non-square
boxes, negative coordinates, non-uniform `viewBox` mapping, axis transforms,
reflection, exact quarter turns, `<use>`, stroke geometry, gradient alpha, and
target opacity, clip, and mask. A finite authored object-box coordinate whose
mapping overflows produces Chromium's transparent operation result rather than
an unfiltered fallback; that boundary is committed exact.

The direct `lighting-color` subset has initial white, is not inherited by
default, admits the established sRGB color forms and `currentColor`, treats an
invalid or reset spelling as the initial, and ignores authored color alpha
while retaining its RGB channels. Missing filter interpolation equals
linearRGB; explicit sRGB is visibly distinct. The operation adapts the light
channels into that selected space before illumination. Explicit inheritance,
CSS-authored `lighting-color`, custom-property substitution, and wider color
functions are all honored by Chromium and now refuse by four focused stable
names. Those independently listed cascade, custom-property, and color-family
gaps—plus the still-open specular applicability—keep the `lighting-color`
presentation-attribute row open. The CSS property twin is unavailable in the
pinned Servo-mode cascade, and no matcher is added around it.

Current Chromium ignores every sampled `kernelUnitLength` spelling on diffuse
lighting: positive one- and two-axis numbers, fractions, zero, negative,
malformed text, units, percentages, CSS math, custom properties, and CSS-wide
keywords all leave a discriminating surface unchanged. The same drop reproduced
on sampled specular-lighting controls. Two diffuse cells commit a valid and an
invalid drop, but `kernelUnitLength` remains open until the specular primitive
itself is admitted. This follows the browser-drop precedent established by
[gridaco/nothing#77](https://github.com/gridaco/nothing/pull/77).

The numeric crux was measured instead of inferred. Source decimals around the
binary32 midpoint, adjacent finite extrema, subnormals, underflow, parser
overflow fallback, and huge signed angles all reproduce Chromium through the
shared ordered SVG-number evaluator. At this 64×64 amplification, the midpoint
witness and both adjacent controls happen to be pixel-identical, so no new
numeric patrol was invented from a non-discriminating raster. This is a
negative probe verdict, not a proof that no larger raster can expose another
alias (measured, not celled).

Two silent backend classes remain and are now named. General target rotation
changed 551 pixels at maximum channel delta 15, a shear changed 543 at delta
12, and a sampled affine map changed 624 at delta 12. Translation, arbitrary
sampled axis maps, reflection, and exact quarter turns are exact. Separately,
using diffuse lighting as the foreground of `feComposite` `in` or `atop`
against a source-derived second input changed 69 pixels at maximum delta 180.
The other composite operators, arithmetic composition, blend, merge,
generated-background composition, lighting in the second input, and adjacent
one-input spatial operations are exact. Two focused refusal fixtures guard
those boundaries in strict and best-effort admission (measured, not celled).

Six twice-deterministic scratch matrices exercised 236 sources across grammar,
light selection, operation semantics, color, units, graph composition,
transforms, extremes, and `kernelUnitLength`. Every candidate rendered through
the actual command path. After the named patrols, 217 candidates are
pixel-exact and admission-identical; the remaining 19 reach one of the new or
already-established stable names. Seventy-one cells entered only through
`just add`; all are Chromium-baked and exact without a tolerance.

Gate sensitivity was proved by temporarily halving the resolved diffuse
coefficient in the painter. `just gate` rejected 61 of the new cells, with up
to all 4,096 pixels changed and maximum channel delta 128. Restoring the
coefficient returned the complete 812-cell gate to green.

Six focused rows join the refusal register, moving it from 146 to 152. The
primitive corpus moves from 741 to 812 cells; the ten exact-time sampled frames
are unchanged. Exactly twelve checklist rows tick: the four elements and eight
attributes named above. This records no conformance score and takes no FLIP
action.
