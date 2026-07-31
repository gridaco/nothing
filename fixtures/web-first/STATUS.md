# Web-first capability status

<!-- GENERATED FILE - do not edit. Regenerate:
UPDATE_CAPABILITY_STATUS=1 cargo test -p websem --test capability_status -->

A generated, freshness-gated view of the two corpora that track the
SVG engine of record's capability: what renders (the Chromium-baked
cells) and what departs by name (the refusal register). Every refusal
line is the compiler's actual departure, recompiled by the gate
(`crates/websem/tests/capability_status.rs`), so this file cannot
drift from behavior. The prose statement of record is
[crates/n0_cli/README.md](../../crates/n0_cli/README.md); the rung
history is the
[D-N register](../../docs/wg/consolidation/svg-engine-of-record.md);
the per-fixture rationale is
[unsupported/README.md](./unsupported/README.md).

Not a conformance claim: no score is computed or implied (FLIP is
unratified), and the corpus enumerates constructs, not the SVG
surface.

## Chromium-baked cells (139)

Each renders byte-exact against its committed Chromium oracle
(six curved cells carry the one declared AA tolerance — see
[README.md](./README.md)).

- `html-inline-svg-currentcolor-rect` (html-inline-svg)
- `html-webpage-mockup` (html-inline-svg)
- `svg-currentcolor-rect` (standalone-svg)
- `svg-fill-named-rect` (standalone-svg)
- `svg-fill-inherited-rect` (standalone-svg)
- `svg-fill-invalid-initial-rect` (standalone-svg)
- `svg-fill-none-rect` (standalone-svg)
- `svg-style-element-fill-rect` (standalone-svg)
- `svg-style-attribute-fill-rect` (standalone-svg)
- `svg-viewbox-uniform-offset-rect` (standalone-svg)
- `svg-viewbox-unequal-default` (standalone-svg)
- `svg-preserve-aspect-ratio-explicit` (standalone-svg)
- `svg-viewbox-only-sizing-rect` (standalone-svg)
- `svg-sizing-auto-rect` (standalone-svg)
- `svg-preserve-aspect-ratio-none-stretch` (standalone-svg)
- `svg-preserve-aspect-ratio-slice-clip` (standalone-svg)
- `svg-preserve-aspect-ratio-align-max-meet` (standalone-svg)
- `svg-circle-fill` (standalone-svg)
- `svg-circle-viewbox-scaled` (standalone-svg)
- `svg-circle-defaults-clip` (standalone-svg)
- `svg-circle-zero-r` (standalone-svg)
- `svg-ellipse-fill` (standalone-svg)
- `svg-ellipse-auto-rx` (standalone-svg)
- `svg-ellipse-negative-rx-auto` (standalone-svg)
- `svg-group-transform-translate` (standalone-svg)
- `svg-group-nested-transforms` (standalone-svg)
- `svg-shape-transform-matrix` (standalone-svg)
- `svg-group-paint-order` (standalone-svg)
- `svg-group-inherited-fill` (standalone-svg)
- `svg-non-rendering-elements` (standalone-svg)
- `svg-group-rotate-quarter` (standalone-svg)
- `svg-group-rotate-diagonal` (standalone-svg)
- `svg-path-polygon-fill` (standalone-svg)
- `svg-path-unclosed-fill` (standalone-svg)
- `svg-path-relative-commands` (standalone-svg)
- `svg-path-hv-shorthand` (standalone-svg)
- `svg-path-cubic-fill` (standalone-svg)
- `svg-path-smooth-cubic` (standalone-svg)
- `svg-path-quadratic` (standalone-svg)
- `svg-path-fill-rule-nonzero` (standalone-svg)
- `svg-path-fill-rule-evenodd` (standalone-svg)
- `svg-path-fill-rule-inherited` (standalone-svg)
- `svg-path-two-subpaths` (standalone-svg)
- `svg-path-draws-nothing` (standalone-svg)
- `svg-path-in-scaled-group` (standalone-svg)
- `svg-path-closed-move-only-contour` (standalone-svg)
- `svg-stroke-cap-butt` (standalone-svg)
- `svg-stroke-cap-round` (standalone-svg)
- `svg-stroke-cap-square` (standalone-svg)
- `svg-stroke-cap-closed-butt` (standalone-svg)
- `svg-stroke-cap-closed-round` (standalone-svg)
- `svg-stroke-cap-closed-square` (standalone-svg)
- `svg-stroke-cap-circle-round` (standalone-svg)
- `svg-stroke-cap-circle-square` (standalone-svg)
- `svg-stroke-cap-ellipse-round` (standalone-svg)
- `svg-stroke-cap-ellipse-square` (standalone-svg)
- `svg-stroke-circle` (standalone-svg)
- `svg-stroke-default-width` (standalone-svg)
- `svg-stroke-ellipse` (standalone-svg)
- `svg-stroke-inherited` (standalone-svg)
- `svg-stroke-invalid-width` (standalone-svg)
- `svg-stroke-join-bevel` (standalone-svg)
- `svg-stroke-join-miter` (standalone-svg)
- `svg-stroke-join-round` (standalone-svg)
- `svg-stroke-length-units` (standalone-svg)
- `svg-stroke-line-fill-never-paints` (standalone-svg)
- `svg-stroke-line` (standalone-svg)
- `svg-stroke-miter-limit` (standalone-svg)
- `svg-stroke-nonuniform-scale` (standalone-svg)
- `svg-stroke-over-fill` (standalone-svg)
- `svg-stroke-path-closed` (standalone-svg)
- `svg-stroke-path-open` (standalone-svg)
- `svg-stroke-rect-centred` (standalone-svg)
- `svg-stroke-scaled-group` (standalone-svg)
- `svg-stroke-zero-extent-rect` (standalone-svg)
- `svg-stroke-zero-length-dot` (standalone-svg)
- `svg-stroke-zero-width` (standalone-svg)
- `svg-points-trailing-comma` (standalone-svg)
- `svg-polygon-fill` (standalone-svg)
- `svg-polygon-fill-rule-evenodd` (standalone-svg)
- `svg-polygon-single-point-square-cap` (standalone-svg)
- `svg-polygon-stroke-closed` (standalone-svg)
- `svg-polyline-fill-implicit-close` (standalone-svg)
- `svg-polyline-single-point-square-cap` (standalone-svg)
- `svg-polyline-stroke-open` (standalone-svg)
- `svg-display-none-group` (standalone-svg)
- `svg-display-none-root` (standalone-svg)
- `svg-display-none-shape` (standalone-svg)
- `svg-visibility-collapse-shape` (standalone-svg)
- `svg-visibility-hidden-shape` (standalone-svg)
- `svg-visibility-rule-beats-attribute` (standalone-svg)
- `svg-visibility-unhide` (standalone-svg)
- `svg-fill-opacity-inherited` (standalone-svg)
- `svg-fill-opacity-overlap` (standalone-svg)
- `svg-fill-opacity-percentage` (standalone-svg)
- `svg-fill-opacity-times-alpha` (standalone-svg)
- `svg-stroke-opacity-join` (standalone-svg)
- `svg-stroke-opacity-over-fill` (standalone-svg)
- `svg-translucent-fill-rgba` (standalone-svg)
- `svg-percent-circle-diagonal` (standalone-svg)
- `svg-percent-ellipse` (standalone-svg)
- `svg-percent-line` (standalone-svg)
- `svg-percent-rect-in-viewbox` (standalone-svg)
- `svg-percent-rect-root-units` (standalone-svg)
- `svg-percent-stroke-width` (standalone-svg)
- `svg-anchor-container` (standalone-svg)
- `svg-css-transform-property` (standalone-svg)
- `svg-css-transform-beats-attribute` (standalone-svg)
- `svg-css-transform-sheet-beats-attribute` (standalone-svg)
- `svg-css-transform-none-restores` (standalone-svg)
- `svg-css-transform-invalid-falls-back` (standalone-svg)
- `svg-css-transform-compound` (standalone-svg)
- `svg-css-transform-percent` (standalone-svg)
- `svg-css-transform-group` (standalone-svg)
- `svg-css-transform-rotate-quadrant` (standalone-svg)
- `svg-css-transform-webkit` (standalone-svg)
- `svg-transform-runtogether` (standalone-svg)
- `svg-transform-no-separator` (standalone-svg)
- `svg-transform-malformed-drops` (standalone-svg)
- `svg-use` (standalone-svg)
- `svg-use-defs-rect` (standalone-svg)
- `svg-use-xy` (standalone-svg)
- `svg-use-transform-xy` (standalone-svg)
- `svg-use-inherit-fill` (standalone-svg)
- `svg-use-own-fill-wins` (standalone-svg)
- `svg-use-xlink-href` (standalone-svg)
- `svg-use-group` (standalone-svg)
- `svg-use-chain` (standalone-svg)
- `svg-use-cycle-nothing` (standalone-svg)
- `svg-use-rendered-twice` (standalone-svg)
- `svg-use-missing-nothing` (standalone-svg)
- `svg-use-forward-ref` (standalone-svg)
- `svg-use-display-none-target` (standalone-svg)
- `svg-use-wh-inert` (standalone-svg)
- `svg-use-duplicate-id-first` (standalone-svg)
- `svg-use-context-differs` (standalone-svg)
- `svg-use-currentcolor` (standalone-svg)
- `svg-use-ancestor-circle` (standalone-svg)
- `svg-use-href-beats-xlink` (standalone-svg)

