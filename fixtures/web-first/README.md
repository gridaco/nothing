# fixtures/web-first

Fixtures for the Web-first engine track's first architecture prototype
(`crates/rframe`, `crates/websem`; see
[docs/wg/consolidation/web-first.md](../../docs/wg/consolidation/web-first.md)).

One concept: **an inline-SVG descendant is painted from a value that crosses
the HTML→SVG boundary through one browser-grade cascade.**

| File | Role |
| --- | --- |
| `html-inline-svg-currentcolor-rect.html` | HTML whose `<style> .mark { color:#16a34a }` cascades to a `<rect class="mark">` inside inline `<svg>`. |
| `svg-currentcolor-rect.svg` | The equivalent standalone SVG (carries `color` via an inline `style`). Renders identically. |
| `chromium/svg-currentcolor-rect.png` | Committed Chromium oracle (64×64, all `#16a34a`), baked at deviceScaleFactor=1. |
| `oracle-bake.json` | Bake provenance (browser version + sha256 of source, oracle, and bake script). |
| `bake_chromium.ts` | Reproduces the oracle. Run: `pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/bake_chromium.ts"`. |

Probe expectation: every pixel of the oracle is `#16a34a` (opaque). The
`rframe` render of both fixtures must match it exactly (see
`crates/websem/tests/reftest_oracle.rs` and `equivalence.rs`).

Render either fixture from the command line through the prototype pipeline
(`websem` compile → `rframe::Frame` → PNG) — a thin host; unsupported
constructs fail explicitly:

```sh
cargo run -p websem --example render -- \
  fixtures/web-first/svg-currentcolor-rect.svg /tmp/out.png
```

## Why `color` + `fill="currentColor"`, not `fill:#16a34a` directly

The workspace compiles Stylo with the **servo** engine, which omits the
gecko-only SVG paint longhands (`fill`, `stroke`, …) — they are absent from
`ComputedValues`, so a `fill` rule cannot be read from the cascade. The
cascade *does* carry `color` (a servo longhand), so the fixture demonstrates
the cross-boundary cascade with `color` and lets SVG's own `currentColor`
resolve the paint. Making the shared Stylo cascade model SVG paint properties
(a gecko-engine or custom-property question) is a filed next-step for the
Web-first track, not something this slice papers over.
