# The text cell suite

Chromium-baked evidence for the `<text>` slice on the SVG engine of record
(`websem → rframe → n0`). Nine Ahem cells are gated byte-exact by
[`crates/websem/tests/svg_text.rs`](../../../crates/websem/tests/svg_text.rs);
six real-font artifact witnesses are gated numerically, before rasterization, by
[`crates/websem/tests/svg_text_geometry.rs`](../../../crates/websem/tests/svg_text_geometry.rs).
The method these cells enforce is the ratified
[text-oracle brief](../../../docs/wg/consolidation/text-oracle.md); this file
states only how the suite is shaped.

The pixel suite currently has **nine** cells. The separate Rung-B geometry
suite under [`geometry/`](./geometry/) has **six** witnesses: four Allerta and
two Bungee. Both manifests are closed enumerations: the Rust gates reject an
unlisted SVG, duplicate source row, undeclared or changed font identity, stale
suite or baker hash, stale shared-capture hash, changed source, or changed
oracle.

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

Both bakers import the same hash-pinned
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
all at delta 255. A post-review probe found two more silent source legs: a
quoted `/*` string hid a later `5119px` declaration and reproduced the
149-pixel/delta-255 quantization error, while inherited `text-anchor="end"`
silently became `start` and changed 1,400 pixels at delta 255. These are scratch
measurements, **not cells**; nine registered unsupported-corpus rows guard their
source classes. The three admitted cells above carry only the exact positive
branches.

Temporarily bypassing the final-CTM patrol made `just gate` accept the
fractional-translation frame and fail loudly in the committed text contract.
Restoring the patrol returns primitive cells, all nine text cells, and the
closed refusal register to green.

## T2 real-font artifact geometry

Rung B makes a different claim from the exact Ahem cells. Chromium's standard
SVG text-query APIs grade the resolved run and character-cell geometry; they
do not grade real-font pixels. Engine pixels are required only to be
deterministic, non-empty for the witness, and identical between strict and
best-effort admission. No real-font Chromium raster is an oracle here.

The first witness uses the existing redistributable
[`Allerta-Regular.ttf`](../../fonts/Allerta/Allerta-Regular.ttf), face index 0:
16,248 bytes, SHA-256
`16d6915227c7560725c037c9c93163cba5367c3ef4cf2ec12bf40b9eb2984a6b`, under
the [SIL Open Font License 1.1](../../fonts/Allerta/OFL.txt) whose bytes are
independently hash-pinned in `geometry/cases.json`. The source is the one LTR
run `Hxi` at 5120px, middle-anchored so its 8600px advance starts at zero.

Chromium 149.0.7827.55 directly reports total and substring length 8600;
character advances 3745, 3400, and 1455; starts at 0, 3745, and 7145; baseline
4080; and a logical character-cell top/height of -1205/6545. The immutable
artifact matches every reported number exactly after promoting its binary32
facts to binary64. Source indices in the browser record are UTF-16 code-unit
indices, while artifact clusters are UTF-8 byte offsets; printable ASCII makes
the mapping 0/1/2 on both sides.

Chromium exposes no glyph identifier or outline-ink box. Those are therefore
not described as browser measurements: the gate checks separately against the
same pinned font bytes that glyphs `H`, `x`, and `i` are cmap ids 42, 88, and
73, that the face has 1024 units per em, and that the shaped outline union is
`(470, -4080, 7765, 4080)`. The resulting frame path has the corresponding
translated bounds `(470, 0, 7765, 4080)`; no font fact crosses `rframe`.

The numeric boundary is two-dimensional. Blink's SVG query projection
encloses horizontal character boundaries on a 1/64 CSS-pixel grid and exposes
vertical cells through integral fixed ascent/descent metrics. Scratch probes
at 80, 85, 1000, and 5120px found both divergence classes: 80px preserves the
horizontal values but not the vertical metrics, 85px and 1000px diverge on
both, and 5120px is exact on both (measured, not celled except for the 5120px
witness). At 1000px, for example, the artifact's first advance is
731.4453125 while Chromium reports 731.453125 after enclosing it to 1/64, and
the artifact ascent is 1032.2265625 while the character cell uses 1032.
A separate Bungee 50px probe isolates the other leg: ascent/descent are already
the integral 51/15, while the artifact's first 37.95 advance is exposed by
Chromium as 37.953125. Both actual command admissions reach the 1/64 refusal
(measured, not celled); the hash-pinned font is a negative-only gate identity,
not a second positive oracle.

