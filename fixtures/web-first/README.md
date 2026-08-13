# fixtures/web-first

> **Scannable status:** [STATUS.md](./STATUS.md) is the generated,
> freshness-gated view of both corpora — the baked cells and the refusal
> register in the compiler's own words
> (`crates/websem/tests/capability_status.rs` regenerates and gates it).

The Chromium-baked oracle corpus for the SVG engine of record — the path an
`.svg` or `.html` source takes through one document, one cascade
(`crates/csscascade`), one compiler (`crates/websem`), one resolved contract
(`crates/rframe`) and one kernel (`crates/n0`).

*Web-first* names the ratified amendment that path implements, not a phase of
work: [docs/wg/consolidation/web-first.md](../../docs/wg/consolidation/web-first.md)
defines it, and `fixtures/test-svg/` and `fixtures/test-html/` hold the **legacy**
renderer's corpora, which is the distinction this directory name carries.

Every cell here is a closed enumeration in `primitives.json` with a committed
Chromium oracle beside it, and the gate is byte equality: what the corpus admits
is exactly what the engine renders pixel-for-pixel.

| File | Role |
| --- | --- |
| `html-inline-svg-currentcolor-rect.html` | HTML whose `<style> .mark { color:#16a34a }` cascades to a `<rect class="mark">` inside inline `<svg>`. |
| `svg-currentcolor-rect.svg` | The equivalent standalone SVG (carries `color` via an inline `style`). Renders identically. |
| `svg-viewbox-uniform-offset-rect.svg` | A non-zero-origin `viewBox` with uniform 2× viewport mapping — the first supported non-identity viewport case. |
| `svg-viewbox-only-sizing-rect.svg` · `svg-sizing-auto-rect.svg` | The viewport rung's sizing cells: no root `width`/`height` — `auto` resolves to 100% of the initial viewport (the baked window / the host's `WxH`), with and without a `viewBox`. |
| `svg-viewbox-unequal-default.svg` · `svg-preserve-aspect-ratio-*.svg` | The viewport rung's `preserveAspectRatio` cells: the default `xMidYMid meet` letterbox, an explicit `none` (equal-aspect admission), non-uniform stretch, slice clipping, and an `xMaxYMid` alignment offset. |
| `svg-circle-fill.svg` · `svg-circle-viewbox-scaled.svg` · `svg-ellipse-fill.svg` | The basic-shapes rung's painting cells: a circle at rest, the same circle carried by a scaling `viewBox`, and an ellipse with distinct radii. |
| `svg-circle-defaults-clip.svg` | `cx`/`cy` default to 0, so three quarters of the circle fall outside the viewport — the frame clip, not a guessed position. |
| `svg-ellipse-auto-rx.svg` · `svg-ellipse-negative-rx-auto.svg` | The `auto` radius matrix: an absent `rx` adopts `ry`, and a *negative* `rx` is invalid-must-be-ignored, which Chromium resolves to that same `auto`. Both bake to the same circle. |
| `svg-circle-zero-r.svg` | `r="0"` disables rendering (SVG2 §10.3) — an admitted nothing, baked as proof rather than asserted. |
| `svg-group-transform-translate.svg` · `svg-group-nested-transforms.svg` · `svg-shape-transform-matrix.svg` | The container rung's composition cells: a group's translate, a translate nested inside a scale (outermost-first, so the inner offset scales), and a `matrix()` on a shape. |
| `svg-group-rotate-quarter.svg` · `svg-group-rotate-diagonal.svg` | Rotation about a pivot: an exact quarter turn, and a 45° turn whose edges land off the pixel grid. Both bake byte-exact. |
| `svg-group-paint-order.svg` · `svg-group-inherited-fill.svg` | Flattening keeps document paint order across and into containers, and `fill` inherits through a group by the one cascade. |
| `svg-non-rendering-elements.svg` | `<title>`/`<desc>` paint nothing and declare nothing — the raster is the same as if they were absent. (`<metadata>` shares the skip in `websem`'s law tests but is not yet in this cell.) |
| `svg-path-polygon-fill.svg` · `svg-path-unclosed-fill.svg` · `svg-path-relative-commands.svg` · `svg-path-hv-shorthand.svg` | The paths rung's straight-edge cells: **one triangle spelled four ways** — closed, unclosed (a fill closes it implicitly), relative with implicit repeats, and the `H`/`V` shorthands. All four oracles are byte-identical, which is the claim. |
| `svg-path-closed-move-only-contour.svg` | `M x y Z` is a zero-length *closed* contour, not nothing: dropping it moves 96 pixels of the surviving triangle in Chromium. Fractional coordinates on purpose — integer axis-aligned edges hide this. |
| `svg-path-arc.svg` · `svg-path-arc-flags.svg` · `svg-path-arc-rotated.svg` | The conic rung's arc cells: the half-ellipse sweep (the byte-identity measurement's own geometry, graduated from the refusal register), all four large-arc/sweep flag combinations selecting four distinct arcs, and a rotated elliptical sweep. Every one bakes byte-exact — the emitted conics are Chromium's own curve class. |
| `svg-path-arc-degenerate.svg` | The arc's measured correct nothings: a zero radius degenerates to the authored straight line, and coincident endpoints elide the segment entirely. |
| `svg-path-arc-stroked.svg` | An open arc under a stroke: the conic geometry feeds the same stroker as every other path, caps and all. Byte-exact. |
| `svg-rect-rounded.svg` · `svg-rect-rounded-elliptical.svg` | `<rect rx ry>` lowers to four quarter-turn conics of weight cos 45° — measured byte-identical to the equivalent `A`-command contour in Chromium itself, circular and elliptical corners alike. |
| `svg-rect-rounded-mirror-auto.svg` · `svg-rect-rounded-negative-rx-auto.svg` | The auto matrix: an absent `rx` adopts `ry`, and a *negative* `rx` is invalid-must-be-ignored, which Chromium resolves to that same `auto`. Both bake to the same rounding. |
| `svg-rect-rounded-clamp.svg` | The measured resolution order: `auto` mirrors the *authored* value first, then each axis clamps to half its own extent independently — `rx="30"` on a 40×48 rect rounds as (20, 24), not (20, 20). |
| `svg-rect-rounded-stroked.svg` | A stroked rounded rect: the corner conics under the stroker. Byte-exact. |
| `svg-stroke-rect-centred.svg` · `svg-stroke-over-fill.svg` | A Web stroke straddles its geometry — an 8-wide stroke on an edge at x=16 inks 12..19 — and paints *over* the fill, which is SVG's default paint order. |
| `svg-stroke-default-width.svg` · `svg-stroke-invalid-width.svg` · `svg-stroke-length-units.svg` · `svg-stroke-zero-width.svg` | The width is a cascaded length: absent is 1, a negative value is an invalid declaration that falls back to the same 1 (byte-identical cells), `0.5em` resolves like any CSS length, and `0` paints nothing. |
| `svg-stroke-inherited.svg` | `stroke` and `stroke-width` inherit through a `<g>` by the one cascade — the shape the tiger is built from. |
| `svg-stroke-circle.svg` · `svg-stroke-ellipse.svg` · `svg-stroke-path-open.svg` · `svg-stroke-path-closed.svg` · `svg-stroke-line.svg` | A stroke on every admitted geometry. The closed path's round join at its closing corner is the corpus's only stroke cell that is not byte-exact — see the tolerance note below. |
| `svg-stroke-line-fill-never-paints.svg` | A `<line>` with a fill and no stroke paints nothing: a line has no interior, and the two-command path it compiles to has zero area. |
| `svg-stroke-cap-butt.svg` · `svg-stroke-cap-round.svg` · `svg-stroke-cap-square.svg` · `svg-stroke-zero-length-dot.svg` | The caps on an **open** contour: butt stops at the endpoint, round and square extend by the radius, and a *zero-length* segment paints a cap-shaped dot (nothing at all under butt) — which is why the path normalization keeps one. |
| `svg-stroke-cap-closed-{butt,round,square}.svg` · `svg-stroke-cap-{circle,ellipse}-{round,square}.svg` | The same caps on a **closed** contour, where SVG makes them inert, at a one-device-pixel width. Chromium's three captures of each are byte-identical to one another; ours were not until the cap was normalized away per closed geometry. Seven cells because the defect was per painter arm, not per element: a path and an oval diverged, a rect never did. |
| `svg-stroke-cap-css-butt.svg` · `svg-stroke-cap-css-round.svg` · `svg-stroke-cap-css-square.svg` · `svg-stroke-cap-css-over-attr.svg` | The caps' CSS twin: every keyword in CSS spelling bakes byte-identical to its attribute cell, and an author `square` beats the `butt` presentation attribute. A garbage value in either spelling drops — the attribute to the initial butt, the declaration entirely so a valid attribute survives (both measured, neither celled). |
| `svg-stroke-join-miter.svg` · `svg-stroke-join-round.svg` · `svg-stroke-join-bevel.svg` · `svg-stroke-miter-limit.svg` | The joins, each with distinct ink at the same corner, plus a miter limit low enough to force the bevel. |
| `svg-stroke-join-miter-clip.svg` · `svg-stroke-join-arcs.svg` | The SVG2-only join values, measured unimplemented: Chromium parses `miter-clip` and `arcs` as invalid declarations, so both drop to the initial miter — byte-identical to the `miter` cell, the same fate as garbage input (measured). Stylo's three-keyword grammar drops them in the same place, so both admissions agree without a special case. The grammar bar for both twin rows is SVG2's property index; the fill-stroke-3 draft's `crop`/`fallback` keywords ship in no engine and sit outside the standard-track surface. |
| `svg-stroke-join-css-miter.svg` · `svg-stroke-join-css-round.svg` · `svg-stroke-join-css-bevel.svg` · `svg-stroke-join-css-over-attr.svg` | The CSS spelling of every implemented join keyword, plus the precedence cell: an author `stroke-linejoin: round` beats the `bevel` presentation attribute. |
| `svg-stroke-join-css-miter-clip.svg` · `svg-stroke-join-css-arcs.svg` | The declaration-level proof the drop happens at parse: CSS `miter-clip` or `arcs` over a `round` **attribute** paints round — the invalid declaration ceases to exist and the hint survives, where an implemented value would have changed the corner. |
| `svg-stroke-miter-limit-css.svg` · `svg-stroke-miter-limit-css-below-one.svg` | The miter limit's CSS twin at the same forced bevel, and a below-one limit — valid in SVG2 where SVG 1.1 forbade it — is not dropped: no miter can satisfy it, so it bevels identically. A negative limit instead drops as invalid and the initial 4 miters, and a CSS limit beats the attribute spelling (both measured, neither celled). |
| `svg-stroke-scaled-group.svg` · `svg-stroke-nonuniform-scale.svg` | The width is a length in local space, so a group's `scale(2)` doubles it and `scale(2,1)` makes the pen elliptical — the stroke *outline* is transformed, not the width. |
| `svg-stroke-zero-extent-rect.svg` | A zero-extent `<rect>` or `r="0"` `<circle>` renders nothing **including its stroke** (SVG2 §10.1) — baked as proof, since a naive stroke of a zero-extent box would draw a line. |
| `svg-polygon-fill.svg` · `svg-points-trailing-comma.svg` | The points rung's fill cells: a triangle authored with mixed comma/whitespace separators, and the same shape with a trailing separator — which Blink accepts in `points` (measured), unlike the `viewBox` grammar. |
| `svg-polygon-fill-rule-evenodd.svg` | A self-intersecting star under `fill-rule="evenodd"`: the hollow core is the cell's proof that the points shapes read the cascaded fill rule. |
| `svg-polygon-stroke-closed.svg` · `svg-polyline-stroke-open.svg` | The closure split, stroked: the same three points as a polygon paint the closing segment and its joins; as a polyline they end in caps with no closing edge. |
| `svg-polyline-fill-implicit-close.svg` | A filled polyline paints as if closed — identical ink to the same polygon's fill (measured), because filling an open contour closes it. |
| `svg-polygon-single-point-square-cap.svg` · `svg-polyline-single-point-square-cap.svg` | A single point splits by closure: the polygon is the zero-length **closed** contour whose square cap paints a dot, the polyline is a move-only open contour that paints nothing — the cap laws from the strokes rung, restated through the points grammar. |
| `svg-display-none-shape.svg` · `svg-display-none-group.svg` | `display: none` generates no box: the shape disappears (its sibling paints), and a container prunes its whole subtree — a `visibility: visible` descendant stays gone. |
| `svg-display-none-root.svg` | The entry split the oracle itself caught: a **standalone** document's outermost `<svg>` ignores `display: none` and paints normally, where an embedded root generates no box. Baked as proof after an embedded-context probe suggested otherwise. |
| `svg-visibility-hidden-shape.svg` · `svg-visibility-collapse-shape.svg` | `visibility: hidden` and `collapse` are identical for shapes: the element's own paint turns off, siblings render. |
| `svg-visibility-unhide.svg` | `visibility` inherits and a descendant whose computed value is `visible` un-hides itself while its sibling stays inherited-hidden — the cell that forces the per-element (not per-subtree) reading. |
| `svg-visibility-rule-beats-attribute.svg` | An author rule beats the presentation attribute: a stylesheet `visibility: visible` un-hides `visibility="hidden"` — the hint-precedence law, baked. |
| `svg-fill-opacity-overlap.svg` · `svg-fill-opacity-percentage.svg` | `fill-opacity` composites over another shape, in both spellings of the one <alpha-value> grammar — baked identically. |
| `svg-translucent-fill-rgba.svg` | A translucent sRGB colour composites exactly as the equivalent `fill-opacity` — alpha is alpha, whichever door it entered by. |
| `svg-fill-opacity-times-alpha.svg` | The multiplied cell: `rgba(…, 0.5)` under `fill-opacity="0.5"` — the colour's alpha and the paint opacity multiply in float and quantize **once**, and this cell pins the rounding against Chromium. |
| `svg-fill-opacity-inherited.svg` | `fill-opacity` inherits through a `<g>`, and two translucent siblings composite over each other. |
| `svg-stroke-opacity-over-fill.svg` | The compositing split: a translucent stroke paints over its own opaque fill — the inner half composites over the fill, the outer over the canvas. |
| `svg-stroke-opacity-join.svg` | A translucent stroke is one paint pass: the miter join's self-overlap does not double-blend. |
| `svg-element-opacity.svg` | The group-scope rung's graduated refusal fixture: element `opacity` on a lone unstroked rect **folds** into the fill's alpha — one float product with the colour's alpha and `fill-opacity`, quantized once, measured byte-identical to Chromium's own fold route. |
| `svg-opacity-fill-stroke.svg` | The fact that kept element opacity a refusal, baked: a stroked shape composites fill and stroke through **one isolated layer** — the stroke-over-fill overlap blends once at the group alpha, where per-paint folding double-blends (measured 57 code values apart). |
| `svg-opacity-group-overlap.svg` · `svg-opacity-group-nonhalf.svg` | Layer isolation: two overlapping opaque children under one group opacity — the overlap is the topmost child at the group alpha over the backdrop, at `0.5` and at the non-half `0.7`. |
| `svg-opacity-nested-groups.svg` · `svg-opacity-use-compound.svg` | Nesting never flattens: `g(.5) > g(.5)` quantizes **per layer** — one code value below the flat `0.25` fold across the entire fill — and `use(.5)` of a translucent target compounds identically. |
| `svg-opacity-times-fill-opacity.svg` | Element opacity joins the one float product: `opacity=".5"` × `fill-opacity=".5"` on a lone shape quantizes once — byte-identical to `fill-opacity=".25"`. |
| `svg-opacity-transform-below.svg` · `svg-opacity-transform-on-element.svg` | The fold's structural boundary: a transform strictly *below* the scope element forces the real layer, while transform and opacity on the *same* element still fold — the discriminating pair, one code value apart. |
| `svg-opacity-hidden-in-group.svg` | A hidden child paints nothing and does not break the fold: the group's one visible draw folds (Chromium's bytes are the fold's, not the layer's). |
| `svg-opacity-stroke-only-fold.svg` | A stroke-only child alone in a translucent group folds into the stroke paint, and its ink outside the geometry bounds still paints — a fold clamps nothing. |
| `svg-opacity-rotated-group.svg` | A 45°-rotated translucent group: straight-edge AA composited once through the layer, byte-exact. |
| `svg-opacity-translucent-overlap.svg` | Translucent rgba children overlapping inside a translucent layer — contents blend among themselves at their own alphas, then the composite restores once at the group alpha. |
| `svg-opacity-zero-sibling.svg` | `opacity="0"` composites nothing — an admitted nothing with the sibling painting, baked as proof. |
| `svg-opacity-gradient-in-group.svg` | A dithered gradient ramp inside a real layer (the fold over a lone gradient refuses by name — see `unsupported/`). Carries the corpus's third `ramp-quantization` tolerance: the layer restore halves every ramp value, and the two Skia builds round one code value apart at 336 of 2304 pixels (measured; a wrong gradient or a wrong layer alpha moves far more, by far more). |
| `svg-percent-rect-in-viewbox.svg` · `svg-percent-rect-root-units.svg` | Percentage geometry resolves against the viewport's user-unit extent: the `viewBox` when one maps the viewport, the root's own extent otherwise. |
| `svg-percent-circle-diagonal.svg` · `svg-percent-ellipse.svg` | The axis split on a non-square viewport: `cx`/`rx` against the width, `cy`/`ry` against the height, and a circle's `r` against the normalized diagonal `sqrt(w²+h²)/√2`. |
| `svg-percent-line.svg` | Percentage line endpoints, per axis. |
| `svg-percent-stroke-width.svg` | A percentage `stroke-width` against the normalized diagonal — `10%` of 64x64 paints 6.4 units, the value measured back when this was a refusal. |
| `svg-anchor-container.svg` | `<a>` is a container like `<g>`: its transform composes and its `href` paints nothing — one container semantics, baked. |
| `svg-css-transform-property.svg` · `svg-css-transform-group.svg` · `svg-css-transform-webkit.svg` | The transform rung's consumption cells: the CSS `transform` property on a shape (the graduated refusal fixture), on a container composing for every descendant, and under the `-webkit-` alias the pinned cascade implements. |
| `svg-css-transform-beats-attribute.svg` · `svg-css-transform-sheet-beats-attribute.svg` · `svg-css-transform-none-restores.svg` · `svg-css-transform-invalid-falls-back.svg` | The precedence cells (CSS Transforms L1 §7): the attribute is a presentation hint, so a style attribute or sheet rule beats it — `transform: none` included — while an *invalid* CSS declaration drops at parse and the attribute stands. |
| `svg-css-transform-compound.svg` · `svg-css-transform-rotate-quadrant.svg` | Composition about the measured origin: a translate-then-scale list composes left to right about the used `transform-origin 0 0` — the local user-space origin — and a `rotate(90deg)` in a negative-min `viewBox` pivots on user `(0,0)`, not the viewBox corner. |
| `svg-css-transform-percent.svg` | Percentage translation resolves against the viewport's user-unit extent — `translate(50%, 25%)` in a 64-unit viewBox moves exactly (+32, +16). |
| `svg-transform-runtogether.svg` · `svg-transform-no-separator.svg` | The measured attribute-grammar leniency no browser ever tightened (csswg-drafts#2623): `translate(10-10)` is (10, −10), and two functions need no separator at all. |
| `svg-transform-malformed-drops.svg` | A malformed attribute list — here a valid function followed by garbage — drops **whole**: the element renders untransformed, exactly as Chromium resolves it, with nothing declared because nothing degrades. |
| `svg-use.svg` · `svg-use-defs-rect.svg` · `svg-use-xy.svg` · `svg-use-transform-xy.svg` | The use/defs rung's resolution cells: the graduated refusal fixture, a defs-held rect referenced in place, `x`/`y` as the appended translate, and the measured composition order — the translate lands *inside* the use's own transform (`scale(2)` then `x=5` is 26 units, not 21). |
| `svg-use-xlink-href.svg` · `svg-use-href-beats-xlink.svg` · `svg-use-forward-ref.svg` · `svg-use-duplicate-id-first.svg` | The reference grammar: the legacy `xlink:href` spelling resolves, the plain `href` beats it when both are present, forward references resolve through the whole-document id table, and a duplicate id resolves to the first in tree order (DOM `getElementById`). |
| `svg-use-group.svg` · `svg-use-chain.svg` · `svg-use-rendered-twice.svg` | Structure: a group target instantiates its subtree, chained uses expand through with each hop's `x`/`y` composing, and a light-tree target paints in place *and* as an instance. |
| `svg-use-cycle-nothing.svg` · `svg-use-missing-nothing.svg` · `svg-use-ancestor-circle.svg` | The correct nothings, each baked: a mutual reference cycle renders nothing while the document renders, an unresolved reference renders nothing, and a reference to a shadow-including ancestor is an invalid circle whose content paints exactly once. |
| `svg-use-inherit-fill.svg` · `svg-use-own-fill-wins.svg` · `svg-use-context-differs.svg` · `svg-use-currentcolor.svg` | The styling model, measured: inheritance flows from the **use site** — a hint on the use colors a clone that authors no fill, the clone's own attribute beats it, a definition-site ancestor's paint does *not* carry (the instance inherits black, not the defs wrapper's blue), and `currentColor` resolves against the use site's `color` (the hint this rung admitted). |
| `svg-use-display-none-target.svg` · `svg-use-wh-inert.svg` | `display: none` clones onto the instance and prunes it, and `width`/`height` on a use are inert for every admitted target (they size only `<svg>`/`<symbol>` targets, which refuse). |
| `svg-gradient-linear.svg` · `svg-gradient-linear-userspace.svg` · `svg-gradient-linear-bbox-offset.svg` | The gradient rung's base cells: the default objectBoundingBox ramp, the byte-identical userSpaceOnUse equivalent (the canary for the box-inverse fold), and a bbox-relative ramp on offset geometry. |
| `svg-gradient-transform.svg` · `svg-gradient-css-transform.svg` | `gradientTransform` and an author `transform` declaration are one computed value — the attribute cell and the non-quarter CSS rotation cell (the discriminating measurement: the value applies about the raw origin of gradient space, both spellings). |
| `svg-gradient-spread-reflect.svg` · `svg-gradient-spread-repeat.svg` · `svg-gradient-hard-stop.svg` · `svg-gradient-stop-nonmonotonic.svg` | The ramp grammar: both non-pad spread methods with their measured seams, equal-offset hard stops rendering crisp, and non-monotonic offsets clamping to the running maximum (never sorted). |
| `svg-gradient-degenerate-pad.svg` · `svg-gradient-degenerate-repeat.svg` · `svg-gradient-radial-r0.svg` | The degenerate rules, resolved by the producer and Chromium-baked: coincident linear endpoints are the last stop under `pad` and the ramp's integral average under `repeat`; a zero radius is a solid of the last stop. |
| `svg-gradient-zero-stops-fallback.svg` · `svg-gradient-fallback.svg` · `svg-gradient-zero-bbox.svg` | The reference semantics: a valid but stopless gradient paints nothing and the authored fallback does **not** fire; a missing id is invalid and the fallback does; an objectBoundingBox gradient on zero-area geometry paints nothing. |
| `svg-gradient-interp-unpremul.svg` · `svg-gradient-fill-opacity.svg` · `svg-gradient-currentcolor.svg` | The color model: stops interpolate unpremultiplied sRGB, `fill-opacity` multiplies the whole ramp at the backend's 8-bit alpha step (measured: ×128/255, not ×0.5), and a `currentColor` stop resolves against the gradient's own ancestor chain — never the referencing element. |
| `svg-gradient-radial.svg` · `svg-gradient-radial-custom.svg` · `svg-gradient-radial-diagonal-percent.svg` | Concentric radials: the default, an off-center `cx`/`cy`/`r` (this corpus's one `ramp-quantization` tolerance — see below), and the userSpaceOnUse `r="50%"` cell that pins the §7.10 normalized-diagonal basis. |
| `svg-gradient-href-cross-type.svg` | A radial templated on a linear inherits its stops — the href chain crosses gradient types for everything but geometry. |
| `svg-gradient-stroke.svg` · `svg-gradient-path-bbox.svg` | The consumers: a gradient stroke's paint box is the geometry's own box (the stroke's inked reach pads beyond it), and a path's paint box anchors at its tight-bounds origin — the glyphless compile's once-deferred decision, taken and baked. |
| `svg-gradient-not-in-defs.svg` · `svg-gradient-use-clone-order.svg` · `svg-gradient-stylesheet-fill.svg` | The table: a gradient outside `<defs>` is non-rendering in place and referencable; a `<use>` clone of a gradient earlier in expanded order does not shadow the document's element; a stylesheet-authored `fill: url(#…)` resolves identically to the attribute spelling (the two same-document URL bases). |
| `svg-path-cubic-fill.svg` · `svg-path-smooth-cubic.svg` · `svg-path-quadratic.svg` | Curved path cells: a cubic, an `S` continuation, and a `Q`+`T` pair. All three bake **byte-exact** — see the note below. |
| `svg-path-fill-rule-nonzero.svg` · `svg-path-fill-rule-evenodd.svg` · `svg-path-fill-rule-inherited.svg` | One self-intersecting star under each fill rule (core filled vs hollow), and the rule inherited from a `<g>` through the one cascade. |
| `svg-path-two-subpaths.svg` · `svg-path-in-scaled-group.svg` | Two closed contours in one `d`, and a path carried by a group's `scale(2)`. |
| `svg-path-draws-nothing.svg` | An empty `d` and a move-only contour: both admitted, both paint nothing, baked as proof rather than asserted. |
| `html-webpage-mockup.html` | A webpage-*design* (header / hero / cards / footer) expressed as 27 inline-SVG rects; the brand purple cascades from the HTML `<style>` via `fill="currentColor"`. Guarded by `crates/websem/tests/webpage_mockup.rs`. Not a real HTML/CSS layout — the slice renders solid-fill shapes only. |
| `primitives.json` | Closed enumeration of every root HTML/SVG primitive, its grammar entry, dimensions, Chromium oracle, and (where its ideal raster is curved) its declared comparison tolerance. Adding an unlisted root input fails the test gate. |
| `chromium/*.png` | One committed Chromium oracle per primitive, capturing the SVG-local raster at deviceScaleFactor=1. |
| `oracle-bake.json` | Bake provenance (browser version + sha256 of the suite, sources, oracles, and bake script). |
| `bake_chromium.ts` | Verifies existing oracle pixels and creates missing oracles; it never overwrites a differing baseline. Run: `pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/bake_chromium.ts"`. |
| `pages/` | The target-only real-world page corpus. It is not a runnable reftest gate yet; see [`pages/README.md`](./pages/README.md). |
| `unsupported/` | Inputs that deliberately have no pixels yet and must fail explicitly instead of being approximated; see [`unsupported/README.md`](./unsupported/README.md). |
| `animation/` | The sampling corpus: animated documents with their static Base projections, baked at a paused Chromium timeline. It carries `svg-scene-cub` — a whole composition that exercises this rung ladder end to end, statically and at exact times; see [`animation/README.md`](./animation/README.md). |

Exact expectation: every primitive's full RGBA raster matches its Chromium
oracle with zero differing pixels. The gate also validates enumeration and
provenance and double-runs both raw raster and PNG encoding (see
`crates/websem/tests/reftest_oracle.rs`).

One class departs from that, and only by declaration: **rational conics**.
A filled ellipse reaches the same `SkCanvas::drawOval` entry point in both
engines, but through different builds of Skia, whose conic scan-converters
disagree on fractional coverage along the curve — measured identical
across every available construction (`draw_oval`, `draw_circle`,
`PathBuilder::add_oval`/`add_circle`, an oval `RRect`), so no choice of
call closes it.

Nothing else in the corpus departs, and the boundary has been narrowed twice
by measurement. Straight edges agree even when a transform puts them off the
pixel grid: the rotated cells above — including a 45° turn — bake byte-exact.
And *curves* as such are not the boundary either: the cubic, smooth-cubic and
quadratic path cells above bake byte-exact too. Only the weighted conic
diverges — and only through the `drawOval` entry point. The conic rung's
measurement narrowed the boundary a third time: an SVG elliptical arc *is*
this corpus now, emitted as explicit `ConicTo` segments, and all eleven
arc and rounded-rect cells — rotated elliptical sweep and strokes included
— bake **byte-exact** with no tolerance at all. The departure class is the
oval construction's, not the curve class's.

The curved fixtures carry a `tolerance` block naming the ideal boundary and
bounding the departure: at most N differing pixels, at most a D-per-channel
delta, every one of them within a pixel of that boundary.
The numbers are the measured values, not headroom. No single cell is worst on
both axes: the largest differing-pixel count is 6 (`svg-circle-viewbox-scaled`,
at delta 3) and the largest per-channel delta is 8 (`svg-ellipse-fill`, in 1
pixel), so the corpus admits at most 6 pixels and at most delta 8, never both at
once. Strokes land in the same class and mostly below it: 30 of the 31 stroke
cells are byte-exact, and the one that is not (`svg-stroke-path-closed`) differs
in 4 pixels at delta 3 along the round join
at its closing corner — an arc, declared with that arc as its boundary. A shape in the wrong place, at the
wrong size, or in the wrong color moves pixels off the boundary ring and
still fails loudly, and `svg-circle-defaults-clip` shows the bar is not
unreachable: it bakes byte-exact and declares no tolerance at all.

The gradient cells brought a second tolerance kind, `ramp-quantization`,
declared on three cells with their measured bounds — always one code value,
never confined to a boundary ring (a ramp has none; none is needed, since a
wrong gradient moves far more pixels by far more than one code value and
still fails loudly). `svg-gradient-radial-custom` differs in 1 pixel: an
off-center radial reaches the backend through the shared radial leaf's unit
circle and a similarity, and Chromium's Skia and the pinned one differ by
an ulp at one knife-edge. `svg-gradient-stop-nonmonotonic` differs in at
most 18 pixels *across this engine's own platforms*: byte-exact under the
macOS Skia build, one code value at 18 clamp-edge pixels under the Linux
build's SIMD path — the corpus's first measured cross-platform departure,
found the day the gate learned to sweep the whole suite before failing.
`svg-opacity-gradient-in-group` (the group-scope rung) differs in at most
336 pixels: the isolated layer's restore halves every dithered ramp value,
and the two Skia builds round one code value apart across the ramp — the
same physics, multiplied by the layer. Every other gradient cell — ramps,
seams, hard stops, the dither itself, the ramp *under* the fold — is
byte-exact.

Render a primitive through the `n0` product command — since
[the engine of record](../../docs/wg/consolidation/svg-engine-of-record.md) it routes through
the same `websem → rframe → n0` pipeline the oracle gate proves, so this is a
manual host check of the one engine, not a second renderer. Arbitrary SVG
outside the closed suite is not capability coverage; beyond-slice *subtree*
constructs render best-effort by default with each skip declared on stderr
(`--strict` refuses them by name), while document-level contracts — the
malformed-grammar and not-yet-consumed sizing class collected in
`unsupported/` — refuse in both admissions:

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/svg-currentcolor-rect.svg /tmp/out.png 64x64
```

## What the `currentColor` cells prove

`svg-currentcolor-rect.svg` and `html-inline-svg-currentcolor-rect.html` are the
two cells for one property: a value crossing the HTML→SVG boundary through a
single cascade. `currentColor` is the sharpest witness for it, because resolving
one requires the computed `color` of the SVG element to have inherited from the
HTML ancestor — through the same Stylo cascade, not a second matcher.

They are about the crossing, not about paint breadth: cascaded `fill` from a
presentation hint and from an SVG-namespace stylesheet has its own cells
elsewhere in this table.

## Tooling

The loop's per-rung commands live in this directory's `justfile` (`just -l`):
`bake` (create missing oracles, verify every existing one pixel-for-pixel),
`gate` (the engine reftest over every cell — byte-exact except the declared
tolerance rows), `status` (regenerate
[STATUS.md](./STATUS.md)), `add <id> <svg>` (register a fixture with a sorted
manifest entry, refusing overwrites), and `probe <script>` (run a scratch
probe).

The Chromium capture posture lives in **one module**,
[`chromium_capture.ts`](./chromium_capture.ts), imported by both the baker and
[`probe_harness.ts`](./probe_harness.ts) — so a probe measures under exactly
the conditions the cells bake under, and the posture cannot silently drift:
`oracle-bake.json` records the module's sha256 and the Rust gate refuses a
stale one. Probe *matrices* stay scratch and are never committed; a probe is a
question asked once, and what it proves lands as cells and README rows, not as
a shadow corpus. The pre-landing verification ritual is the saved
`verify-rung` workflow (`.agents/workflows/verify-rung.js`).
