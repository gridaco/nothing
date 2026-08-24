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
  draws them through), `<circle>`, `<ellipse>`, `<path>` (the complete
  `none | <path-data>` presentation-attribute grammar with `fill-rule`): source
  numbers follow Blink's ordered float evaluation; every complete segment
  before a syntax error survives; an empty prefix paints nothing; ordinary
  non-finite derived verbs invalidate the path while an extreme arc may append
  no segment and preserve prior ink; and elliptical arcs resolve through the
  pinned Skia conic construction. Six new `d` cells plus one companion cell
  for the shared `points` scanner carry those boundaries. The CSS `d` property
  remains a named refusal because the pinned
  cascade has no corresponding longhand. Also admitted are `<line>`, `<polygon>`
  and `<polyline>` (the `points` grammar through the same number scanner as
  path data; an erroneous list refuses the whole element by name where
  Chromium renders its valid pair prefix — a declared divergence), nested in
  `<g>` (and `<a>`, the same container semantics) with the whole `transform`
  grammar, under the outer `<svg>`.
  On `<circle>` and `<ellipse>`, the `cx`/`cy` presentation attributes default
  to zero and accept the admitted finite number/percentage route; negative
  centers remain valid. A circle with missing, zero, or negative `r` does not
  materialize a frame node. Percentages use the x/y axis bases for `cx`/`cy`
  and the normalized diagonal for `r`, in unmapped root units and through a
  `viewBox`, and retain those meanings through `<use>`, transforms, and
  strokes. Five Chromium-baked cells carry that subset. The three attribute
  checklist rows remain open: valid source decimals whose raw f32 parse loses
  Chromium's used-value provenance refuse as `unsupported SVG geometry`, and
  CSS comments in numeric presentation values still refuse as bad numbers.
  Finite percentage tokens whose basis operation overflows and resolved
  centers and positive radii outside the admitted Web used-value range also
  refuse by attribute (a negative radius remains invalid no-node geometry);
  every derived circle corner and extent is checked before a frame fact is
  built. Chromium's percentage-drop and fixed-value clamp split is measured,
  but that clamp is not implemented here.
  Unit-bearing values, CSS math, `var()`, and CSS-wide keywords also refuse by
  their exact attribute; each belongs to its own open value-type row. The CSS
  property spellings stay separate named refusals because the pinned Stylo
  build has no `cx`/`cy`/`r` longhands; no matcher is layered around it.
  On `<rect>`, `x`/`y` default to zero and accept the same admitted finite
  number/percentage route; negative coordinates remain valid. Missing, zero,
  or negative `width`/`height` disables the element's fill and stroke, while
  percentages use their own x/y axis bases in unmapped root units and through
  a `viewBox`. The existing two rect-percentage basis cells plus three new
  grammar, `<use>`, and transform-plus-stroke cells carry that admitted subset.
  The four attribute rows remain open. Both source-number alias classes refuse
  before choosing the wrong adjacent value; overflowing percentages and
  drawable values outside Chromium's fixed used-length clamp refuse by exact
  attribute (negative extents keep their invalid no-paint meaning). Unit
  values, CSS math, `var()`, all CSS-wide keywords, comments, and rect `auto`
  size keywords likewise refuse by exact attribute. Chromium-honored CSS
  `x`/`y` declarations are quarantined at their authored
  stylesheet/style-attribute ingress because those longhands are absent at the
  Stylo pin; represented CSS `width`/`height` continue to refuse from computed
  style. Those Chromium-side alias, range, value-family, and CSS-ingress facts
  are measured, not celled; their corresponding refusals are registered. Root
  `auto` remains admitted as the absent dimension, while root percentage
  sizing and CSS sizing remain the document-level contracts above. `<image>`
  and `<pattern>` retain their own element/resource refusals; mask-region
  geometry is admitted only by the separately bounded mask slice below, so
  this rect evidence does not close the generic `x`/`y`/`width`/`height` rows.
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
  Geometric `clip-path` is consumed on admitted non-root SVG targets. The
  presentation attribute is a hint for the pinned cascade's typed property,
  so inline style and stylesheet declarations beat it, an invalid declaration
  exposes it, `none` removes it, and the `-webkit-clip-path` alias and `var()`
  substitution take the same computed route. Same-document `url(#…)`
  references use the whole-document, first-id-wins table; a missing id, a
  non-`<clipPath>` target, or an invalid URL installs no clip, as Chromium
  does. A resource contributes the union of its visible direct admitted
  geometry children and direct shape-valued `<use>` children. Fill, stroke,
  opacity, and nested containers on those children do not turn clipping into
  painting; an empty union clips everything. `clip-rule` is inherited across
  the resource and its children, with `nonzero`, `evenodd`, and the CSS-wide
  behavior Chromium gives the presentation attribute. Its CSS property twin
  is unavailable at this Stylo pin and remains a named authored-CSS refusal.
  `clipPathUnits` is complete: missing and `userSpaceOnUse` use target user
  space, while `objectBoundingBox` maps the unit square through the target's
  fill-geometry box before the resource's own transform. The box excludes
  stroke; a zero-area box produces the valid empty clip. Child, resource, and
  target transforms, outer `viewBox` mapping, groups, `<use>` targets and
  contributors, centered stroke, resource-to-resource chains, nested target
  clips, and target opacity all retain their measured ordering. The resolved
  frame carries only path geometry: one union per resource and an intersection
  of chained resources, with no URL, DOM node, mask, or backend object.
  Eight Chromium-baked cells carry this path-strategy slice. Seven are
  byte-exact; the direct oval clip differs at six boundary pixels by at most
  three channel values, the existing measured native-oval Skia boundary.
  What refuses by name: CSS basic shapes and geometry boxes, root or HTML-host
  clipping, external and cyclic references, animation inside a resource,
  visible text, a contributor with its own clip, and 43 or more visible
  contributors — the cases where Chromium takes a CSS-layer or raster-mask
  strategy that this geometric contract intentionally cannot express.
  Comments, escapes, and `var()` in the direct `clip-rule` attribute also
  refuse rather than bypassing the absent longhand. Those guarded branches
  keep `<clipPath>`, both `clip-path` rows, and both `clip-rule` rows open;
  only the independently listed `clipPathUnits` row closes.
  Same-document SVG image masks are consumed on admitted non-root SVG targets.
  The direct `mask` presentation attribute carries `none` and one
  same-document `url(#…)`, including CSS comments around the URL. References
  use the whole-document, document-order, first-id-wins table; a missing id,
  a wrong-kind id, malformed syntax, or `none` installs no mask. A valid empty
  source always hides the target; opaque black hides in luminance mode and
  reveals under `mask-type="alpha"`. External URLs refuse because this
  command owns no resource I/O, and an active root mask refuses in both
  admissions because Chromium applies it through the host CSS-layer route.
  The CSS mask shorthand and every mask-family longhand stay named authored
  refusals: this Servo-mode Stylo pin furnishes no computed mask route the
  compiler can consume, and no matcher is layered around the cascade.
  A mask source is one isolated image. Its admitted shape, path, stroke,
  gradient, group/transform, `<use>`, clip, opacity, and nested-mask children
  composite with each other before the source becomes alpha. The missing
  `mask-type` and explicit `luminance` presentation values use Chromium's
  luminance weights; explicit `alpha` uses source alpha. CSS-authored
  `mask-type`, `inherit`, and `var()` refuse by name through their own open
  rows. Any unsupported source child makes source construction transactional:
  strict mode refuses and best-effort skips the whole affected target, never
  a partially masked pixel. The mask element's own opacity, transform, mask,
  display, and `clip-path` (attribute or CSS) are inert as Chromium measured.
  A CSS `filter` on the resource is also inert; its attribute twin remains an
  over-refusal under the independently listed filter row.
  Other unrepresented inline declarations on the resource refuse before they
  can change a source descendant silently. Chromium inherits resource-own
  `shape-rendering: crispEdges` exactly like the child spelling (96 pixels at
  maximum delta 63 from the default), while the former route emitted the
  default byte-identically; resource-own `color-interpolation: linearRGB`
  likewise changes 30 pixels at delta 1 (measured, not celled).
  `maskUnits` and `maskContentUnits` carry their complete case-sensitive
  `userSpaceOnUse | objectBoundingBox` grammars and specified defaults.
  Region `x`/`y`/`width`/`height` accept finite numbers, percentages, and `px`;
  the default object-box region is `-10% -10% 120% 120%`, based on the target's
  fill-geometry box rather than its stroke. User-space percentages use the
  current viewport or mapped `viewBox`. CSS-wide and invalid region spellings
  take the per-field default. The region is a hard clip; zero or negative
  extents yield an empty mask. Target transforms carry the region,
  source transforms stay in source space, a target clip encloses the mask,
  and target opacity encloses the masked result.
  Scratch probes disproved the proposed region source-number alias (measured,
  not celled): direct-number and `px` midpoint sources selected the lower
  adjacent control in Chromium and n0 under an admitted pure translation;
  with the independent upscale patrol temporarily bypassed, a percentage
  source selected the upper control in both. Each opposite control differed by
  96 pixels.
  Percentage resolution keeps Blink's observable
  `basis × percentage ÷ 100` operation order. Finite region coordinates beyond
  the unimplemented Web used-length clamp refuse by field; Chromium clamps the
  measured x witness to 33,554,428, while the former route lost 1,728 pixels
  for huge sources and 96/192 for its adjacent-high controls (measured, not
  celled). One separate hard-region precision boundary refuses before
  rasterization. Translation and sampled
  positive axis-aligned downscales through identity are exact; at x-scale 1.01
  the threshold-aligned lower and upper controls differ from Chromium by 96
  and 48 pixels respectively, both at maximum delta 255 (measured, not
  celled). The route refuses upscales and conservatively over-refuses
  rotations, reflections, and shears. Non-`px` units, CSS math, and `var()`
  retain their own value-type rows. Nineteen Chromium-baked mask cells
  carry the admitted slice; eighteen are byte-exact, and the luminance-gradient
  cell has the measured one-code-value ramp bound (576 pixels). The former
  broad refusal is replaced by sixteen focused rows. The `<mask>` element and
  `mask` presentation-attribute rows stay open for the named resource/layer
  remainder; the `mask-type`, `maskUnits`, and `maskContentUnits` attribute
  rows close.
  Same-document SVG filters are consumed on admitted non-root SVG targets.
  The direct `filter` presentation attribute carries `none`, CSS-wide reset
  values, and one same-document URL token with quoted or unquoted content;
  whole-document lookup is first-id-wins, comments around the URL are
  accepted, and missing/wrong/malformed references install no filter. A valid
  empty graph instead hides the target. The CSS property stays a separate
  named boundary: this pinned Servo-mode cascade represents filter functions
  but not the URL computed variant, so authored CSS is quarantined rather
  than matched by another parser.
  The resolved frame carries a checked backend-neutral graph, never the URL or
  authored result names. Its current operations are `feGaussianBlur`, integer
  `feOffset`, zero-input `feFlood`, all seven `feComposite` operators, and
  ordered `feMerge`/`feMergeNode`. Inputs resolve to `SourceGraphic`,
  `SourceAlpha`, the previous result, or an earlier named result before the
  frame; unknown values follow Chromium's measured first/previous fallback.
  Flood carries initial black/one, admitted sRGB colors and `currentColor`,
  number/percentage opacity with clamping, float alpha multiplication, and a
  hard primitive region. Composite carries `over`, `in`, `out`, `atop`, `xor`,
  `lighter`, and `arithmetic`; its four coefficients carry signed decimal and
  exponent number forms, initial zero, and channel clamping. Merge is empty-
  transparent through ordered N-input and reads only direct `feMergeNode`
  children. The complete crisp shadow graph — offset source alpha, flood,
  composite-in, then merge under source graphic — is exact.
  Offset carries absent/zero, signed integer displacement, both primitive unit
  systems, input/result routing, and hard/default subregions. Valid source
  fractions, integer offsets mapped to fractional device displacement, and
  every blur-plus-offset graph remain named backend-precision boundaries.
  Direct flood CSS, `var()`, explicit inheritance, wider color functions, and
  CSS math likewise refuse under their independently listed rows; the pinned
  cascade has no flood longhands and no second matcher is added around it.
  Gaussian blur keeps its measured number-list and axis behavior, source and
  color-space routing, and safe multi-node graphs. The older graph-depth
  diagnosis is withdrawn: three chained safe-sigma kernels, identity-merge
  chains, and parallel branches are exact. The actual pinned-backend boundary
  is sampled from effective `.5` through `1.875`, while `.25` and `2` are
  exact (measured, not celled). The patrol conservatively refuses the open
  interval between those endpoints after target mapping. Current Chromium
  ignores `edgeMode` on blur (measured, not celled); its global row remains open
  because the attribute also applies to `<feConvolveMatrix>`.
  `filterUnits` and `primitiveUnits` carry their complete case-sensitive
  `userSpaceOnUse | objectBoundingBox` grammars, defaults, and invalid-value
  fallbacks. Filter and primitive regions accept admitted finite numbers,
  percentages, and `px`, with hard clipping and object-box/viewport bases.
  Target/group isolation, fill and stroke, transforms, non-uniform `viewBox`,
  `<use>`, nesting, and the filter, then mask, then opacity, then clip order
  remain one composition route. Non-`px` units, CSS math, `var()`, used-range
  gaps, non-positive primitive regions, unsupported primitives, `href`, and
  external/root/list routes still refuse by stable name.
  Flood percentages retain the CSS parser's parse/divide/narrow order; the
  raw-f32 neighbour alias is an exact regression cell. A zero-sigma blur still
  applies its explicit primitive region; a dedicated cell guards that crop.
  Internal Porter-Duff composition distinguishes exact byte-domain generated
  sources from floating source-image coverage, and the filtered scope restores
  through an F16 layer so x86 cannot select its approximate low-precision
  SrcOver path. ARM and x86 are exact without a tolerance. Eighty-six
  Chromium-baked filter cells are exact: twenty-six from the chassis/blur slice
  and sixty from the shadow-graph rung. The complete corpus is 447 Chromium-baked
  cells plus 10 sampled frames, with 124 named refusal
  rows. `feFlood`, `feComposite`, `feMerge`, `feMergeNode`, and `k1`–`k4`
  close; `feOffset`, `feGaussianBlur`, `<filter>`, `filter`, `in`, `in2`,
  `operator`, `result`, `dx`, `dy`, `flood-color`, and `flood-opacity` remain
  open for the named precision, applicability, resource, cascade, or value
  remainder.
  A stroke is centred, its width is a cascaded length in either spelling —
  numbers, absolute units, `em`/`rem` against an authored or default
  font-size, percentages against the normalized diagonal, and pure-length
  `calc()`/`min()`; the CSS property beats the attribute and an invalid
  declaration drops so the attribute survives. The `px`, `em`, `rem`, percentage,
  `calc()`/`min()`, precedence, and fallback claims are Chromium-baked
  cells; the remaining absolute units are pinned by the strokes contract
  against the same cascade constants (`6pt ≡ 8px` measured). Its
  cap, join and miter limit come from the one cascade. Pure fixed widths clamp
  to Chromium's Web used-length ceiling (33,554,429 authored, 33,554,428 as
  the resolved f32 fact). Both source spellings are Chromium-baked in one
  large-user-space repair cell. An unambiguous extreme direct percentage
  follows Chromium's separate percentage path: positive overflow in
  the authored-percentage-times-basis intermediate resolves to the maximum
  finite width. It stays one exact stroke fact for the painter — it is not
  normalized to absence — and four attribute/CSS cells cover the resulting
  join-, cap-, transform-, dash-, and topology-dependent ink.
  One narrower percentage class refuses by name. The pinned cascade can map
  distinct authored percentages to one retained computed bucket even when
  Chromium gives those sources distinct used widths; measured on a 64×32 user
  space, `100.00000762939453%` and `100.00001525878906%` share that bucket but
  differ by 16 pixels after amplification. Once the distinction is erased, the
  producer cannot choose either raster honestly. Presentation attributes,
  inline style, stylesheets, and inheritance guard this precision alias.
  Non-identity percentage math that this cascade folds to a pure percentage
  refuses under the same name — including authored length terms that cancel to
  zero. The final percentage survives, but not the operation history needed to
  reproduce Chromium's used width.
  A width whose basis this cascade lacks (viewport-, container-, and
  font-metric-relative units, root-relative twins included), a `calc()`
  mixing lengths and percentages, a font-size that would poison the `em`
  basis, and the spellings the authored-text patrol cannot read — `var()`
  indirection and CSS escapes — all refuse by name. The SVG2-only
  join values `miter-clip` and `arcs` drop as invalid declarations exactly
  as Chromium drops them (measured, celled) — an agreement, not a hole.
  `stroke-dasharray` is consumed in both spellings from that same cascade:
  numbers/lengths/percentages, comma or whitespace separators, CSS math,
  odd-list repetition, inheritance through containers and `<use>`, and
  author-over-hint precedence resolve to one checked even cycle in local
  user-space distance. Every admitted geometry receives it, every contour
  restarts at phase zero, and transforms scale the resolved cycle
  with the geometry. `none`, an all-zero list, and an invalid negative list retain
  Chromium's solid fallback; zero painted intervals remain meaningful under
  round/square caps, including on closed contours. These claims are covered by
  27 Chromium-baked cells. Pure fixed dash members clamp individually to the
  same Web used-length ceiling before odd-list doubling. Extreme percentages
  do not take that fixed ceiling: if their resolution makes the cycle
  non-finite, Chromium drops the dash effect, leaving a solid stroke with the
  authored cap. Byte-identical attribute/CSS cells pin the clamp, doubling,
  per-contour restart, and percentage result on discriminating large geometry.
  `stroke-dashoffset` is consumed in both spellings as the cycle's signed
  local-space phase: positive/negative numbers and lengths, pure-length math,
  normalized-diagonal percentages, inheritance through containers and
  `<use>`, CSS-wide values, and author-over-hint precedence. The phase is
  canonical modulo the positive cycle only after odd-list doubling, restarts
  on every contour, moves zero-length cap slots, remains local under transforms,
  and reaches every admitted geometry route. Thirteen Chromium-baked cells are
  byte-exact, including the asymmetric fixed used-value floor (-33,554,430)
  and the positive carried ceiling (33,554,428) before phase normalization.
  The named remainder is exact. Distinct valid authored percentages around
  57,384% and at the finite/overflow boundary can collapse into one pinned
  Stylo f32 while Chromium retains different used phases; percentage-bearing
  math loses the same source history. The stable percentage-precision-alias
  refusal guards that class, so both dashoffset checklist twins remain open
  under the gridaco/nothing#81 split precedent. Viewport/container/font-metric
  units, `var()` indirection, CSS escapes, and a poisoned em basis retain their
  narrower registered guards.
  `pathLength` is consumed on all seven admitted geometry elements. Its SVG
  `<number>` attribute is non-inherited: absence and a negative value leave
  dash distances uncalibrated, while zero/negative zero and a malformed
  present value follow Chromium's authored-zero fallback. On geometry with a
  non-zero local metric, the resulting scale saturates rather than becoming
  zero; a zero local metric instead yields a zero calibration factor. A
  positive value applies one actual-local-length/authored-length scale to every
  already resolved dash member and to the signed raw phase before cycle
  canonicalization. When a saturated scale leaves the tiny interval cycle
  finite but overflows a large finite phase, Chromium drops the dash effect and
  retains a solid stroke; strict and best-effort admit that same result. Fixed
  lengths and normalized-diagonal percentages receive
  that same scale in Chromium, including in one mixed list; this deliberately
  follows the browser even though the current SVG2 text says percentage
  distance-along-path calculations are not affected by `pathLength`.
  Open, closed, and mixed contours contribute to the actual local metric;
  quadratic, cubic, conic/arc, rounded-rect, rect, and native oval routes are
  measured, and each contour still restarts at the one calibrated phase.
  Calibration precedes transforms and viewBox mapping. On `<use>`, the
  attribute belongs to the referenced geometry; a use-site or group spelling
  is inapplicable and does not inherit. A calibrated dashed ellipse retains
  its absolute local bounds until paint because Chromium's f32 oval metric and
  dash traversal are translation-sensitive. None of this adds source syntax
  to the resolved frame: it still carries only the final local-space interval
  cycle and canonical phase.
  Current SVG2 also defines the non-inherited CSS twin
  `path-length: none | <length [0,∞]>` and maps the attribute as a pixel-length
  presentation hint. Chromium 149 ships that experimental property disabled,
  so valid inline and stylesheet declarations drop wholesale and do not
  override the active legacy attribute. The engine matches that drop without
  a second CSS matcher. Nine byte-exact Chromium cells cover the attribute,
  the CSS drop, every geometry, algebra, percentage basis, coordinate/instance
  routes, metrics, contours, and numeric extremes; the former broad
  `pathLength` refusal has graduated.
  The stroke's `<paint>` grammar is celled: hex and named colours,
  `currentColor` against the `color` hint, `none` (the initial — an
  invalid paint drops to it), and the full `url() [none | <color>]?`
  reference semantics (a dead reference falls back, a stopless gradient
  paints nothing with the fallback inert) in the attribute spelling; the
  hex declaration, the CSS-over-attribute precedence pair, the
  invalid-declaration fallback, and the gradient reference in the CSS
  spelling — one computed paint, the CSS cells byte-identical to their
  attribute twins. The remaining cross products (a named colour,
  `currentColor`, or a fallback in CSS spelling) are measured, not
  celled. `context-fill` and `context-stroke` complete that standard-track
  surface for both `fill` and `stroke`, in both source spellings. With no
  context element they select no paint. In an instantiated subtree they
  select the immediate `<use>` element's computed fill or stroke; another
  context keyword recurses outward until an ordinary no-paint, solid, or
  gradient value is found. The eventual owner's `currentColor`, colour alpha,
  URL fallback, coordinate space, and object bounding box stay attached to
  that selected paint, while fill/stroke opacity remains an ordinary
  independently inherited property. Linear and radial gradients are rebased
  before the frame boundary and remain continuous across transformed clone
  leaves; hidden and zero-opacity geometry contributes to the use box, while a
  display-pruned subtree does not. Twenty-two Chromium-baked cells cover the
  two keywords, two destination properties, two source spellings, cascade and
  recursion edges, independent instances, URL fallback, both gradient kinds
  and units, ultimate-owner anchoring, and box participation. Additional
  Chromium measurements pin accumulated `<use>` `x`/`y` translations, transformed
  local-AABB box construction, singular destinations painting nothing, and
  paint-opacity separation. The standard-invalid context-plus-fallback parser
  extension refuses by name; patterns, external paint resources, marker
  context, and author stylesheets across a use-shadow boundary retain their
  own named rows rather than entering through context paint.
  Paint is solid
  sRGB, opaque or translucent: `fill-opacity`, `stroke-opacity`, and a
  colour's own alpha multiply in float and quantize once (the translucency
  rung), Chromium-baked.
  Element `opacity` is consumed in every spelling (presentation attribute,
  style attribute, stylesheet — one <alpha-value> grammar, clamped exactly
  as Chromium clamps). A single un-transformed, un-folded solid draw uses the
  group-scope rung's measured fold: the element factor joins the colour and
  fill/stroke opacity product before its one quantization. A valid gradient
  instead keeps its intrinsic paint opacity and carries the element factor
  separately across the resolved contract; the painter materializes the
  paint's own alpha first, then multiplies the factor in float without a
  second 8-bit quantization. A one-stop server remains a two-identical-stop
  constant gradient so it retains that raster route. An invalid URL fallback
  remains an ordinary solid and takes the solid fold.
  Everything that is genuinely a group composites through a real isolated
  layer: a shape's fill and stroke together, a group of several draws, nested
  opacities (which quantize per layer and never flatten to a product — measured
  one code value apart), and any opacity whose content carries a transform
  strictly below it. `opacity: 0` renders the correct nothing. `<use>` and `<a>`
  scope exactly as `<g>`. Root opacity encloses the complete SVG-local frame
  and preserves its transparent outer surface in both standalone and inline
  entries. Every non-identity HTML ancestor opacity is a distinct outer scope
  around the selected inline SVG; explicit `inherit` on the SVG compounds with
  those host scopes rather than flattening them.
  `<linearGradient>` and `<radialGradient>` paint servers are consumed
  (the gradient rung): `fill`/`stroke` `url(#…)` references resolve through
  a whole-document, first-id-wins gradient table (shadow-content clones
  excluded — the document's element wins, measured), with both
  `gradientUnits`, `spreadMethod`, stops from attributes (`offset` clamps
  to the running maximum and is never sorted; equal-offset hard stops render
  crisp), `href`/`xlink:href`
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
  zero or negative radial radius and linear endpoints inside the backend's
  degenerate threshold resolve to the tile-specific measured solid: the last
  stop under `pad`, or the ramp's integral average under `reflect`/`repeat`.
  The `<stop>` presentation attributes are consumed at their listed
  grammars (the stop rung). `stop-color` carries the `color` property's
  `<color>`: hex in all four lengths, named colours, `transparent`,
  `rgb()`/`rgba()` with a number or percentage alpha, and `currentColor`
  resolved against the gradient's own ancestor chain — an unparseable value
  is the initial black. `stop-opacity` carries the `opacity` property's
  `<number> | <percentage>`, clamped exactly as Chromium clamps. A stop
  colour's own alpha resolves to its byte (measured: `rgb(… / 0.5)` is
  byte-identical to `#…80`); `stop-opacity` then multiplies in float, and
  the resolved contract carries that product **unquantized**, because the
  rasterizer interpolates the ramp before it quantizes. `initial`, `unset`
  and `revert` coincide with each attribute's initial and are admitted.
  What refuses by name: a focal radial (`fx`/`fy` off
  the center or `fr > 0` — the shared radial leaf is concentric),
  `color-interpolation: linearRGB`, a degenerate paint server whose
  substituted colour does not land on a byte (Chromium keeps a dithering
  shader for those and no flat colour reproduces one — guarded as
  `svg-gradient-degenerate-precision`, and a gradient-geometry gap rather
  than a stop-grammar one: it fires on ramp averages carrying no
  `stop-opacity` at all — [gridaco/nothing#93](https://github.com/gridaco/nothing/issues/93)), a stop attribute that needs a resolver this build
  does not run — `inherit`, `var()`, or a CSS math function, each measured
  painting a silently wrong pixel before the patrol and each naming a
  construct with its own checklist row — a non-legacy sRGB stop colour
  (which changes how Chromium interpolates the whole ramp, unlike the same
  value as a solid),
  author CSS on stops (`stop-color` /
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