That formerly admissible 1000px route now refuses by the stable
`Chromium SVG text query` reason in strict admission and skips/degrades the
same text node in best effort. Its refusal row is committed as
`svg-text-geometry-grid`. A supporting scratch raster comparison found 8,310
differing pixels at maximum channel delta 170 against Chromium before the
guard; this is evidence of the unsafe route, not a real-font pixel-fidelity
claim. The T2 samples `fi` and `AV` happened not to exercise either ligature
substitution or pair positioning in this face (measured, not celled). At this
checkpoint the hash-pinned Bungee face was a negative-only gate identity; T3c
below reuses the same bytes for positive offset witnesses. T3
therefore starts from explicit cluster/source mapping rather than generalizing
those two strings.

Gate sensitivity is direct. Temporarily bypassing the query-geometry admission
let both negative runs lower; `just gate` passed the unrelated 1,051 primitive
cells and nine Ahem cells, then failed loudly in both the vertical-only Allerta
80px and horizontal-only Bungee 50px contract tests. Restoring the admission
returned the complete gate, including all 207 named refusal rows, to green.
Independently changing the committed run length by exactly 1/64 made the
exact-number comparison fail (`8600` versus `8600.015625`), and restoration
returned it green. Re-registering the existing id/source was refused as
immutable. Scratch source mutations remained discriminating: `Hxi` to `HxW`
changed 715,311 Chromium pixels and middle to start anchoring changed 715,768,
both at maximum channel delta 255 (measured, not celled and not used as a
real-font pixel oracle).

## T3 direct cluster mapping and default kerning

Oracle v1 makes shaping cardinality a stated artifact fact. Each admitted
cluster records its source UTF-8 byte range, source UTF-16 code-unit range,
and placed-glyph range; each glyph points back to that cluster. The shaping
policy explicitly fixes monotone-grapheme clustering for this LTR profile.
Printable ASCII still makes the two source ranges numerically equal, but no
consumer may infer one coordinate space from the other.

The second Allerta witness is the run `ff` at 5120px. Chromium and the
artifact agree exactly on total advance 4685, character advances 2330/2355,
and starts 0/2330. Separately, the pinned font bytes grade glyph ids 70/70 and
the outline union `(275, -3905, 4190, 3905)`, while the artifact states direct
UTF-8, UTF-16, and glyph ranges. The first 25-unit reduction is default OpenType
pair positioning: a scratch `font-kerning:none` control measures 4710 total
and 2355 for each character. At 120px the default/control Chromium rasters
differ at 353 pixels with maximum channel delta 206 (measured, not celled;
real-font pixels remain outside the oracle claim). Disabling kerning through
CSS is not admitted by this rung: authored `font-kerning` retains the existing
unconsumed-property refusal.

A PT Serif `fi` probe exposes the adjacent unsafe class. The pinned shaper
forms one glyph (id 715) with advance 3000 and source-cluster start 0.
Chromium still exposes two addressable UTF-16 characters, assigning each a
1500-unit query segment. With standard ligatures disabled it instead exposes
two glyph advances 1710/1505 and total 3215; the 120px default/control rasters
differ at 1,259 pixels with maximum delta 255 (measured, not celled).
Before this rung, both actual CLI admissions accepted and painted the merged
cluster without an artifact fact capable of stating the two browser character
segments. The defect was contractual, not a painter error: the immediate
cause was a lone cluster-start integer, and the systemic cause was an artifact
with no source/glyph spans or internal-caret policy.

Merged and decomposed clusters now leave before lowering by the stable
`shaping cluster mapping` reason. Strict refuses the run; best effort skips
the same text node and declares the same reason. The committed PT Serif
refusal and the direct resolver/producer patrols hold that boundary. This is a
deliberate over-refusal until a later oracle version can state caret stops and
browser character geometry inside an inseparable glyph set.

