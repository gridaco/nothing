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

(The former `svg-viewbox-unequal-default.svg` and
`svg-preserve-aspect-ratio-explicit.svg` graduated to root primitives when
the viewport rung landed their mappings.)
