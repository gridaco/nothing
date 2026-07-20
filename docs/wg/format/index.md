---
title: Format & Import Mapping
description: Specifications for authored engine formats and legacy-v1 trackers for importing external formats.
format: md
tags:
  - internal
  - wg
  - format
---

# Format & Import Mapping

Specifications for authored engine formats, plus the existing import trackers
whose target is the legacy v1 IR. During consolidation those trackers describe
converter-input coverage; they do not define the end-state scene model.

## Specifications and RFDs

| Page                                              | Description                                                |
| ------------------------------------------------- | ---------------------------------------------------------- |
| [v1 Grida IR](./grida.md)                         | Legacy `.grida` IR reference and converter-input context   |
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
sections describe the legacy v1 target. End-state capability is granted
through the consolidation scoreboard, not by extending that frozen target.

For the on-disk `.grida` file format, see the [FlatBuffers
schema](../../../format/grida.fbs).

## Related

- **Scene box contract:** [The `anchor` Box Model](../feat-layout/anchor.md)
- **Frozen v1 packed schema:** [`format/grida.fbs`](../../../format/grida.fbs) — the converter-input format
- **Legacy v1 Rust model:** [`crates/grida/src/node/schema.rs`](../../../crates/grida/src/node/schema.rs)
- **Legacy v1 TypeScript model:** [`packages/grida-canvas-schema/grida.ts`](https://github.com/gridaco/grida/blob/main/packages/grida-canvas-schema/grida.ts)
- **HTML import pipeline:** [`crates/grida/src/import/html/`](../../../crates/grida/src/import/html)
- **SVG import pipeline:** [`crates/grida/src/import/svg/`](../../../crates/grida/src/import/svg)
