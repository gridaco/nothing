---
title: Format & Import Mapping
description: Specifications for Grida's authored formats and trackers for importing external formats into the Grida IR.
format: md
tags:
  - internal
  - wg
  - format
---

# Format & Import Mapping

Tracking docs for the Grida IR schema and how external formats map into it.

## Specifications and RFDs

| Page                                                                  | Description                                                  |
| --------------------------------------------------------------------- | ------------------------------------------------------------ |
| [Grida IR](./grida.md)                                                | Canonical IR reference — node types, paint, layout           |
| [n0 XML](./n0-xml.md)                                           | Open RFD for the authored, inspectable `.n0.xml` source   |
| [n0 XML properties](./n0-xml-properties.md)                     | XML property names, applicability, and design placeholders   |
| [n0 XML modules](./n0-xml-modules.md)                           | Open linking/component RFD with a proving implementation     |
| [n0 XML component parameters](./n0-xml-component-parameters.md) | Open typed prop/arg RFD with a proving implementation        |
| [n0 XML component slots](./n0-xml-component-slots.md)           | Open named slot projection RFD with a proving implementation |
| [n0 XML durable addressing](./n0-xml-addressing.md)             | Version 4 node/use identity and typed effective-value RFD    |
| [n0 XML animation](./n0-xml-animation.md)                       | Decision deferring native syntax while SVG proves the kernel |

## Import mappings

| Page              | Description                                      |
| ----------------- | ------------------------------------------------ |
| [CSS](./css.md)   | CSS → Grida IR property mapping and TODO tracker |
| [HTML](./html.md) | HTML element → Grida IR node mapping             |
| [SVG](./svg.md)   | SVG → usvg → Grida IR mapping and TODO tracker   |

## How to use these docs

The CSS, HTML, and SVG trackers use this status key: ✅ mapped | ⚠️ partial |
🔧 IR exists, not wired | ❌ IR missing | 🚫 out of scope. Their **IR Gaps**
sections identify schema changes that would unblock further progress.

For the on-disk `.grida` file format, see the [FlatBuffers
schema](../../../format/grida.fbs).

## Related

- **FlatBuffers schema:** [`format/grida.fbs`](../../../format/grida.fbs) — the canonical on-disk file format
- **Rust runtime model:** [`crates/grida/src/node/schema.rs`](../../../crates/grida/src/node/schema.rs)
- **TypeScript model:** [`packages/grida-canvas-schema/grida.ts`](https://github.com/gridaco/grida/blob/main/packages/grida-canvas-schema/grida.ts)
- **HTML import pipeline:** [`crates/grida/src/import/html/`](../../../crates/grida/src/import/html)
- **SVG import pipeline:** [`crates/grida/src/import/svg/`](../../../crates/grida/src/import/svg)
