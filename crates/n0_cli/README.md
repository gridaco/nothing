# n0 CLI

`n0_cli` builds the `n0` executable: the thin product host for file-to-output
rendering on the SVG engine of record
([the SVG engine of record](../../docs/wg/consolidation/svg-engine-of-record.md)).
The command owns
arguments, source I/O, raster surfaces, and encoding. It does not own source
semantics, layout, the drawlist, or an authored document model.

The route is one pipeline: the websem compiler lowers standalone SVG or
inline-HTML SVG from the retained document session to the shared
`rframe::Frame`, which the n0 engine compiles and paints on a CPU raster.
Static renders are the Base view; `--time-ns` renders one exact
signed-nanosecond Sample of the same compile:

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/svg-fill-named-rect.svg /tmp/rect.png 64x64

cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-rect-x-animation.svg /tmp/t1s.png 64x32 \
  --time-ns 1000000000

# the cub: a whole composition (viewBox-only, containers, curves, strokes,
# a <line>) with one animated rect — the same file, static and at 1s
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub.png 96x96
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub-1s.png 96x96 \
  --time-ns 1000000000

# text: the font is a declared, verified input of the render — the family
# names bytes, and the bytes are checked before any pixel
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/text/svg-text-em-box.svg /tmp/text.png 100x100 \
  --font Ahem=fixtures/web-first/fonts/ahem.ttf@sha256:b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448

# dev harness: refuse on the first beyond-slice construct instead of
# rendering best-effort with declared degradations (the default)
cargo run -p n0_cli --bin n0 -- \
  fixtures/test-svg/L0/basic-shapes.svg /tmp/probe.png 500x500 --strict
