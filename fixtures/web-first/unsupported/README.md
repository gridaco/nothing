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
| `svg-viewbox-invalid-token.svg` | Reject the malformed `viewBox`; do not discard the bad token. |
| `svg-viewbox-repeated-comma.svg` | Reject a repeated comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-trailing-comma.svg` | Reject a trailing comma in the `viewBox` number list; do not filter empty separators. |
| `svg-preserve-aspect-ratio-invalid-align.svg` | Reject the unknown alignment keyword; Chromium silently renders the default mapping, this engine refuses by name. |
| `svg-preserve-aspect-ratio-case-folded.svg` | Reject the case-folded alignment keyword — the SVG grammar is case-sensitive. |
| `svg-preserve-aspect-ratio-defer.svg` | Reject the SVG 1.1 `defer` prefix as malformed grammar: SVG2 dropped it and Chromium treats the whole value as unparseable. |
| `svg-width-percentage.svg` | Reject percentage root sizing by name — its basis is the host window itself, a cell the element-capture baker cannot express, so it graduates only with a host-level oracle. (Shape-geometry and stroke-width percentages graduated with the percentages rung.) |
| `svg-path-arc.svg` | Refuse the elliptical arc **by name**, not as malformed: Chromium rasterizes an arc through the same rational conics as an `<ellipse>` (measured byte-identical over the rows they share), and the resolved contract carries no conic command yet. Following Blink's cubic *normalizer* instead differs from Chromium's own render of those same cubics by 77 pixels at up to a 170-per-channel delta. |
| `svg-path-malformed-d.svg` | Refuse the whole path, naming the byte offset. Chromium renders the valid prefix (SVG2 §9.3.9); this slice does not ship an unbaked partial geometry — a deliberate, declared divergence. |
| `svg-path-no-leading-moveto.svg` | Refuse path data that does not begin with a moveto. Chromium's valid prefix is empty here, so the refusal costs no pixels. |
| `svg-path-trailing-dot-number.svg` | Refuse `10.` in path data. SVG's BNF allows a trailing dot; Blink requires a digit after it and renders nothing — the browser is the authority. |
| `svg-path-css-d-property.svg` | Declare a stylesheet's `d: path(…)`: Chromium honors it in place of the attribute, and the pinned Stylo build drops the declaration entirely. |
| `svg-path-pathlength.svg` | Refuse by name — pure over-refusal. `pathLength` only scales what measures along the path (dashing, markers, text on a path), and every one of those already refuses; the patrol exists so the dashing rung cannot silently inherit a gap. |
| `svg-path-marker-end.svg` | Refuse by name — **load-bearing**. Nothing else reads a marker property: the property *is* the paint trigger, so this refusal is what keeps Chromium's arrowhead from becoming a silent hole. |
| `svg-stroke-dasharray.svg` | Refuse by name: the stroke's paint, geometry, and (since the translucency rung) compositing are consumed; its *dashing* is not. A dash array that would paint nothing (`none`, all-zero, invalid) is admitted instead — Chromium renders those solid. |
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
| `svg-switch.svg` · `svg-foreign-object.svg` | Container-shaped elements real exports emit. `<switch>` needs SVG2 conditional-processing selection; `<foreignObject>` refuses by name on this path — there is no HTML box producer to recurse into. (`<a>` graduated: it is a container like `<g>`.) |
| `svg-element-opacity.svg` | Element `opacity` needs a compositing scope: it composites fill and stroke through one layer, which no per-paint alpha fold can express without double-blending where they overlap. Declared until the group-scope rung; `fill-opacity`, `stroke-opacity`, and translucent sRGB graduated with the translucency rung. |
| `svg-display-contents.svg` | `display: contents` paints children in the parent's place — the flattened walk cannot express that without silently dropping the element's transform, so it stays a named refusal while `display: none` and `visibility` (the visibility rung) render the correct nothing. |
| `svg-rect-rounded.svg` | `rx` declares by name: the contract's rect carries no corner radius, and lowering the corner to cubics is the substitution the arc measurement showed to be wrong (Chromium rasterizes it as conics). |
| `svg-css-transform-origin.svg` · `svg-css-transform-box.svg` | The transform's two knobs, still refused by name after the property graduated: `transform-origin` computes but stays unread (the slice implements the measured SVG used origin `0 0` only), and `transform-box` does not exist in the pinned servo-mode build at all — each would move every pixel the transform touches. |
| `svg-css-transform-3d.svg` | The beyond-2D function family (`translate3d`, `matrix3d`, `rotateX`, `perspective`, …) refuses naming the function: Chromium composes these on SVG content (measured), so a silent drop would move nothing where Chromium moves, and flattening them is a future rung's measured work. |
| `svg-css-individual-rotate.svg` | The individual transform properties (`rotate`, `translate`, `scale`) stay refused: Chromium composes them *with* `transform`, so consuming one without the others would compose a different matrix. (`transform` itself graduated with the transform rung — both spellings now resolve through the one cascade.) |

(The former `svg-viewbox-unequal-default.svg` and
`svg-preserve-aspect-ratio-explicit.svg` graduated to root primitives when
the viewport rung landed their mappings.)
