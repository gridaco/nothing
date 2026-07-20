---
title: "Flex Layout Profile"
description: "Open RFD for the bounded flex layout contract consumed by the anchor box model."
tags:
  - internal
  - wg
  - layout
  - canvas
format: md
---

# Flex Layout Profile

**Status:** Open RFD — extracted semantic profile.

This document is written for an engine implementer resolving in-flow child
boxes. It is the semantic home for the bounded flex behavior exposed by the
scene model and its authored languages. The adopted clauses below are a
profile rather than a claim of full CSS Flexbox conformance; valid but
ungated edge cases remain explicit unresolved semantics.

## Thesis

Flex consumes and produces sizing boxes.

The [`anchor` Box Model](./anchor.md) supplies each child's size intent,
natural size, constraints, activity, layout participation, and
visual-transform boundary. This profile arranges the in-flow boxes. It never
reads paint or visual transforms, never rewrites authored intent, and never
gives a free-positioned child a flow slot.

## Scope

This profile owns:

- row and column main axes;
- source-order participation;
- one or multiple lines;
- main- and cross-axis gaps;
- container padding;
- positive-space growth;
- main-axis distribution;
- cross-axis alignment and self-alignment;
- the distinction between container cross-stretch and child self-stretch;
- free-positioned-child exclusion; and
- flex contribution to an automatic container extent.

It does not own source-language spelling, grid, block flow, baseline
alignment, reverse ordering, margin, intrinsic sizing keywords, layout
inference, or visual transforms.

## Vocabulary

| Term | Meaning |
| --- | --- |
| **Flex container** | A box whose selected layout mode is this profile. |
| **In-flow child** | A child that receives a flex slot and participates in gaps, growth, and alignment. |
| **Free-positioned child** | A child whose anchor bindings place it without a flex slot. |
| **Main axis** | The row or column axis along which items and main gaps are placed. |
| **Cross axis** | The axis perpendicular to the main axis. |
| **Basis** | The child's resolved sizing extent before positive free space is distributed. |
| **Line** | One ordered run of in-flow children produced by wrapping. |
| **Inner box** | The flow-layout box after container padding is removed. |

## Container inputs

A flex container declares:

- direction: row or column;
- wrapping: disabled or enabled;
- main- and cross-axis gaps;
- top, right, bottom, and left padding;
- main alignment: start, center, end, space-between, space-around, or
  space-evenly; and
- cross alignment: start, center, end, or stretch.

Padding and gaps are non-negative. Padding establishes the inner box in which
flow layout resolves.

Each in-flow child may declare a non-negative grow factor and an optional
self-alignment of auto, start, center, end, or stretch. Auto uses the
container's cross alignment.

Those inputs may impose an extent only where the child's box-source rule
admits layout-owned size. Whether grow or stretch may alter a derived box is
unresolved by the `anchor` RFD. A consumer must report or reject that case
until the derived-kind applicability row is adopted; it must not silently
treat a derived box as an ordinary resizable box.

## Resolution contract

### Participation and basis

Active in-flow children participate in source order. An inactive child and a
free-positioned child consume no slot, no gap, and no share of free space.

An adopted basis is the anchor pre-layout sizing extent after applicable size
intent, aspect, and constraints resolve. This covers declared and derived axes
and a Fixed axis on a measured kind. How an Auto extent supplied by
measurement becomes the main-axis basis in a definite flex container remains
unresolved below. Rotation, flips, content ink, and paint do not change an
adopted basis.

### Lines and gaps

With wrapping disabled, every in-flow child belongs to one line. Main gaps
occur only between adjacent children on that line. An empty or single-item
line does not invent a gap.

The input vocabulary also admits wrapping and a cross gap, but the graduation
does not yet adopt multi-line geometry. Definite- and Auto-main wrapping,
cross-axis line distribution, and cross-gap placement remain unresolved
below.

### Growth and overflow

On a definite main axis, when remaining space is positive, every participating
child's main-axis basis is admitted by the basis clause above, no growing child
has an applicable min/max or aspect constraint, and the sum of positive grow
factors is at least one, that space is divided in proportion to those factors.
A zero grow factor receives none. On an Auto main axis, growth does not invent
free space; the single-line contribution uses the children's bases.

This profile performs no implicit shrink. When the bases and gaps exceed the
available main extent, the line overflows unless another applicable
constraint has already changed a basis. Sub-unit total grow and constraint
freeze/redistribution remain unresolved below.

### Main alignment

When remaining space is non-negative after growth and the line contains no
unresolved basis or growth case:

- start, center, and end place the packed line at the corresponding main
  position;
