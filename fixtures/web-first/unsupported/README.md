# Unsupported Web-first fixtures

Purpose-built inputs that must fail explicitly rather than render an
approximation. They are not part of `primitives.json` because they have no
pixel output; `crates/websem/tests/viewport_contract.rs` locks the rejection.

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

(The former `svg-viewbox-unequal-default.svg` and
`svg-preserve-aspect-ratio-explicit.svg` graduated to root primitives when
the viewport rung landed their mappings.)