Gate sensitivity was proved on both legs. Temporarily disabling default
`kern` shaping left the primitive and Ahem estates green, then failed the new
exact-number witness loudly at artifact total 4710 versus Chromium 4685.
Independently bypassing both direct-cluster fences recreated the former silent
route: PT Serif `fi` lowered to one path, and the focused producer contract
failed because strict unexpectedly accepted it. Restoring shaping and both
fences returned the complete primitive, text, geometry, and 208-row refusal
gate to green.

At this T3a checkpoint the text estate was nine exact Ahem cells plus two
exact-number Allerta geometry witnesses, and the named refusal register had
208 rows. It did not admit ligatures, decompositions, nonzero offsets,
combining marks, non-ASCII repertoire, authored feature controls, or multiple
runs, and closed no checklist row.

## T3b bounded precomposed Latin repertoire

Oracle v2 extends the source admit-list by exactly 53 Latin-1 letters:
U+00C0–00C5, U+00C7–00CF, U+00D1–00D6, U+00D9–00DD, U+00E0–00E5,
U+00E7–00EF, U+00F1–00F6, U+00F9–00FD, and U+00FF. Every member has a
canonical decomposition to one ASCII Latin base plus one combining mark.
That property defines the set; the font's broader coverage does not. `Æ`,
`Ð`, `Ø`, `Þ`, `ß` and their lowercase peers, combining sequences, and all
wider Unicode remain refused.

The third exact-number Allerta witness is
`AÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåçèéêëìíîïñòóôõöùúûüýÿZ`
at 5120px. Chromium 149.0.7827.55 reports 55 addressable characters and total
advance 171785. The pinned face independently grades all 55 direct glyph
identities and outline union `(315, -5285, 171085, 6545)`. Most importantly,
the artifact records 108 UTF-8 bytes and 55 UTF-16 units: `À` occupies bytes
`1..3` but units `1..2`, while the final `Z` occupies bytes `107..108` but
units `54..55`. Every Chromium character advance, start, end, logical extent,
and rotation is compared exactly. Engine pixels retain only the existing
determinism, non-empty, and strict-equals-best claims.

The source-form boundary was measured separately. Chromium paints `AéZ` and
`Ae` + U+0301 + `Z` identically, while `AeZ` differs from either accented
spelling at 256 pixels with maximum delta 255. Its text-query API reports
three addressable characters for the precomposed spelling and four for the
decomposed spelling; the combining mark shares the base character's start,
end, and substring length (measured, not celled). Resolution therefore never
normalizes authored text. At the T3b checkpoint the precomposed run rendered
through both CLI admissions with byte-identical outputs while the decomposed
run refused by the stable v2 repertoire reason. The T3c checkpoint below
supersedes that broad refusal with a bounded combining grammar.

The hidden defect was a coordinate-space equality that printable ASCII had
made look structural: the browser-facing admission counted bytes in a UTF-8
cluster range and called a one-byte range one source scalar. The artifact
already carried correct scalar-aligned UTF-8 and UTF-16 spans. The admission
now verifies one scalar from the recorded source slice instead. Deliberately
restoring the byte-count assumption made `just gate` fail loudly on the new
witness at source bytes `1..3`; restoring the scalar check returned the whole
gate green.

At this T3b checkpoint the text estate was nine exact Ahem cells plus three
exact-number Allerta geometry witnesses, and the named refusal register had
209 rows. No checklist row closed: combining marks and offsets, merged clusters
and carets, non-decomposable Latin letters, wider repertoire, feature inputs,
multiple runs, and every complete text/font grammar remain separate work.

## T3c combining clusters and glyph offsets

Oracle v3 adds exactly U+0301 and U+030B when one is the sole combining mark
immediately after an ASCII Latin letter. It does not infer general combining
support from those two cases. Leading, repeated, non-letter-attached, and
unlisted marks remain outside the source grammar; a permitted mark missing
from the declared face refuses at the mark's own byte instead of selecting
tofu or an ambient fallback.

