# fixtures/web-first

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
| `svg-non-rendering-elements.svg` | `<title>`/`<desc>`/`<metadata>` paint nothing and declare nothing — the raster is the same as if they were absent. |
| `svg-path-polygon-fill.svg` · `svg-path-unclosed-fill.svg` · `svg-path-relative-commands.svg` · `svg-path-hv-shorthand.svg` | The paths rung's straight-edge cells: **one triangle spelled four ways** — closed, unclosed (a fill closes it implicitly), relative with implicit repeats, and the `H`/`V` shorthands. All four oracles are byte-identical, which is the claim. |
| `svg-path-closed-move-only-contour.svg` | `M x y Z` is a zero-length *closed* contour, not nothing: dropping it moves 96 pixels of the surviving triangle in Chromium. Fractional coordinates on purpose — integer axis-aligned edges hide this. |
| `svg-stroke-rect-centred.svg` · `svg-stroke-over-fill.svg` | A Web stroke straddles its geometry — an 8-wide stroke on an edge at x=16 inks 12..19 — and paints *over* the fill, which is SVG's default paint order. |
| `svg-stroke-default-width.svg` · `svg-stroke-invalid-width.svg` · `svg-stroke-length-units.svg` · `svg-stroke-zero-width.svg` | The width is a cascaded length: absent is 1, a negative value is an invalid declaration that falls back to the same 1 (byte-identical cells), `0.5em` resolves like any CSS length, and `0` paints nothing. |
| `svg-stroke-inherited.svg` | `stroke` and `stroke-width` inherit through a `<g>` by the one cascade — the shape the tiger is built from. |
| `svg-stroke-circle.svg` · `svg-stroke-ellipse.svg` · `svg-stroke-path-open.svg` · `svg-stroke-path-closed.svg` · `svg-stroke-line.svg` | A stroke on every admitted geometry. The closed path's round join at its closing corner is the corpus's only stroke cell that is not byte-exact — see the tolerance note below. |
| `svg-stroke-line-fill-never-paints.svg` | A `<line>` with a fill and no stroke paints nothing: a line has no interior, and the two-command path it compiles to has zero area. |
| `svg-stroke-cap-butt.svg` · `svg-stroke-cap-round.svg` · `svg-stroke-cap-square.svg` · `svg-stroke-zero-length-dot.svg` | The caps on an **open** contour: butt stops at the endpoint, round and square extend by the radius, and a *zero-length* segment paints a cap-shaped dot (nothing at all under butt) — which is why the path normalization keeps one. |
| `svg-stroke-cap-closed-{butt,round,square}.svg` · `svg-stroke-cap-{circle,ellipse}-{round,square}.svg` | The same caps on a **closed** contour, where SVG makes them inert, at a one-device-pixel width. Chromium's three captures of each are byte-identical to one another; ours were not until the cap was normalized away per closed geometry. Seven cells because the defect was per painter arm, not per element: a path and an oval diverged, a rect never did. |
| `svg-stroke-join-miter.svg` · `svg-stroke-join-round.svg` · `svg-stroke-join-bevel.svg` · `svg-stroke-miter-limit.svg` | The joins, each with distinct ink at the same corner, plus a miter limit low enough to force the bevel. |
| `svg-stroke-scaled-group.svg` · `svg-stroke-nonuniform-scale.svg` | The width is a length in local space, so a group's `scale(2)` doubles it and `scale(2,1)` makes the pen elliptical — the stroke *outline* is transformed, not the width. |
| `svg-stroke-zero-extent-rect.svg` | A zero-extent `<rect>` or `r="0"` `<circle>` renders nothing **including its stroke** (SVG2 §10.1) — baked as proof, since a naive stroke of a zero-extent box would draw a line. |
| `svg-polygon-fill.svg` · `svg-points-trailing-comma.svg` | The points rung's fill cells: a triangle authored with mixed comma/whitespace separators, and the same shape with a trailing separator — which Blink accepts in `points` (measured), unlike the `viewBox` grammar. |
| `svg-polygon-fill-rule-evenodd.svg` | A self-intersecting star under `fill-rule="evenodd"`: the hollow core is the cell's proof that the points shapes read the cascaded fill rule. |
| `svg-polygon-stroke-closed.svg` · `svg-polyline-stroke-open.svg` | The closure split, stroked: the same three points as a polygon paint the closing segment and its joins; as a polyline they end in caps with no closing edge. |
| `svg-polyline-fill-implicit-close.svg` | A filled polyline paints as if closed — identical ink to the same polygon's fill (measured), because filling an open contour closes it. |
| `svg-polygon-single-point-square-cap.svg` · `svg-polyline-single-point-square-cap.svg` | A single point splits by closure: the polygon is the zero-length **closed** contour whose square cap paints a dot, the polyline is a move-only open contour that paints nothing — the cap laws from the strokes rung, restated through the points grammar. |
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
diverges. (That is also why an SVG elliptical arc is not in this corpus:
Chromium rasterizes one through the ellipse's conics — measured
byte-identical — so admitting arcs means emitting conics, and it inherits
exactly this departure. See `unsupported/`.)

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