```

- Input: one UTF-8 `.html`, `.htm`, or `.svg` file. A `<!DOCTYPE …>`
  declaration is accepted and ignored — as Chromium ignores it for SVG — but
  a document carrying an internal DTD subset (entity declarations) refuses as
  not-well-formed XML in both admissions; entity content is never silently
  dropped.
- Output: one `.png` file at an explicit positive `WxH` size. For a
  standalone SVG, `WxH` is also the **initial viewport** (SVG2 §8.2) — the
  window the document is loaded into: explicit root `width`/`height` win, a
  missing dimension is `auto` and resolves to 100% of `WxH`, and `viewBox`
  maps user units into the viewport under the full `preserveAspectRatio`
  grammar. A viewBox-only SVG therefore renders at the requested raster.
  Shape-geometry and `stroke-width` percentages resolve against the
  viewport's user-unit extent (the `viewBox` when present) per SVG2 §7.10 —
  x-axis against width, y-axis against height, radii and stroke widths
  against the normalized diagonal; _root_ percentage sizing stays a
  document-level refusal until a host-level oracle can bake it.
- Resources: self-contained input only; external images and stylesheets are
  not resolved.
- Capability: the admitted slice is deliberately narrow — solid- or
  gradient-filled and -stroked `<rect>` (rounded corners included: `rx`/`ry`
  resolve by the measured auto/clamp matrix and lower to the conics Chromium
  draws them through), `<circle>`, `<ellipse>`, `<path>` (the whole path-data
  grammar — the elliptical arc resolves to conic segments — with
  `fill-rule`), `<line>`, `<polygon>`
  and `<polyline>` (the `points` grammar through the same number scanner as
  path data; an erroneous list refuses the whole element by name where
  Chromium renders its valid pair prefix — a declared divergence), nested in
  `<g>` (and `<a>`, the same container semantics) with the whole `transform`
  grammar, under the outer `<svg>`.
  `transform` is consumed in both spellings: the attribute is a presentation
  attribute of the one CSS `transform` property (CSS Transforms L1 §7),
  entering the cascade at hint level, so author CSS beats it —
  `transform: none` included — an invalid CSS declaration falls back to it,
  and a malformed attribute list drops whole and renders untransformed,
  each exactly as Chromium resolves the pair (Chromium-baked, including the
  measured run-together leniency no browser ever tightened). Transforms
  pivot on the measured SVG used origin — the local user-space origin —
  and percentage translations resolve against the viewport's user-unit
  extent; authored `transform-origin` and `transform-box`, the individual
  `rotate`/`translate`/`scale` properties, and the beyond-2D function forms
  (`translate3d`, `matrix3d`, `perspective`, …) stay named refusals, and
  the root `<svg>`'s own transform refuses in both spellings (it applies
  to the CSS box outside the viewBox mapping).
  `<use>` and `<defs>` are consumed: same-document references resolve
  through a whole-document, first-id-wins table (forward references and
  the legacy `xlink:href` spelling included; a plain `href` beats it), the
  instance renders as the use's shadow content with inheritance from the
  use site (a `fill` or `color` on the use colors clones that author none
  of their own — `color` is an admitted hint since this rung), `x`/`y`
  append a translate inside the use's transform, and the measured correct
  nothings render as nothing: an unresolved reference, a reference cycle,
  an ancestor reference. What refuses by name: a document with any author
  stylesheet (the measured shadow boundary scopes selectors to the cloned
  subtree, which the flattened tree cannot express), an external
  reference, authored element children, a `<symbol>`/nested-`<svg>`
  target, and reference chains beyond the expansion budget.
  A stroke is centred, its width is a cascaded length in either spelling —
  numbers, absolute units, `em`/`rem` against an authored or default
  font-size, percentages against the normalized diagonal, and pure-length
  `calc()`/`min()`; the CSS property beats the attribute and an invalid
  declaration drops so the attribute survives. The `px`, `em`, `rem`, percentage,
  `calc()`/`min()`, precedence, and fallback claims are Chromium-baked
  cells; the remaining absolute units are pinned by the strokes contract
  against the same cascade constants (`6pt ≡ 8px` measured). Its
  cap, join and miter limit come from the one cascade; dashing does not.
  A width whose basis this cascade lacks (viewport-, container-, and
  font-metric-relative units, root-relative twins included), a `calc()`
  mixing lengths and percentages, a font-size that would poison the `em`
  basis, and the spellings the authored-text patrol cannot read — `var()`
  indirection and CSS escapes — all refuse by name. The SVG2-only
  join values `miter-clip` and `arcs` drop as invalid declarations exactly
  as Chromium drops them (measured, celled) — an agreement, not a hole.
  Paint is solid
  sRGB, opaque or translucent: `fill-opacity`, `stroke-opacity`, and a
  colour's own alpha multiply in float and quantize once (the translucency
  rung), Chromium-baked.
  Element `opacity` is consumed (the group-scope rung), in every spelling
  (presentation attribute, style attribute, stylesheet — one <alpha-value>
  grammar, clamped exactly as Chromium clamps), by the measured fold rule:
  over a single un-transformed, un-folded draw it folds into that draw's
  paint — joining the translucency rung's one float product, quantized once
  (byte-identical in Chromium) — and everything else composites through a
  real isolated layer: a shape's fill and stroke together, a group of
  several draws, nested opacities (which quantize per layer and never
  flatten to a product — measured one code value apart), and any opacity
  whose content carries a transform strictly below it. `opacity: 0` renders
  the correct nothing. `<use>` and `<a>` scope exactly as `<g>`. What
  refuses by name: element opacity folding over a gradient or `url()`
  paint (the paint carries one quantized alpha, and Chromium composites
  the element factor after that quantization), and the root `<svg>`'s own
  opacity (it composites the whole canvas, which the opaque raster surface
  cannot express — like the root's transform).
  `<linearGradient>` and `<radialGradient>` paint servers are consumed
  (the gradient rung): `fill`/`stroke` `url(#…)` references resolve through
  a whole-document, first-id-wins gradient table (shadow-content clones
  excluded — the document's element wins, measured), with both
  `gradientUnits`, `spreadMethod`, stops from attributes (`offset` clamps
  to the running maximum and is never sorted; `stop-color` — `currentColor`
  against the gradient's own ancestor chain — and `stop-opacity` fold and
  quantize once; equal-offset hard stops render crisp), `href`/`xlink:href`
  template chains (stops all-or-nothing from the first owner; geometry
  never crosses gradient types; a cycle kills only the edge), and
  `gradientTransform` as the transform property's presentation attribute on
  gradient elements — an author `transform` declaration beats it, the plain
  `transform` attribute is inert there, and the value applies about the raw
  origin of gradient space, all Chromium-measured. Ramps interpolate
  unpremultiplied sRGB and dither exactly as Chromium's rasterizer does.
  The authored fallback fires only on an _invalid_ reference (a missing id
  or a non-gradient target); the measured correct nothings — zero stops
  (fallback unfired), a self-cycle, a non-invertible gradient transform, an
  object-bounding-box gradient on zero-area geometry — paint nothing. A
  zero or negative radial radius is a solid of the last stop, and linear
  endpoints inside the backend's degenerate threshold resolve to the
  measured solid (last stop under `pad`, the ramp's integral average under
  `reflect`/`repeat`). What refuses by name: a focal radial (`fx`/`fy` off
  the center or `fr > 0` — the shared radial leaf is concentric),
  `color-interpolation: linearRGB`, author CSS on stops (`stop-color` /
  `stop-opacity` — the pinned cascade has no such longhands, so a sheet
  declaring one is a document-level declaration and a stop's style
  attribute refuses the paint), font-relative units in gradient geometry,
  a percentage in a gradient's computed transform (Chromium resolves it
  against mismatched spaces), an external reference, and `<pattern>`.
  `<text>` is consumed (the text rung), and its font environment is the
  host's: text resolves only against fonts declared with
  `--font FAMILY=PATH@sha256:HEX` (repeatable), whose bytes are **verified
  against the declared digest before any pixel exists** — a family name is
  not a font identity, and a mismatch refuses the render rather than
  producing a silently different one. A `<text>` run whose family was never
  declared refuses by name; there is no system fallback, no ambient face,
  and therefore no machine-local pixel anywhere on this path. Inside that
  environment one run resolves once through
  [the text oracle](../../docs/wg/feat-paragraph/text-layout.md) at its v0
  profile — one style run of printable ASCII, horizontal and
  left-to-right, no wrapping and no fallback — and its glyph outlines lower
  to the contract's ordinary path facts, so no font identity crosses into
  the resolved frame. `x`, `y`, and the `text-anchor` attribute
  (`start`/`middle`/`end`) place the run; `font-family` and `font-size`
  come from the one cascade, where an author rule beats the presentation
  attribute exactly as Chromium measured. Geometry is admitted only inside
  the ratified [numeric domain](../../docs/wg/consolidation/text-oracle.md)
  — integer position, a `font-size` that is an integer multiple of 5, an
  integer anchor-resolved start — because that is where every rasterizer's
  per-pixel coverage is 0 or 1 and the byte-exact gate holds; Chromium
  snaps everything else by a rasterizer-internal rule, and this refuses by
  name instead of codifying it. What refuses by name: the CSS spelling of
  `text-anchor` (Chromium consumes it from the cascade, the pinned Stylo
  build has no such longhand — a silent drop before the rung), a generic
  family (which names no declared font), `<tspan>` and any other element
  child, `dx`/`dy`/`rotate` lists, `textLength`, decorations, letter and
  word spacing, writing mode and direction, stroke on text, a colour or
  bitmap face, and any character outside the v0 repertoire. The inline-HTML
  entry declares no fonts, so its `<text>` refuses there.
  `display: none` and `visibility` are consumed from the one cascade
  (attribute and CSS spellings alike): a pruned or hidden element renders
  the correct nothing rather than a declared hole, a `visibility: visible`
  descendant un-hides inside a hidden container, and a standalone
  document's outermost `<svg>` ignores `display: none` exactly as Chromium
  does (`display: contents` stays a named refusal).
  The
  default admission is **best-effort**: the admitted subset renders and
  every beyond-slice construct is declared on stderr with its node path and
  reason (`degraded: skipped svg/polygon[1]: unsupported element <polygon>`);
  a beyond-inventory dynamic surface that leaves the Base view honest (an
  event handler, a CSS animation carrier) samples as the Base view, while a
  beyond-inventory _animation element_ — active at document load in
  Chromium, so its target's authored state never honestly renders — skips
  its target in every view, declared at the target's path (one that cannot
  be attributed to a skippable element, an `href` retarget or a
  root-`<svg>` target, refuses in both admissions like `<script>`). Declared
  holes, never guessed pixels — the patrol is per attribute and per
  cascaded property, so an admitted element carrying a rendering attribute
  or stylesheet value the slice does not consume becomes a declared hole,
  not a wrong paint (cascaded properties beyond the enumerated patrol are
  a named open boundary; see the websem compiler doc). `--strict` refuses
  loudly on the first beyond-slice construct instead — the harness that names
  the slice's edge (`--best-effort` is the explicit spelling of the default).
  Document-level contracts (no `<svg>` root, malformed standalone XML, a
  script-suspended standalone parse, the outer viewport grammar —
  percentage root dimensions, malformed `preserveAspectRatio`,
  CSS-cascaded root sizing, missing dimensions on the HTML entry — and root
  patrols) refuse in both admissions. A stylesheet declaring a property the
  cascade cannot represent is document-level too, but only `--strict`
  refuses it: the default declares it against the sheet and renders, since
  a sheet is not attributable to one element without selector matching.
- The HTML entry compiles exactly the document's first inline SVG; when that
  subtree is admitted the render succeeds and the surrounding page
  contributes nothing (a pinned contract). Sampling inline HTML refuses
  under `--strict` and samples as the Base view (declared) by default.

The retired mature `htmlcss` route must not return silently (locked by this
crate's architecture test). The binary name does not imply that Web sources
are converted into the n0 authored model. n0 XML, directory input, resource
loading, and additional encoders enter only when their actual contracts are
implemented.

The governing topology is the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md); the
succession decision is
[svg-engine-of-record.md](../../docs/wg/consolidation/svg-engine-of-record.md);
the donor-mining map is
[web-renderer-adoption.md](../../docs/wg/consolidation/web-renderer-adoption.md).