The geometry manifest now carries multiple exact font identities and a v2
font-fact shape. Every cluster records UTF-8, UTF-16, scalar, and glyph ranges;
every placed glyph records pen x, x/y offset, advance, and cluster index. A
direct cluster remains one scalar and one glyph. A permitted base-plus-mark
cluster is two source scalars and either one composed glyph or two glyphs; the
second glyph in the latter form has zero advance and owns any displacement.
Every UTF-16 unit in either form is checked against Chromium's complete shared
cluster cell.

| Witness | Discriminating branch |
| --- | --- |
| `svg-text-allerta-decomposed-acute.svg` | At 5120px, `Ae` + U+0301 + `Z` shapes to three glyphs without rewriting four source scalars. Chromium and the artifact agree on total 10340; `e` and the mark each expose start 3795, end 7115, and length 3320. The cluster records bytes `1..4`, UTF-16 units `1..3`, scalars `1..3`, and glyphs `1..2`. |
| `svg-text-bungee-acute-offset.svg` | At 1000px, the mark remains a separate glyph at pen 1467, x offset -369, local y offset 0, and zero advance. Chromium exposes both `x` and mark over the shared 737-unit cell; total advance is 2127. |
| `svg-text-bungee-double-acute-offset.svg` | The same attached branch uses U+030B and adds a nonzero y-up shaper displacement, recorded exactly once as local y-down offset -7. |

Scratch Chromium rasters prove the placement is material without becoming
real-font pixel oracles. Disabling mark attachment changes 3,488 pixels for
U+0301 and 4,441 for U+030B, both at maximum delta 255; changing U+0301 to
U+030B changes 1,800 pixels at delta 255. Stacking a second U+0301 and disabling
mark-to-mark placement changes 2,087 pixels at delta 255. Allerta's composed
and decomposed spellings are byte-identical, while deleting the accent changes
256 pixels at delta 255 (measured, not celled).

The first exact outline projection exposed a pre-existing bounds defect. The
font's stored Bungee acute box reported local top/height -965/965, but the
quadratic path's actual extrema are
-961.39019775390625/961.39019775390625. `textlayout` now derives ink bounds
from the same offset- and y-flip-aware outline stream it exposes to lowering,
solving quadratic and cubic extrema. The gate compares those bounds with the
streamed local `PathData` exactly; no tolerance was introduced.

The former `svg-text-combining-sequence` row is replaced by
`svg-text-combining-malformed`, `svg-text-combining-missing-glyph`, and
`svg-text-combining-unlisted-mark`. Focused contracts additionally exercise
leading, repeated, digit-attached, precomposed-letter-attached, missing-glyph,
and unlisted-mark sources in strict and best-effort admission at the same
`svg/text[1]` path.

Gate sensitivity was proved by temporarily zeroing every shaped x offset. The
full gate failed on the committed Bungee fact at `0` versus `-369`; restoring
the offset returned it to green. All three sources render through the actual
CLI under both policies with byte-identical outputs, and attempting to add an
existing geometry source/oracle is refused. The estate is now nine exact Ahem
cells plus six exact-number real-font geometry witnesses; the named refusal
register has 211 rows. No checklist row closes.

## Tooling

Run from `fixtures/web-first/`:

```sh
just text-add <svg-text-id> <scratch-source>
just text-bake
just text-geometry-add <svg-text-id> <scratch-source> <scratch-font-facts>
just text-geometry-bake
just text-gate
```

`text-add` refuses an existing source or manifest row and never writes an
oracle. `text-bake` creates only missing oracles, verifies every existing one
pixel-for-pixel, and refreshes hash provenance. `text-gate` is the focused
Rust gate for both the exact pixels and artifact geometry; the broader
`just gate` also runs it and the refusal register. `text-geometry-add` likewise
refuses an existing source or oracle path, while `text-geometry-bake` creates
only a missing JSON oracle and requires every existing numeric record to remain
byte-identical.

## Re-baking

```sh
cd fixtures/web-first && just text-bake
```

A committed oracle is verification-only: a differing re-capture fails instead
of blessing a new baseline.
