# Unsupported Web-first fixtures

Purpose-built inputs that must fail explicitly rather than render an
approximation. They are not part of `primitives.json` because they have no
pixel output.

**`crates/websem/tests/unsupported_corpus.rs` is the gate.** It reads this
directory with `read_dir` and holds it against a declared table, so a file added
here without a row fails, and a row without a file fails too. For each entry it
asserts the departure names its construct in **both** admissions: the
document-level ones (the viewport grammar and root sizing) refuse under
best-effort as well, and the attributable ones are declared by name at a
structural path. What that gate defends is the invariant, stated over a whole
directory — *nothing here renders silently*. Individual constructs are pinned a
second time, from inline sources, by the contract law that owns each rung.

The scannable, generated view of this register (beside the baked cells) is
[../STATUS.md](../STATUS.md), freshness-gated by
`crates/websem/tests/capability_status.rs`.

| File | Required result |
| --- | --- |
| `svg-context-paint-fallback-extension.svg` | Refuse Stylo's non-standard context-paint fallback extension by name. SVG2 permits a fallback only after a URL, and Chromium drops `context-fill red` as an invalid paint (measured); the pinned parser accepts it. The standard-track grammar remains the bar under gridaco/nothing#77, so this registered over-refusal cannot hold the four `fill`/`stroke` rows open. Attribute, inline-style, and stylesheet ingresses are guarded. |
| `svg-viewbox-invalid-token.svg` | Reject the malformed `viewBox`; do not discard the bad token. |
| `svg-viewbox-repeated-comma.svg` | Reject a repeated comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-trailing-comma.svg` | Reject a trailing comma in the `viewBox` number list; do not filter empty separators. |
| `svg-preserve-aspect-ratio-invalid-align.svg` | Reject the unknown alignment keyword; Chromium silently renders the default mapping, this engine refuses by name. |
| `svg-preserve-aspect-ratio-case-folded.svg` | Reject the case-folded alignment keyword — the SVG grammar is case-sensitive. |
| `svg-preserve-aspect-ratio-defer.svg` | Reject the SVG 1.1 `defer` prefix as malformed grammar: SVG2 dropped it and Chromium treats the whole value as unparseable. |
| `svg-width-percentage.svg` | Reject percentage root sizing by name — its basis is the host window itself, a cell the element-capture baker cannot express, so it graduates only with a host-level oracle. (Shape-geometry and stroke-width percentages graduated with the percentages rung.) |
| `svg-path-malformed-d.svg` | Refuse the whole path, naming the byte offset. Chromium renders the valid prefix (SVG2 §9.3.9); this slice does not ship an unbaked partial geometry — a deliberate, declared divergence. |
| `svg-path-no-leading-moveto.svg` | Refuse path data that does not begin with a moveto. Chromium's valid prefix is empty here, so the refusal costs no pixels. |
| `svg-path-trailing-dot-number.svg` | Refuse `10.` in path data. SVG's BNF allows a trailing dot; Blink requires a digit after it and renders nothing — the browser is the authority. |
| `svg-path-css-d-property.svg` | Declare a stylesheet's `d: path(…)`: Chromium honors it in place of the attribute, and the pinned Stylo build drops the declaration entirely. |
| `svg-path-pathlength.svg` | Refuse by name — **load-bearing now**. Chromium scales dash intervals through `pathLength` on path, rect, circle, and ellipse, and scales dash offset on path (measured); the zero-calibration frame contract carries no such fact. The same patrol covers all seven admitted geometry elements so a dash cycle never paints in the wrong distance space. |
| `svg-path-marker-end.svg` | Refuse by name — **load-bearing**. Nothing else reads a marker property: the property *is* the paint trigger, so this refusal is what keeps Chromium's arrowhead from becoming a silent hole. |
| `svg-stroke-dasharray-escape.svg` | Refuse by name — a CSS escape can hide a basis-less unit from the authored-text patrol (`1\76 w` tokenizes as `1vw`), so the presentation attribute, style attribute, and stylesheet spellings all refuse rather than silently use the pinned device basis. |
| `svg-stroke-dasharray-font-basis.svg` | Refuse by name — `em`/`rem` are admitted only while their cascaded `font-size` basis is trustworthy. A basis-less unit, `var()`, or escape in that basis would resolve a different cycle from Chromium; the exact poison classes and all ancestor/sheet ingresses share the stroke-width rung's guarded patrol. |
| `svg-stroke-dasharray-sheet-unit.svg` | Refuse by name — the stylesheet spelling of a dash interval in a unit whose basis this build lacks. Viewport-, container-, font-metric-, and root-font-metric-relative unit classes are all guarded; the units' own checklist rows carry those gaps, following the stroke-width precedent. |
| `svg-stroke-dasharray-var.svg` | Refuse by name — `var()` hides the interval's authored provenance. Chromium substitutes it in both presentation and CSS spellings (measured); this patrol cannot follow the indirection, so even an honest `8px` substitution over-refuses until a resolver can. |
| `svg-stroke-dashoffset.svg` | Refuse both spellings by name — Chromium shifts the cycle for positive/negative unitless and px values plus percentages (measured), while this rung's frame amendment is explicitly zero-phase. The separate `stroke-dashoffset` checklist rows carry the remainder. |
| `svg-stroke-vector-effect.svg` · `svg-stroke-paint-order.svg` | Refuse by name. Both were provably inert while strokes refused; consuming strokes made them load-bearing, which is exactly the trap the earlier patrol was kept for. |
| `svg-stroke-sheet-unit-width.svg` | Refuse by name — the third spelling of the basis-less unit. The attribute patrol walks ancestors, so it never saw a `<style>` rule: this document rendered a 16-unit stroke silently while `stroke-width="2ex"` and `style="stroke-width:2ex"` both refused. A sheet is not attributable to one element without selector matching, so it refuses document-level (strict) and declares against the sheet (best-effort). |
| `svg-stroke-width-calc-mixed.svg` | Refuse by name — a `calc()` stroke-width mixing lengths and percentages. Chromium resolves the sum with the percentage against the normalized diagonal (measured: `calc(10% + 0.8px)` ≡ an authored `7.2`); the resolve here reads pure lengths and pure percentages only, so the mixed computed value refuses rather than dropping either term. Unlike the basis-less units, this refusal sits at resolve, where the element is known — so every spelling, the `<style>` sheet included, declares at the element's own path. |
| `svg-stroke-width-var.svg` | Refuse by name — `var()` hides the unit from the authored-text patrol. This document painted a silent 12.8 (`--w: 1vw` substituted against the pinned 1280px device) where Chromium paints 0.64 (measured), and Chromium substitutes `var()` in every spelling, the presentation attribute included (measured). Which declaration feeds a substitution is a resolver question, so every `var(` in stroke-width-bearing text refuses — a benign `--w: 8px` included, over-refusal by design. |
| `svg-stroke-width-font-basis.svg` | Refuse by name — the admitted `em`/`rem` basis is the *cascaded font-size*, which makes font-size an ingress for every basis this build lacks. This document painted a silent ~25.6 (`font-size: 2vw` computed against the pinned device, times `1em`) where Chromium paints 1.28 (measured). A font-relative stroke-width therefore requires every authored font-size in scope — attribute, style attribute (`font` shorthand included), ancestor, or sheet — to be free of basis-less units, `var()`, and escapes; `svg-stroke-width-em-font-size` is the admitted half, Chromium-baked at an authored `8px`. |
| `svg-stroke-width-percentage-overflow.svg` | Refuse an extreme pure percentage or percentage-only `calc()` by name. Chromium accepts the standard-track grammar, but overflow in its used-value multiplication has a join- and cap-dependent result: under a discriminating transform, a butt-capped round or bevel join paints, while the default miter and round/square-cap variants do not. It is not the dasharray property's solid fallback and cannot be normalized to one width fact here. The fixed huge-length clamp is separately celled. Because this magnitude class has no independent checklist row, it reopens both `stroke-width` twins under the gridaco/nothing#81 split precedent. |
| `svg-smil-set-load-active.svg` | A `<set>` on a consumed attribute of an admitted rect. SMIL defaults `begin` to offset `0s`, so Chromium paints the overridden fill at load — the target's authored state never honestly renders. Strict refuses at construction; best-effort skips the target and declares it at the target's stable path. Before this row landed, a Base render painted the authored fill with exit 0 and zero declarations in both admissions — the silent wrong pixel recorded open in the D-N register at the paths rung. |
| `svg-smil-animate-transform.svg` | An `<animateTransform>` on a `<g>`: the override targets the container, so the whole subtree is the declared hole (best-effort) and strict refuses at construction. |
| `svg-smil-retarget-href.svg` | A `<set href="#id">` retarget. href resolves by id, which this slice does not own, so the override cannot be attributed to one skippable element — document-level, both admissions refuse, exactly as `<script>` does. |
| `svg-points-odd-coordinate.svg` | An odd trailing coordinate in a `points` list. Chromium renders the valid pair prefix; this slice refuses the whole element by name instead — the paths rung's declared divergence, restated for the points grammar. |
| `svg-text-undeclared-font.svg` | Text is admitted only against the rung's one declared font identity. An undeclared family refuses at the text element instead of silently substituting different glyph metrics. |
| `svg-text-tspan.svg` | `<tspan>` retains its own named text-residue refusal: the admitted text run is one direct text-node sequence, so nested positioning and styling cannot be silently flattened into it. |
| `svg-use-stylesheet.svg` | Author CSS and `<use>` refuse together: the measured shadow boundary scopes selector matching to the cloned subtree alone — no ancestor outside it participates, not even through descendant combinators — and the one flattened tree cannot express that scoping. The shadow-matching rung earns it. (`<use>` and `<defs>` themselves graduated with the use/defs rung; the presentation-attribute slice is baked.) |
| `svg-use-external.svg` | An external reference refuses by name: the engine is declared resource-free, and Chromium with a network would render the target — silence would be a wrong pixel. |
| `svg-use-authored-children.svg` | Authored element children of a `<use>` refuse by name: Chromium renders the shadow content in their place, and this slice refuses rather than models that replacement. |
| `svg-use-symbol.svg` | A `<symbol>` target declares at the clone's own path as an unsupported element: instantiated, a symbol renders like a nested `<svg>` viewport — a scope the flat frame cannot hold, same as `svg-nested-svg`. |
| `svg-image.svg` | `<image>` refuses as an element: the glyphless product is declared resource-free, and websem is forbidden I/O by its architecture lock. The data:-URI sub-slice enters with the resource environment, not before. |
| `svg-nested-svg.svg` | A nested `<svg>` establishes a new viewport and clip — a scope the flat frame cannot hold. Declared until the group-scope rung. |
| `svg-pattern-paint-server.svg` | `<pattern>` declares twice: the element by name, and the referencing `fill="url(#…)"` as an unsupported fill value. Never approximated to a solid. (`svg-gradient-paint-server` graduated with the gradient rung — gradients are baked cells now.) |
| `svg-gradient-focal.svg` | A focal radial (`fx`/`fy` off the center, `fr > 0` alike) refuses by name: the shared radial paint leaf is concentric, and Chromium's focal cone — unclamped, leaving pixels unpainted (measured) — is inexpressible in it until its owner amendment. |
| `svg-gradient-linearrgb.svg` | `color-interpolation="linearRGB"` is honored by Chromium (measured: the linear-light midpoint, not the sRGB one) and refuses by name — one backend ramp cannot interpolate in a second space. |
| `svg-gradient-stop-css.svg` | A stylesheet declaring `stop-color` is a document-level declaration: the pinned cascade has no such longhand (Gecko-only at the Stylo pin), so the sheet is named and the gradient renders with its attribute colors — a declared divergence, since Chromium honors the sheet. |
| `svg-gradient-stop-style-attr.svg` | `stop-color` in a stop's own style attribute wins in Chromium (measured); the cascade cannot represent it, so the referencing paint refuses by name. |
| `svg-gradient-unit-basis.svg` | Font-relative units in gradient geometry (`x2="4em"`) refuse by name — the basis chain this slice does not consume; numbers, `px`, and percentages are the admitted grammar. |
| `svg-clip-path.svg` · `svg-mask.svg` · `svg-filter.svg` | The resource scopes: each element declares by name, and the referencing attribute never silently drops — the flat frame carries exactly one clip (the viewport), and the kernel owns no arbitrary-path clip, mask composite, or filter graph yet. |
| `svg-switch.svg` · `svg-foreign-object.svg` | Container-shaped elements real exports emit. `<switch>` needs SVG2 conditional-processing selection; `<foreignObject>` refuses by name on this path — there is no HTML box producer to recurse into. (`<a>` graduated: it is a container like `<g>`.) |
| `svg-element-opacity-gradient.svg` | Element `opacity` graduated with the group-scope rung, except over a lone gradient draw: the paint carries one 8-bit-quantized alpha (the fill-opacity pin), and Chromium composites the element opacity *after* that quantization (measured: one code value apart across most of the ramp) — expressing both needs a second paint-alpha factor. Declared by name until that amendment. |
| `svg-root-opacity.svg` | `opacity` on the outermost `<svg>` composites the whole canvas: the captured SVG-local raster carries the multiplied alpha (measured, identically in both entries), which this engine's opaque raster surface cannot express. Refuses in both admissions until a translucent-surface entry, like the root's `transform`. |
| `svg-display-contents.svg` | `display: contents` paints children in the parent's place — the flattened walk cannot express that without silently dropping the element's transform, so it stays a named refusal while `display: none` and `visibility` (the visibility rung) render the correct nothing. |
| `svg-css-transform-origin.svg` · `svg-css-transform-box.svg` | The transform's two knobs, still refused by name after the property graduated: `transform-origin` computes but stays unread (the slice implements the measured SVG used origin `0 0` only), and `transform-box` does not exist in the pinned servo-mode build at all — each would move every pixel the transform touches. |
| `svg-css-transform-3d.svg` | The beyond-2D function family (`translate3d`, `matrix3d`, `rotateX`, `perspective`, …) refuses naming the function: Chromium composes these on SVG content (measured), so a silent drop would move nothing where Chromium moves, and flattening them is a future rung's measured work. |
| `svg-css-individual-rotate.svg` | The individual transform properties (`rotate`, `translate`, `scale`) stay refused: Chromium composes them *with* `transform`, so consuming one without the others would compose a different matrix. (`transform` itself graduated with the transform rung — both spellings now resolve through the one cascade.) |

(The former `svg-viewbox-unequal-default.svg` and
`svg-preserve-aspect-ratio-explicit.svg` graduated to root primitives when
the viewport rung landed their mappings. The former
`svg-stroke-context-paint.svg` graduated into the 22-cell context-paint matrix;
the parser-extension row above replaced it without changing that rung's total.
The former `svg-stroke-dasharray-cycle-overflow.svg` then graduated into the
two used-range cells when the browser probe established that pure fixed dash
lengths clamp before a cycle is formed. The new percentage-overflow width row
replaces it numerically, so the register remains at 56 rows.)
