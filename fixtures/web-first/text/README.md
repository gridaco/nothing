# The text cell suite

Chromium-baked cells for the `<text>` slice on the SVG engine of record
(`websem → rframe → n0`), gated byte-exact by
[`crates/websem/tests/svg_text.rs`](../../../crates/websem/tests/svg_text.rs).
The method these cells enforce is the ratified
[text-oracle brief](../../../docs/wg/consolidation/text-oracle.md); this file
states only how the suite is shaped.

Text lives in its own suite rather than the primitive root, per the brief's
corpus-growth law: the root is closed to text, probes are never committed,
and the tracked set is a gate — one cell per admitted construct — not
coverage.

## The font is the environment, not the document

A fixture here is **the document**. The font is a **declared input of the
render**, exactly as it is for the engine: `websem` receives it as a
`textlayout::Environment` of exact bytes the host has verified, and the baker
declares the same identity to Chromium by injecting an `@font-face` whose
source is the pinned font, inline, at capture time.

So the committed `.svg` carries no font bytes. That is deliberate — six
copies of a pinned font is a corpus, and the fonts directory
[grows per identity only](../fonts/README.md) — and it keeps the two sides
symmetric: both the engine and the oracle render the same document under a
declared environment neither reads ambiently. Opening a fixture directly in a
browser therefore shows fallback glyphs, not Ahem's boxes.

## Bake posture

Inherited from the primitive suite's baker, plus two text-specific facts,
both recorded verbatim in `oracle-bake.json`:

| Fact | Value |
| --- | --- |
| viewport | the fixture's declared size, as the initial viewport |
| deviceScaleFactor | 1 |
| JavaScript | disabled |
| network | every route aborted |
| font declaration | the pinned face injected as an inline `@font-face`, awaited ready before capture |
| raster posture | `-webkit-font-smoothing: none` on each text element, carried by the fixture |
| comparison | full RGBA, byte-exact — no tolerance is admissible here |
| repeats | two captures per cell, byte-equal required |

`-webkit-font-smoothing: none` is bake posture, not engine semantics: it
suppresses the one rasterizer behavior (macOS smoothing dilation) that paints
outside a glyph's true coverage. Inside the admitted numeric domain every
raster policy agrees, so the declaration changes no engine-visible meaning —
the measurement behind that claim is in the brief.

## Re-baking

```sh
pnpm -C packages/grida-reftest exec tsx fixtures/web-first/text/bake_chromium.ts
```

A committed oracle is verification-only: a differing re-capture fails instead
of blessing a new baseline.
