# Unsupported Web-first fixtures

Purpose-built inputs that must fail explicitly rather than render an
approximation. They are not part of `primitives.json` because they have no
pixel output; `crates/websem/tests/viewport_contract.rs` locks the rejection.

| File | Required result |
| --- | --- |
| `svg-viewbox-invalid-token.svg` | Reject the malformed `viewBox`; do not discard the bad token. |
| `svg-viewbox-repeated-comma.svg` | Reject a repeated comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-trailing-comma.svg` | Reject a trailing comma in the `viewBox` number list; do not filter empty separators. |
| `svg-viewbox-unequal-default.svg` | **Admitted by the viewport rung** (default `xMidYMid meet` letterbox — `viewport_contract.rs` pins the mapping); moves to the root corpus with the rung's bake step. |
| `svg-preserve-aspect-ratio-explicit.svg` | **Admitted by the viewport rung** (`preserveAspectRatio` grammar landed); moves to the root corpus with the rung's bake step. |
