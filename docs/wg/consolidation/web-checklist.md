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
- [ ] `opacity`
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
- [ ] `mask-border-slice`
- [ ] `mask-border-width`
- [ ] `mask-border-outset`
- [ ] `mask-border-repeat`


### Filter effects

- [ ] `filter`
- [ ] `backdrop-filter`
- [ ] `color-interpolation-filters`
- [ ] `flood-color`
- [ ] `flood-opacity`
- [ ] `lighting-color`


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
- [ ] `rx`
- [ ] `ry`
- [ ] `x`
- [ ] `y`
- [ ] `d`
- [ ] `fill`
- [x] `fill-opacity`
- [x] `fill-rule`
- [ ] `stroke`
- [ ] `stroke-width`
- [x] `stroke-opacity`
- [ ] `stroke-dasharray`
- [ ] `stroke-dashoffset`
- [x] `stroke-linecap`
- [x] `stroke-linejoin`
- [x] `stroke-miterlimit`
- [ ] `marker`
- [ ] `marker-start`
- [ ] `marker-mid`
- [ ] `marker-end`
- [ ] `paint-order`
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
- [x] `<defs>`
- [x] `<desc>`
- [x] `<ellipse>`
- [ ] `<feBlend>`
- [ ] `<feColorMatrix>`
- [ ] `<feComponentTransfer>`
- [ ] `<feComposite>`
- [ ] `<feConvolveMatrix>`
- [ ] `<feDiffuseLighting>`
- [ ] `<feDisplacementMap>`
- [ ] `<feDistantLight>`
- [ ] `<feDropShadow>`
- [ ] `<feFlood>`
- [ ] `<feFuncA>`
- [ ] `<feFuncB>`
- [ ] `<feFuncG>`
- [ ] `<feFuncR>`
- [ ] `<feGaussianBlur>`
- [ ] `<feImage>`
- [ ] `<feMerge>`
- [ ] `<feMergeNode>`
- [ ] `<feMorphology>`
- [ ] `<feOffset>`
- [ ] `<fePointLight>`
- [ ] `<feSpecularLighting>`
- [ ] `<feSpotLight>`
- [ ] `<feTile>`
- [ ] `<feTurbulence>`
- [ ] `<filter>`
- [ ] `<foreignObject>`
- [x] `<g>`
- [ ] `<image>`
- [x] `<line>`
- [x] `<linearGradient>`
- [ ] `<marker>`
- [ ] `<mask>`
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
- [ ] `color`
- [ ] `color-interpolation`
- [ ] `color-interpolation-filters`
- [ ] `cursor`
- [ ] `cx`
- [ ] `cy`
- [ ] `d`
- [ ] `direction`
- [ ] `display`
- [ ] `dominant-baseline`
- [ ] `fill`
- [x] `fill-opacity`
- [x] `fill-rule`
- [ ] `filter`
- [ ] `flood-color`
- [ ] `flood-opacity`
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
- [ ] `mask-type`
- [ ] `opacity`
- [ ] `overflow`
- [ ] `paint-order`
- [ ] `pointer-events`
- [ ] `r`
- [x] `rx`
- [x] `ry`
- [ ] `shape-rendering`
- [ ] `stop-color`
- [ ] `stop-opacity`
- [ ] `stroke`
- [ ] `stroke-dasharray`
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


### SVG attributes

- [ ] `amplitude`
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
- [ ] `baseFrequency`
- [ ] `bias`
- [ ] `class`
- [ ] `clipPathUnits`
- [ ] `crossorigin`
- [ ] `data-*`
- [ ] `decoding`
- [ ] `diffuseConstant`
- [ ] `divisor`
- [ ] `download`
- [ ] `dx`
- [ ] `dy`
- [ ] `edgeMode`
- [ ] `elevation`
- [ ] `exponent`
- [ ] `fetchpriority`
- [ ] `filterUnits`
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
- [ ] `intercept`
- [ ] `k1`
- [ ] `k2`
- [ ] `k3`
- [ ] `k4`
- [ ] `kernelMatrix`
- [ ] `kernelUnitLength`
- [ ] `lang`
- [ ] `lengthAdjust`
- [ ] `limitingConeAngle`
- [ ] `markerHeight`
- [ ] `markerUnits`
- [ ] `markerWidth`
- [ ] `maskContentUnits`
- [ ] `maskUnits`
- [ ] `media`
- [ ] `method`
- [ ] `mode`
- [ ] `numOctaves`
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
- [ ] `order`
- [ ] `orient`
- [ ] `path`
- [ ] `pathLength`
- [ ] `patternContentUnits`
- [ ] `patternTransform`
- [ ] `patternUnits`
- [ ] `ping`
- [ ] `playbackorder`
- [ ] `points`
- [ ] `pointsAtX`
- [ ] `pointsAtY`
- [ ] `pointsAtZ`
- [ ] `preserveAlpha`
- [x] `preserveAspectRatio`
- [ ] `primitiveUnits`
- [ ] `radius`
- [ ] `referrerpolicy`
- [ ] `refX`
- [ ] `refY`
- [ ] `rel`
- [ ] `requiredExtensions`
- [ ] `result`
- [ ] `role`
- [ ] `rotate`
- [ ] `scale`
- [ ] `seed`
- [ ] `side`
- [ ] `slope`
- [ ] `spacing`
- [ ] `specularConstant`
- [ ] `specularExponent`
- [x] `spreadMethod`
- [ ] `startOffset`
- [ ] `stdDeviation`
- [ ] `stitchTiles`
- [ ] `style`
- [ ] `surfaceScale`
- [ ] `systemLanguage`
- [ ] `tabindex`
- [ ] `tableValues`
- [ ] `target`
- [ ] `targetX`
- [ ] `targetY`
- [ ] `textLength`
- [ ] `timelinebegin`
- [ ] `title`
- [ ] `type`
- [ ] `viewBox`
- [ ] `x1`
- [ ] `x2`
- [ ] `xChannelSelector`
- [ ] `xlink:href`
- [ ] `xlink:title`
- [ ] `xml:space`
- [ ] `y1`
- [ ] `y2`
- [ ] `yChannelSelector`
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