## The refusal register (43)

What the slice refuses, by name, in the compiler's own words —
**both refuse** is a document-level contract; **declared** renders
the rest and names the hole. A rung that admits a construct moves
its row into the cells above.

- `svg-clip-path` — declared: skipped svg/clipPath[1]: unsupported element <clipPath>; skipped svg/rect[2]: unsupported rendering attribute clip-path on <rect> (not yet consumed)
- `svg-css-individual-rotate` — declared: skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares rotate, which this cascade does not represent
- `svg-css-transform-3d` — declared: skipped svg/rect[2]: unsupported computed style: transform on <rect> uses translate3d(), which is outside the 2D affine function set this slice consumes
- `svg-css-transform-box` — declared: skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares transform-box, which this cascade does not represent
- `svg-css-transform-origin` — declared: skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares transform-origin, which this cascade does not represent
- `svg-display-contents` — declared: skipped svg/g[1]: unsupported computed style: display: contents is not yet consumed
- `svg-element-opacity` — declared: skipped svg/rect[2]: unsupported rendering attribute opacity on <rect> (not yet consumed)
- `svg-filter` — declared: skipped svg/filter[1]: unsupported element <filter>; skipped svg/rect[2]: unsupported rendering attribute filter on <rect> (not yet consumed)
- `svg-foreign-object` — declared: skipped svg/foreignObject[1]: unsupported element <foreignObject>
- `svg-gradient-paint-server` — declared: skipped svg/linearGradient[1]: unsupported element <linearGradient>; skipped svg/rect[2]: unsupported fill value "url(about:blank#g)"
- `svg-image` — declared: skipped svg/image[1]: unsupported element <image>
- `svg-mask` — declared: skipped svg/mask[1]: unsupported element <mask>; skipped svg/rect[2]: unsupported rendering attribute mask on <rect> (not yet consumed)
- `svg-nested-svg` — declared: skipped svg/svg[1]: unsupported element <svg>
- `svg-path-arc` — declared: skipped svg/path[1]: path command A on <path> is not yet consumed (an elliptical arc reaches Chromium's rasterizer as conics, which this slice does not emit)
- `svg-path-css-d-property` — declared: declaration ignored at svg/style[1]: a stylesheet declares d, which this cascade does not represent; elements it matches render without it
- `svg-path-malformed-d` — declared: skipped svg/path[1]: path data on <path> is invalid at byte 29 (near "qqq")
- `svg-path-marker-end` — declared: skipped svg/path[1]: unsupported rendering attribute marker-end on <path> (not yet consumed)
- `svg-path-no-leading-moveto` — declared: skipped svg/path[1]: path data on <path> is invalid at byte 0 (near "L10 10 L54 54 Z")
- `svg-path-pathlength` — declared: skipped svg/path[1]: unsupported rendering attribute pathLength on <path> (not yet consumed)
- `svg-path-trailing-dot-number` — declared: skipped svg/path[1]: path data on <path> is invalid at byte 1 (near "10. 10 L54 10 L54 54 Z")
- `svg-pattern-paint-server` — declared: skipped svg/pattern[1]: unsupported element <pattern>; skipped svg/rect[2]: unsupported fill value "url(about:blank#p)"
- `svg-points-odd-coordinate` — declared: skipped svg/polygon[1]: points on <polygon> is invalid at byte 17 (near "")
- `svg-preserve-aspect-ratio-case-folded` — **both refuse**: preserveAspectRatio "xmidymid meet" is invalid
- `svg-preserve-aspect-ratio-defer` — **both refuse**: preserveAspectRatio "defer xMidYMid meet" is invalid
- `svg-preserve-aspect-ratio-invalid-align` — **both refuse**: preserveAspectRatio "xMidYMiddle meet" is invalid
- `svg-rect-rounded` — declared: skipped svg/rect[2]: unsupported rendering attribute rx on <rect> (not yet consumed)
- `svg-smil-animate-transform` — declared: skipped svg/g[1]: its authored state is overridden at document load by the unsupported animation at svg/g[1]/animateTransform[1]: animation element <animateTransform> is outside the rect-x proving slice
- `svg-smil-retarget-href` — **both refuse**: SVG animation at svg/rect[2]/set[1] is unsupported: animation element <set> is outside the rect-x proving slice; it carries href, so its target cannot be attributed to one element without id resolution; it is active at document load, so the authored state it overrides cannot render as the Base view
- `svg-smil-set-load-active` — declared: skipped svg/rect[2]: its authored state is overridden at document load by the unsupported animation at svg/rect[2]/set[1]: animation element <set> is outside the rect-x proving slice
- `svg-stroke-dasharray` — declared: skipped svg/path[1]: unsupported rendering attribute stroke-dasharray on <path> (not yet consumed)
- `svg-stroke-paint-order` — declared: skipped svg/rect[1]: unsupported rendering attribute paint-order on <rect> (not yet consumed)
- `svg-stroke-sheet-unit-width` — declared: declaration ignored at svg/style[1]: a stylesheet declares a stroke-width in ex, which needs a basis this cascade does not have; elements it matches render at the wrong width
- `svg-stroke-vector-effect` — declared: skipped svg/g[1]/rect[1]: unsupported rendering attribute vector-effect on <rect> (not yet consumed)
- `svg-switch` — declared: skipped svg/switch[1]: unsupported element <switch>
- `svg-text` — declared: skipped svg/text[1]: unsupported element <text>
- `svg-use-authored-children` — declared: skipped svg/use[1]: unsupported <use>: it has authored element children, which Chromium replaces with the shadow content
- `svg-use-external` — declared: skipped svg/use[1]: unsupported <use>: its reference is not a same-document fragment, and external resources are not resolved
- `svg-use-stylesheet` — declared: skipped svg/use[1]: unsupported <use>: the document carries author CSS, and shadow-scoped selector matching is not yet consumed (selectors must match inside the cloned subtree alone — measured)
- `svg-use-symbol` — declared: skipped svg/symbol[1]: unsupported element <symbol>; skipped svg/use[1]/symbol[1]: unsupported element <symbol>
- `svg-viewbox-invalid-token` — **both refuse**: viewBox "0 0 invalid 64 64" is invalid
- `svg-viewbox-repeated-comma` — **both refuse**: viewBox "0 0,,64 64" is invalid
- `svg-viewbox-trailing-comma` — **both refuse**: viewBox "0 0 64 64," is invalid
- `svg-width-percentage` — **both refuse**: unsupported SVG viewport sizing: percentage width="50%" on the root <svg> is not yet consumed