- space-between distributes free space only between items;
- space-around gives every item equal surrounding shares; and
- space-evenly makes every outer and inner interval equal.

Those distributed rules are adopted for at least two children. The
single-child fallback for space-between, space-around, and space-evenly
remains unresolved below.

The declared main gap remains a minimum interval before distributable space
is added. If the line overflows, start alignment keeps its packed start
position. Overflow positioning for center, end, and the distributed modes is
unresolved below.

### Cross alignment and the two stretches

Start, center, and end place a child within its line's cross extent.

Container cross-stretch and child self-stretch are different contracts:

- container cross-stretch changes only a child whose authored cross-axis size
  is Auto; a Fixed cross size remains fixed;
- explicit child self-stretch is fill intent and overrides a Fixed cross size.

If stretch changes the constraint of measured content, that content is
resolved once at the final stretched extent before its dependent axis is
chosen.

These adopted stretch clauses cover an axis without an applicable min/max
constraint. Stretch combined with min/max or aspect constraints remains
unresolved below.

### Free-positioned children

A free-positioned child is removed from line construction. Its anchor
bindings resolve against the container sizing box, not a flex slot or the
padded flow box. It may overlap in-flow children and does not affect their
slots, gaps, growth, or alignment.

### Single-line automatic container extent

With wrapping disabled, an Auto main-axis contribution is the sum of child
bases and intervening main gaps plus main-axis padding. An Auto cross-axis
contribution is the greatest child cross extent plus cross-axis padding. An
empty container contributes its padding box. Growth does not create free
space on an Auto main axis.

Re-resolution may move or resize children without writing those results into
the document. Automatic extents for multi-line layout remain unresolved.

## Adopted conformance contracts

| ID | Contract | Gate |
| --- | --- | --- |
| **FLEX-1** | Only active in-flow children consume slots and gaps, in source order. | Mixed active/inactive and in-flow/free-positioned corpus. |
| **FLEX-2** | A basis from a declared or derived axis, or a Fixed axis on a measured kind, comes from the anchor sizing box and ignores visual transforms and paint. | Box-source × transformed/untransformed cases, excluding Auto measured main-axis basis. |
| **FLEX-3** | With total grow ≥ 1, positive free space is divided by grow when every participating main-axis basis is FLEX-2-admitted and no growing child has min/max or aspect constraints; negative free space does not implicitly shrink. | Analytic admitted-growth and start-aligned overflow cases. |
| **FLEX-4** | With non-negative remaining space and only FLEX-2/FLEX-3-admitted inputs, main alignment and gaps follow the declared distribution rules. | Every main alignment with two or more children; start/center/end with one child. |
| **FLEX-5** | Without a cross-axis min/max constraint, container cross-stretch respects Fixed and child self-stretch overrides it. | Auto/Fixed × container/self matrix. |
| **FLEX-6** | With wrapping disabled, active in-flow children form one line with main gaps only between adjacent children. | Zero/one/many child and zero-size child cases. |
| **FLEX-7** | Free-positioned children resolve against the container sizing box and do not affect flow. | Binding and layout-diff cases. |
| **FLEX-8** | Single-line automatic extents use bases, gaps, and padding without source writeback or invented grow space. | Empty/non-empty Auto-axis and document-identity cases. |

For behavior shared with the web platform, the oracle is the applicable web
contract and Chromium. Deliberate deviations, including the absence of
implicit shrink, are explicit profile rules rather than accidental
differences.

## Unresolved profile semantics

The graduation evidence does not yet settle:

- an Auto extent supplied by measurement as the main-axis basis in a definite
  flex container;
- grow on a main-axis basis supplied by measurement;
- positive growth when the sum of grow factors is between zero and one;
- grow or stretch combined with min/max or aspect constraints, including
  freeze and redistribution after a tentative extent;
- `wrap=true`, including an Auto main axis, line breaking, cross gaps,
  multi-line cross-axis distribution, and automatic multi-line extents;
- center, end, space-between, space-around, and space-evenly positioning when
  a no-shrink line overflows;
- the single-child fallback for space-between, space-around, and
  space-evenly; and
- grow or stretch applied to a derived box.

These rows require an independent web-contract and Chromium corpus before
adoption. A layout library's current default is evidence, not a decision, and
neither existing engine is the oracle. Until a row is adopted, a consumer must
preserve the source intent and report the unsupported or implementation-defined
result rather than silently presenting it as profile conformance.

## Source projections

The [n0 XML RFD](../format/n0-xml.md) owns one authored spelling and its strict
applicability/default rules. Other source languages may expose different
syntax, but they project to this same profile or report an explicit
degradation.
