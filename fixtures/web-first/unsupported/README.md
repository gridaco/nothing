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

| File | Required result |
| --- | --- |
| `svg-viewbox-invalid-token.svg` | Reject the malformed `viewBox`; do not discard the bad token. |
| `svg-viewbox-repeated-comma.svg` | Reject a repeated comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-trailing-comma.svg` | Reject a trailing comma in the `viewBox` number list; do not filter empty separators. |
| `svg-preserve-aspect-ratio-invalid-align.svg` | Reject the unknown alignment keyword; Chromium silently renders the default mapping, this engine refuses by name. |
| `svg-preserve-aspect-ratio-case-folded.svg` | Reject the case-folded alignment keyword — the SVG grammar is case-sensitive. |
| `svg-preserve-aspect-ratio-defer.svg` | Reject the SVG 1.1 `defer` prefix as malformed grammar: SVG2 dropped it and Chromium treats the whole value as unparseable. |
| `svg-width-percentage.svg` | Reject percentage root sizing by name until the percentage basis chain is consumed; do not misreport valid grammar as a bad number. |
| `svg-path-arc.svg` | Refuse the elliptical arc **by name**, not as malformed: Chromium rasterizes an arc through the same rational conics as an `<ellipse>` (measured byte-identical over the rows they share), and the resolved contract carries no conic command yet. Following Blink's cubic *normalizer* instead differs from Chromium's own render of those same cubics by 77 pixels at up to a 170-per-channel delta. |
| `svg-path-malformed-d.svg` | Refuse the whole path, naming the byte offset. Chromium renders the valid prefix (SVG2 §9.3.9); this slice does not ship an unbaked partial geometry — a deliberate, declared divergence. |
| `svg-path-no-leading-moveto.svg` | Refuse path data that does not begin with a moveto. Chromium's valid prefix is empty here, so the refusal costs no pixels. |
| `svg-path-trailing-dot-number.svg` | Refuse `10.` in path data. SVG's BNF allows a trailing dot; Blink requires a digit after it and renders nothing — the browser is the authority. |
| `svg-path-css-d-property.svg` | Declare a stylesheet's `d: path(…)`: Chromium honors it in place of the attribute, and the pinned Stylo build drops the declaration entirely. |
| `svg-path-pathlength.svg` | Refuse by name — pure over-refusal. `pathLength` only scales what measures along the path (dashing, markers, text on a path), and every one of those already refuses; the patrol exists so the dashing rung cannot silently inherit a gap. |
| `svg-path-marker-end.svg` | Refuse by name — **load-bearing**. Nothing else reads a marker property: the property *is* the paint trigger, so this refusal is what keeps Chromium's arrowhead from becoming a silent hole. |
| `svg-stroke-opacity.svg` · `svg-stroke-dasharray.svg` | Refuse by name: the stroke's paint and geometry are consumed, its *compositing* and *dashing* are not. A dash array that would paint nothing (`none`, all-zero, invalid) is admitted instead — Chromium renders those solid. |
| `svg-stroke-percentage-width.svg` | Refuse by name: a percentage `stroke-width` resolves against the viewport's normalized diagonal (measured — `10%` of a 64x64 viewport is 6.4 units), a basis chain this slice does not consume. |
| `svg-stroke-vector-effect.svg` · `svg-stroke-paint-order.svg` | Refuse by name. Both were provably inert while strokes refused; consuming strokes made them load-bearing, which is exactly the trap the earlier patrol was kept for. |
| `svg-stroke-sheet-unit-width.svg` | Refuse by name — the third spelling of the basis-less unit. The attribute patrol walks ancestors, so it never saw a `<style>` rule: this document rendered a 16-unit stroke silently while `stroke-width="2ex"` and `style="stroke-width:2ex"` both refused. A sheet is not attributable to one element without selector matching, so it refuses document-level (strict) and declares against the sheet (best-effort). |
| `svg-smil-set-load-active.svg` | A `<set>` on a consumed attribute of an admitted rect. SMIL defaults `begin` to offset `0s`, so Chromium paints the overridden fill at load — the target's authored state never honestly renders. Strict refuses at construction; best-effort skips the target and declares it at the target's stable path. Before this row landed, a Base render painted the authored fill with exit 0 and zero declarations in both admissions — the silent wrong pixel recorded open in the D-N register at the paths rung. |
| `svg-smil-animate-transform.svg` | An `<animateTransform>` on a `<g>`: the override targets the container, so the whole subtree is the declared hole (best-effort) and strict refuses at construction. |
| `svg-smil-retarget-href.svg` | A `<set href="#id">` retarget. href resolves by id, which this slice does not own, so the override cannot be attributed to one skippable element — document-level, both admissions refuse, exactly as `<script>` does. |
| `svg-points-odd-coordinate.svg` | An odd trailing coordinate in a `points` list. Chromium renders the valid pair prefix; this slice refuses the whole element by name instead — the paths rung's declared divergence, restated for the points grammar. |
| `svg-text.svg` | `<text>` refuses as an element: the contract holds no glyph, and shaped text is a program (D-M's open text stage), not a rung. |
| `svg-use.svg` | `<use>` and `<defs>` each declare by name: the compiler owns no id-resolution facility yet. `<defs>` is deliberately not a non-rendering skip — its contents change what referencing elements paint. |
| `svg-image.svg` | `<image>` refuses as an element: the glyphless product is declared resource-free, and websem is forbidden I/O by its architecture lock. The data:-URI sub-slice enters with the resource environment, not before. |
| `svg-nested-svg.svg` | A nested `<svg>` establishes a new viewport and clip — a scope the flat frame cannot hold. Declared until the group-scope rung. |
| `svg-gradient-paint-server.svg` · `svg-pattern-paint-server.svg` | Paint servers declare twice: the server element by name, and the referencing `fill="url(#…)"` as an unsupported fill value. Neither is approximated to a solid. |
| `svg-clip-path.svg` · `svg-mask.svg` · `svg-filter.svg` | The resource scopes: each element declares by name, and the referencing attribute never silently drops — the flat frame carries exactly one clip (the viewport), and the kernel owns no arbitrary-path clip, mask composite, or filter graph yet. |
| `svg-anchor.svg` · `svg-switch.svg` · `svg-foreign-object.svg` | Container-shaped elements real exports emit. `<a>` is a plain flattening container awaiting its (cheap) rung; `<switch>` needs SVG2 conditional-processing selection; `<foreignObject>` refuses by name on this path — there is no HTML box producer to recurse into. |
| `svg-element-opacity.svg` · `svg-fill-opacity.svg` · `svg-translucent-fill.svg` | The translucency ladder, refused at three distinct doors: element `opacity` needs a compositing scope; `fill-opacity` and a translucent sRGB value fold into the paint's alpha but are not yet gated against Chromium's compositing cells. Declared, never approximated to opaque. |
| `svg-display-none.svg` · `svg-visibility-hidden.svg` | Both are pure absence of a visual fact, so they are over-refusals kept honest: the attribute patrol declares the element rather than consuming the property, until the rung that renders the (correct) nothing — including `visibility`'s descendant-un-hides inheritance. |
| `svg-rect-rounded.svg` | `rx` declares by name: the contract's rect carries no corner radius, and lowering the corner to cubics is the substitution the arc measurement showed to be wrong (Chromium rasterizes it as conics). |
| `svg-rect-percentage-geometry.svg` | A shape-geometry percentage declares by name: the axis bases and the normalized diagonal are not yet threaded through the walk. (Root percentage sizing is the `svg-width-percentage` row — that one refuses in both admissions.) |
| `svg-css-transform-property.svg` | The CSS `transform` property (style attribute or sheet) declares by name: only the SVG `transform` *attribute* grammar is admitted, and the pinned cascade build does not represent the CSS property family. |

(The former `svg-viewbox-unequal-default.svg` and
`svg-preserve-aspect-ratio-explicit.svg` graduated to root primitives when
the viewport rung landed their mappings.)
