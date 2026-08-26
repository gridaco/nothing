---
title: "Web checklist"
description: "The master rung tracker: the HTML, CSS, and SVG surface as bare spec-vocabulary items, each checked when a Chromium-gated rung lands it on the n0 path."
tags:
  - internal
  - wg
  - program
  - consolidation
format: md
---

# Web checklist

One master tracker for the Web surface of the n0 path
(`websem → rframe → n0`) — the SVG engine of record, with the HTML and CSS
surface ahead of it — HTML, CSS, and SVG in one document, because the three
share one cascade and half a vocabulary. Items only, in the platform's
own words, enumerated from the external spec indexes linked at each section
head — never from this tree. A box ticks only when a Chromium-gated rung
lands the named construct at its full listed grammar — where cited
references disagree, the standard-track grammar is the bar, and an
experimental draft grammar does not raise it; a partial admission
stays unchecked — the remaining grammar is remaining work, and a construct's
finer-grained gaps live in their own rows (`<rect>` is landed; `rx`/`ry` are
not). A row ticks only for its own section's index entry: an attribute
spelling and its CSS property twin are separate rows, each ticked on its own
docs-commit evidence. A rung's docs commit ticks its rows. **To *see* what is done, open
[the visual corpus](../../../fixtures/web-first/STATUS.md)** — semantics live
in [the statement of record](../../../crates/n0_cli/README.md), rung history
in [the D-N register](./svg-engine-of-record.md), and no score is computed
from this list (FLIP is unratified).


## HTML

