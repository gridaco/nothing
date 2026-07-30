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
from eight addenda:

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
  `<line>`, `<polygon>` and `<polyline>`, filled and stroked; `<g>` and the
  whole `transform` grammar; viewBox-only root sizing with the full
  `preserveAspectRatio` grammar; and one exact-time
  `<animate attributeName="x">` on a top-level `<rect>`.
  `crates/n0_cli/README.md` is the statement of record.
- **The corpus** is 85 Chromium-baked primitive cells plus 10 sampled frames.
  All byte-exact except six curved cells carrying a declared, geometrically
  confined tolerance; the departure is the weighted rational conic alone.
- **Not claimed:** no conformance score exists or may be computed — FLIP is
  unratified. `opacity` and `rx`/`ry` are the two constructs still missing from
  the scoreboard suite, so its coverage is 10 of 14.

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
the load-bearing one and `pathLength` is the over-refusal.

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
  the reason `pathLength` is patrolled today even though nothing reads it yet.
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
admitted probe. The points shapes inherit the path patrols — `pathLength`
and the marker properties stay refusals — and `points_contract.rs` is the
rung's law file.
