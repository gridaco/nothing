---
title: "SVG checklist"
description: "The compact rung tracker: the SVG surface by section, each item checked when a Chromium-gated rung lands it on the n0 path."
tags:
  - internal
  - wg
  - program
  - consolidation
format: md
---

# SVG checklist

The compact tracker for the SVG engine of record (`websem → rframe → n0`).
One line per construct; a checked box means a Chromium-gated rung landed it.
An unchecked box is a departure the compiler already names (see
[the refusal register](../../../fixtures/web-first/STATUS.md)) or surface not
yet reached. This list tracks *position only*: semantics live in
[the statement of record](../../../crates/n0_cli/README.md), rung history in
[the D-N register](./svg-engine-of-record.md), and no score is computed from
it (FLIP is unratified). A rung's docs commit ticks its rows.

## Document & viewport

- [x] standalone SVG entry (strict XML, UTF-8, no DTD)
- [x] HTML entry (the document's first inline SVG)
- [x] root `width`/`height`/auto sizing (WxH as initial viewport)
- [x] `viewBox` (malformed refuses in both admissions)
- [x] `preserveAspectRatio` (full grammar; malformed refuses)
- [ ] percentage root sizing
- [ ] nested `<svg>`
- [ ] `<symbol>`
- [ ] `<switch>`
- [ ] `<foreignObject>`

## Structure & references

- [x] `<g>` / `<a>` containers
- [x] `<defs>` (non-rendering, skipped by name)
- [x] `<use>` same-document (first-id-wins table, forward refs, `xlink:href`, x/y translate, cycle/unresolved → correct nothing)
- [ ] `<use>` under author CSS (shadow-scoped selector matching)
- [ ] `<use>` external references
- [ ] `<use>` of `<symbol>` / nested-`<svg>` targets

## Geometry

- [x] `<rect>`
- [ ] `<rect>` `rx`/`ry` (rounded corners)
- [x] `<circle>` / `<ellipse>` (auto and negative radii)
- [x] `<line>` / `<polyline>` / `<polygon>` (points grammar; declared prefix divergence)
- [x] `<path>`: `M L H V C S Q T Z` + `fill-rule`
- [ ] `<path>`: `A` elliptical arc (needs the rframe conic amendment)
- [ ] `pathLength`
- [ ] CSS geometry properties (`x` `y` `cx` `cy` `r` `rx` `ry` `d` — Gecko-only at the Stylo pin)

## Transforms

- [x] `transform` attribute (whole SVG grammar; malformed list drops whole)
- [x] CSS `transform` property + `-webkit-` alias (2D affine set)
- [x] attribute↔CSS precedence (presentation-hint level)
- [ ] `transform-origin` / `transform-box`
- [ ] individual `rotate` / `translate` / `scale` properties
- [ ] beyond-2D function family
- [ ] root `<svg>` transform

## Paint

- [x] `fill`: named/hex/rgb(a) sRGB, `none`, `currentColor`, inherit
- [x] `color` (the currentColor basis)
- [x] `fill-opacity` / `stroke-opacity` / color alpha (float multiply, one quantize)
- [ ] beyond-sRGB color spaces
- [ ] `paint-order`
- [ ] `context-fill` / `context-stroke`

## Paint servers

- [x] `<linearGradient>`
- [x] `<radialGradient>` (concentric; focal `fx`/`fy`/`fr` refuse by name)
- [x] `<stop>` (`offset`, `stop-color`, `stop-opacity` — attribute reads; author CSS on stops refuses)
- [x] `gradientUnits` / `gradientTransform` / `spreadMethod`
- [x] `href` inheritance between gradients (cycles → correct nothing)
- [x] `fill`/`stroke` `url(#…)` with fallback
- [ ] `<pattern>`

## Stroke

- [x] solid stroke paint
- [x] `stroke-width` (cascaded length; percentage against the viewport diagonal)
- [x] `stroke-linecap` / `stroke-linejoin` / `stroke-miterlimit`
- [ ] `stroke-dasharray` / `stroke-dashoffset`
- [ ] `vector-effect`

## Compositing & clipping

- [ ] element `opacity` (needs the compositing scope)
- [ ] `clip-path`
- [ ] `mask`
- [ ] `<filter>` / `filter`
- [ ] `mix-blend-mode` / `isolation`

## Disposition

- [x] `display: none` (attribute and CSS; root ignores it, as Chromium does)
- [x] `visibility` (descendant `visible` un-hides)
- [ ] `display: contents`

## CSS integration

- [x] one Stylo cascade: `<style>` elements, `style` attributes, presentation hints (13 admitted)
- [x] SVG-namespace stylesheet intake
- [ ] shadow-scoped matching (author CSS + `<use>`)
- [ ] longhands the pinned cascade cannot represent (`stop-color`, `marker-*`, `paint-order`, `d`, …)

## Animation

- [x] the time axis: Base / exact signed-nanosecond Sample, no ambient clock
- [x] SMIL `animate` on rect `x` (the proving slice)
- [ ] wider animated attributes (`y` `width` `height` `cx` `cy` `r` …)
- [ ] `animateTransform`
- [ ] CSS animations / transitions sampling
- [ ] beyond-slice SMIL (`<set>`, href retargets)

## Markers

- [ ] `marker-start` / `marker-mid` / `marker-end`

## Text

- [ ] `<text>` (all of it)

## Images & resources

- [ ] `<image>`
- [ ] external resources (self-contained input is the current contract)

## Layout (HTML)

- [ ] any HTML box layout (the surrounding page contributes nothing — pinned)
