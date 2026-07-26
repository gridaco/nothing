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