Source: [the WHATWG HTML Living Standard indexes](https://html.spec.whatwg.org/multipage/indices.html) — current
standard surface; obsolete and non-conforming features excluded.


### HTML elements

- [ ] `<a>`
- [ ] `<abbr>`
- [ ] `<address>`
- [ ] `<area>`
- [ ] `<article>`
- [ ] `<aside>`
- [ ] `<audio>`
- [ ] `<b>`
- [ ] `<base>`
- [ ] `<bdi>`
- [ ] `<bdo>`
- [ ] `<blockquote>`
- [ ] `<body>`
- [ ] `<br>`
- [ ] `<button>`
- [ ] `<canvas>`
- [ ] `<caption>`
- [ ] `<cite>`
- [ ] `<code>`
- [ ] `<col>`
- [ ] `<colgroup>`
- [ ] `<data>`
- [ ] `<datalist>`
- [ ] `<dd>`
- [ ] `<del>`
- [ ] `<details>`
- [ ] `<dfn>`
- [ ] `<dialog>`
- [ ] `<div>`
- [ ] `<dl>`
- [ ] `<dt>`
- [ ] `<em>`
- [ ] `<embed>`
- [ ] `<fieldset>`
- [ ] `<figcaption>`
- [ ] `<figure>`
- [ ] `<footer>`
- [ ] `<form>`
- [ ] `<h1>`
- [ ] `<h2>`
- [ ] `<h3>`
- [ ] `<h4>`
- [ ] `<h5>`
- [ ] `<h6>`
- [ ] `<head>`
- [ ] `<header>`
- [ ] `<hgroup>`
- [ ] `<hr>`
- [ ] `<html>`
- [ ] `<i>`
- [ ] `<iframe>`
- [ ] `<img>`
- [ ] `<input>`
- [ ] `<ins>`
- [ ] `<kbd>`
- [ ] `<label>`
- [ ] `<legend>`
- [ ] `<li>`
- [ ] `<link>`
- [ ] `<main>`
- [ ] `<map>`
- [ ] `<mark>`
- [ ] `<math>`
- [ ] `<menu>`
- [ ] `<meta>`
- [ ] `<meter>`
- [ ] `<nav>`
- [ ] `<noscript>`
- [ ] `<object>`
- [ ] `<ol>`
- [ ] `<optgroup>`
- [ ] `<option>`
- [ ] `<output>`
- [ ] `<p>`
- [ ] `<picture>`
- [ ] `<pre>`
- [ ] `<progress>`
- [ ] `<q>`
- [ ] `<rp>`
- [ ] `<rt>`
- [ ] `<ruby>`
- [ ] `<s>`
- [ ] `<samp>`
- [ ] `<script>`
- [ ] `<search>`
- [ ] `<section>`
- [ ] `<select>`
- [ ] `<selectedcontent>`
- [ ] `<slot>`
- [ ] `<small>`
- [ ] `<source>`
- [ ] `<span>`
- [ ] `<strong>`
- [ ] `<style>`
- [ ] `<sub>`
- [ ] `<summary>`
- [ ] `<sup>`
- [ ] `<svg>`
- [ ] `<table>`
- [ ] `<tbody>`
- [ ] `<td>`
- [ ] `<template>`
- [ ] `<textarea>`
- [ ] `<tfoot>`
- [ ] `<th>`
- [ ] `<thead>`
- [ ] `<time>`
- [ ] `<title>`
- [ ] `<tr>`
- [ ] `<track>`
- [ ] `<u>`
- [ ] `<ul>`
- [ ] `<var>`
- [ ] `<video>`
- [ ] `<wbr>`
- [ ] `autonomous custom elements`


### HTML global attributes

- [ ] `accesskey`
- [ ] `aria-*`
- [ ] `autocapitalize`
- [ ] `autocorrect`
- [ ] `autofocus`
- [ ] `class`
- [ ] `contenteditable`
- [ ] `data-*`
- [ ] `dir`
- [ ] `draggable`
- [ ] `enterkeyhint`
- [ ] `headingoffset`
- [ ] `headingreset`
- [ ] `hidden`
- [ ] `id`
- [ ] `inert`
- [ ] `inputmode`
- [ ] `is`
- [ ] `itemid`
- [ ] `itemprop`
- [ ] `itemref`
- [ ] `itemscope`
- [ ] `itemtype`
- [ ] `lang`
- [ ] `nonce`
- [ ] `popover`
- [ ] `role`
- [ ] `slot`
- [ ] `spellcheck`
- [ ] `style`
- [ ] `tabindex`
- [ ] `title`
- [ ] `translate`
- [ ] `writingsuggestions`
- [ ] `event handler content attributes (on*)`


### HTML attributes

- [ ] `abbr`
- [ ] `accept`
- [ ] `accept-charset`
- [ ] `action`
- [ ] `allow`
- [ ] `allowfullscreen`
- [ ] `alpha`
- [ ] `alt`
- [ ] `as`
- [ ] `async`
- [ ] `autocomplete`
- [ ] `autoplay`
- [ ] `blocking`
- [ ] `charset`
- [ ] `checked`
- [ ] `cite`
- [ ] `closedby`
- [ ] `color`
- [ ] `colorspace`
- [ ] `cols`
- [ ] `colspan`
- [ ] `command`
- [ ] `commandfor`
- [ ] `content`
- [ ] `controls`
- [ ] `coords`
- [ ] `crossorigin`
- [ ] `data`
- [ ] `datetime`
- [ ] `decoding`
- [ ] `default`
- [ ] `defer`
- [ ] `dir`
- [ ] `dirname`
- [ ] `disabled`
- [ ] `download`
- [ ] `enctype`
- [ ] `fetchpriority`
- [ ] `for`
- [ ] `form`
- [ ] `formaction`
- [ ] `formenctype`
- [ ] `formmethod`
- [ ] `formnovalidate`
- [ ] `formtarget`
- [ ] `headers`
- [ ] `height`
- [ ] `high`
- [ ] `href`
- [ ] `hreflang`
- [ ] `http-equiv`
- [ ] `imagesizes`
- [ ] `imagesrcset`
- [ ] `integrity`
- [ ] `ismap`
- [ ] `kind`
- [ ] `label`
- [ ] `list`
- [ ] `loading`
- [ ] `loop`
- [ ] `low`
- [ ] `max`
- [ ] `maxlength`
- [ ] `media`
- [ ] `method`
- [ ] `min`
- [ ] `minlength`
- [ ] `multiple`
- [ ] `muted`
- [ ] `name`
- [ ] `nomodule`
- [ ] `novalidate`
- [ ] `open`
- [ ] `optimum`
- [ ] `pattern`
- [ ] `ping`
- [ ] `placeholder`
- [ ] `playsinline`
- [ ] `popovertarget`
- [ ] `popovertargetaction`
- [ ] `poster`
- [ ] `preload`
- [ ] `readonly`
- [ ] `referrerpolicy`
- [ ] `rel`
- [ ] `required`
- [ ] `reversed`
- [ ] `rows`
- [ ] `rowspan`
- [ ] `sandbox`
- [ ] `scope`
- [ ] `selected`
- [ ] `shadowrootclonable`
- [ ] `shadowrootcustomelementregistry`
- [ ] `shadowrootdelegatesfocus`
- [ ] `shadowrootmode`
- [ ] `shadowrootserializable`
- [ ] `shadowrootslotassignment`
- [ ] `shape`
- [ ] `size`
- [ ] `sizes`
- [ ] `span`
- [ ] `src`
- [ ] `srcdoc`
- [ ] `srclang`
- [ ] `srcset`
- [ ] `start`
- [ ] `step`
- [ ] `target`
- [ ] `title`
- [ ] `type`
- [ ] `usemap`
- [ ] `value`
- [ ] `width`
- [ ] `wrap`


## CSS

Sources: [the W3C CSS property index](https://www.w3.org/Style/CSS/all-properties.en.html) and
[the MDN CSS reference](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference) —
standard-track, non-experimental surface; vendor-prefixed forms
excluded.


### CSS custom properties for cascading variables

- [ ] `--*`


### CSS cascading and inheritance

- [ ] `all`


### CSS box model

- [ ] `margin`
- [ ] `margin-top`
- [ ] `margin-right`
- [ ] `margin-bottom`
- [ ] `margin-left`
- [ ] `padding`
- [ ] `padding-top`
- [ ] `padding-right`
- [ ] `padding-bottom`
- [ ] `padding-left`


### CSS box sizing

- [ ] `width`
- [ ] `height`
- [ ] `min-width`
- [ ] `min-height`
- [ ] `max-width`
- [ ] `max-height`
- [ ] `aspect-ratio`
- [ ] `box-sizing`
- [ ] `contain-intrinsic-size`
- [ ] `contain-intrinsic-width`
- [ ] `contain-intrinsic-height`
- [ ] `contain-intrinsic-block-size`
- [ ] `contain-intrinsic-inline-size`


### CSS display

- [ ] `display`
- [ ] `order`
- [ ] `visibility`


### CSS positioned layout

- [ ] `position`
- [ ] `top`
- [ ] `right`
- [ ] `bottom`
- [ ] `left`
- [ ] `inset`
- [ ] `inset-block`
- [ ] `inset-block-start`
- [ ] `inset-block-end`
- [ ] `inset-inline`
- [ ] `inset-inline-start`
- [ ] `inset-inline-end`
- [ ] `float`
- [ ] `clear`
- [ ] `z-index`


### CSS logical properties and values

- [ ] `block-size`
- [ ] `inline-size`
- [ ] `min-block-size`
- [ ] `min-inline-size`
- [ ] `max-block-size`
- [ ] `max-inline-size`
- [ ] `margin-block`
- [ ] `margin-block-start`
- [ ] `margin-block-end`
- [ ] `margin-inline`
- [ ] `margin-inline-start`
- [ ] `margin-inline-end`
- [ ] `padding-block`
- [ ] `padding-block-start`
- [ ] `padding-block-end`
- [ ] `padding-inline`
- [ ] `padding-inline-start`
- [ ] `padding-inline-end`
- [ ] `border-block`
- [ ] `border-block-start`
- [ ] `border-block-end`
- [ ] `border-block-color`
- [ ] `border-block-start-color`
- [ ] `border-block-end-color`
- [ ] `border-block-style`
- [ ] `border-block-start-style`
- [ ] `border-block-end-style`
- [ ] `border-block-width`
- [ ] `border-block-start-width`
- [ ] `border-block-end-width`
- [ ] `border-inline`
- [ ] `border-inline-start`
- [ ] `border-inline-end`
- [ ] `border-inline-color`
- [ ] `border-inline-start-color`
- [ ] `border-inline-end-color`
- [ ] `border-inline-style`
- [ ] `border-inline-start-style`
- [ ] `border-inline-end-style`
- [ ] `border-inline-width`
- [ ] `border-inline-start-width`
- [ ] `border-inline-end-width`
- [ ] `border-start-start-radius`
- [ ] `border-start-end-radius`
- [ ] `border-end-start-radius`
- [ ] `border-end-end-radius`


### CSS flexible box layout

- [ ] `flex`
- [ ] `flex-basis`
- [ ] `flex-direction`
- [ ] `flex-flow`
- [ ] `flex-grow`
- [ ] `flex-shrink`
- [ ] `flex-wrap`


### CSS grid layout

- [ ] `grid`
- [ ] `grid-area`
- [ ] `grid-auto-columns`
- [ ] `grid-auto-flow`
- [ ] `grid-auto-rows`
- [ ] `grid-column`
- [ ] `grid-column-start`
- [ ] `grid-column-end`
- [ ] `grid-row`
- [ ] `grid-row-start`
- [ ] `grid-row-end`
- [ ] `grid-template`
- [ ] `grid-template-areas`
- [ ] `grid-template-columns`
- [ ] `grid-template-rows`


### CSS box alignment

- [ ] `align-content`
- [ ] `align-items`
- [ ] `align-self`
- [ ] `justify-content`
- [ ] `justify-items`
- [ ] `justify-self`
- [ ] `place-content`
- [ ] `place-items`
- [ ] `place-self`
- [ ] `gap`
- [ ] `row-gap`
- [ ] `column-gap`


### CSS multi-column layout

- [ ] `columns`
- [ ] `column-count`
- [ ] `column-width`
- [ ] `column-fill`
- [ ] `column-span`
- [ ] `column-rule`
- [ ] `column-rule-color`
- [ ] `column-rule-style`
- [ ] `column-rule-width`


### CSS table

- [ ] `border-collapse`
- [ ] `border-spacing`
- [ ] `caption-side`
- [ ] `empty-cells`
- [ ] `table-layout`


### CSS fragmentation

- [ ] `box-decoration-break`
- [ ] `break-after`
- [ ] `break-before`
- [ ] `break-inside`
- [ ] `orphans`
- [ ] `widows`


### CSS paged media

- [ ] `page`


### CSS backgrounds and borders

- [ ] `background`
- [ ] `background-attachment`
- [ ] `background-clip`
- [ ] `background-color`
- [ ] `background-image`
- [ ] `background-origin`
- [ ] `background-position`
- [ ] `background-position-x`
- [ ] `background-position-y`
- [ ] `background-repeat`
- [ ] `background-size`
- [ ] `border`
- [ ] `border-color`
- [ ] `border-style`
- [ ] `border-width`
- [ ] `border-top`
- [ ] `border-top-color`
- [ ] `border-top-style`
- [ ] `border-top-width`
- [ ] `border-right`
- [ ] `border-right-color`
- [ ] `border-right-style`
- [ ] `border-right-width`
- [ ] `border-bottom`
- [ ] `border-bottom-color`
- [ ] `border-bottom-style`
- [ ] `border-bottom-width`
- [ ] `border-left`
- [ ] `border-left-color`
- [ ] `border-left-style`
- [ ] `border-left-width`
- [ ] `border-radius`
- [ ] `border-top-left-radius`
- [ ] `border-top-right-radius`
- [ ] `border-bottom-right-radius`
- [ ] `border-bottom-left-radius`
- [ ] `border-image`
- [ ] `border-image-source`
- [ ] `border-image-slice`
- [ ] `border-image-width`
- [ ] `border-image-outset`
- [ ] `border-image-repeat`
- [ ] `box-shadow`


### CSS color

- [ ] `color`
- [ ] `color-scheme`
- [ ] `dynamic-range-limit`
- [ ] `forced-color-adjust`
- [x] `opacity`
- [ ] `print-color-adjust`


### CSS images

- [ ] `image-orientation`
- [ ] `image-rendering`
- [ ] `object-fit`
- [ ] `object-position`


### CSS shapes

- [ ] `shape-outside`
- [ ] `shape-margin`
- [ ] `shape-image-threshold`


### CSS masking

- [ ] `clip-path`
- [ ] `clip-rule`

> **2026-08-24 split:** the URL/`none` part of `clip-path` now enters through
> the pinned cascade on non-root SVG targets, including presentation hints,
> inline style, stylesheets, the `-webkit-` alias, `var()`, and normal cascade
> precedence. The row stays open for basic shapes, geometry boxes, the root
> CSS-layer route, external and cyclic resources, and Chromium's raster-mask
> strategies. The `clip-rule` property stays open because this Servo-mode
> Stylo pin has no such longhand; its CSS ingresses are quarantined rather
> than matched outside the cascade. The presentation-attribute evidence is
> recorded below.

- [ ] `mask`
- [ ] `mask-image`
- [ ] `mask-mode`
- [ ] `mask-position`
- [ ] `mask-size`
- [ ] `mask-repeat`
- [ ] `mask-origin`
- [ ] `mask-clip`
- [ ] `mask-composite`
- [ ] `mask-type`
- [ ] `mask-border`
- [ ] `mask-border-source`
- [ ] `mask-border-mode`
- [ ] `mask-border-slice`
- [ ] `mask-border-width`
- [ ] `mask-border-outset`
- [ ] `mask-border-repeat`

> **2026-08-24 split:** same-document SVG image masks now render through the
> direct presentation-attribute route described below. Every CSS mask-family
> row remains open: this Servo-mode Stylo pin furnishes no computed mask route
> the compiler can consume, so authored declarations are quarantined by name
> across style attributes, stylesheets, shorthands, longhands, border
> longhands, and the `-webkit-mask-image` alias. No matcher was added around
> the cascade. `mask-border-mode`, present in CSS Masking Level 1, is added to
> the checklist here; it was missing from the earlier enumeration.


### Filter effects

- [ ] `filter`
- [ ] `backdrop-filter`
- [ ] `color-interpolation-filters`
- [ ] `flood-color`
- [ ] `flood-opacity`
- [ ] `lighting-color`

> **2026-08-25 split:** same-document SVG resource filters now enter through
> the direct presentation-attribute route recorded below. The CSS `filter`
> row stays open: the pinned Servo-mode cascade represents filter functions
> but not the URL variant needed by an SVG resource, and function lists are a
> separate unresolved operation grammar. Authored declarations are therefore
> quarantined by name rather than matched by a second cascade. The other four
> filter-effect properties are Gecko-only at this Stylo pin and also remain
> open. The direct `feFlood` attribute route now carries an admitted subset,
> but it does not create a CSS computed-value route and does not change these
> property rows.


### Compositing and blending

- [ ] `mix-blend-mode`
- [ ] `background-blend-mode`
- [ ] `isolation`


### CSS fonts

- [ ] `font`
- [ ] `font-family`
- [ ] `font-size`
- [ ] `font-size-adjust`
- [ ] `font-stretch`
- [ ] `font-style`
- [ ] `font-weight`
- [ ] `font-feature-settings`
- [ ] `font-variation-settings`
- [ ] `font-kerning`
- [ ] `font-language-override`
- [ ] `font-optical-sizing`
- [ ] `font-palette`
- [ ] `font-synthesis`
- [ ] `font-synthesis-small-caps`
- [ ] `font-synthesis-style`
- [ ] `font-synthesis-weight`
- [ ] `font-variant`
- [ ] `font-variant-alternates`
- [ ] `font-variant-caps`
- [ ] `font-variant-east-asian`
- [ ] `font-variant-emoji`
- [ ] `font-variant-ligatures`
- [ ] `font-variant-numeric`
- [ ] `font-variant-position`


### CSS text

- [ ] `hanging-punctuation`
- [ ] `hyphenate-character`
- [ ] `hyphenate-limit-chars`
- [ ] `hyphens`
- [ ] `letter-spacing`
- [ ] `line-break`
- [ ] `overflow-wrap`
- [ ] `tab-size`
- [ ] `text-align`
- [ ] `text-align-last`
- [ ] `text-autospace`
- [ ] `text-indent`
- [ ] `text-justify`
- [ ] `text-transform`
- [ ] `text-wrap`
- [ ] `text-wrap-mode`
- [ ] `text-wrap-style`
- [ ] `white-space`
- [ ] `white-space-collapse`
- [ ] `word-break`
- [ ] `word-spacing`


### CSS text decoration

- [ ] `text-decoration`
- [ ] `text-decoration-line`
- [ ] `text-decoration-color`
- [ ] `text-decoration-style`
- [ ] `text-decoration-thickness`
- [ ] `text-decoration-skip-ink`
- [ ] `text-underline-offset`
- [ ] `text-underline-position`
- [ ] `text-emphasis`
- [ ] `text-emphasis-color`
- [ ] `text-emphasis-position`
- [ ] `text-emphasis-style`
- [ ] `text-shadow`


### CSS writing modes

- [ ] `direction`
- [ ] `text-combine-upright`
- [ ] `text-orientation`
- [ ] `unicode-bidi`
- [ ] `writing-mode`


### CSS inline layout

- [ ] `alignment-baseline`
- [ ] `baseline-shift`
- [ ] `baseline-source`
- [ ] `dominant-baseline`
- [ ] `initial-letter`
- [ ] `line-height`
- [ ] `text-box`
- [ ] `text-box-edge`
- [ ] `text-box-trim`
- [ ] `vertical-align`


### CSS ruby annotation layout

- [ ] `ruby-align`
- [ ] `ruby-overhang`
- [ ] `ruby-position`


### CSS lists and counters

- [ ] `list-style`
- [ ] `list-style-image`
- [ ] `list-style-position`
- [ ] `list-style-type`
- [ ] `counter-increment`
- [ ] `counter-reset`
- [ ] `counter-set`


### CSS generated content

- [ ] `content`
- [ ] `quotes`


### CSS overflow

- [ ] `overflow`
- [ ] `overflow-x`
- [ ] `overflow-y`
- [ ] `overflow-block`
- [ ] `overflow-inline`
- [ ] `overflow-clip-margin`
- [ ] `text-overflow`
- [ ] `line-clamp`
- [ ] `scroll-behavior`
- [ ] `scrollbar-gutter`


### CSS scroll snap

- [ ] `scroll-snap-type`
- [ ] `scroll-snap-align`
- [ ] `scroll-snap-stop`
- [ ] `scroll-margin`
- [ ] `scroll-margin-top`
- [ ] `scroll-margin-right`
- [ ] `scroll-margin-bottom`
- [ ] `scroll-margin-left`
- [ ] `scroll-margin-block`
- [ ] `scroll-margin-block-start`
- [ ] `scroll-margin-block-end`
- [ ] `scroll-margin-inline`
- [ ] `scroll-margin-inline-start`
- [ ] `scroll-margin-inline-end`
- [ ] `scroll-padding`
- [ ] `scroll-padding-top`
- [ ] `scroll-padding-right`
- [ ] `scroll-padding-bottom`
- [ ] `scroll-padding-left`
- [ ] `scroll-padding-block`
- [ ] `scroll-padding-block-start`
- [ ] `scroll-padding-block-end`
- [ ] `scroll-padding-inline`
- [ ] `scroll-padding-inline-start`
- [ ] `scroll-padding-inline-end`


### CSS scroll anchoring

- [ ] `overflow-anchor`


### CSS overscroll behavior

- [ ] `overscroll-behavior`
- [ ] `overscroll-behavior-x`
- [ ] `overscroll-behavior-y`
- [ ] `overscroll-behavior-block`
- [ ] `overscroll-behavior-inline`


### CSS scrollbars styling

- [ ] `scrollbar-color`
- [ ] `scrollbar-width`


### CSS transforms

- [ ] `transform`
- [ ] `transform-box`
- [ ] `transform-origin`
- [ ] `transform-style`
- [ ] `translate`
- [ ] `rotate`
- [ ] `scale`
- [ ] `perspective`
- [ ] `perspective-origin`
- [ ] `backface-visibility`


### CSS motion path

- [ ] `offset`
- [ ] `offset-anchor`
- [ ] `offset-distance`
- [ ] `offset-path`
- [ ] `offset-position`
- [ ] `offset-rotate`


### CSS transitions

- [ ] `transition`
- [ ] `transition-behavior`
- [ ] `transition-delay`
- [ ] `transition-duration`
- [ ] `transition-property`
- [ ] `transition-timing-function`


### CSS animations

- [ ] `animation`
- [ ] `animation-composition`
- [ ] `animation-delay`
- [ ] `animation-direction`
- [ ] `animation-duration`
- [ ] `animation-fill-mode`
- [ ] `animation-iteration-count`
- [ ] `animation-name`
- [ ] `animation-play-state`
- [ ] `animation-timeline`
- [ ] `animation-timing-function`


### CSS scroll-driven animations

- [ ] `animation-range`
- [ ] `animation-range-start`
- [ ] `animation-range-end`
- [ ] `scroll-timeline`
- [ ] `scroll-timeline-axis`
- [ ] `scroll-timeline-name`
- [ ] `view-timeline`
- [ ] `view-timeline-axis`
- [ ] `view-timeline-inset`
- [ ] `view-timeline-name`
- [ ] `timeline-scope`


### CSS view transitions

- [ ] `view-transition-class`
- [ ] `view-transition-name`


### CSS anchor positioning

- [ ] `anchor-name`
- [ ] `anchor-scope`
- [ ] `position-anchor`
- [ ] `position-area`
- [ ] `position-try`
- [ ] `position-try-fallbacks`
- [ ] `position-try-order`
- [ ] `position-visibility`


### CSS basic user interface

- [ ] `accent-color`
- [ ] `appearance`
- [ ] `caret-color`
- [ ] `cursor`
- [ ] `field-sizing`
- [ ] `outline`
- [ ] `outline-color`
- [ ] `outline-offset`
- [ ] `outline-style`
- [ ] `outline-width`
- [ ] `pointer-events`
- [ ] `resize`
- [ ] `user-select`


### Pointer events

- [ ] `touch-action`


### CSS containment and container queries

- [ ] `contain`
- [ ] `content-visibility`
- [ ] `container`
- [ ] `container-name`
- [ ] `container-type`


### CSS will change

- [ ] `will-change`


### CSS viewport

- [ ] `zoom`


### MathML Core

- [ ] `math-depth`
- [ ] `math-shift`
- [ ] `math-style`


### CSS speech

- [ ] `speak`


### SVG presentation properties

- [ ] `cx`
- [ ] `cy`
- [ ] `r`

> **2026-08-22 split:** the CSS twins remain open at the pinned Stylo cap:
> this build has no `cx`/`cy`/`r` longhands, and both authored CSS ingresses
> are quarantined rather than matched outside the cascade. The attribute-rung
> evidence is recorded with the presentation-attribute rows below.

- [ ] `rx`
- [ ] `ry`
- [ ] `x`
- [ ] `y`

> **2026-08-22 split:** the CSS `x`/`y` twins remain open at the pinned Stylo
> cap. Chromium honors both authored CSS ingresses (measured, not celled),
> while this build has no corresponding longhands; the ingresses now refuse by
> name rather than painting the attribute position. The attribute-rung evidence
> is recorded below.

- [ ] `d`

> **2026-08-23 split:** the CSS property remains open at the pinned Stylo cap:
> this build has no `d` longhand, and the stylesheet and inline-style ingresses
> remain quarantined by name rather than matched outside the cascade. The
> presentation attribute closes independently below.

- [x] `fill`
- [x] `fill-opacity`
- [x] `fill-rule`
- [x] `stroke`
- [ ] `stroke-width`
- [x] `stroke-opacity`
- [x] `stroke-dasharray`
- [ ] `stroke-dashoffset`
- [x] `stroke-linecap`
- [x] `stroke-linejoin`
- [x] `stroke-miterlimit`
- [ ] `marker`
- [ ] `marker-start`
- [ ] `marker-mid`
- [ ] `marker-end`
- [ ] `paint-order`
- [x] `path-length`
- [ ] `color-interpolation`
- [ ] `shape-rendering`
- [ ] `text-rendering`
- [ ] `text-anchor`
- [ ] `vector-effect`
- [ ] `stop-color`
- [ ] `stop-opacity`


### CSS selectors and combinators

- [ ] `*`
- [ ] `E`
- [ ] `.class`
- [ ] `#id`
- [ ] `[attr]`
- [ ] `[attr=value]`
- [ ] `[attr~=value]`
- [ ] `[attr|=value]`
- [ ] `[attr^=value]`
- [ ] `[attr$=value]`
- [ ] `[attr*=value]`
- [ ] `[attr=value i]`
- [ ] `[attr=value s]`
- [ ] `E F`
- [ ] `E > F`
- [ ] `E + F`
- [ ] `E ~ F`
- [ ] `ns|E`
- [ ] `A, B`
- [ ] `&`


### CSS pseudo-classes

- [ ] `:active`
- [ ] `:active-view-transition`
- [ ] `:active-view-transition-type()`
- [ ] `:any-link`
- [ ] `:autofill`
- [ ] `:blank`
- [ ] `:buffering`
- [ ] `:checked`
- [ ] `:current`
- [ ] `:default`
- [ ] `:defined`
- [ ] `:dir()`
- [ ] `:disabled`
- [ ] `:empty`
- [ ] `:enabled`
- [ ] `:first`
- [ ] `:first-child`
- [ ] `:first-of-type`
- [ ] `:focus`
- [ ] `:focus-visible`
- [ ] `:focus-within`
- [ ] `:fullscreen`
- [ ] `:future`
- [ ] `:has()`
- [ ] `:has-slotted`
- [ ] `:heading`
- [ ] `:heading()`
- [ ] `:host`
- [ ] `:host()`
- [ ] `:host-context()`
- [ ] `:hover`
- [ ] `:in-range`
- [ ] `:indeterminate`
- [ ] `:invalid`
- [ ] `:is()`
- [ ] `:lang()`
- [ ] `:last-child`
- [ ] `:last-of-type`
- [ ] `:left`
- [ ] `:link`
- [ ] `:local-link`
- [ ] `:modal`
- [ ] `:muted`
- [ ] `:not()`
- [ ] `:nth-child()`
- [ ] `:nth-last-child()`
- [ ] `:nth-last-of-type()`
- [ ] `:nth-of-type()`
- [ ] `:only-child`
- [ ] `:only-of-type`
- [ ] `:open`
- [ ] `:optional`
- [ ] `:out-of-range`
- [ ] `:past`
- [ ] `:paused`
- [ ] `:picture-in-picture`
- [ ] `:placeholder-shown`
- [ ] `:playing`
- [ ] `:popover-open`
- [ ] `:read-only`
- [ ] `:read-write`
- [ ] `:required`
- [ ] `:right`
- [ ] `:root`
- [ ] `:scope`
- [ ] `:seeking`
- [ ] `:stalled`
- [ ] `:state()`
- [ ] `:target`
- [ ] `:target-current`
- [ ] `:user-invalid`
- [ ] `:user-valid`
- [ ] `:valid`
- [ ] `:visited`
- [ ] `:volume-locked`
- [ ] `:where()`
- [ ] `:xr-overlay`


### CSS pseudo-elements

- [ ] `::after`
- [ ] `::backdrop`
- [ ] `::before`
- [ ] `::checkmark`
- [ ] `::column`
- [ ] `::cue`
- [ ] `::cue()`
- [ ] `::details-content`
- [ ] `::file-selector-button`
- [ ] `::first-letter`
- [ ] `::first-line`
- [ ] `::grammar-error`
- [ ] `::highlight()`
- [ ] `::marker`
- [ ] `::part()`
- [ ] `::picker()`
- [ ] `::picker-icon`
- [ ] `::placeholder`
- [ ] `::scroll-button()`
- [ ] `::scroll-marker`
- [ ] `::scroll-marker-group`
- [ ] `::selection`
- [ ] `::slotted()`
- [ ] `::spelling-error`
- [ ] `::target-text`
- [ ] `::view-transition`
- [ ] `::view-transition-group()`
- [ ] `::view-transition-image-pair()`
- [ ] `::view-transition-new()`
- [ ] `::view-transition-old()`


### CSS at-rules

- [ ] `@annotation`
- [ ] `@character-variant`
- [ ] `@charset`
- [ ] `@container`
- [ ] `@counter-style`
- [ ] `@font-face`
- [ ] `@font-feature-values`
- [ ] `@font-palette-values`
- [ ] `@historical-forms`
- [ ] `@import`
- [ ] `@keyframes`
- [ ] `@layer`
- [ ] `@media`
- [ ] `@namespace`
- [ ] `@ornaments`
- [ ] `@page`
- [ ] `@position-try`
- [ ] `@property`
- [ ] `@scope`
- [ ] `@starting-style`
- [ ] `@styleset`
- [ ] `@stylistic`
- [ ] `@supports`
- [ ] `@swash`
- [ ] `@view-transition`


### CSS value functions

- [ ] `anchor()`
- [ ] `anchor-size()`
- [ ] `translate()`
- [ ] `translate3d()`
- [ ] `translateX()`
- [ ] `translateY()`
- [ ] `translateZ()`
- [ ] `rotate()`
- [ ] `rotate3d()`
- [ ] `rotateX()`
- [ ] `rotateY()`
- [ ] `rotateZ()`
- [ ] `scale()`
- [ ] `scale3d()`
- [ ] `scaleX()`
- [ ] `scaleY()`
- [ ] `scaleZ()`
- [ ] `skew()`
- [ ] `skewX()`
- [ ] `skewY()`
- [ ] `matrix()`
- [ ] `matrix3d()`
- [ ] `perspective()`
- [ ] `calc()`
- [ ] `calc-size()`
- [ ] `min()`
- [ ] `max()`
- [ ] `clamp()`
- [ ] `round()`
- [ ] `mod()`
- [ ] `rem()`
- [ ] `progress()`
- [ ] `sin()`
- [ ] `cos()`
- [ ] `tan()`
- [ ] `asin()`
- [ ] `acos()`
- [ ] `atan()`
- [ ] `atan2()`
- [ ] `pow()`
- [ ] `sqrt()`
- [ ] `hypot()`
- [ ] `log()`
- [ ] `exp()`
- [ ] `abs()`
- [ ] `sign()`
- [ ] `blur()`
- [ ] `brightness()`
- [ ] `contrast()`
- [ ] `drop-shadow()`
- [ ] `grayscale()`
- [ ] `hue-rotate()`
- [ ] `invert()`
- [ ] `opacity()`
- [ ] `saturate()`
- [ ] `sepia()`
- [ ] `rgb()`
- [ ] `hsl()`
- [ ] `hwb()`
- [ ] `lab()`
- [ ] `lch()`
- [ ] `oklab()`
- [ ] `oklch()`
- [ ] `color()`
- [ ] `color-mix()`
- [ ] `contrast-color()`
- [ ] `device-cmyk()`
- [ ] `alpha()`
- [ ] `light-dark()`
- [ ] `dynamic-range-limit-mix()`
- [ ] `linear-gradient()`
- [ ] `radial-gradient()`
- [ ] `conic-gradient()`
- [ ] `repeating-linear-gradient()`
- [ ] `repeating-radial-gradient()`
- [ ] `repeating-conic-gradient()`
- [ ] `image()`
- [ ] `image-set()`
- [ ] `cross-fade()`
- [ ] `element()`
- [ ] `paint()`
- [ ] `counter()`
- [ ] `counters()`
- [ ] `symbols()`
- [ ] `circle()`
- [ ] `ellipse()`
- [ ] `inset()`
- [ ] `rect()`
- [ ] `xywh()`
- [ ] `polygon()`
- [ ] `path()`
- [ ] `shape()`
- [ ] `ray()`
- [ ] `attr()`
- [ ] `env()`
- [ ] `url()`
- [ ] `var()`
- [ ] `fit-content()`
- [ ] `minmax()`
- [ ] `repeat()`
- [ ] `stylistic()`
- [ ] `styleset()`
- [ ] `character-variant()`
- [ ] `swash()`
- [ ] `ornaments()`
- [ ] `annotation()`
- [ ] `palette-mix()`
- [ ] `linear()`
- [ ] `cubic-bezier()`
- [ ] `steps()`
- [ ] `scroll()`
- [ ] `view()`
- [ ] `layer()`


### CSS units and value types

- [ ] `px`
- [ ] `cm`
- [ ] `mm`
- [ ] `Q`
- [ ] `in`
- [ ] `pt`
- [ ] `pc`
- [ ] `em`
- [ ] `rem`
- [ ] `ex`
- [ ] `rex`
- [ ] `cap`
- [ ] `rcap`
- [ ] `ch`
- [ ] `rch`
- [ ] `ic`
- [ ] `ric`
- [ ] `lh`
- [ ] `rlh`
- [ ] `vw`
- [ ] `vh`
- [ ] `vi`
- [ ] `vb`
- [ ] `vmin`
- [ ] `vmax`
- [ ] `svw`
- [ ] `svh`
- [ ] `svi`
- [ ] `svb`
- [ ] `svmin`
- [ ] `svmax`
- [ ] `lvw`
- [ ] `lvh`
- [ ] `lvi`
- [ ] `lvb`
- [ ] `lvmin`
- [ ] `lvmax`
- [ ] `dvw`
- [ ] `dvh`
- [ ] `dvi`
- [ ] `dvb`
- [ ] `dvmin`
- [ ] `dvmax`
- [ ] `cqw`
- [ ] `cqh`
- [ ] `cqi`
- [ ] `cqb`
- [ ] `cqmin`
- [ ] `cqmax`
- [ ] `deg`
- [ ] `grad`
- [ ] `rad`
- [ ] `turn`
- [ ] `s`
- [ ] `ms`
- [ ] `Hz`
- [ ] `kHz`
- [ ] `dpi`
- [ ] `dpcm`
- [ ] `dppx`
- [ ] `x`
- [ ] `fr`
- [ ] `%`
- [ ] `inherit`
- [ ] `initial`
- [ ] `unset`
- [ ] `revert`
- [ ] `revert-layer`


## SVG

Sources: [the SVG 2 element index](https://svgwg.org/svg2-draft/eltindex.html),
[attribute index](https://svgwg.org/svg2-draft/attindex.html), and
[property index](https://svgwg.org/svg2-draft/propidx.html) —
including the Filter Effects primitives and the SMIL animation
elements the element index carries — plus
[the MDN SVG attribute reference](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute)
for attributes the platform ships ahead of the SVG 2 indexes.


### SVG elements

- [x] `<a>`
- [ ] `<animate>`
- [ ] `<animateMotion>`
- [ ] `<animateTransform>`
- [x] `<circle>`
- [ ] `<clipPath>`

> **2026-08-24 split:** direct admitted geometry and `<use>` contributors now
> form Chromium-gated path unions, and chained resources form intersections.
> The element stays open because visible text, a child carrying its own clip,
> animation, and the 43-or-more-contributor route switch Chromium to a raster
> mask that the resolved contract deliberately cannot express.

- [x] `<defs>`
- [x] `<desc>`
- [x] `<ellipse>`
- [x] `<feBlend>`

> **2026-08-26 close:** `feBlend` carries the complete static sixteen-mode
> Compositing Level 1 vocabulary over two checked image inputs. Chromium-baked
> evidence covers opaque and translucent arithmetic, foreground/backdrop
> order, graph fallbacks and result reuse, both filter color spaces, hard
> regions and primitive units, source shapes and paint servers, `<use>`,
> `viewBox`, safe mappings, opacity and mask order, and neighboring admitted
> filter operations. General target mappings, geometric clipping, and the
> independently discovered translucent-source multi-input composition class
> refuse by three stable precision names before paint. Shared graph, region,
> interpolation, filter-resource, and dynamics rows remain open.
> Hosted x86 initially contradicted the ARM-local blend result in eight cells.
> The pinned backend's low-precision path divides by 255 exactly on NEON but
> approximates that step as division by 256 on x86. Nine modes now state their
> exact byte-domain arithmetic; the seven high-precision modes remain native.
> That repair removed seven failures and every opaque mismatch. One translucent
> atlas remained at 2,816 pixels / delta 1 across eleven mode tiles, locating a
> separate final sRGB layer-restore split. A blend-scoped exact restore closes
> it and clears across later color-space conversion. The complete 572-cell
> gate is exact on ARM and hosted x86 with no tolerance. Restoring the
> approximate division made twelve blend cells fail locally, up to 4,096
> pixels / delta 3; restoring exact arithmetic returned the gate to green.
- [x] `<feColorMatrix>`
- [x] `<feComponentTransfer>`
- [x] `<feComposite>`
- [x] `<feConvolveMatrix>`

> **2026-08-27 close/split:** `feConvolveMatrix` carries Chromium's complete
> static convolution behavior: rectangular kernels through 256 coefficients,
> kernel reversal, divisor and bias arithmetic, asymmetric targets, all three
> edge modes, alpha preservation, both filter color spaces, graph inputs and
> reuse, hard regions and primitive units, safe mappings, source geometry,
> blur/morphology ordering, target effects, `<use>`, and `viewBox`.
> Invalid operation states and Chromium's 257-coefficient construction limit
> produce the browser's transparent result rather than an unfiltered fallback.
> Chromium ignores `kernelUnitLength` on this primitive; the drop is celled,
> while that attribute row stays open for its lighting applicability. General
> affine mappings, paint-server source images, and a divisor whose reciprocal
> exceeds finite resolved arithmetic refuse by three stable names before paint.
> Forty exact convolution cells and one blur-edge drop cell move the complete
> gate to 741. Removing the required kernel reversal makes eleven cells fail,
> up to 2,814 pixels and maximum channel delta 250; restoration returns the
> gate to green. The shared graph, region, interpolation, filter-resource, and
> dynamics rows remain open.
- [ ] `<feDiffuseLighting>`
- [x] `<feDisplacementMap>`
- [ ] `<feDistantLight>`
- [x] `<feDropShadow>`
- [x] `<feFlood>`
- [x] `<feFuncA>`
- [x] `<feFuncB>`
- [x] `<feFuncG>`
- [x] `<feFuncR>`

> **2026-08-25 close:** component transfer carries the complete static
> direct-child vocabulary for all four channels: missing channels are
> identity, later same-channel children win, and `identity | table |
> discrete | linear | gamma` resolve through exact byte lookup tables. The
> committed Chromium evidence covers straight RGBA, alpha creation/removal,
> both filter color spaces, graph inputs, regions and units, safe transforms,
> shapes, stroke, `<use>`, and neighboring filter operations. Paint-server
> sources and the unsafe transform envelope remain named backend-precision
> refusals; the generic same-scope clip-plus-opacity boundary is likewise
> quarantined before paint. Transfer-function animation remains a separate
> open axis.
- [ ] `<feGaussianBlur>`
- [ ] `<feImage>`
- [x] `<feMerge>`
- [x] `<feMergeNode>`
- [x] `<feMorphology>`

> **2026-08-26 close:** `feMorphology` carries the complete static
> case-sensitive `erode | dilate` operation and SVG
> `<number-optional-number>` radius grammar. Missing and invalid operation
> text selects `erode`; missing and invalid radius text selects zero; one
> number supplies both axes; and negative members clamp independently to
> zero, so the other axis remains active. Chromium-baked evidence covers
> mapped half-step rounding, non-uniform `viewBox` and object-box units,
> SourceAlpha and generated inputs, result reuse, hard regions, both filter
> color spaces, premultiplied channel extrema, paths, strokes, rounded
> rectangles, `<use>`, safe mappings, target effects, and ordering with the
> admitted filter family. General affine mappings, paint-server source images,
> and active filled-circle/ellipse sources refuse by three stable precision
> names before paint; the last boundary leaves the older fill-only ellipse
> issue to gridaco/nothing#88. Thirty-seven exact cells move the complete gate
> to 609 cells, and swapping erosion with dilation makes thirty-five of those
> cells fail loudly. The shared `operator`, `radius`, graph, region, color,
> filter-resource, and dynamics rows remain open for their wider
> applicability. The first full-workspace hosted-x86 run contradicted the
> ARM-local result in nine active sRGB morphology cells: 1,633 pixels differed
> in total, all by one channel level. They shared the final filter-layer
> restore, where the pinned backend's low-precision division differs by CPU
> family. An active-sRGB-scoped exact byte-domain restore clears the class;
> zero radius and later color-space conversion retain their prior paths. The
> complete 609-cell gate is exact on ARM and hosted x86 without tolerance.
- [ ] `<feOffset>`
- [ ] `<fePointLight>`
- [ ] `<feSpecularLighting>`
- [ ] `<feSpotLight>`
- [ ] `<feTile>`
- [x] `<feTurbulence>`

> **2026-08-26 close:** `feTurbulence` carries both static procedural-noise
> formulas, the complete five-parameter attribute vocabulary, generated-source
> regions, both filter color spaces, graph reuse, safe mappings, `<use>`, and
> `viewBox`. `feDisplacementMap` carries two ordered images, signed scale, all
> four non-premultiplied channel selectors, hard regions and object-box units,
> color conversion, source/generated/procedural maps, source shapes, opacity,
> safe mappings, `<use>`, and `viewBox`. A user-space procedural filter may
> paint even when its target contributes a fully transparent source; an
> object-box filter region on the same zero-area target paints nothing.
> Ninety-one exact Chromium cells move the complete gate to 700. General
> affine mappings for both primitives and geometric clipping for displacement
> refuse by three stable precision names before paint. The shared `type`, `in`,
> `in2`, `result`, region, interpolation, filter-resource, and dynamics rows
> remain open for their wider applicability. Swapping the two noise formulas
> and red/alpha displacement selection made fifty-five new cells fail, up to
> all 4,096 pixels and maximum channel delta 202; restoration returned the
> complete gate to green. A review-triggered color-space/operation matrix found
> that procedural images must remain floating through blend arithmetic, while
> active sRGB morphology materializes a direct procedural image. Restoring the
> old policy makes five dedicated cells fail at up to 2,365 pixels and maximum
> channel delta 6. The first hosted-x86 run found 294 one-code-value pixels
> across twenty-two displacement/procedural cells. Its first repair cleared all
> eighteen displacement failures; the second hosted run left 729 delta-1
> pixels across four procedural cells, including 726 in the blend control.
> Pinned Skia source and a four-atlas, sixty-four-mode Chromium replay then
> separated direct sRGB byte-domain blend products from promoted floating
> products and made every atlas exact. Six committed difference/exclusion
> controls guard both routes: forcing floating arithmetic breaks the two direct
> controls by 3,497 and 3,532 pixels, while forcing byte arithmetic breaks the
> four promoted controls by 3,287–3,697 pixels. A further thirty-four-source
> operation-chain matrix found that an sRGB blend result materializes before a
> later blend; carrying the earlier floating state silently changed 3,468 and
> 3,700 pixels. Two committed chain controls now guard that transition.
> Explicit procedural mode arithmetic and composed-result half-up quantization
> kept all 700 ARM cells exact. The third hosted-x86 run cleared the 726-pixel
> blend control and left three singleton delta-1 pixels: one each in the
> default-color, linear-color, and stitched procedural controls. Pinned Skia
> source located that final split in process startup: no caller initialized the
> runtime-selected raster pipeline, so hosted x86 retained baseline non-fused
> Perlin arithmetic while ARM used its fused NEON path. Initializing Skia before
> drawlist replay selects x86 AVX2's fused path as well. The fourth hosted-x86
> gate and the ARM gate are byte-exact across all 700 cells, with no tolerance.
- [ ] `<filter>`

> **2026-08-25 split:** the static filter graph carries safe-kernel
> `feGaussianBlur` operations over `SourceGraphic`, `SourceAlpha`, a prior
> result, or a resolved result name. It includes same-document first-id lookup,
> quoted and unquoted forms of a single URL token,
> both coordinate systems, hard effect and primitive regions, both filter
> color spaces, transforms, `<use>`, stroke, nesting, and the established
> filter-before-mask-before-opacity-before-clip order. Both element rows stay
> open: `<filter>` still has the rest of the primitive family, inheritance,
> multi-operation lists, host, and dynamics surface; `<feGaussianBlur>` still
> has valid empty primitive results, inherited raw-syntax gaps, animation, and
> a measured small-kernel backend precision boundary. Current Chromium
> drops every `edgeMode` spelling on this primitive. The later convolution rung
> commits that drop and closes the shared attribute row; it does not close the
> blur element's other remainders.
> **2026-08-25 close/split:** `feFlood`, `feComposite`, `feMerge`, and
> `feMergeNode` now carry the complete static primitive behavior for their
> rows: zero-, two-, and ordered N-input graph nodes; all seven composite
> operators; arithmetic coefficients; input/result routing; hard primitive
> regions; color-space placement; and the crisp compositional shadow graph.
> `feOffset` carries integer displacement, both unit systems, signed values,
> regions, and graph routing, but stays open. Valid fractional displacements
> differ at 48 pixels / maximum delta 128, and every sampled graph combining
> blur with offset differs at a second backend boundary; all such graphs
> conservatively refuse by stable name. Integer source offsets that target
> mapping turns fractional differ by 12–97 pixels, up to maximum delta 122;
> that mapped class has its own stable refusal too (measured, not celled). The
> earlier depth diagnosis is withdrawn: three safe-sigma blurs are
> exact, directly and through identity merges. Sampled effective sigmas from
> `.5` through `1.875` differ while `.25` and `2` are exact; the patrol
> conservatively refuses the open interval between those exact endpoints
> after target mapping (measured, not celled). It keeps `<feGaussianBlur>`
> open. A final opacity-normalization audit found one more silent class before
> close: raw `f32` parse-then-divide selected the lower neighbour for
> `57.384267578125007%`, changing all 4,096 amplified pixels at maximum delta
> 16. The direct decoder now keeps the CSS token's divide-then-narrow order,
> and one exact cell distinguishes the route. Sixty exact cells carry the
> shadow-graph rung. One additional review cell proves that a zero-sigma blur
> still applies its explicit primitive region; bypassing that crop changes
> 1,160 pixels at maximum delta 218. Hosted x86 verification also exposed seven
> composite/merge departures of one code value. Keeping generated-source byte
> rounding distinct from source-image floating coverage fixes six. The seventh
> is a one-input merge, so it has no internal composition stage on which that
> rule can run. The final layer restore now makes the same distinction:
> generated-only results use exact byte-domain SrcOver, while source-derived
> coverage stays floating. Together those boundaries restore exact output on
> both processor families without tolerance.
>
> **2026-08-25 close/split:** `<feDropShadow>` now carries its complete static
> primitive behavior through one native, checked, one-input operation. Missing
> `dx`, `dy`, and `stdDeviation` use `2`; the measured number-list grammar,
> independent negative-axis clamp, source and result routing, both primitive
> unit systems, hard regions, direct flood values, admitted sRGB placement,
> transforms, `viewBox`, `<use>`, stroke, groups, target opacity, clips, and
> neighboring admitted graph operations are covered by twenty-eight exact
> Chromium cells. A default-linear endpoint-color cell also consumes an earlier
> blur. The native operation includes its source foreground and is not rewritten
> into the separately guarded blur-plus-offset graph. Hosted processor-family
> verification first found all twenty-eight cells departing through the
> backend's direct helper, by as much as eight code values. Making the shadow's
> internal byte-domain compositions exact narrowed that to twenty-five
> source-derived sRGB cells, all at one code value; replacing the internal
> colorization and changing offset sampling left that exact set unchanged, and
> a zero-blur member was still in it. The remaining operation was the filtered
> layer's restore onto its backdrop. Exact byte-domain restore for sRGB native-
> shadow descendants makes both processor families exact without tolerance;
> applying that rule to every source-derived filter instead changes three
> unrelated floating-path cells, so color-space conversion clears it and the
> default-linear endpoint stays on the floating route. This is painter policy:
> the resolved contract remains one native operation. Four new refusal rows name
> native-shadow range, transform, source-layer, and linearRGB color-conversion
> precision boundaries; the shared small-kernel and flood cascade/value patrols
> also apply. Those independently registered
> boundaries leave `dx`, `dy`, `stdDeviation`, `flood-color`, `flood-opacity`,
> `color-interpolation-filters`, `filter`, and `<filter>` open for their wider
> applicability or value surface. Only the element row closes. The corpus is
> 475 Chromium-baked cells plus 10 sampled frames; the refusal register has 128
> rows.
>
> **2026-08-25 close/split:** `<feColorMatrix>` now carries its complete static
> primitive behavior as one checked one-input, non-premultiplied RGBA matrix.
> Missing and invalid `type` use `matrix`; all four case-sensitive members
> (`matrix`, `saturate`, `hueRotate`, `luminanceToAlpha`) are covered. The
> complete measured SVG number-list grammar, exact value counts and
> pass-through fallbacks, unclamped saturation, Blink-ordered large-angle hue
> arithmetic, ignored luminance `values`, channel crossing, alpha scaling and
> creation, output clamping, source/result routing, generated input, both color
> spaces, safe transforms, target opacity and clips are carried by twenty-seven
> exact Chromium cells. A fresh direct-shape probe corrected the earlier enum
> reading before close: wrong-case and surrounding-whitespace spellings are
> invalid and therefore select the default `matrix` behavior; surrounding
> whitespace does not make `hueRotate` valid. Three new refusal rows quarantine
> the measured source-layer, non-quarter-transform, and blur/shadow-composition
> precision boundaries. Their conservative safe envelope over-refuses some
> exact sources rather than releasing wrong edge pixels. The generic `type`,
> `values`, `in`, `result`, region, `color-interpolation-filters`, `filter`, and
> `<filter>` rows remain open for their wider applicability, cascade, resource,
> or dynamics surface. Only the element row closes. The corpus is 502
> Chromium-baked cells plus 10 sampled frames; the refusal register has 131
> rows.

- [ ] `<foreignObject>`
- [x] `<g>`
- [ ] `<image>`
- [x] `<line>`
- [x] `<linearGradient>`
- [ ] `<marker>`
- [ ] `<mask>`

> **2026-08-24 split:** one isolated alpha/luminance source image, both
> coordinate systems, hard regions, admitted graphics children, transforms,
> gradients, clips, `<use>`, nesting, and target opacity now render. The
> element stays open because valid nested cycles and source children whose own
> element rows remain open still refuse transactionally; the root host-layer
> and external-resource routes remain separate boundaries. An unrepresented
> inline declaration on the resource also refuses before its inherited effect
> can change a source descendant silently.

- [ ] `<metadata>`
- [ ] `<mpath>`
- [x] `<path>`
- [ ] `<pattern>`
- [x] `<polygon>`
- [x] `<polyline>`
- [x] `<radialGradient>`
- [x] `<rect>`
- [ ] `<script>`
- [ ] `<set>`
- [x] `<stop>`
- [ ] `<style>`
- [ ] `<svg>`
- [ ] `<switch>`
- [ ] `<symbol>`
- [ ] `<text>`
- [ ] `<textPath>`
- [x] `<title>`
- [ ] `<tspan>`
- [ ] `<use>`
- [ ] `<view>`


### SVG presentation attributes

- [ ] `alignment-baseline`
- [ ] `baseline-shift`
- [ ] `clip`
- [ ] `clip-path`
- [ ] `clip-rule`

> **2026-08-24 split:** the `clip-path` presentation hint carries computed
> `none` and same-document `url(#…)` through the one cascade on admitted
> non-root SVG targets. Eight Chromium cells cover CSS ingress and precedence,
> first-id lookup, invalid references, geometric unions, inherited fill rules,
> both clip coordinate systems, transforms, `viewBox`, groups, `<use>`, stroke
> boxes, resource chains, and opacity ordering. Its basic-shape, geometry-box,
> root, external/cyclic, animation, and raster-mask branches remain registered
> refusals, so the row stays open. The direct inherited `clip-rule` route
> admits `nonzero`/`evenodd` and CSS-wide behavior, but valid comments, escapes,
> and `var()` still need the unavailable cascade longhand and refuse by name;
> that row also stays open.

- [ ] `color`
- [ ] `color-interpolation`
- [ ] `color-interpolation-filters`
- [ ] `cursor`
- [ ] `cx`
- [ ] `cy`

> **2026-08-22 split:** five Chromium-baked cells now cover the admitted
> `cx`/`cy`/`r` number/percentage route on `<circle>` and `<ellipse>`, including
> defaults, negative centers, invalid/zero radius, axis and normalized-diagonal
> bases, `viewBox`, `<use>`, transforms, and stroke. The three rows remain open:
> valid authored decimals can lose Chromium rounding provenance in the raw f32
> route, finite sources can cross the unimplemented Chromium used-range split,
> CSS comments in otherwise numeric values still refuse, and the registered
> guards name all of those gaps before a non-finite frame or silent backend
> drop. Unit values, CSS math, `var()`, and
> CSS-wide keywords retain their own unchecked value-type rows.

- [x] `d`

> **2026-08-23 close:** the complete `none | <path-data>` attribute grammar is
> Chromium-baked. Erroneous data now retains every complete segment before the
> error, including completed implicit repeats, while an error before a complete
> leading moveto is the correct empty geometry. Two source-number witnesses pin
> Blink's ordered float evaluation in both rounding directions; the same repair
> is independently celled for the shared polygon/polyline scanner. Extreme
> finite construction follows the browser's split between an invalid whole
> path and an arc that appends no segment while preserving the prior prefix.
> Seven new exact cells carry those branches; the three former path-data refusal
> rows graduate. The CSS `d` property twin stays open under the split above.

- [ ] `direction`
- [ ] `display`
- [ ] `dominant-baseline`
- [x] `fill`
- [x] `fill-opacity`
- [x] `fill-rule`
- [ ] `filter`

> **2026-08-25 split:** the direct presentation attribute carries `none`,
> CSS-wide reset values, and one same-document URL token with quoted or
> unquoted content on admitted non-root SVG targets. Committed cells cover
> comments around the URL, first-id lookup,
> missing/wrong/malformed-reference fallback, groups, `<use>`, transforms,
> stroke, clip/mask/opacity ordering, and nesting. The row stays open for
> filter functions and lists, `var()`, inheritance, CSS precedence, root and
> external host routes, resource `href`, the measured small-kernel boundary,
> and the remaining primitives. The direct inherited
> `color-interpolation-filters` reader
> carries `linearRGB`, `sRGB`, `auto`, reset behavior, and inheritance for the
> admitted blur graph; CSS ingress, comments, escapes, and `var()` remain
> named gaps, and that row stays open too.

- [ ] `flood-color`
- [ ] `flood-opacity`

> **2026-08-25 split:** direct attributes on `feFlood` now carry initial
> black/one, admitted sRGB colors and `currentColor`, number/percentage
> opacity with clamping, reset and invalid fallbacks, separate color-alpha ×
> opacity multiplication, non-inheritance, and hard primitive regions. Both
> rows stay open for explicit inheritance, unavailable CSS ingress, and the
> independently listed wider color/math/custom-property value families; the
> native `feDropShadow` route reuses the admitted direct subset and the same
> named patrols.

- [ ] `font-family`
- [ ] `font-size`
- [ ] `font-size-adjust`
- [ ] `font-stretch`
- [ ] `font-style`
- [ ] `font-variant`
- [ ] `font-weight`
- [ ] `glyph-orientation-horizontal`
- [ ] `glyph-orientation-vertical`
- [ ] `height`
- [ ] `image-rendering`
- [ ] `letter-spacing`
- [ ] `lighting-color`
- [ ] `marker-end`
- [ ] `marker-mid`
- [ ] `marker-start`
- [ ] `mask`
- [x] `mask-type`

> **2026-08-24 split:** the `mask` presentation attribute admits `none` and
> one same-document `url(#…)`, including CSS comments, first-id lookup,
> invalid-reference fallback, `<use>`, and nested-mask use. It stays open for
> the full shorthand/multiple-layer grammar, `var()`, root and external
> resource routes, cycles, unsupported source children, and the measured
> target-transform region-precision boundary. `mask-type` closes at its
> complete `luminance | alpha` grammar, including the missing/default value, invalid
> fallback, and reset keywords. Explicit inheritance and `var()` remain named
> refusals under their independently listed CSS-wide/custom-property rows;
> the CSS property twin stays open at the pinned-cascade boundary.
> A valid empty source always hides its target; opaque black hides in luminance
> mode and reveals under `mask-type="alpha"`.

- [x] `opacity`
- [ ] `overflow`
- [ ] `paint-order`
- [ ] `pointer-events`
- [ ] `r`
- [x] `rx`
- [x] `ry`
- [ ] `shape-rendering`
- [x] `stop-color`
- [x] `stop-opacity`
- [x] `stroke`
- [x] `stroke-dasharray`
- [ ] `stroke-dashoffset`
- [x] `stroke-linecap`
- [x] `stroke-linejoin`
- [x] `stroke-miterlimit`
- [x] `stroke-opacity`
- [ ] `stroke-width`
- [ ] `text-anchor`
- [ ] `text-decoration`
- [ ] `text-overflow`
- [ ] `text-rendering`
- [ ] `transform`
- [ ] `transform-origin`
- [ ] `unicode-bidi`
- [ ] `vector-effect`
- [x] `visibility`
- [ ] `white-space`
- [ ] `width`
- [ ] `word-spacing`
- [ ] `writing-mode`
- [ ] `x`
- [ ] `y`

> **2026-08-22 split:** the existing two rect-percentage basis cells plus
> three new grammar, `<use>`, and transform-plus-stroke cells cover the
> admitted finite number/percentage route for rect `x`/`y`/`width`/`height`,
> including defaults, negative coordinates, invalid non-positive extents,
> root units, `viewBox`, instances, transforms, and stroke. All four rows stay
> open: valid source decimals can lose Chromium rounding provenance, finite
> drawable values cross an unimplemented used-range clamp, and CSS comments
> plus rect `auto` sizes still refuse (measured, not celled); root
> percentage/CSS sizing and applications to unadmitted
> `<image>`/`<pattern>`/`<mask>` elements remain declared gaps. Units, CSS math,
> `var()`, and CSS-wide values retain their own unchecked rows.


### SVG attributes

- [x] `amplitude`
- [ ] `aria-activedescendant`
- [ ] `aria-atomic`
- [ ] `aria-autocomplete`
- [ ] `aria-busy`
- [ ] `aria-checked`
- [ ] `aria-colcount`
- [ ] `aria-colindex`
- [ ] `aria-colspan`
- [ ] `aria-controls`
- [ ] `aria-current`
- [ ] `aria-describedby`
- [ ] `aria-details`
- [ ] `aria-disabled`
- [ ] `aria-dropeffect`
- [ ] `aria-errormessage`
- [ ] `aria-expanded`
- [ ] `aria-flowto`
- [ ] `aria-grabbed`
- [ ] `aria-haspopup`
- [ ] `aria-hidden`
- [ ] `aria-invalid`
- [ ] `aria-keyshortcuts`
- [ ] `aria-label`
- [ ] `aria-labelledby`
- [ ] `aria-level`
- [ ] `aria-live`
- [ ] `aria-modal`
- [ ] `aria-multiline`
- [ ] `aria-multiselectable`
- [ ] `aria-orientation`
- [ ] `aria-owns`
- [ ] `aria-placeholder`
- [ ] `aria-posinset`
- [ ] `aria-pressed`
- [ ] `aria-readonly`
- [ ] `aria-relevant`
- [ ] `aria-required`
- [ ] `aria-roledescription`
- [ ] `aria-rowcount`
- [ ] `aria-rowindex`
- [ ] `aria-rowspan`
- [ ] `aria-selected`
- [ ] `aria-setsize`
- [ ] `aria-sort`
- [ ] `aria-valuemax`
- [ ] `aria-valuemin`
- [ ] `aria-valuenow`
- [ ] `aria-valuetext`
- [ ] `autofocus`
- [ ] `azimuth`
- [x] `baseFrequency`

> **2026-08-26 close:** `baseFrequency` applies only to `feTurbulence` and
> carries the complete SVG one-or-two-number grammar. One value supplies both
> axes; comma-wsp, leading plus, exponent, and the measured lone trailing comma
> are carried. Missing, malformed, unit-bearing, and overlong lists select the
> initial zero pair; either negative member makes the whole pair initial.
- [x] `bias`

> **2026-08-27 close:** convolution `bias` carries one signed SVG number with
> initial zero; missing, empty, malformed, unit-bearing, CSS-function, and
> CSS-wide text selects that initial. `divisor` carries the same one-number
> grammar. Missing, exactly empty, and explicit positive or negative zero use
> the kernel's ordered binary32 sum; a zero sum becomes one. A present nonempty
> malformed divisor uses one. Signed nonzero divisors remain active. The wider
> invalid-spelling matrix is measured, not all separately celled.
- [ ] `class`
- [x] `clipPathUnits`

> **2026-08-24 close:** the complete `userSpaceOnUse | objectBoundingBox`
> grammar is Chromium-baked, including the missing-value default, explicit
> user space, the exact object-box map, and invalid-value fallback. Percentages
> inside object-box content retain their viewport basis before that map;
> zero-area target boxes produce an empty clip.

- [ ] `crossorigin`
- [ ] `data-*`
- [ ] `decoding`
- [ ] `diffuseConstant`
- [x] `divisor`
- [ ] `download`
- [ ] `dx`
- [ ] `dy`
- [x] `edgeMode`

> **2026-08-27 close:** the complete case-sensitive `duplicate | wrap | none`
> grammar is Chromium-baked at actual input boundaries; missing and every
> invalid spelling select `duplicate`. The attribute's other listed
> applicability is blur, where current Chromium drops all spellings; one
> committed drop cell records that browser behavior.
- [ ] `elevation`
- [x] `exponent`
- [ ] `fetchpriority`
- [x] `filterUnits`

> **2026-08-25 close:** the complete case-sensitive
> `userSpaceOnUse | objectBoundingBox` grammar is Chromium-baked. Missing and
> invalid values take `objectBoundingBox`; the explicit opposite changes the
> filter region against a discriminating blur.

- [ ] `fr`
- [ ] `fx`
- [ ] `fy`
- [x] `gradientTransform`
- [x] `gradientUnits`
- [ ] `href`
- [ ] `hreflang`
- [ ] `id`
- [ ] `in`
- [ ] `in2`
- [x] `intercept`
- [x] `k1`
- [x] `k2`
- [x] `k3`
- [x] `k4`

> **2026-08-25 close:** all four arithmetic coefficients carry the complete
> SVG number grammar and initial zero. Individual interaction, foreground,
> background, and constant terms are Chromium-baked; signs, decimals, and
> exponents are carried, and output channels clamp to the unit interval.

- [x] `kernelMatrix`
- [ ] `kernelUnitLength`

> **2026-08-27 close/split:** `kernelMatrix` carries the complete SVG
> number-list grammar and must contain exactly `order-x × order-y` values.
> Missing, malformed, non-finite, or wrong-count matrices produce the measured
> transparent result. Kernels at the measured native strategy boundaries—28,
> 29, 64, 65, and 256 coefficients—are exact; 257 coefficients produce the
> celled Chromium drop. `kernelUnitLength` stays open: Chromium ignores every
> sampled valid and invalid spelling on convolution, while the attribute also
> applies to the still-open lighting primitives.
- [ ] `lang`
- [ ] `lengthAdjust`
- [ ] `limitingConeAngle`
- [ ] `markerHeight`
- [ ] `markerUnits`
- [ ] `markerWidth`
- [x] `maskContentUnits`
- [x] `maskUnits`

> **2026-08-24 close:** both attributes carry their complete case-sensitive
> `userSpaceOnUse | objectBoundingBox` grammar, missing-value defaults, and
> invalid-value fallback. Committed Chromium evidence discriminates both
> coordinate systems for the source image and the hard mask region, including
> target fill-box mapping, stroke exclusion, viewport/`viewBox` percentages,
> and explicit spellings of every enum member.

- [ ] `media`
- [ ] `method`
- [x] `mode`

> **2026-08-26 close:** `mode` applies only to `feBlend` and carries the
> complete case-sensitive `normal | multiply | screen | overlay | darken |
> lighten | color-dodge | color-burn | hard-light | soft-light | difference |
> exclusion | hue | saturation | color | luminosity` grammar. Missing, empty,
> invalid, wrong-case, whitespace-padded, legacy camelCase, CSS-wide, and
> draft-only spellings select the initial `normal`; Chromium ignores the
> sampled `no-composite` spelling without changing a valid mode.
- [x] `numOctaves`

> **2026-08-26 close:** `numOctaves` applies only to `feTurbulence`. Missing or
> invalid integer text selects one, leading plus and surrounding SVG whitespace
> are carried, positive values cap at nine, zero reaches the selected formula,
> and a negative integer produces the measured transparent result.
- [ ] `offset`
- [ ] `onabort`
- [ ] `onafterprint`
- [ ] `onbeforeprint`
- [ ] `onbegin`
- [ ] `oncancel`
- [ ] `oncanplay`
- [ ] `oncanplaythrough`
- [ ] `onchange`
- [ ] `onclick`
- [ ] `onclose`
- [ ] `oncopy`
- [ ] `oncuechange`
- [ ] `oncut`
- [ ] `ondblclick`
- [ ] `ondrag`
- [ ] `ondragend`
- [ ] `ondragenter`
- [ ] `ondragexit`
- [ ] `ondragleave`
- [ ] `ondragover`
- [ ] `ondragstart`
- [ ] `ondrop`
- [ ] `ondurationchange`
- [ ] `onemptied`
- [ ] `onend`
- [ ] `onended`
- [ ] `onerror`
- [ ] `onfocus`
- [ ] `onhashchange`
- [ ] `oninput`
- [ ] `oninvalid`
- [ ] `onkeydown`
- [ ] `onkeypress`
- [ ] `onkeyup`
- [ ] `onload`
- [ ] `onloadeddata`
- [ ] `onloadedmetadata`
- [ ] `onloadstart`
- [ ] `onmessage`
- [ ] `onmousedown`
- [ ] `onmouseenter`
- [ ] `onmouseleave`
- [ ] `onmousemove`
- [ ] `onmouseout`
- [ ] `onmouseover`
- [ ] `onmouseup`
- [ ] `onoffline`
- [ ] `ononline`
- [ ] `onpagehide`
- [ ] `onpageshow`
- [ ] `onpaste`
- [ ] `onpause`
- [ ] `onplay`
- [ ] `onplaying`
- [ ] `onpopstate`
- [ ] `onprogress`
- [ ] `onratechange`
- [ ] `onrepeat`
- [ ] `onreset`
- [ ] `onresize`
- [ ] `onscroll`
- [ ] `onseeked`
- [ ] `onseeking`
- [ ] `onselect`
- [ ] `onshow`
- [ ] `onstalled`
- [ ] `onstorage`
- [ ] `onsubmit`
- [ ] `onsuspend`
- [ ] `ontimeupdate`
- [ ] `ontoggle`
- [ ] `onunload`
- [ ] `onvolumechange`
- [ ] `onwaiting`
- [ ] `onwheel`
- [ ] `operator`
- [x] `order`

> **2026-08-27 close:** convolution `order` carries one or two values; one
> supplies both axes, and Chromium truncates each finite value toward zero.
> Missing, empty, malformed, wrong-count, unit-bearing, CSS-function, and
> CSS-wide text selects the initial 3×3 order. A parsed non-positive member or
> an unconstructable finite area produces the measured transparent result. The
> wider invalid-spelling matrix is measured, not all separately celled.
- [ ] `orient`
- [ ] `path`
- [x] `pathLength`
- [ ] `patternContentUnits`
- [ ] `patternTransform`
- [ ] `patternUnits`
- [ ] `ping`
- [ ] `playbackorder`
- [ ] `points`
- [ ] `pointsAtX`
- [ ] `pointsAtY`
- [ ] `pointsAtZ`
- [x] `preserveAlpha`

> **2026-08-27 close:** the complete case-sensitive `false | true` grammar is
> Chromium-baked. Missing and invalid spellings select `false`; committed
> SourceAlpha and positive-bias pairs distinguish convolved alpha from
> preserved input alpha.
- [x] `preserveAspectRatio`
- [x] `primitiveUnits`

> **2026-08-25 close:** the complete case-sensitive
> `userSpaceOnUse | objectBoundingBox` grammar is Chromium-baked. Missing and
> invalid values take user space; the object-box branch scales each blur axis
> through the target fill-geometry box.

- [ ] `radius`
- [ ] `referrerpolicy`
- [ ] `refX`
- [ ] `refY`
- [ ] `rel`
- [ ] `requiredExtensions`
- [ ] `result`
- [ ] `role`
- [ ] `rotate`
- [x] `scale`
- [x] `seed`

> **2026-08-26 close:** displacement `scale` and turbulence `seed` each carry
> the complete signed SVG-number grammar with initial zero, including leading
> plus, exponent, and the measured lone trailing comma. Invalid, unit-bearing,
> percentage, CSS-function, and multi-member text selects the initial. Scale
> keeps its sign and fractions; the noise formula truncates fractional seed
> values toward zero as Chromium does.
- [ ] `side`
- [x] `slope`
- [ ] `spacing`
- [ ] `specularConstant`
- [ ] `specularExponent`
- [x] `spreadMethod`
- [ ] `startOffset`
- [ ] `stdDeviation`
- [x] `stitchTiles`

> **2026-08-26 close:** `stitchTiles` applies only to `feTurbulence` and carries
> the complete case-sensitive `stitch | noStitch` grammar. Missing, empty,
> invalid, wrong-case, whitespace-padded, and CSS-wide spellings select the
> initial `noStitch`; stitched fractional primitive regions are Chromium-baked.
- [ ] `style`
- [ ] `surfaceScale`
- [ ] `systemLanguage`
- [ ] `tabindex`
- [x] `tableValues`

> **2026-08-25 close:** the five function-parameter attributes close at their
> complete applicability to `<feFuncR>`/`G`/`B`/`A`. `amplitude` and
> `exponent` carry gamma's initial one; `offset` remains open because it is a
> wider shared attribute. `slope` carries linear's initial one, `intercept`
> its initial zero, and `tableValues` carries the complete SVG number-list
> grammar for table/discrete, including absent, empty, singleton, multiple,
> malformed, signed, exponent, separator, trailing-comma, and out-of-range
> cases. Blink's ordered source-number normalization, function arithmetic,
> clamping, and byte truncation are Chromium-gated. The wider shared `type`
> row remains open.
- [ ] `target`
- [x] `targetX`
- [x] `targetY`

> **2026-08-27 close:** both target coordinates carry the signed SVG-integer
> grammar. Missing values default independently to the floor of half their
> order axis. An authored lexical failure or integer-storage overflow selects
> zero; a valid negative or value outside its kernel axis produces the measured
> transparent result. Asymmetric x/y cells distinguish both coordinates.
- [ ] `textLength`
- [ ] `timelinebegin`
- [ ] `title`
- [ ] `type`
- [ ] `viewBox`
- [ ] `x1`
- [ ] `x2`
- [x] `xChannelSelector`
- [ ] `xlink:href`
- [ ] `xlink:title`
- [ ] `xml:space`
- [ ] `y1`
- [ ] `y2`
- [x] `yChannelSelector`

> **2026-08-26 close:** both displacement selectors carry the complete
> case-sensitive `R | G | B | A` grammar independently. Missing, empty,
> invalid, wrong-case, whitespace-padded, and CSS-wide spellings select the
> initial `A`; committed pairs prove that selection occurs on the
> non-premultiplied displacement image.
- [ ] `z`


### SMIL timing and animation attributes

- [ ] `accumulate`
- [ ] `additive`
- [ ] `attributeName`
- [ ] `begin`
- [ ] `by`
- [ ] `calcMode`
- [ ] `dur`
- [ ] `end`
- [ ] `fill`
- [ ] `from`
- [ ] `keyPoints`
- [ ] `keySplines`
- [ ] `keyTimes`
- [ ] `max`
- [ ] `min`
- [ ] `origin`
- [ ] `repeatCount`
- [ ] `repeatDur`
- [ ] `restart`
- [ ] `to`
- [ ] `values`
