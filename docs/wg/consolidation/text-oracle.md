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

**Status: proposed.** This brief is the text-0 milestone: it puts the *method*
for gating text on the SVG engine of record (`websem → rframe → n0`) before
the owner, with the Chromium probe record that grounds it. It decides no code
and touches no open decision — the D-M shaped-text stage in
[the n0 join point](./n0-join-point.md) stays deliberately undecided, and
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

Rung A is the only rung text-0 asks to ratify. Rung B is named now so the
real-font milestone (text-3+) has a destination; it is not exercised. Rung C
exists so nobody reinvents it informally.

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

Everything outside the domain **refuses by name at a stable node path** — the
standing law, now applied to numbers. Chromium's snapping behavior is a
declared divergence-by-refusal: Chromium renders a snapped box; the engine
names the construct instead. Over-refusal beats codifying a rasterizer's
rounding.

**The oracle posture declaration.** Text fixtures carry
`-webkit-font-smoothing: none` on the text element as **bake posture**, the
way they already carry width/height as viewport posture. It suppresses the one
rasterizer behavior (macOS smoothing dilation) that breaks coincidence; on
non-macOS Chromium it is a no-op and the raster already coincides. Within the
admitted domain the declaration is *semantically empty for the engine* — AA
and bilevel raster are byte-identical there — so no quality flag becomes
readable from `websem`, `rframe`, or resolve: the one-meaning tripwire holds.
Text-1 must verify (and pin in the corpus) that websem's strict admission
treats the declaration as standard CSS unknown-property discard rather than a
refusal.

**Why the bake stays maintainer-portable.** The committed cell is the
contract; the bake harness already fails loudly on a differing re-capture
rather than blessing a new baseline. Because the admitted domain makes every
rasterizer's output coincide, a re-bake on FreeType or DirectWrite Chromium is
expected to reproduce the committed bytes — and if one ever does not, the
harness refuses loudly instead of drifting.

## The hermetic font environment

Per [the text-layout RFD](../feat-paragraph/text-layout.md), the resolution
environment is a manifest, not an ambient promise, and a family name is not a
font identity. Applied to the engine of record:

- A render's fonts are **declared inputs pinned by content hash**. Proposed
  n0_cli surface: a repeatable `--font FAMILY=PATH` (exact grammar owned by
  [the statement of record](../../../crates/n0_cli/README.md) when text-1
  lands).
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
4. **The hermetic environment surface** — empty by default, `--font`-declared,
   content-hash identity.
5. **The corpus-growth law.**
6. **The lowering shape for text-1: outlines first, no rframe amendment.**
   Resolved glyphs lower to the contract's existing path facts (`rframe`
   refuses resource references, and a glyph run referencing a font key is
   one). The shaped-text *fact* enters `rframe` only if the D-M deciding
   spike says the join is high — that spike, per
   [the join point](./n0-join-point.md), runs once the Web family's producer
   exists, i.e. after text-1.
7. **The resolver's home**: a new backend-free workspace crate implementing
   the RFD's minimal profile (proposed name `textlayout`; shaping via
   rustybuzz + ttf-parser, both already in the lock via vendored usvg), whose
   identity-by-refusal is: owns shaping, breaking, metrics, and the resolved
   artifact; refuses font discovery, paint, clocks, and estimates — with an
   architecture test forbidding ambient font access.

What this brief does **not** decide: the D-M shaped-text join, rung C, and any
real-font method detail beyond naming rung B as its destination.
