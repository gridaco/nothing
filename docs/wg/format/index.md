---
title: Format & Import Mapping
description: Specifications for authored engine formats, the frozen legacy converter input, and import know-how migrating to the chassis.
format: md
tags:
  - internal
  - wg
  - format
---

# Format & Import Mapping

Specifications for authored engine formats, plus the existing import trackers
whose target is the legacy v1 IR. During consolidation those trackers preserve
external-format parsing and mapping know-how for the Phase 3 SVG and Phase 4
HTML/CSS front ends; they are not `.grida` converter coverage and do not define
the end-state scene model.

## Specifications and RFDs

| Page                                              | Description                                                |
| ------------------------------------------------- | ---------------------------------------------------------- |
| [v1 Grida IR](./grida.md)                         | Non-normative legacy implementation inventory and converter-input context |
| [n0 XML](./n0-xml.md)                             | Open RFD for the authored, inspectable `.n0.xml` source    |
| [n0 XML properties](./n0-xml-properties.md)       | XML property names, applicability, and design placeholders |
| [n0 XML modules](./n0-xml-modules.md)             | Open linking/component RFD with a proving implementation   |
| [n0 XML component parameters](./n0-xml-component-parameters.md) | Open typed prop/arg RFD with a proving implementation        |
| [n0 XML component slots](./n0-xml-component-slots.md)           | Open named slot projection RFD with a proving implementation |
| [n0 XML durable addressing](./n0-xml-addressing.md)             | Version 4 node/use identity and typed effective-value RFD    |
| [n0 XML animation](./n0-xml-animation.md)                       | Decision deferring native syntax while SVG proves the kernel |

## Import mappings

| Page              | Description                                  |
| ----------------- | -------------------------------------------- |
| [CSS](./css.md)   | CSS → legacy v1 IR property mapping tracker  |
| [HTML](./html.md) | HTML element → legacy v1 IR mapping          |
| [SVG](./svg.md)   | SVG → usvg → legacy v1 IR mapping            |

## How to use these docs

The CSS, HTML, and SVG trackers use this status key: ✅ mapped | ⚠️ partial |
🔧 IR exists, not wired | ❌ IR missing | 🚫 out of scope. Their **IR Gaps**
sections describe the legacy v1 target and its accumulated import know-how.
That know-how migrates through agnostic front-end contracts; end-state
capability is granted through the consolidation scoreboard, not by extending
the legacy model. Only the packed schema named below is a frozen surface.

For the frozen legacy `.grida` converter input, see the [FlatBuffers
schema](../../../format/grida.fbs).

## Related

- **Scene box contract:** [The `anchor` Box Model](../feat-layout/anchor.md)
- **Frozen v1 packed schema:** [`format/grida.fbs`](../../../format/grida.fbs) — the converter-input format
- **Legacy v1 Rust model:** [`crates/grida/src/node/schema.rs`](../../../crates/grida/src/node/schema.rs)
- **Legacy v1 TypeScript model:** [`packages/grida-canvas-schema/grida.ts`](https://github.com/gridaco/grida/blob/main/packages/grida-canvas-schema/grida.ts)
- **HTML import pipeline:** [`crates/grida/src/import/html/`](../../../crates/grida/src/import/html)
- **SVG import pipeline:** [`crates/grida/src/import/svg/`](../../../crates/grida/src/import/svg)
