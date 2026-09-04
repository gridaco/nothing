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

# ordered declared-family selection: resource order is Bungee then Ahem, but
# each computed request in this cell deliberately selects Ahem
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/text/svg-text-family-list-selection.svg \
  target/text-family-list.png 100x100 \
  --font Bungee=fixtures/fonts/Bungee/Bungee-Regular.ttf@sha256:b90c3ca443713b070cb1dec6a3bb1ef7572c2b565c431d9a85d74bbfa07e24cc \
  --font Ahem=fixtures/web-first/fonts/ahem.ttf@sha256:b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448

# real-font Rung B: artifact geometry is graded exactly before rasterization;
# this render makes no Chromium real-font pixel claim
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/text/geometry/svg-text-allerta-geometry.svg \
  /tmp/text-allerta.png 1200x500 \
  --font Allerta=fixtures/fonts/Allerta/Allerta-Regular.ttf@sha256:16d6915227c7560725c037c9c93163cba5367c3ef4cf2ec12bf40b9eb2984a6b

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
- Capability: the admitted slice is deliberately narrow — `<rect>` filled and
  stroked with solid, gradient, or admitted repeating-pattern paint (rounded
  corners included: `rx`/`ry`
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
  path data; a final unmatched x coordinate is dropped after all complete
  pairs, while every lexical or numeric parse failure resolves to the empty
  list), nested in
  `<g>` (and `<a>`, the same container semantics) with the whole `transform`
  grammar, under the outer `<svg>`.
  On `<line>`, `x1`/`y1`/`x2`/`y2` default to zero and accept finite numbers
  and percentages; signed coordinates remain valid. Percentages use the width
  axis for each x and the height axis for each y, both in unmapped root units
  and through a `viewBox`, and retain those meanings through transforms, stroke
  geometry, and `<use>`. Three Chromium-baked cells carry that subset. The four
  attribute rows remain open:
  two valid source-decimal classes and CSS comments would otherwise select or
  parse differently from Chromium. Overflowing percentages, values outside the
  admitted Web used-length range, wider units, CSS math, `var()`, and CSS-wide
  keywords all refuse by their exact coordinate in both admissions. In
  particular, `px` is admitted on the gradient consumer below but remains in
  the line consumer's unit-family refusal.
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
  retains its own element/resource refusal; pattern tile and mask-region
  geometry are admitted only by their separately bounded slices below, so this
  rect evidence does not close the generic `x`/`y`/`width`/`height` rows.
  A direct non-root `<svg>` now establishes its own static viewport. Missing
  `x`/`y` use zero; missing or explicit-`auto` dimensions use 100% of the
  nearest parent viewport; zero and negative extents paint no subtree; and the
  complete already-admitted `viewBox`/`preserveAspectRatio` mapper establishes
  descendant user space. The measured composition order is parent transform,
  the nested element's computed transform, `x`/`y` placement, then `viewBox`.
  Descendant geometry, stroke, gradient, pattern, clip, mask, filter, marker,
  and ordinary `<use>` percentage consumers all receive that nearest
  viewport's independent axes and normalized diagonal. The nested element's
  own percentage transform still uses its parent viewport.
  Non-root `<svg>` gets SVG's scoped `overflow:hidden` user-agent default in
  the one Stylo cascade. Presentation (including the two-value shorthand),
  inline, and stylesheet declarations select the computed overflow; `hidden`,
  `clip`, and `scroll` clip while
  `visible` and `auto` leave the viewport open. The resolved clip is an
  ordinary antialiased rectangle. Descendant effects are inside it; the
  viewport element's own filter is outside it, and the established
  same-element filter/mask/opacity/`clip-path` order remains around that source
  clip. Twenty-eight exact Chromium cells carry defaults,
  mappings, all affected consumer families, effects, nesting, a viewport in a
  referenced group, and standalone/inline entry parity. No viewport, DOM, or
  source token crosses `rframe`.
  Direct units, CSS math/custom properties/CSS-wide values, comments, numeric
  provenance/range edges, and competing CSS geometry remain the existing
  named geometry/value refusals. Chromium currently ignores CSS
  `x`/`y`/`width`/`height` for direct inner-viewport used geometry (measured,
  not celled), but this slice over-refuses that ingress instead of silently
  dropping it. A `<use>` whose referenced root is `<svg>` remains a separate
  instance-sized viewport refusal: Chromium honors the use-site dimensions,
  and treating them as inert changes 2,880 pixels at maximum delta 233
  (measured, not celled). Nested viewports inside pattern, mask, and marker
  sources likewise retain those resource elements' own transactional source-
  program refusals; their simple measured controls are exact, but do not grant
  the wider source contracts. `<symbol>`, root host sizing, viewport animation,
  and the shared geometry/overflow attribute rows remain independent work.
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
  reference, authored element children, an instance-viewport `<svg>` or
  `<symbol>` target, and reference chains beyond the expansion budget.
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
  `feOffset`, zero-input `feFlood`, all seven `feComposite` operators,
  all sixteen two-input `feBlend` modes, ordered `feMerge`/`feMergeNode`,
  native one-input `feDropShadow`, one-input `feColorMatrix`,
  `feComponentTransfer`, `feMorphology`, `feConvolveMatrix`, and
  `feDiffuseLighting` with one direct `feDistantLight`, `fePointLight`, or
  `feSpotLight` child, plus zero-input `feTurbulence` and two-input
  `feDisplacementMap`. Inputs
  resolve to `SourceGraphic`, `SourceAlpha`, the previous result, or an earlier
  named result before the frame; unknown values follow Chromium's measured
  first/previous fallback.
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
  ignores `edgeMode` on blur; a dedicated Chromium-baked drop cell and the
  complete convolution behavior now close that shared attribute row.
  Native drop shadow carries its own operation rather than lowering to the
  blur-plus-offset graph. Missing `dx`, `dy`, and `stdDeviation` use `2`; one
  or two sigma axes, negative-axis clamping, measured number spellings,
  source/result routing, both unit systems, hard regions, direct flood values,
  and safe transforms and composition are admitted. The shared small-kernel
  patrol applies. Four focused native-shadow patrols also refuse out-of-range
  parameters, non-quarter/fractional target mappings, paint-server or
  descendant-opacity source layers, and interior-channel linearRGB shadow
  colors. The direct sRGB route, a default-linear endpoint-color route that
  consumes an earlier blur, exact quarter turns, integer axis mappings, solid
  source layers, `<use>`, stroke, groups, target opacity, and clip order are
  Chromium-baked exact. CSS flood ingress, inheritance, wider color/math values,
  and custom-property substitution retain their existing named rows.
  Hosted x86 initially exposed all twenty-eight native-shadow cells through the
  backend helper, then twenty-five source-derived sRGB cells at delta one after
  the internal color and foreground stages were made byte-exact. A zero-blur
  control and unchanged colorization/sampling classifiers located the remaining
  fault at the outer filtered-layer restore. The painter now gives sRGB native-
  shadow descendants an exact byte-domain restore and clears that policy across
  color-space conversion; applying it globally would change three unrelated
  floating-path cells. ARM and x86 now match every committed shadow oracle
  without tolerance while the resolved frame still carries one native operation.
  Color matrix carries one finite row-major 4×5 operation over
  non-premultiplied RGBA. Missing and invalid `type` use `matrix`; the complete
  `matrix | saturate | hueRotate | luminanceToAlpha` behavior, exact value
  counts, pass-through fallbacks, SVG number-list grammar, unclamped
  saturation, Blink-ordered hue arithmetic, ignored luminance values, channel
  crossing, alpha scaling/creation, clamping, generated input, SourceAlpha,
  and both filter color spaces are admitted. The source-neutral frame never
  carries the authored type or list. Source-dependent matrix output is limited
  to one direct admitted geometry with an opaque solid fill, no stroke, and no
  children; generated-only input bypasses that source profile. Non-quarter
  target mappings, broader source layers, and source-dependent graphs that
  also contain blur or native shadow refuse by three stable precision names.
  Fractional axis maps, reflections, exact quarter turns, target opacity,
  target clips, circles, and paths are Chromium-baked exact inside the admitted
  envelope.
  Component transfer carries four independent 256-byte channel tables over
  straight RGBA. Missing channel functions are identity and the last direct
  child for a repeated channel wins. The complete case-sensitive `identity`,
  `table`, `discrete`, `linear`, and `gamma` behavior is admitted, including
  initial parameters, invalid fallback, singleton and multi-member tables, the
  full SVG number-list grammar, out-of-range clamping, negative exponents, and
  alpha creation/removal inside the hard primitive region. Authored function
  elements, type names, and numeric lists resolve before the frame; the frame
  carries only the four checked tables and one input.
  Gamma `offset` carries its number-only grammar and initial zero on all four
  function elements. A lone trailing comma retains the parsed prefix; a
  percentage or any other invalid scalar takes the initial. Other function
  types ignore the attribute. Its long-decimal witness uses the same ordered
  source-number evaluation as the list grammar.
  Blink's ordered SVG-number normalization is observable: the source
  `slope="1.654435761" intercept=".18682"` selects the upper binary32 control
  and differs from the raw lexical lower control by 2,304 pixels at delta one.
  Table, discrete, and gamma arithmetic then use the measured double route;
  linear keeps the measured float products and sum; all four clamp and
  truncate to bytes. All 256 source-byte values were probed for every kind
  (measured, not celled).
  Source-derived and generated inputs, SourceAlpha, both color spaces, hard
  regions, both primitive unit systems, safe transforms, paths, circles,
  strokes, `<use>`, alpha, clip/opacity on separate scopes, and ordering with
  blur, offset, matrix, native shadow, composite, and merge are Chromium-baked
  exact. Paint-server source pixels and the unsafe source-transform envelope
  refuse by two component-transfer precision names. A third generic patrol
  refuses any filtered descendant under one element that combines geometric
  clipping with partial opacity: even identity filters reproduce that
  backend effect-stack split. Transfer-function animation remains outside the
  static slice. The separately tracked fill-only ellipse/box-world boundary
  is unchanged.
  Blend carries the complete case-sensitive `normal`, `multiply`, `screen`,
  `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`, `hard-light`,
  `soft-light`, `difference`, `exclusion`, `hue`, `saturation`, `color`, and
  `luminosity` vocabulary. Missing and invalid mode text uses `normal`; that
  includes wrong case, surrounding whitespace, legacy camelCase, CSS-wide,
  and sampled draft-only spellings. The first checked input is foreground and
  the second is backdrop. Omitted, empty, and unknown input names retain the
  graph's previous-or-first-SourceGraphic fallback.
  Opaque and translucent arithmetic, both color spaces, hard regions and
  primitive units, direct and generated sources, path/stroke/gradient/group,
  `<use>`, `viewBox`, fractional axis mapping, exact quarter turns, target
  opacity and mask, and safe ordering with admitted neighboring operations are
  Chromium-baked exact. Three stable patrols guard the measured remainder:
  general blend-output mappings, blend across geometric clipping, and the
  operation-independent case where an authored-translucent source feeds a
  later multi-input composite or merge. The last class reproduces without a
  blend and is therefore a generic graph refusal, not a mode exception.
  Blend precision is architecture-neutral rather than backend-default. The
  nine modes on the pinned backend's low-precision path use exact byte-domain
  divide-by-255 arithmetic because that backend approximates the division on
  x86; its seven high-precision modes remain native. Hosted x86 then isolated
  one further one-code-value split in the final translucent sRGB restore. A
  blend-scoped exact restore closes it and clears on later color-space
  conversion. ARM and x86 reproduce every committed blend cell exactly with
  no tolerance.
  Morphology carries case-sensitive `erode | dilate` with initial `erode` and
  the complete measured SVG `<number-optional-number>` radius grammar with
  initial zero. One radius supplies both axes; two remain independent;
  negative members clamp independently to zero. Invalid number-list, unit,
  percentage, CSS-math, custom-property, CSS-wide, comment, overflow, and
  extra-member spellings use the initial zero rather than a parsed prefix.
  Blink-ordered source-number normalization is retained before the checked
  two-radius fact.
  Device-space radii round at positive half-pixel boundaries after mapping,
  independently per axis, and the pinned operation caps each axis at 256
  pixels. Both color spaces and SourceAlpha preserve their distinct
  premultiplied channel extrema. Zero radius remains a graph operation because
  its primitive region still hard-crops the input.
  Exact committed evidence covers generated and source inputs, previous and
  named results, object-box and user-space primitive units, non-uniform
  `viewBox`, paths, strokes, rounded rectangles, `<use>`, fractional axis maps,
  exact quarter turns, target opacity/clip/mask, and neighboring filter
  operations. The filter output region never preclips source pixels needed by
  a spatial kernel; each graph node applies the hard output crop.
  Three stable morphology patrols guard the measured remainder. General
  rotations and shears cross the mapped-kernel/source-raster boundary;
  paint-server source images cross a source-layer precision boundary, even at
  zero radius; and an active filled `<circle>` or `<ellipse>` crosses the
  retained fill-only ellipse coverage boundary. Rounded rectangles, curved
  paths, and circle/path strokes stay admitted. That last patrol leaves
  gridaco/nothing#88 separate and unchanged.
  Convolve matrix carries one checked rectangular kernel of at most 256 finite
  coefficients. One/two-member `order` values normalize by truncation toward
  zero; the matrix must contain exactly the resulting product. The authored
  coefficients reverse once to state SVG convolution rather than correlation.
  Missing or malformed order selects 3×3, while non-positive order, a missing
  or wrong-count matrix, and an over-bound kernel produce Chromium's
  transparent result instead of an unfiltered fallback.
  `divisor` carries one signed number. Missing, exactly empty, and signed-zero
  values use the ordered binary32 kernel sum, with a zero sum becoming one; a
  present nonempty malformed value uses one. `bias` carries one signed number
  with initial zero. `targetX`/`targetY` use signed integer text, default to
  half their respective order axes, reset malformed authored text to zero, and
  produce transparent when a valid value lies outside the kernel. The complete
  case-sensitive `duplicate | wrap | none` edge vocabulary and `false | true`
  alpha-preservation vocabulary are admitted. Chromium ignores
  `kernelUnitLength` on convolution; that drop is baked, while its shared
  lighting applicability remains open.
  Both filter color spaces, SourceGraphic/SourceAlpha/previous/named/generated
  inputs, result reuse, hard regions, primitive units, paths, strokes, groups,
  safe axis mappings, exact quarter turns, target opacity/clip/mask,
  blur/morphology ordering, `<use>`, and `viewBox` are exact. General affine target
  mappings, source-dependent paint servers, and divisors whose reciprocal is
  not finite refuse by three stable convolution names before paint. Fractional
  axis maps, reflections, exact quarter turns, generated inputs, and every
  accepted kernel-size strategy through 256 stay admitted. Representative
  fallback branches are celled; the wider invalid-spelling matrix is measured,
  not all separately celled.
  Diffuse lighting consumes one input's alpha as a height field and carries one
  already-resolved distant, point, or spot light. The first recognized direct
  light child wins; non-light children are ignored, nested lights do not
  participate, and no light produces transparent black. The operation's own
  output is opaque across its primitive subregion, including opaque black at
  zero diffuse constant. Missing input follows the established first/previous
  graph fallback; SourceGraphic and SourceAlpha therefore give the same
  illumination for the same source coverage.
  `surfaceScale` and `diffuseConstant` carry signed SVG numbers with initial
  one; an exactly empty attribute becomes zero, malformed nonempty text uses
  the initial, surface height keeps its sign, and a negative diffuse constant
  clamps to zero. Distant angles are signed and periodic. Point and spot
  coordinates default independently to zero. Under object-box primitive units,
  their x/y coordinates use the target axes and z uses the normalized diagonal.
  Spot exponent defaults to one and clamps to 1–128. A missing, zero, or
  out-of-range cone angle uses the measured 90-degree behavior; an in-range
  negative angle equals its positive magnitude.
  Direct `lighting-color` carries initial white, admitted sRGB forms,
  `currentColor`, reset/invalid fallback, non-inheritance, and ignored authored
  alpha. The light channels adapt to the selected filter color space; missing
  interpolation is linearRGB and explicit sRGB differs. CSS lighting color,
  explicit inheritance, `var()`, and wider color functions refuse by stable
  name. General affine target mappings and diffuse output used as the
  foreground of `feComposite` `in`/`atop` against a source-derived second input
  have two further precision patrols. Axis maps, reflection, exact quarter
  turns, other composite operators, blend/merge, neighboring one-input spatial
  operations, regions, `<use>`, `viewBox`, stroke and gradient alpha, and target
  opacity/clip/mask are Chromium-baked exact. Chromium ignores sampled valid
  and invalid `kernelUnitLength` spellings; two cells carry the diffuse drop,
  while that shared row and `feSpecularLighting` remain open.
  Turbulence carries both procedural formulas: the case-sensitive values
  `turbulence` and `fractalNoise`, one/two-axis non-negative `baseFrequency`,
  integer `numOctaves` capped at nine, signed `seed`, and the case-sensitive
  values `stitch` and `noStitch`. Missing and invalid fields take their measured
  initials; either negative frequency resets the pair, zero octaves still
  reaches the selected formula, and a negative octave count produces a
  transparent image. The generated source is bounded by its primitive region
  and participates in its declared filter color space. A user-space filter may
  therefore paint a target
  whose isolated source is fully transparent; object-box filter units on the
  same zero-area target produce no region and paint nothing.
  Displacement map carries two ordered image inputs, a finite signed `scale`,
  and independent case-sensitive `R | G | B | A` selectors with initial alpha.
  Selection reads non-premultiplied map channels after conversion into the
  primitive's filter color space. Under object-box primitive units Chromium's
  one native displacement scalar uses the target width for both axes. Exact
  evidence covers transparent and partial-alpha maps, source/source-alpha and
  generated maps, procedural maps, hard regions, stroke/path sources, opacity,
  safe axis mappings, exact quarter turns, `<use>`, and non-uniform `viewBox`.
  General rotations and shears for either primitive, and geometric clipping
  around displacement, refuse by three stable precision names. Axis maps,
  fractional translation and scale, reflections, and exact quarter turns stay
  admitted. The wider shared graph, region, color, resource, and animation
  surfaces retain their own boundaries.
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
  Internal Porter-Duff composition and the final layer restore distinguish
  exact byte-domain generated sources from floating source-image coverage. A
  one-input merge has no internal composition stage, so the final restore is
  where its generated-only rounding is enforced. Native sRGB shadow descendants
  add the independently measured exact-restore case described above. Active
  sRGB morphology adds another measured instance of the same CPU-family
  boundary: the first full-workspace hosted-x86 run differed in nine cells and
  1,633 pixels, all by one channel level, at the final filter-layer restore.
  Its scoped exact byte-domain restore leaves zero radius and later color-space
  conversion on their prior paths. Procedural sources carry floating
  provenance through blend arithmetic. Direct sRGB procedural
  `difference`/`exclusion` retains the pinned backend's byte-domain product
  rounding; linear or component-transferred inputs use floating products. An
  sRGB blend result materializes before a later blend; linear output remains
  floating, and a later transfer promotes either route again. The final
  procedural restore quantizes only the composed output with explicit
  half-up byte rounding. Direct active sRGB morphology materializes procedural
  provenance; blend before morphology preserves it. sRGB displacement output
  takes the exact byte-domain restore. Empty generated-source scopes seed
  damage coverage from their transformed filter region, so procedural
  parameter edits damage pixels even without a source draw. Fifteen
  operation-order, color-transition, and blend-domain controls guard these
  distinctions. The first hosted-x86 run found 294 one-code-value pixels across
  eighteen sRGB displacement and four procedural cells. The displacement repair
  cleared all eighteen on the second run, which left 729 delta-1 pixels in four
  procedural cells. Scoped procedural arithmetic cleared the 726-pixel blend
  control on the third run, leaving three singleton direct-noise pixels. Pinned
  Skia source located them in an uninitialized runtime raster-pipeline dispatch:
  x86 stayed on baseline non-fused Perlin arithmetic while ARM used fused NEON.
  Initializing Skia before drawlist replay selects the fused AVX2 path on x86.
  The 700-cell baseline is byte-exact on ARM and hosted x86 without a
  tolerance. The forty-one-cell convolution rung keeps the complete 741-cell
  gate byte-exact on ARM and hosted x86 without a new tolerance. The
  seventy-one-cell diffuse-lighting rung keeps the complete 812-cell gate
  byte-exact without a new tolerance. All four hundred fifty-three
  Chromium-baked filter cells are exact.
  The filter estate contains 26 chassis/blur cells, 60 shadow-graph, 28 native
  drop-shadow, 27 color-matrix, 34 component-transfer, 38 blend, 37 morphology,
  91 turbulence/displacement, 41 convolution-rung, and 71 diffuse-lighting
  cells. The complete primitive corpus contains 1,126 Chromium-baked cells plus
  16 sampled frames; the text estate contains fifteen exact text pixel cells and
  eight exact-number artifact-geometry witnesses (six Allerta and two
  Bungee), and the named refusal register has 230 rows. `feFlood`, `feComposite`,
  `feMerge`, `feMergeNode`, `feDropShadow`, `feColorMatrix`,
  `feComponentTransfer`, `feBlend`, `feMorphology`, `feConvolveMatrix`,
  `feDiffuseLighting`, `feDistantLight`, `fePointLight`, `feSpotLight`,
  `feTurbulence`, `feDisplacementMap`,
  `feFuncR`, `feFuncG`, `feFuncB`, `feFuncA`, `k1`–`k4`, `amplitude`,
  `exponent`, `intercept`,
  `slope`, `tableValues`, shared stop/function `offset`, blend-only `mode`,
  `baseFrequency`, `numOctaves`,
  `seed`, `stitchTiles`, displacement `scale`, `xChannelSelector`, and
  `yChannelSelector`, `bias`, `divisor`, `edgeMode`, `kernelMatrix`,
  convolution `order`, `preserveAlpha`, `targetX`, `targetY`, `azimuth`,
  `diffuseConstant`, `elevation`, `limitingConeAngle`, `pointsAtX`,
  `pointsAtY`, `pointsAtZ`, and `surfaceScale` close;
  `feOffset`, `feGaussianBlur`, `feSpecularLighting`, `<filter>`,
  `filter`, `color-interpolation-filters`, `in`, `in2`, `operator`, `result`,
  `radius`, `kernelUnitLength`, `lighting-color`, `specularExponent`, `x`, `y`,
  `z`, `dx`, `dy`, `stdDeviation`, `flood-color`, and `flood-opacity` remain
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
  The direct `vector-effect` presentation attribute carries the complete
  standard-track grammar. `none` retains ordinary local-space stroking;
  `non-scaling-stroke` maps the centerline into frame space before applying
  the nominal width as one circular pen. Chromium 149 drops the at-risk
  `non-scaling-size`, `non-rotation`, `fixed-position`, combination,
  `viewport`, and `screen` members to `none`, and those drops are baked.
  The source-neutral resolved stroke records only `Local | Frame`
  construction space. Frame construction reaches every admitted geometry,
  dash and `pathLength` route, affine and viewport mapping, paint server,
  clip/mask/filter/opacity composition, pattern/mask/marker source program,
  `<use>` target, and marker-unit branch. `markerUnits="strokeWidth"` follows
  Blink's RMS affine scale; `userSpaceOnUse` remains independent. Exact-linear
  identity mappings retain the established local f32 execution order, while
  a zero, non-finite, underflowed, or overflowed f32 determinant suppresses
  both the frame-space stroke and its stroke-width marker. Forty-seven exact
  Chromium cells guard the slice. Direct `var()`, `env()` fallback, typed
  `attr()`, and experimental `if()` forms remain stable function-named
  refusals, and authored CSS `vector-effect` remains a separate stable property
  refusal because the pinned Stylo build has no corresponding longhand; no
  second matcher is added. Graphics elements outside this admitted slice keep
  their own element-level refusals.
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
  Element `opacity` is consumed in every spelling within the admitted coverage
  profiles (presentation attribute, style attribute, stylesheet — one
  <alpha-value> grammar, clamped exactly as Chromium clamps). A single
  un-transformed, un-folded opacity pass uses the
  group-scope rung's measured fold: the element factor joins the colour and
  fill/stroke opacity product before its one quantization. A valid gradient
  instead keeps its intrinsic paint opacity and carries the element factor
  separately across the resolved contract; the painter materializes the
  paint's own alpha first, then multiplies the factor in float without a
  second 8-bit quantization. A one-stop server remains a two-identical-stop
  constant gradient so it retains that raster route. An invalid URL fallback
  remains an ordinary solid and takes the solid fold.
  Fold eligibility follows Chromium's recorded paint/effect structure rather
  than visible pixels. A selected transparent colour, zero paint opacity,
  valid stopless or transparent server, and zero-ink dash remain opacity
  passes; `none`, an invalid URL without fallback, and zero stroke width do
  not. A non-identity opacity stage on non-pruned paintless geometry blocks an
  enclosing fold even at zero, while zero-extent, empty, hidden, and
  display-pruned geometry does not. Empty geometry nodes remain available to
  context-paint and box consumers without becoming opacity subjects.
  A `<line>`'s selected fill is likewise a structural pass even though its
  interior paints nothing. Default fill plus stroke therefore composites
  through the isolated post-coverage layer in direct, inline/stylesheet CSS,
  inherited-container, `<a>`, and `<use>` routes; explicit `fill="none"`
  leaves the single stroke fold. Both opacity checklist rows are closed and
  the former line-coverage refusal has graduated.
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
  `gradientUnits`, `spreadMethod`, stops from attributes (`offset` is one
  strict number or percentage, defaults invalid text to zero, clamps to the
  unit interval and then to the running maximum, and is never sorted;
  equal-offset hard stops render crisp; source numbers use Blink's ordered
  evaluation before percentage normalization), `href`/`xlink:href`
  template chains (stops all-or-nothing from the first owner; geometry
  never crosses gradient types; a cycle kills only the edge), and
  `gradientTransform` as the transform property's presentation attribute on
  gradient elements — an author `transform` declaration beats it, the plain
  `transform` attribute is inert there, and the value applies about the raw
  origin of gradient space, all Chromium-measured. Ramps interpolate
  unpremultiplied sRGB and dither exactly as Chromium's rasterizer does.
  For `<linearGradient>`, `x1`/`y1`/`x2`/`y2` default independently to
  `0%`/`0%`/`100%`/`0%` and accept signed finite numbers, percentages, and
  case-insensitive `px` spellings. Object-box values use fractions or
  percentages of the painted box; user-space values use the viewport axes.
  Template chains inherit each missing coordinate independently. Three
  Chromium-baked cells carry those branches, including transformed stroke and
  degenerate-vector controls. The same source-decimal, comment, range, and
  wider-value refusals as `<line>` keep the four shared attribute rows open.
  The authored fallback fires only on an _invalid_ reference (a missing id
  or a non-gradient target); the measured correct nothings — zero stops
  (fallback unfired), a self-cycle, a non-invertible gradient transform, an
  object-bounding-box gradient on zero-area geometry — paint nothing. A
  zero or negative radial radius and linear endpoints inside the backend's
  degenerate threshold resolve to the tile-specific measured solid: the last
  stop under `pad`, or the ramp's integral average under `reflect`/`repeat`.
  On zero-area line geometry, object-box paint is nothing before one-stop or
  degenerate classification; user-space one-stop and concentric degenerate
  ramps retain those source-neutral results before the live-gradient boundary.
  Two companion Chromium cells carry the complete unit/ordering split.
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
  attribute refuses the paint), a live user-space ramp on zero-area geometry,
  unit families beyond `px` in gradient geometry,
  a percentage in a gradient's computed transform (Chromium resolves it
  against mismatched spaces), and an external reference.
  `<pattern>` paint servers are consumed in a bounded static profile. A
  same-document `url(#…)` resolves first-id-wins for each consuming fill or
  stroke. Plain `href` beats `xlink:href`; template chains inherit each missing
  attribute independently and take children all-or-nothing from the first
  owner that has them, while cycles remove only their cyclic edge. An invalid
  server activates the authored paint fallback. A valid pattern with no
  painting children instead paints transparent and leaves that fallback inert.
  Source compilation is transactional, so an unsupported child refuses the
  complete affected client instead of leaking a partial tile.
  `patternUnits` defaults to `objectBoundingBox` and
  `patternContentUnits` defaults to `userSpaceOnUse`; both complete
  `userSpaceOnUse | objectBoundingBox` grammars and invalid fallback are
  admitted. Tile numbers and percentages resolve per client against the
  correct independent axes. A pattern `viewBox` uses the complete
  `preserveAspectRatio` resolver and supersedes `patternContentUnits`.
  `patternTransform` is the transform property's presentation hint: CSS beats
  it and a plain `transform` attribute is inert. Translation, axis scale,
  reflection, and exact quarter turns are admitted. The repeating source can
  contain admitted rectangles, gradients, `<use>`, masks, filters, and a
  pattern nested alone; a filter may wrap that sole nested-pattern draw.
  Pattern paint covers admitted rect, ellipse, and path fills and strokes, and
  target opacity, clip, mask, and filter scopes retain their established
  order. A pattern may be selected through all four destination
  fill/stroke × context-fill/context-stroke crossings, including CSS ingress,
  recursive `<use>` owners, object-box coordinates, transforms, opacity,
  fallback/nothing semantics, masks, and filters. It may also supply a
  source-derived filter's input when the filtered source is one direct
  sharp-cornered rectangle with exactly one pattern-painted fill or simple
  stroke channel. That filter profile includes transparent and alpha-bearing
  tiles, gradients, masks, strokes, and nested patterns. One hundred
  twenty-four Chromium-baked cells cover the core and composition profiles,
  including independent object-box clients and one inline-HTML SVG entry; all
  are exact without a new tolerance.
  What refuses by stable name: an external template dependency; a non-`px`
  unit, CSS math, `var()`, CSS-wide tile value, or CSS comments around an
  otherwise valid tile length; a source child outside the admitted element
  slice; curved source coverage; isolated multi-draw source
  opacity or a geometric source clip; another source draw mixed with a nested
  pattern; a fractional final tile extent; and a final tile map carrying a
  general rotation or shear. A source-derived filter over curved, rounded,
  multi-draw, or wider pattern-painted target geometry has its own
  filtered-pattern coverage refusal. The context-aware source classifier also
  keeps eventual gradients behind the existing color-matrix, component-
  transfer, convolution, morphology, native-shadow, and translucent-source
  precision names instead of letting a context keyword hide them. Those
  picture-shader and source-layer boundaries are measured, not guessed
  omissions. Before its patrol, the valid
  comment spelling silently selected fallback in both admissions and changed
  all 2,304 target pixels at maximum delta 202. A derived template whose
  author stylesheet may contribute `transform:none` also refuses because the
  pinned computed value loses the provenance needed to decide inheritance.
  A CSS percentage transform on the pattern resource refuses until its
  reference box can be carried without invention. Chromium resolves inline
  `translate(50%, 0px)` against the 64-unit viewport; the former tile-width
  basis changed 1,008 target pixels at maximum delta 205 (measured, not
  celled). A ninth distinct nested pattern likewise refuses before its source
  walk begins; the resolved contract admits at most eight programs, while
  cycles retain their separate active-id refusal.
  Finite tile coordinates beyond Chromium's Web used-length clamp refuse too:
  the former raw route changed 768–2,112 pixels at maximum delta 205 for the
  signed huge, adjacent, and beyond-binary32 witnesses instead of selecting
  Chromium's clamped repetition phase (measured, not celled).
  The source-number alias probe found no discriminating pattern raster at
  64×64: Chromium's adjacent controls were pixel-identical (measured, not
  celled), but the raw decoder still cannot prove Blink's used value, so the
  conservative provenance patrol remains. The `<pattern>` and
  `patternTransform` checklist rows therefore stay open; only `patternUnits`
  and `patternContentUnits` close.
  The exact-time `<animate attributeName="x">` slice may target the same direct
  sharp-cornered rectangle while it carries admitted pattern paint, including
  a filter inside the tile and a source-derived filter around the target. Six
  committed Chromium frames cover Base and exact samples at 0, 0.25, 1, 2,
  and 3 seconds; strict and best-effort pixels are exact and identical.
  Pattern source geometry and tile geometry remain static. A fractional X or
  Y tile phase refuses at the measured picture-shader boundary: six
  discriminating witnesses differ from Chromium by 96–576 pixels at maximum
  channel delta 1, including both object-box client-origin axes. Animation
  endpoints use SVG-number syntax and a one-way
  Chromium-normalization patrol; trailing-dot and Unicode-whitespace forms
  formerly admitted only by Rust, and one valid midpoint-adjacent decimal
  formerly selected the wrong binary32 neighbour. Three focused refusal rows
  guard those classes. The `<pattern>` and `<animate>` rows remain open.
  `<marker>` is consumed in one bounded, same-document static profile.
  Direct inherited `marker-start`, `marker-mid`, and `marker-end` references
  apply to `<line>`, `<path>`, `<polyline>`, and `<polygon>`; `none`, malformed
  hints, missing or wrong-kind targets, first-id lookup, quoted/escaped URL
  forms, and `<use>` clients follow Chromium's measured selection behavior.
  CSS marker properties remain authored-ingress refusals because the pinned
  Stylo build has no marker longhands. Chromium's bare `marker` attribute and
  marker attributes on rect/circle/ellipse are inert and celled as such.
  Placement retains authored vertex topology independently of raster path
  decomposition: start and end belong to the whole path, later subpath moves
  are mids, close back-patches the start tangent and contributes its duplicate
  vertex, and one authored cubic, quadratic, or arc contributes one marker
  edge. Move-only, one-point, valid-prefix, zero-length, degenerate-tangent,
  angle-wrap, and exact-opposite cases are baked.
  The marker viewport admits numeric and percentage `markerWidth`,
  `markerHeight`, `refX`, and `refY`, plus `px`; both `markerUnits` branches;
  explicit `orient` angles in every listed angle unit, `auto`, and
  `auto-start-reverse`; and `viewBox` through the complete admitted
  `preserveAspectRatio` mapper. Length percentages use the outer SVG viewport
  axes even when the marker has a `viewBox`, as Chromium measures. Invalid or
  non-positive viewport geometry produces the measured nothing. The marker
  viewport is a hard clip, not ordinary anti-aliased clip-path coverage.
  A source may contain the admitted solid/context-solid shapes and groups with
  transforms. Client transforms, computed stroke-width scaling even when the
  stroke paint is none, opacity, clip, mask, filter, root mapping, and context
  paint retain their measured order. Every instance lowers to ordinary
  source-neutral frame nodes and hard geometric clip scopes; no marker,
  resource id, URL, or authored topology crosses `rframe`.
  Marker opacity composition begins only when an authored vertex kind selects
  an actual marker resource. Missing ids, wrong-kind targets, and a valid
  marker property for a vertex kind the client does not have are
  Chromium-equivalent to `none`; a selected real marker still chooses the
  combined span when its viewport is zero or its valid source is empty. Five
  exact cells guard this distinction.
  What refuses by stable marker name: external URLs; author CSS around a used
  marker; CSS math, custom properties, wider units, and CSS-wide resource
  values; resource-root overflow, effects, opacity, transform, or an inherited
  rendering declaration; source
  dynamics, paint servers, effects, nested markers, text, and `<use>`; and the
  checked position/source/client fan-out limits. Source compilation is one
  transaction: strict refuses, while best effort removes the complete client
  span rather than leaking its ordinary fill or stroke. Ninety exact
  Chromium cells carry M1. The element, three presentation attributes, four
  CSS property rows, and six marker-resource attribute rows remain open for
  the wider source, cascade, grammar, dynamics, and external-I/O surface.
  `<text>` is consumed (the text rung), and its font environment is the
  host's: text resolves only against fonts declared with
  `--font FAMILY=PATH@sha256:HEX[;weight=N][;style=normal|italic][;stretch=POINT]`
  (repeatable), whose bytes are **verified against the declared digest before
  any pixel exists**. Descriptor fields are optional and order-independent;
  omitted fields mean the complete `400`/`normal`/`100%` tuple. Weight is an
  integer from 1 through 1000, style is `normal` or `italic`, and stretch is
  `ultra-condensed`, `extra-condensed`, `condensed`, `semi-condensed`, `normal`,
  `semi-expanded`, `expanded`, `extra-expanded`, or `ultra-expanded`; their
  canonical `50%`, `62.5%`, `75%`, `87.5%`, `100%`, `112.5%`, `125%`, `150%`,
  and `200%` spellings are aliases. Shell callers quote a declaration carrying
  `;`. Unknown, repeated, malformed, or wider fields refuse at the host
  boundary. A family name is not a font identity, and a mismatch refuses the
  render rather than producing a silently different one. A `<text>` run
  carries its complete computed ordered family list and static face
  request into the declared environment. Unavailable names fall through under
  the measured Unicode 17 BMP simple-fold comparison. The first available
  named face supplies vertical metrics independently of glyph coverage. Each
  complete admitted cluster then walks the list from the start. Every reached
  named family narrows by stretch, then style, then weight under Chromium's
  measured directional static search, and its one winner shapes the complete
  cluster. A result containing any missing glyph advances to the next family;
  another descriptor in the same family is never searched as fallback. A
  winning-tuple tie and a reached generic are terminal typed boundaries. If
  the face that can shape the cluster would require synthetic weight or style,
  resolution refuses before accepting it. Exhausting the declared list
  reports the first missing source scalar by name. There is no system
  fallback, ambient face, manifest-order guess, or backend synthetic posture,
  and therefore no machine-local pixel anywhere on this path. Inside that
  environment one text source resolves once through
  [the text oracle](../../docs/wg/feat-paragraph/text-layout.md) at its v8
  profile — one shaping-style run of printable ASCII plus exactly the 53 canonical
  precomposed Latin-1 letters in U+00C0–00C5, U+00C7–00CF, U+00D1–00D6,
  U+00D9–00DD, U+00E0–00E5, U+00E7–00EF, U+00F1–00F6, U+00F9–00FD, and
  U+00FF, plus U+0301 or U+030B only as the sole mark immediately after one
  ASCII Latin letter; horizontal and left-to-right; direct clusters remain one
  source scalar and one glyph, while an admitted base-plus-mark cluster is two
  source scalars and either one composed glyph or two glyphs with a
  zero-advance displaced mark; no wrapping or synthesis. Fallback is by
  complete cluster across declared families only: a base never separates from
  its mark, canonical composition is tested by shaping rather than scalar cmap
  membership, and adjacent clusters selecting the same exact resource shape as
  one face run. The artifact retains the primary metrics face, every used face
  and contiguous face run, and each glyph's face identity until all outlines
  lower to source-neutral paths.
  Direct character data may be partitioned by flat direct `<tspan>` children
  that preserve the
  same resolved face, size, direction, opacity, and effect profile and select
  only an opaque solid fill. A child may also carry complete unitless
  `x`/`y`/`dx`/`dy` number lists whose consumed values stay finite and
  integral. Whitespace collapses once across the complete subtree. Each
  consumed child `x` or `y` starts an explicit shaping and anchor chunk;
  `dx`/`dy` shift typographic characters without splitting shaping, including
  carrying a middle combining-scalar shift to the next character. Source-run
  tags assign each complete cluster to the paint run owning its first scalar.
  The glyph outlines then lower by paint run to the contract's ordinary path
  facts, so no font, run, or chunk identity crosses into the resolved frame.
  Parent `x`, `y`, and a direct `text-anchor` attribute
  (`start`/`middle`/`end`) place the source and anchor every chunk; inherited
  ancestor spellings refuse until anchor has one computed route.
  `font-family`, `font-size`, `font-weight`, `font-style`, and `font-stretch`
  come from the one cascade, where an author rule beats the presentation
  attribute exactly as Chromium measured. Family lists retain quoted-name
  versus generic classification, CSS escapes, inheritance, and source order;
  face descriptors retain presentation, inline, stylesheet, and inherited
  selection through their bounded static profile and one shared nearest-face
  policy. Environment resource order never replaces request or tuple identity.
  Until the source environment is carried more widely, `font-size` admits
  only a direct finite non-negative unitless presentation value or `px` value
  that survives the pinned Stylo
  quantizer unchanged and is an integer multiple of five; `inherit`/`unset`
  may select the same proved ancestor profile. Viewport-, container-,
  font-metric-, percentage-, math-, variable-, escaped-, shorthand-, and
  wider-unit routes refuse by stable text-specific names instead of trusting
  a computed value whose source basis or precision is gone. Geometry is
  admitted only inside
  the ratified [numeric domain](../../docs/wg/consolidation/text-oracle.md)
  — integer position, a `font-size` that is an integer multiple of 5, an
  integer anchor-resolved start, and a complete final CTM with identity linear
  part plus integer device translation after root mapping, ancestors, and
  `<use>` placement — because that is where every rasterizer's per-pixel
  coverage is 0 or 1 and the byte-exact gate holds. Authored transforms that
  cancel exactly are admitted; any non-identity or fractional final mapping
  refuses. Chromium
  snaps everything else by a rasterizer-internal rule, and this refuses by
  name instead of codifying it. Real-font Rung B adds an independent
  pre-raster geometry boundary. Chromium exposes each horizontal SVG
  character cell after enclosing its start/end on a 1/64 CSS-pixel grid and
  exposes vertical cells through integral fixed ascent/descent metrics. A run
  is admitted only when every cluster's logical boundary already lies on
  that grid and both metrics are already integral; the compiler never changes
  the artifact to imitate the query. A pinned Allerta `Hxi` witness at 5120px
  matches Chromium's total advance, per-character starts/ends/extents,
  baseline, and anchor placement exactly. Glyph ids, clusters, units-per-em,
  and outline bounds are checked separately against the same pinned font bytes
  because browser text-query APIs do not expose those facts. A second Allerta
  `ff` witness carries default pair positioning exactly: Chromium and the
  artifact agree on advances 2330/2355 and total advance 4685. Separately,
  the pinned font bytes grade glyph ids 70/70 and outline bounds, while the
  oracle records every cluster's source UTF-8 range, source UTF-16 range, and
  glyph range explicitly. A third Allerta witness contains every admitted
  precomposed Latin letter between ASCII sentinels. Chromium reports 55
  addressable characters and total advance 171785; the artifact records 108
  UTF-8 bytes versus 55 UTF-16 units, including `À` at bytes `1..3` and UTF-16
  units `1..2`, and `Z` at bytes `107..108` and units `54..55`. A merged
  cluster refuses by stable `shaping cluster mapping` name before lowering;
  PT Serif `fi` is the committed negative witness, because Chromium exposes
  two addressable character segments while shaping produces one ligature
  glyph. A fourth Allerta witness admits decomposed `Ae` + U+0301 + `Z`
  without normalization: the two source units share Chromium's 3320-unit
  base cluster while shaping composes one glyph. Two Bungee witnesses retain
  a separate zero-advance mark glyph with x offset -369 and, for U+030B, local
  y offset -7. Every cluster records UTF-8, UTF-16, scalar, and glyph ranges;
  every glyph records pen position, displacement, advance, cluster owner, and
  resolved face identity.
  A fifth Allerta witness puts the second `f` of the exact 2330/2355 pair in a
  differently painted `<tspan>` and still totals 4685, proving that a paint
  boundary does not create a second shaping call. A sixth resolves
  `fffe` + U+0301 + `Z` in two explicit chunks: the absolute boundary changes
  the first `f` advance to 2355, the second chunk retains the 2330/2355 pair,
  and Chromium and the projection agree on total 13585 and positioned starts
  5000, 8330, 10685, and 15005. The last start proves that a relative shift on
  the combining scalar carries to the next character. The fifteen-cell exact
  suite separately makes per-run paint, list placement, per-chunk anchoring,
  transforms, `<use>`, ten ordered-family-selection branches, and sixteen exact
  face-descriptor branches plus sixteen static-nearest branches byte-exact.
  The descriptor cell covers numeric and
  keyword weight, relative weight, normal/italic style, all three cascade
  ingresses, inheritance, keyword/percentage stretch aliases, and one complete
  tuple. Its actual strict CLI render has zero differing pixels and zero
  maximum channel delta against the committed Chromium oracle. The nearest
  cell covers both stretch directions, all three weight search regions, axis
  order, real-style fallback, and the reached-family boundary through the
  same cascade ingresses; all sixteen branches select pinned Ahem and are
  byte-exact to Chromium. The T6 cell adds presentation, inline, inherited,
  canonical-composition, precomposed/decomposed fallback, paint-run,
  positioned-chunk, transform, and `<use>` branches against one hash-pinned
  Ahem derivative. It is exact to an explicit-face Chromium construction and
  both actual admissions; replacing fallback with the primary face changes
  700 pixels at maximum channel delta 247 (measured control, not another
  cell).
  Malformed or unlisted combining sequences and a mark missing from the
  selected face refuse at the text node in both admissions. The 1000px
  control misses both query grids: strict refuses by stable
  `Chromium SVG text query` name, while best effort skips and declares that
  text node. Real-font
  pixels are checked only for engine determinism and admission identity; no
  Chromium raster claim or tolerance is made, and no text fact crosses
  `rframe`. What refuses by name: the CSS spelling of
  `text-anchor` (Chromium consumes it from the cascade, the pinned Stylo
  build has no such longhand — a silent drop before the rung), inherited
  `text-anchor` presentation attributes, generic-family mapping, ambiguous
  winning same-family tuples, fractional font weights, arbitrary stretch
  percentages, oblique angles, synthetic weight/style, exhausted named-family
  lists, nested `<tspan>`
  content, child shaping-style changes, non-opaque/wider child paint and child/parent
  effects, any other element child, parent position lists, child position
  units/percentages/functions, malformed or fractional child lists,
  positioned non-canonical whitespace, an absolute reset inside a combining
  cluster, `rotate`, `textLength`, font shorthands and unconsumed
  font variants, decorations, letter and word spacing, baselines, writing mode
  and direction, stroke on text, a colour or
  bitmap face, a supporting face requiring synthetic weight or style, a cluster
  outside the direct-or-one-mark cardinality, malformed
  combining placement, a cluster missing from every declared family, generic
  or system fallback, and any character outside the v8 repertoire. The inline-HTML
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
