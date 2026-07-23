# Unsupported Web-first fixtures

Purpose-built inputs that must fail explicitly rather than render an
approximation. They are not part of `primitives.json` because they have no
pixel output; `crates/websem/tests/viewport_contract.rs` locks the rejection.

| File | Required result |
| --- | --- |
| `svg-viewbox-invalid-token.svg` | Reject the malformed `viewBox`; do not discard the bad token. |
| `svg-viewbox-repeated-comma.svg` | Reject a repeated comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-trailing-comma.svg` | Reject a trailing comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-unequal-default.svg` | Reject until default `preserveAspectRatio` mapping is implemented; do not stretch. |
| `svg-preserve-aspect-ratio-explicit.svg` | Reject explicit `preserveAspectRatio` until its value grammar is implemented. |
