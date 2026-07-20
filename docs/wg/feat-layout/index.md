---
title: Layout
description: "Working-group home for the engine's box, positioning, flex, grid, and cross-node layout contracts."
tags:
  - internal
  - wg
  - layout
  - canvas
format: md
---

# Layout

This cluster owns the engine's layout contracts: how node boxes are authored,
how parents place children, and how structured layout consumes those boxes.
It bridges free-positioned graphics and layout-managed interfaces without
making either context a special scene model.

## Specifications

| Page | Status | Scope |
| --- | --- | --- |
| [The `anchor` Box Model](./anchor.md) | Open RFD — graduation draft | Box sources, parent-relative bindings, size intent, layout participation, visual-only transforms, derived boxes, pure resolution, and read/write semantics |
| [Flex Layout Profile](./flex.md) | Open RFD — extracted semantic profile | The bounded row/column, wrap, gap, grow, alignment, stretch, and free-positioned-child exclusion contract |

## Domain boundary

The `anchor` box model is the current parent-relative foundation. It owns
the box that every layout algorithm consumes, but it does not make one
algorithm part of that box:

- free-positioned children resolve parent-relative pins and spans;
- [flex](./flex.md) places in-flow child boxes and is judged against the
  applicable web contract and Chromium evidence;
- grid is a named extension, not current `anchor` behavior; and
- arbitrary-node anchoring is a named extension beyond the present
  direct-parent binding model.

Editor commands that create or infer layout belong to
[Canvas](../canvas/index.md). Source-language spelling belongs to
[Format](../format/index.md). Text measurement belongs to
[Universal Shaped Text Layout](../feat-paragraph/text-layout.md). These
domains project into the box model and do not restate it.

Interoperability with CSS, SVG, and design-tool models is a measured
conformance requirement. It is not assumed to be lossless, and neither
existing engine is the oracle.
