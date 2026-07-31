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
One row per construct; ✅ means a Chromium-gated rung landed it, ⬜ is a
departure the compiler already names or surface not yet reached. **To *see*
what is done, open
[the visual corpus](../../../fixtures/web-first/STATUS.md)** — the generated
status view renders every baked cell as a thumbnail gallery (each image is
the committed Chromium oracle, which byte-exactness makes the engine's own
render too) with the refusal register beneath it. This list tracks
*position only*: semantics live in
[the statement of record](../../../crates/n0_cli/README.md), rung history in
[the D-N register](./svg-engine-of-record.md), and no score is computed from
it (FLIP is unratified). A rung's docs commit ticks its rows.

## Document & viewport

| | Construct | Notes |
| --- | --- | --- |
| ✅ | standalone SVG entry | strict XML, UTF-8, no DTD |
| ✅ | HTML entry | the document's first inline SVG |
| ✅ | root `width`/`height`/auto sizing | WxH as initial viewport |
| ✅ | `viewBox` | malformed refuses in both admissions |
| ✅ | `preserveAspectRatio` | full grammar; malformed refuses |
| ⬜ | percentage root sizing | |
| ⬜ | nested `<svg>` | |
| ⬜ | `<symbol>` | |
| ⬜ | `<switch>` | |
| ⬜ | `<foreignObject>` | |

## Structure & references

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `<g>` / `<a>` containers | |
| ✅ | `<defs>` | non-rendering, skipped by name |
| ✅ | `<use>` same-document | first-id-wins table, forward refs, `xlink:href`, x/y translate, cycle/unresolved → correct nothing |
| ⬜ | `<use>` under author CSS | shadow-scoped selector matching |
| ⬜ | `<use>` external references | |
| ⬜ | `<use>` of `<symbol>` / nested-`<svg>` targets | |

## Geometry

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `<rect>` | |
| ⬜ | `<rect>` `rx`/`ry` | rounded corners |
| ✅ | `<circle>` / `<ellipse>` | auto and negative radii |
| ✅ | `<line>` / `<polyline>` / `<polygon>` | points grammar; declared prefix divergence |
| ✅ | `<path>`: `M L H V C S Q T Z` + `fill-rule` | |
| ⬜ | `<path>`: `A` elliptical arc | needs the rframe conic amendment |
| ⬜ | `pathLength` | |
| ⬜ | CSS geometry properties | `x` `y` `cx` `cy` `r` `rx` `ry` `d` — Gecko-only at the Stylo pin |

## Transforms

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `transform` attribute | whole SVG grammar; malformed list drops whole |
| ✅ | CSS `transform` property + `-webkit-` alias | 2D affine set |
| ✅ | attribute↔CSS precedence | presentation-hint level |
| ⬜ | `transform-origin` / `transform-box` | |
| ⬜ | individual `rotate` / `translate` / `scale` properties | |
| ⬜ | beyond-2D function family | |
| ⬜ | root `<svg>` transform | |

## Paint

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `fill` | named/hex/rgb(a) sRGB, `none`, `currentColor`, inherit |
| ✅ | `color` | the currentColor basis |
| ✅ | `fill-opacity` / `stroke-opacity` / color alpha | float multiply, one quantize |
| ⬜ | beyond-sRGB color spaces | |
| ⬜ | `paint-order` | |
| ⬜ | `context-fill` / `context-stroke` | |

## Paint servers

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `<linearGradient>` | |
| ✅ | `<radialGradient>` | concentric; focal `fx`/`fy`/`fr` refuse by name |
| ✅ | `<stop>` | `offset`, `stop-color`, `stop-opacity` — attribute reads; author CSS on stops refuses |
| ✅ | `gradientUnits` / `gradientTransform` / `spreadMethod` | |
| ✅ | `href` inheritance between gradients | cycles → correct nothing |
| ✅ | `fill`/`stroke` `url(#…)` with fallback | |
| ⬜ | `<pattern>` | |

## Stroke

| | Construct | Notes |
| --- | --- | --- |
| ✅ | solid stroke paint | |
| ✅ | `stroke-width` | cascaded length; percentage against the viewport diagonal |
| ✅ | `stroke-linecap` / `stroke-linejoin` / `stroke-miterlimit` | |
| ⬜ | `stroke-dasharray` / `stroke-dashoffset` | |
| ⬜ | `vector-effect` | |

## Compositing & clipping

| | Construct | Notes |
| --- | --- | --- |
| ⬜ | element `opacity` | needs the compositing scope |
| ⬜ | `clip-path` | |
| ⬜ | `mask` | |
| ⬜ | `<filter>` / `filter` | |
| ⬜ | `mix-blend-mode` / `isolation` | |

## Disposition

| | Construct | Notes |
| --- | --- | --- |
| ✅ | `display: none` | attribute and CSS; root ignores it, as Chromium does |
| ✅ | `visibility` | descendant `visible` un-hides |
| ⬜ | `display: contents` | |

## CSS integration

| | Construct | Notes |
| --- | --- | --- |
| ✅ | one Stylo cascade | `<style>` elements, `style` attributes, presentation hints (13 admitted) |
| ✅ | SVG-namespace stylesheet intake | |
| ⬜ | shadow-scoped matching | author CSS + `<use>` |
| ⬜ | longhands the pinned cascade cannot represent | `stop-color`, `marker-*`, `paint-order`, `d`, … |

## Animation

| | Construct | Notes |
| --- | --- | --- |
| ✅ | the time axis | Base / exact signed-nanosecond Sample, no ambient clock |
| ✅ | SMIL `animate` on rect `x` | the proving slice |
| ⬜ | wider animated attributes | `y` `width` `height` `cx` `cy` `r` … |
| ⬜ | `animateTransform` | |
| ⬜ | CSS animations / transitions sampling | |
| ⬜ | beyond-slice SMIL | `<set>`, href retargets |

## Markers

| | Construct | Notes |
| --- | --- | --- |
| ⬜ | `marker-start` / `marker-mid` / `marker-end` | |

## Text

| | Construct | Notes |
| --- | --- | --- |
| ⬜ | `<text>` | all of it |

## Images & resources

| | Construct | Notes |
| --- | --- | --- |
| ⬜ | `<image>` | |
| ⬜ | external resources | self-contained input is the current contract |

## Layout (HTML)

| | Construct | Notes |
| --- | --- | --- |
| ⬜ | any HTML box layout | the surrounding page contributes nothing — pinned |
