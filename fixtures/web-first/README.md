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
| `svg-viewbox-uniform-offset-rect.svg` | A non-zero-origin `viewBox` with uniform 2× viewport mapping; locks the proving shell's one supported non-identity viewport case. |
| `html-webpage-mockup.html` | A webpage-*design* (header / hero / cards / footer) expressed as 27 inline-SVG rects; the brand purple cascades from the HTML `<style>` via `fill="currentColor"`. Guarded by `crates/websem/tests/webpage_mockup.rs`. Not a real HTML/CSS layout — the slice renders solid-fill `<rect>` only. |
| `primitives.json` | Closed enumeration of every root HTML/SVG primitive, its grammar entry, dimensions, and Chromium oracle. Adding an unlisted root input fails the test gate. |
| `chromium/*.png` | One committed Chromium oracle per primitive, capturing the SVG-local raster at deviceScaleFactor=1. |
| `oracle-bake.json` | Bake provenance (browser version + sha256 of the suite, sources, oracles, and bake script). |
| `bake_chromium.ts` | Verifies existing oracle pixels and creates missing oracles; it never overwrites a differing baseline. Run: `pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/bake_chromium.ts"`. |
| `pages/` | The target-only real-world page corpus. It is not a runnable reftest gate yet; see [`pages/README.md`](./pages/README.md). |
| `unsupported/` | Inputs that deliberately have no pixels yet and must fail explicitly instead of being approximated; see [`unsupported/README.md`](./unsupported/README.md). |

Exact expectation: every primitive's full RGBA raster matches its Chromium
oracle with zero differing pixels. The gate also validates enumeration and
provenance and double-runs both raw raster and PNG encoding (see
`crates/websem/tests/reftest_oracle.rs`).

Render a primitive from the command line through the prototype pipeline
(`websem` compile → `rframe::Frame` → PNG), a thin host. The patrolled inputs
under `unsupported/` fail explicitly; arbitrary SVG outside the closed suite
is not yet capability coverage:

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
