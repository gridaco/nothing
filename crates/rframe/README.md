# rframe

`rframe` is the resolved render contract: the visual facts a producer states
after it has finished resolving its own source, in the form the renderer
consumes. A producer says _what is on the canvas_; this crate is the vocabulary
it says it in.

Everything here is derived. A `Frame` is a viewport and a list of nodes in
painter order, and each node carries an identity, a transform into frame space,
one geometry in local space, that geometry's exact bounds, a paint stack and an
optional stroke. There is no document, no cascade, no source syntax and no
element — those belong to whoever produced the frame.

```text
producer (e.g. websem, from SVG)
    -> rframe::Frame          <- this crate: resolved facts, backend-free
    -> n0                     <- the one consumer: private drawlist and painter
```

## What it holds

| Module   | Ownership                                                                        |
| -------- | -------------------------------------------------------------------------------- |
| `frame`  | `Frame`, `FrameNode`, `Geometry`, the admitted paint stack, and product identity |
| `path`   | `PathData` — checked absolute commands, fill rule, tight bounds solved once      |
| `stroke` | `Stroke` — centred, one width, cap, join, miter limit, and its `outset`          |

Two details are load-bearing enough to state here. A node's `bounds` is the
**geometry's** box, never the ink's: a stroke paints outside it, so a consumer
that needs covered area inflates by `Stroke::outset()`. And a resolved value is
resolved — a stroke that would paint nothing is `None` rather than a stroke with
zero width, so no consumer re-derives "is this visible".

## Anti-goals

- **Not an authored source.** It cannot express a document, a selector, a
  cascade, an element or an attribute. A producer resolves those away first.
- **Not a file format.** Provisional, internal and breakable by design; nothing
  serializes it and there is no round-trip promise. The repository's serialized
  format is `format/grida.fbs`.
- **Not a renderer.** The crate is backend-free, and a test locks that: no Skia,
  no canvas, no paint call can enter it.
- **Not a second engine's contract.** It has exactly one consumer. The
  source-neutrality claim is checked by a canary that feeds the kernel from a
  second, independent producer (`crates/n0/tests/glyphless_canary.rs`) — the
  contract is neutral because two producers prove it, not because it says so.

## Boundaries

The vocabulary is deliberately narrower than SVG or CSS. Solid, linear- and
radial-gradient paints only — a gradient is a self-contained normal-blend
color ramp stated in the unit square of the geometry's own box, so a paint
that _references_ something (a pattern, an image resource, a context paint)
or needs a focal geometry the shared radial leaf cannot state remains
inexpressible here. Beyond paint: one stroke width; geometry is rect, ellipse
or path. Constructs outside that — dashes, clips, groups as first-class
nodes — are absent rather than approximated, so a producer that meets one must
refuse or declare it rather than lower it into something this contract can
hold.

Why this shape is chosen, and where the renderer joins it, is recorded in
[docs/wg/consolidation/n0-join-point.md](../../docs/wg/consolidation/n0-join-point.md).
