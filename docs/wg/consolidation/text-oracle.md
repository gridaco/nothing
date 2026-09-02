---
title: "The text oracle method"
description: "How text enters the byte-exact Chromium gate without weakening it: the deterministic-font rung, the measured admitted domain, and the corpus-growth law."
tags:
  - internal
  - wg
  - program
  - consolidation
  - text
format: md
---

# The text oracle method

**Status: ratified 2026-08-04** ([gridaco/nothing#68](https://github.com/gridaco/nothing/pull/68)).
Its decisions bind the text arc's rungs; the D-M shaped-text stage it left
open was later closed **low** by text-2, the two-producer spike
([the text-stage evidence](./n0-join-point.md#the-text-stage-evidence)).
This brief is the text-0 milestone: it puts the *method*
for gating text on the SVG engine of record (`websem → rframe → n0`) before
the owner, with the Chromium probe record that grounds it. It decides no code
and touched no then-open decision, and
nothing here produces or implies a conformance score (FLIP is unratified).

The reader is the owner ratifying a gate method before any text code exists,
and the implementer of the first text rung executing against it.

## The problem

Every rung on the engine of record has gated on byte-exact Chromium cells.
Glyph rasterization normally destroys that: glyph masks are produced by the
platform rasterizer (CoreText on macOS, DirectWrite on Windows, FreeType on
Linux), so the same glyph run yields different bytes on different rasterizers
by design. The tree has already conceded this once — the v1 exact manifest's
"1.0 Chromium floor" **hides text** to reach zero threshold
([web-renderer-adoption](./web-renderer-adoption.md)). Text therefore needs a
declared method before its first rung, or the gate silently weakens.

The web platform's own test suite answers the same problem the same way this
brief proposes: WPT gates text layout with the
[Ahem font](https://web-platform-tests.org/writing-tests/ahem.html) — every
visible glyph a solid em-square box — and degrades to annotated fuzzy matching
only where real fonts are unavoidable. Chromium's text correctness is itself
graded this way; adopting the ladder aligns our gate with the oracle's own.

## The gate ladder

Three rungs, each declaring what it can gate and what it refuses to gate. A
construct enters the corpus only under a rung that can actually grade it.

| Rung | Method | Gates | Refuses to gate |
| --- | --- | --- | --- |
| **A — deterministic font, byte-exact** | Pinned Ahem bytes, admitted numeric domain (below), existing byte-exact cell gate unchanged | The whole text *resolution* pipeline: positions, advances, anchoring, whitespace, baseline math, paint integration — pixel-perfect | Raster quality: anti-aliasing, hinting, real-font fidelity |
| **B — geometry oracle** | Chromium's SVG text measurement APIs (`getExtentOfChar`, `getComputedTextLength`, …) captured under the bake posture grade the *resolved artifact's geometry* for real fonts; pixels are governed by the engine's own laws (reuse ≡ fresh), not graded against Chromium | Shaping facts with real fonts: cluster extents, advances, anchoring | Any pixel claim |
| **C — thresholded raster** | Perceptual/threshold pixel comparison | — | **Parked.** Requires an owner decision adjacent to FLIP; never lands as a score. |

Rung A was the only rung text-0 asked the owner to activate. Rung B was then
exercised on 2026-09-01 by the real-font geometry addendum below. Rung C exists
so nobody reinvents it informally; it remains parked.

## The probe record

Measured before written, per program practice. Probe fixtures and captures are
scratch and **not committed** (see the growth law below); this table is the
record.

**Identity:** Chromium `149.0.7827.55` (Playwright `chromium-1228`), macOS
arm64, the exact [bake posture](../../../fixtures/web-first/bake_chromium.ts)
(deviceScaleFactor 1, JavaScript disabled, `en-US`, UTC, light scheme,
standalone-SVG data URL, element screenshot with transparent background).
**Font:** [the pinned Ahem](../../../fixtures/web-first/fonts/README.md)
(`b719ecb3…94b8448`), embedded as a data-URI `@font-face`. Every probe was
pixel- and byte-stable across repeated captures.

### Round 1 — default smoothing: the fringe

| Probe | Expected if naive | Measured |
| --- | --- | --- |
| fs50 `X` at (25,60) | solid box x[25,75) y[20,70), no AA | opaque core exactly 50×50, **plus a deterministic 1px AA fringe** (alphas 28–138), ink x[24,76) y[19,70) |
| fractional x/size variants | — | same shape: exact core, deterministic fringe |
| undeclared font family | — | ambient system fallback renders: 89 distinct alpha levels of machine-local raster |

The fringe is CoreText smoothing: macOS glyph rasterization **dilates ink
beyond the glyph's true coverage**. It is repeat-stable, but it is
rasterizer-owned — an engine that reproduced it would be imitating CoreText,
not resolving text. Plain Ahem alone therefore does not deliver
engine ↔ oracle byte-exactness.

### Round 2 — `-webkit-font-smoothing: none`: exact

With the declaration on the text element, every probe went bilevel — alphas
`{0,255}`, zero AA pixels, solid boxes at exactly the predicted coordinates:

| Probe | Measured |
| --- | --- |
| fs50 `X` at (25,60) | **exact** solid x[25,75) y[20,70), opaque 2500 |
| `antialiased` variant of the same | **byte-identical PNG** to `none` — over integer-aligned boxes the raster policies coincide |
| x=25.5 | snaps: byte-identical to x=25 |
| y=60.5 | snaps: box shifts to y[21,71) (rounds up) |
| fs48 (em split 38.4/9.6) | quantizes to a 48×48 box at thresholded edges x[25,73) y[22,70) |
| `text-anchor="middle"` fs20 `XXX` at x=50 | exact x[20,80) y[44,64) |
| `middle` with fractional start (fs15, start 27.5) | snaps to x[27,72) |
| `text-anchor="end"` fs20 `XXX` at x=90 | exact x[30,90) |
| `"  X X  "` fs20 at x=10 | collapse verified by column profile: ink exactly [10,30) ∪ [50,70) — leading/trailing stripped, internal run → one 1em advance |
| `XX` fs20 at x=10 | seamless x[10,50): advance exactly 1em, no kerning, no seam |

Two findings shape the method:

1. **Fractional inputs never refuse in Chromium — they snap**, by a
   deterministic but rasterizer-internal rounding rule. The slice must
   *refuse* fractional resolved geometry rather than codify that rule.
2. **Over integer-aligned box glyphs, every raster policy produces identical
   bytes** — bilevel, grayscale AA, and (by construction) any exact-coverage
   rasterizer agree wherever coverage per pixel is 0 or 1. Only default macOS
   smoothing diverges, because it alone paints outside true coverage.

### The engine-side crux

The Chromium rounds establish the oracle half; the method also claims the
engine half — that a run shaped hermetically and lowered as outline path
facts rasters the same bytes. A scratch spike (never committed, per the
growth law) proved it: rustybuzz `0.20.1` + the pinned Ahem shaped three
runs — the em-box `X`, the collapsed `X X` (the space is a glyph with
advance and no outline), and a middle-anchored `XXX` — lowered them to
`rframe` path facts under the nonzero rule with the y-flip applied
(font units are y-up), and rendered through `n0::glyphless` on the reftest
gate's exact posture (transparent clear, identity view, straight-alpha
readback). **All three came back byte-identical to the round-2 captures,
deterministic across double renders.** The coincidence argument is now
measured on both sides: within the admitted numeric domain, Chromium's
bilevel glyph raster and the engine's anti-aliased path raster produce the
same bytes.

## The method (rung A, concrete)

**The admitted numeric domain.** A text cell is admissible only where the
probes proved all rasterizers coincide:

- integer resolved `x`/`y`;
- `font-size` divisible by 5, so Ahem's 0.8/0.2 em split lands on integers;
- anchor-resolved start positions integer (a `middle` whose half-advance is
  fractional refuses);
- content whose Ahem glyphs are the em box or the space.

The domain binds **final device-space geometry**: every glyph-box edge must
land on an integer device coordinate after the full CTM, so integer local
inputs qualify only under a transform that preserves them — the slice admits
`<text>` under identity and integer-translation CTMs first, and a scaling,
rotating, or fractional transform above a text node refuses by name like any
other out-of-domain number. At ratification the admitted constructs were one
LTR run with no `tspan`, `dx`/`dy`/`rotate` lists, `direction`, or `textLength`;
the dated evidence addenda below advance only explicitly measured splits. The
slice's statement of record owns the current construct boundary per standing
practice. Rung A's pipeline claim gates that admitted slice, not all of SVG
text.

Everything outside the domain **refuses by name at a stable node path** — the
standing law, now applied to numbers. Chromium's snapping behavior is a
declared divergence-by-refusal: Chromium renders a snapped box; the engine
names the construct instead. Over-refusal beats codifying a rasterizer's
rounding.

**The oracle posture declaration.** Text fixtures carry
`-webkit-font-smoothing: none` on the text element as **bake posture**, the
way they already carry width/height as viewport posture. It suppresses the one
rasterizer behavior (macOS smoothing dilation) that breaks coincidence; on
non-macOS Chromium the property is a no-op by design, and coincidence there
rests on the exact-coverage argument below until a re-bake measures it. Within the
admitted domain the declaration is *semantically empty for the engine* — AA
and bilevel raster are byte-identical there — so no quality flag becomes
readable from `websem`, `rframe`, or resolve: the one-meaning tripwire holds.
Text-1 must verify (and pin in the corpus) that websem's strict admission
treats the declaration as standard CSS unknown-property discard rather than a
refusal.

**Why the bake stays maintainer-portable.** The measured oracle identity is
macOS arm64 Chromium 149 — the platform behind every probe above — and the
engine-side spike measures the Skia half of the coincidence. Cross-rasterizer
coincidence (FreeType, DirectWrite) over the admitted domain is an argument
from exact pixel coverage, not yet a measurement. The harness is what makes
that safe to leave unmeasured: an existing oracle is verification-only, so a
differing re-capture fails loudly instead of blessing a drift, and the first
re-bake that reproduces the cells on another rasterizer graduates the
expectation into evidence.

## The hermetic font environment

Per [the text-layout RFD](../feat-paragraph/text-layout.md), the resolution
environment is a manifest, not an ambient promise, and a family name is not a
font identity. Applied to the engine of record:

- A render's fonts are **declared inputs pinned by content hash**, and the
  hash is executable, not documentation. Proposed n0_cli surface: a
  repeatable, hash-bearing `--font FAMILY=PATH@sha256:HEX` (exact grammar
  owned by [the statement of record](../../../crates/n0_cli/README.md) when
  text-1 lands); the host verifies the loaded bytes against the declared
  digest and refuses — typed, before any pixel — on mismatch, so a swapped
  file can never silently re-bake.
- The default environment is **empty**: any `<text>` with no declared font is
  a typed refusal, never tofu and never a system-font fallback. The
  missing-font probe shows what ambient fallback costs — machine-local pixels
  no oracle can pin. There is no silent system fallback, exactly as the RFD
  demands.
- Pinned gate identities live in
  [`fixtures/web-first/fonts/`](../../../fixtures/web-first/fonts/README.md),
  one entry per admitted identity, hash-recorded.

## The corpus-growth law

The oracle corpus must not grow into a conformance suite by accretion. Four
rules, the first two already program practice, now stated as law:

1. **Probes are never committed.** Probe fixtures, runners, and captures are
   scratch; only the verdict record enters a brief or the D-N register. Every
   rung so far has worked this way ("measured before written"); text does not
   change it.
2. **The tracked corpus is a gate, not coverage.** Each cell exists to enforce
   one admitted construct or one named refusal; a rung adds the smallest set
   that does so. Breadth — real-font suites, WPT text, stress corpora — lives
   untracked under `fixtures/local/`, downloaded per-machine like resvg and
   the W3C SVG 1.1 suite already do, and (FLIP being unratified) is never
   scored.
3. **Text is contained in its own suite.** Text cells land in
   `fixtures/web-first/text/` with their own suite manifest and bake, on the
   `animation/` precedent. The primitive root — already ~180 files — stays
   closed to text.
4. **The fonts directory grows per pinned identity only**, never per fixture,
   and never holds a corpus.

## Decisions for the owner

Ratifying this brief decides:

1. **The gate ladder**, with rung A as the only active rung and rung C parked.
2. **The admitted numeric domain** and refusal-over-snapping stance.
3. **The oracle posture declaration** (`-webkit-font-smoothing: none` as bake
   posture carried by text fixtures).
4. **The hermetic environment surface** — empty by default, hash-bearing
   `--font` declarations verified before rendering.
5. **The corpus-growth law.**
6. **The lowering shape for text-1: outlines first, no rframe amendment.**
   Resolved glyphs lower to the contract's existing path facts: `rframe`
   refuses paints that reference a resource, and a glyph run naming a font
   key is exactly such a reference — lowering to outlines resolves the font
   away before the fact enters the contract, so the path fact carries
   geometry only. The shaped-text *fact* enters `rframe` only if the D-M
   deciding spike says the join is high — that spike, per
   [the join point](./n0-join-point.md), runs once the Web family's producer
   exists, i.e. after text-1.
7. **The resolver's home**: a new backend-free workspace crate implementing
   the RFD's minimal profile (proposed name `textlayout`; shaping via
   rustybuzz + ttf-parser, both already in the lock via vendored usvg), whose
   identity-by-refusal is: owns shaping, breaking, metrics, and the resolved
   artifact; refuses font discovery, paint, clocks, and estimates — with an
   architecture test forbidding ambient font access.

What the original ratification did **not** decide: the D-M shaped-text join,
rung C, and any real-font method detail beyond naming rung B as its
destination. D-M later closed low; the following evidence addendum records the
first execution of B without reopening either decision.

## Rung-B evidence addendum — real-font artifact geometry (2026-09-01)

The verdict is EXERCISED for one exact real face and one horizontal LTR run.
This is an evidence checkpoint, not completion of `<text>` or any broad font,
shaping, or text-layout grammar.

The selected identity is the redistributable Allerta Regular face, index 0,
16,248 bytes, SHA-256
`16d6915227c7560725c037c9c93163cba5367c3ef4cf2ec12bf40b9eb2984a6b`, under
the SIL Open Font License 1.1. The committed witness uses `Hxi` at 5120 CSS
pixels with middle anchoring. That size is evidence-driven: it makes both
normalization stages below exact, rather than rounding the record until it
looks equal.

The evidence keeps direct browser facts and pinned-font facts separate.
Chromium's standard SVG text APIs directly report the run length, substring
lengths, character starts, ends, extents, rotations, anchor placement, and
source character indexing. Those APIs index source text in UTF-16 code units.
They expose neither glyph identifiers nor outline-ink bounds. Glyph identity,
units-per-em, UTF-8 cluster mapping, and ink bounds are therefore verified
independently against the exact declared font bytes and are never described as
Chromium measurements. Printable ASCII makes the witness's UTF-8 and UTF-16
positions both 0/1/2; later repertoires may not assume that coincidence.

The comparison is exact and representation-aware. A browser JSON number is an
IEEE-754 binary64 value; each artifact binary32 number is promoted to binary64
and compared for equality. There is no decimal pre-rounding, epsilon, or image
threshold. For the committed witness Chromium reports total advance 8600,
individual advances 3745/3400/1455, character starts 0/3745/7145, baseline
4080, and character-cell top/height -1205/6545. The artifact states those
browser-visible facts exactly. Its separate font-derived record names glyphs
42/88/73, 1024 units per em, clusters 0/1/2, and outline union
`(470, -4080, 7765, 4080)`.

Measurement found two normalization stages at the browser boundary. Horizontal
character cells enclose their start and end on a 1/64 CSS-pixel fixed grid;
vertical cells expose integral fixed ascent and descent. Probes at 80, 85,
1000, and 5120 pixels separated them: 80 preserves horizontal geometry but not
the vertical metrics, 85 and 1000 differ on both, and 5120 is exact on both
(measured, not celled except for the 5120 witness). At 1000, the artifact's
first advance is 731.4453125 while Chromium exposes 731.453125, and the
artifact ascent is 1032.2265625 while Chromium's character cell uses 1032.
A separate Bungee 50px probe isolates the horizontal stage: its ascent and
descent are already integral at 51/15, while the artifact's first 37.95
advance is exposed as 37.953125 (measured, not celled). At this checkpoint
that face is a negative-only boundary witness; T3c later reuses the same exact
identity for positive offset geometry.

The method does not normalize the artifact to those values. Geometry that
would change at either projection departs by a stable text-specific name before
rasterization: strict refuses, while best effort skips and declares the text
node. That boundary is conservative until a later oracle version deliberately
owns the normalization.
The selected face's sampled `fi` and `AV` runs exposed no second shaping class
(measured, not celled); ligature, kerning, nonzero-offset, and wider cluster
semantics remain outside this checkpoint.

Real-font pixels remain outside the claim. The engine raster is guarded for
determinism, non-empty realization, and identity between its admission
policies, but it is never compared with Chromium and receives no tolerance.
The resolved font and glyph identity still disappear before the shared frame;
Rung B requires no new shared render fact. The complete evidence record and
tooling live in the [text fixture estate](../../../fixtures/web-first/text/README.md).

## T3 evidence addendum — direct clusters and pair positioning (2026-09-01)

The verdict is ADMIT/SPLIT, with no checklist closure. The oracle advances to
v1 by making source/glyph cardinality explicit and admits default horizontal
pair positioning only while each cluster remains one source scalar to one
glyph. Merged and decomposed clusters split into a named refusal until the
artifact also owns caret positions inside inseparable glyph sets.

This boundary follows the two coordinate systems already exposed by Rung B.
The shaper receives monotonically increasing UTF-8 byte offsets and preserves
monotone clusters for this horizontal left-to-right profile. SVG text-query
APIs instead address source UTF-16 code units. Oracle v1 therefore records,
for every admitted cluster, its UTF-8 source range, UTF-16 source range, and
contiguous glyph range; every glyph maps back to one such cluster. Printable
ASCII makes the two source ranges numerically equal today, but that equality
is no longer an implicit consumer assumption.

The positive witness is Allerta Regular `ff` at 5120px. Chromium and the
artifact agree exactly on character advances 2330/2355, starts 0/2330, and
total advance 4685. Separately, the pinned font bytes grade glyph identifiers
70/70 and outline union `(275, -3905, 4190, 3905)`, while the artifact states
their direct source/glyph ranges. A feature-disabled browser control reports
2355/2355 and total 4710, establishing that the 25-unit first-character
reduction is default pair positioning (measured, not celled). Its 120px
default/control raster pair differs at 353 pixels with maximum channel delta
206 (measured, not celled and not a real-font pixel-oracle claim).

The split witness is PT Serif `fi` at 5000px. Default shaping produces one
glyph, identifier 715, with one source-cluster start and total advance 3000.
Chromium nevertheless reports two addressable characters and divides that
advance into two 1500-unit query cells. Disabling standard ligatures produces
two advances 1710/1505 and total 3215; the corresponding 120px raster pair
differs at 1,259 pixels with maximum delta 255 (measured, not celled). SVG 2
permits a user agent either to forbid a caret inside an inseparable glyph set
or allocate portions of its area to the represented characters. The measured
browser takes the latter route here; a lone cluster-start offset cannot state
that decision.

The prior route admitted and painted the ligature in both policies. Its
presentation was a clean render, its proximate cause was that shaping's
default substitution reached an artifact carrying only one start offset, and
its systemic cause was a source-mapping contract with no ranges or internal
caret policy. The correction is therefore at the oracle boundary rather than
in rasterization: a non-direct source/glyph cardinality leaves by the stable
`shaping cluster mapping` reason before any frame fact exists. Strict refuses;
best effort skips and declares the same text node. This over-refusal is
deliberate until a later oracle version can state and grade the missing caret
geometry.

Sensitivity is guarded independently on both sides of the split. Disabling
default pair positioning changes the committed Allerta artifact total from
4685 to 4710 and fails the exact browser comparison. Bypassing both
source-cardinality boundaries lets the PT Serif ligature lower and fails the
contract that strict admission must refuse it. Restoring both returns the
complete primitive, text, geometry, and refusal gate to green.

The exact Ahem estate remains nine cells. The real-font geometry estate now
has two Allerta witnesses, and the named refusal register has 208 rows.
Ligatures, decompositions, nonzero offsets, combining marks, non-ASCII
repertoire, script/language/feature inputs, and multiple runs remain outside
this checkpoint. Real-font pixels remain outside the Chromium oracle claim,
and no shared render-contract fact is added.

## T3b evidence addendum — precomposed Latin source mappings (2026-09-01)

The verdict is ADMIT/SPLIT, with no checklist closure. Oracle v2 widens the
source repertoire from printable ASCII by exactly 53 precomposed Latin-1
letters: U+00C0–00C5, U+00C7–00CF, U+00D1–00D6, U+00D9–00DD,
U+00E0–00E5, U+00E7–00EF, U+00F1–00F6, U+00F9–00FD, and U+00FF. Every
member has a canonical decomposition to one ASCII Latin base plus one
combining mark. This source property, not the selected face's coverage,
defines the admit-list; neighboring non-decomposable Latin-1 letters remain
outside it.

The exact positive witness places every member between ASCII `A` and `Z` in
the pinned Allerta face at 5120px. Chromium exposes 55 addressable characters,
all direct, at total advance 171785. Pinned-font facts independently grade the
55 glyph identities and outline union `(315, -5285, 171085, 6545)`. The
source ranges make the purpose of the witness explicit: the run is 108 UTF-8
bytes but 55 UTF-16 units. The first precomposed member occupies bytes `1..3`
and units `1..2`; the final ASCII sentinel occupies bytes `107..108` and units
`54..55`. Every browser-visible character cell is exact, without decimal
rounding or tolerance. Real-font pixels keep only the engine-law checks and
remain outside the Chromium claim.

Chromium paints precomposed `AéZ` and decomposed `Ae` + U+0301 + `Z`
identically in this face, while the unaccented control differs at 256 pixels
with maximum delta 255. The two accented sources are not the same geometry
contract: browser queries expose three addressable characters for the former
and four for the latter, with the combining mark sharing its base's segment.
These source-form facts are measured, not celled. Resolution therefore
preserves source spelling and performs no normalization. The precomposed
source admits; at this checkpoint the decomposed sequence leaves through a new
registered v2-repertoire refusal in strict and best-effort paths. T3c below
supersedes that broad refusal with a bounded two-mark grammar.

The defect exposed by the new coordinate split was not in shaping or paint.
The artifact already retained scalar-aligned UTF-8 and UTF-16 ranges, but one
consumer treated the byte length of a UTF-8 range as source-scalar
cardinality. Printable ASCII had made that assumption observationally true.
The corrected law validates one scalar in the recorded source slice. Restoring
the old byte test deliberately makes the complete gate fail on bytes `1..3`;
restoring the scalar law returns it green.

The exact Ahem estate remains nine cells. The real-font geometry estate now
has three Allerta witnesses, and the named refusal register has 209 rows.
Non-decomposable Latin letters, combining marks and offsets, merged clusters
and internal caret stops, wider Unicode, script/language/feature inputs, and
multiple runs remain open. No shared render-contract fact is added.

## T3c evidence addendum — combining clusters and glyph offsets (2026-09-02)

The verdict is ADMIT/SPLIT, with no checklist closure. Oracle v3 admits one
bounded decomposed grammar in addition to v2: U+0301 COMBINING ACUTE ACCENT or
U+030B COMBINING DOUBLE ACUTE ACCENT may appear as the sole mark immediately
after one ASCII Latin letter. Leading marks, repeated marks, marks after a
non-letter or precomposed letter, every other combining scalar, and any
missing-glyph/fallback route remain typed refusals. Resolution preserves the
authored source and never normalizes it.

The artifact now states the cardinality and placement needed by that grammar.
Each cluster records source UTF-8, UTF-16, and scalar ranges plus its glyph
range. A direct cluster remains one scalar to one glyph. An admitted
base-plus-mark cluster is exactly two source scalars and either one composed
glyph or two glyphs; in the two-glyph form the second glyph has zero advance
and may carry the shaper's x/y displacement. The y-up shaping displacement is
converted once into the artifact's y-down local space. Every UTF-16 unit in
one cluster projects to the same complete Chromium character cell, matching
SVG's source-addressable query surface without inventing a caret inside a
glyph.

The composed positive witness is Allerta `Ae` + U+0301 + `Z` at 5120px.
Chromium and the artifact agree exactly on total advance 10340. The `e` and
mark are separate source scalars and UTF-16 units but both report start 3795,
end 7115, and length 3320; shaping composes them into glyph 141. The artifact
retains cluster source bytes `1..4`, UTF-16 units `1..3`, scalars `1..3`, and
glyphs `1..2`. Chromium's raster is byte-identical to precomposed `AéZ`, while
the unaccented control differs by 256 pixels at maximum channel delta 255
(measured, not celled; real-font pixels are not this oracle).

Two Bungee witnesses exercise attached glyphs at 1000px. Both `Ax` + mark +
`Z` runs have total advance 2127; Chromium exposes the `x` and its mark as two
addressable units sharing the 737-unit cell from 830 to 1567. The pinned font
facts place the mark at pen 1467 with x offset -369 and zero advance. U+0301
has local y offset 0; U+030B has local y offset -7, proving both placement
axes. Disabling mark attachment changes 3,488 pixels for U+0301 and 4,441 for
U+030B, both at maximum delta 255; substituting U+030B for U+0301 changes 1,800
pixels at delta 255. Stacking a second U+0301 and disabling mark-to-mark
placement changes 2,087 pixels at delta 255 (measured, not celled and not a
Chromium real-font pixel claim).

The new bounds check exposed a pre-existing artifact defect before landing.
The resolver called the font's stored glyph box a tight ink box, but that box
may enclose quadratic control points the outline never reaches. On the Bungee
acute witness it reported local top/height -965/965 while the streamed path's
actual extrema are -961.39019775390625/961.39019775390625. Bounds now consume
the same mapped outline stream as painting and solve both quadratic and cubic
Bézier extrema locally, preserving textlayout's Rustybuzz-only dependency
perimeter. Synthetic curve laws and every real-font witness require artifact
bounds to equal the streamed local path exactly.

The former all-combining refusal graduates. Three narrower register rows now
guard malformed admitted sequences, an admitted mark missing from the exact
declared face, and marks outside the two-scalar vocabulary. Strict refuses;
best effort skips and declares the same `svg/text[1]` node. PT Serif `fi`
continues to hold the merged-ligature/internal-caret boundary independently.

Sensitivity is direct. Temporarily replacing every shaped x offset with zero
left the primitive and Ahem gates green, then failed the committed Bungee fact
at `0` versus `-369`. Restoring the offset returned `just gate` to green. All
three committed sources also render through the actual CLI in strict and
best-effort modes with byte-identical policy outputs, and re-registering an
existing geometry id/source is refused as immutable.

The primitive corpus remains 1,051 Chromium-baked cells plus 16 sampled
frames. The exact Ahem estate remains nine cells. The real-font geometry
estate now has six witnesses: four Allerta and two Bungee. Replacing one broad
refusal with three bounded rows moves the named register from 209 to 211.
Ligatures and internal carets, general combining sequences, font fallback,
wider Unicode, script/language/feature inputs, multiple runs, and every
complete font/text grammar remain open. No shared render-contract fact is
added, no conformance score is produced, and no FLIP record changes.

## T4a evidence addendum — paint-only source runs (2026-09-03)

The verdict is ADMIT/SPLIT, with no checklist closure. Flat direct `<tspan>`
children may partition one `<text>` source only by opaque solid fill while
preserving the resolved face, size, direction, position, and effect envelope.
Whitespace collapse, shaping, and anchoring happen once over the complete
source. The producer then projects the resulting glyphs into ordinary path
nodes selected by opaque source-run tags; no text/run fact enters `rframe`.

The artifact validates exact ordered UTF-8 source-run coverage at Unicode
scalar boundaries before font work. Every cluster and glyph carries the tag
of the cluster's first source scalar, including when an authored run boundary
falls inside a combining cluster. This association is metadata only and does
not alter geometry, so the oracle remains v3.

The committed Allerta `ff` witness crosses a child paint boundary and retains
Chromium's exact 2330/2355 advances and 4685 total; independently shaping the
two runs would produce 2355/2355 and 4710. Bungee probes establish that a
mark-only child does not recolor an attached mark, while a child owning the
base recolors the whole cluster (measured, not celled). The exact Ahem cell
crosses parent/child paints, inherited paint, whitespace, and middle anchor.
Replacing every child paint with the parent changes 1,600 pixels at maximum
channel delta 197; anchoring runs independently changes 1,200 pixels at delta
218. An off-grid Allerta 1000px same-paint wrapper changes query geometry by
1/64 and 2,752 raster pixels, but that source is already refused by the
ratified query-grid boundary (measured, not celled).

Four focused rows replace the blanket child refusal: positioning and lists,
shaping-style changes, non-opaque/wider paint and effects, and nested content.
Both admissions stop at the same parent text node. Sensitivity was proved at
both seams: nominal per-glyph advances failed the geometry gate at 4710 versus
4685, and parent-paint-only projection failed the Ahem oracle by exactly 1,600
pixels. Restoring both returned the complete gate to green.

The text estate is now ten exact Ahem cells plus seven exact-number geometry
witnesses (five Allerta and two Bungee). Replacing one broad refusal with four
focused rows moves the named register from 211 to 214. Positioning, shaping
style, wider paint/effects, nesting, font selection, and the complete
`<text>`/`<tspan>` grammars remain open. No shared render-contract fact is
added, no conformance score is produced, and no FLIP record changes.

## T4b evidence addendum — positioned chunks (2026-09-03)

The verdict is ADMIT/SPLIT, with no checklist closure. Chromium establishes
two distinct operations inside one SVG text source: a consumed `<tspan>` `x`
or `y` value begins a new shaping and anchor chunk, while `dx` and `dy` alter
character placement without breaking shaping. Allerta `ff` totals 4710 across
an absolute reset and 4685 across a relative shift. Parent `text-anchor`
applies once to every absolute chunk. Omitted members preserve the running
position, and excess members are inert.

This geometry-producing distinction advances the text oracle to v4. The
producer supplies explicit shaping chunks as a complete ordered UTF-8
partition. Resolution shapes each partition independently and returns global
UTF-8, UTF-16, scalar, cluster, and glyph ranges plus canonical origin and
advance for every chunk. Coverage errors are typed and occur before font
selection. Source-run tags remain orthogonal metadata: a paint boundary may
cross a shaping chunk without creating one, while an absolute-position
boundary creates a chunk even when paint stays equal. Authored placement and
anchoring remain outside the oracle result, and no text fact enters `rframe`.

The exact Allerta witness resolves `fffe` + U+0301 + `Z` into byte chunks
`0..1` and `1..7`, with advances 2355 and 11230 and total 13585. Chromium and
the projection agree on starts 0, 5000, 8330, 10685/10685, and 15005. The
shared 10685 position is the base/mark character cell; a `dx=1000` assigned
to that middle combining scalar carries to `Z` rather than moving or splitting
the cluster. The exact Ahem cell separately grades the final outline pixels
for x/y resets, dx/dy lists, chunk anchoring, paint ownership, an integer
transform, canonical whitespace, and `<use>`.

Repeated absolute values, y-only chunks, negative positions, excess list
members, and percentage axis bases are measured, not celled. T4b admits only
complete unitless lists whose consumed values stay integral. Length and
percentage units, malformed lists, fractional positions, indexing across
discarded whitespace, and absolute positioning inside a combining cluster
remain stable refusals, as do parent lists, inherited positioning, `rotate`,
`textLength`, wider styles, effects, and nested text.

Removing first-character `dx` consumption made the complete gate fail the
Ahem oracle by 550 pixels; restoration returned it to green. The estate is now
eleven exact Ahem cells plus eight exact-number geometry witnesses (six
Allerta and two Bungee), and the named refusal register has 217 rows. The
complete text grammars remain open. No Chromium real-font raster claim,
conformance score, shared frame text fact, or FLIP change is made.
