# The text cell suite

Chromium-baked cells for the `<text>` slice on the SVG engine of record
(`websem → rframe → n0`), gated byte-exact by
[`crates/websem/tests/svg_text.rs`](../../../crates/websem/tests/svg_text.rs).
The method these cells enforce is the ratified
[text-oracle brief](../../../docs/wg/consolidation/text-oracle.md); this file
states only how the suite is shaped.

The suite currently has **nine** cells. Its manifest is a closed enumeration:
the Rust gate rejects an unlisted SVG, duplicate source row, stale suite or
baker hash, stale shared-capture hash, changed source, or changed oracle.

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

So the committed `.svg` carries no font bytes. That is deliberate — one font
copy per cell would be a second corpus, and the fonts directory
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

The baker imports the same hash-pinned
[`chromium_capture.ts`](../chromium_capture.ts) module as scratch probes and
the primitive baker. Text adds only its declared-font injection; it does not
carry a second browser launch, context, viewport, network, or screenshot
posture.

`-webkit-font-smoothing: none` is bake posture, not engine semantics: it
suppresses the one rasterizer behavior (macOS smoothing dilation) that paints
outside a glyph's true coverage. Inside the admitted numeric domain every
raster policy agrees, so the declaration changes no engine-visible meaning —
the measurement behind that claim is in the brief.

## T1 safety-fence evidence

The original six cells retain run geometry, advance/spacing, anchors,
whitespace collapse, and fill. Three exact cells add the safety boundary:

| Cell | Discriminating branch |
| --- | --- |
| `svg-text-font-size-cascade.svg` | Direct number and `px` presentation values, inline `font-size`, an author rule beating a different attribute, and exact inherited `px` all reach the one cascade and the same Ahem geometry. |
| `svg-text-final-integer-ctm.svg` | Integer translations contributed by the root `viewBox`, the text, a group, and a `<use>` instance remain inside the final-device domain. |
| `svg-text-final-ctm-cancel.svg` | Authored scale and fractional-translation pairs are judged after composition: exact cancellation back to the admitted final CTM remains renderable. |

A separate Chromium mutation matrix proves the cell branches. Changing each of
the five cascade sources moves 64, 64, 64, 132, and 64 pixels at maximum delta
255. Removing the root, text, group, or `<use>` integer translation moves 208,
52, 52, and 52 pixels. Removing scale cancellation moves 118 pixels. Removing
the half-pixel cancellation is Chromium-pixel-identical because native text
snaps that final fractional origin; that member is therefore an
**admission** witness, paired with the committed fractional-final-CTM refusal,
not a pixel-difference claim. A guard on authored transform syntax would reject
the admitted cell, while a missing final-device guard fails its negative pair.

The admitted size-source profile is deliberately narrow: a direct finite,
non-negative unitless presentation value or `px` value that survives the
pinned Stylo quantizer unchanged and is an integer multiple of five;
`inherit`/`unset` may transparently select such an ancestor value. The final
mapping must have an identity linear part and integer device translation.
Wider size syntax and mappings refuse by name in both admissions.

Before that fence, Chromium 149 probes found silent geometry/pixel differences
in both strict and best-effort rendering. An authored `5119px` was quantized to
`5120px` before the old local check and changed 149 pixels at maximum channel
delta 255. Viewport-, container-, and font-metric-relative sources changed the
glyph result: the `3.125vw` ingress family reached 1,591 wrong pixels at delta
255; stylesheet `vmin`, mixed `calc()`/`vw`, `2ex`, `2ch`, and `25cqw`
witnesses changed 391, 398, 624, 1,200, and 225 pixels respectively. Fractional
text/group/`<use>` translations changed 40 pixels at delta 128; a 1.1 scale,
45-degree rotation, and skew changed 44/103, 107/255, and 40/64. The same audit
found ignored text semantics: italic `font` shorthand changed 68 pixels,
`letter-spacing` 200, vertical writing mode 1,520, and dominant baseline 320,
all at delta 255. These are scratch measurements, **not cells**; seven registered
unsupported-corpus rows guard their source classes. The three admitted cells
above carry only the exact positive branches.

Temporarily bypassing the final-CTM patrol made `just gate` accept the
fractional-translation frame and fail loudly in the committed text contract.
Restoring the patrol returns primitive cells, all nine text cells, and the
closed refusal register to green.

## Tooling

Run from `fixtures/web-first/`:

```sh
just text-add <svg-text-id> <scratch-source>
just text-bake
just text-gate
```

`text-add` refuses an existing source or manifest row and never writes an
oracle. `text-bake` creates only missing oracles, verifies every existing one
pixel-for-pixel, and refreshes hash provenance. `text-gate` is the focused
exact-byte Rust gate; the broader `just gate` also runs it and the refusal
register.

## Re-baking

```sh
cd fixtures/web-first && just text-bake
```

A committed oracle is verification-only: a differing re-capture fails instead
of blessing a new baseline.
