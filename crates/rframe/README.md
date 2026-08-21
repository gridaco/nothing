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

| Module   | Ownership                                                                                                  |
| -------- | ---------------------------------------------------------------------------------------------------------- |
| `frame`  | `Frame`, `FrameNode`, `Geometry`, the admitted paint stack and its post-paint alpha factor, and product identity |
| `path`   | `PathData` — checked absolute commands, fill rule, tight bounds solved once                                |
| `stroke` | `Stroke` — centred width, cap, join, miter limit, optional checked dash pattern, and finite `f64` `outset` |

Two details are load-bearing enough to state here. A node's `bounds` is the
**geometry's** box, never the ink's: a stroke paints outside it, so a consumer
that needs covered area inflates by `Stroke::outset()`. And a resolved value is
resolved — a stroke that would paint nothing is `None` rather than a stroke with
zero width, so no consumer re-derives "is this visible".

A `PaintStack` has one source-neutral `PaintAlphaFactor`. Each paint's own
alpha materializes first; the factor then modulates that entry before coverage
and source-over. On a multi-paint stack it applies independently to every entry
without changing their order. It is deliberately not opacity over the stack's
composite and creates no layer — that byte-distinct group meaning is a
`Scope`. Identity is the default, and zero resolves the complete stack to no
paint. Because a `Stroke` owns the same `PaintStack`, fill and stroke cross the
contract with one meaning and no source-specific duplicate field.

`Stroke::outset()` widens only the arithmetic for that derived,
direction-free bound. The resolved width and miter limit remain exact `f32`
facts, while every stroke admitted from finite members has a finite `f64`
outset. The square-cap case is rounded outward by one representable step so the
helper never understates the mathematical bound.

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
that still _references_ something (a pattern, an image resource, or an
unresolved context-paint relationship) or needs a focal geometry the shared
radial leaf cannot state remains inexpressible here. Source-level context paint
is not a new render fact: a producer must select and fully rebase its eventual
no-paint, solid, or gradient result before this boundary, without carrying the
context relation or its reference-box ownership into the frame. Beyond paint:
one stroke width and an optional immutable dash pattern. The pattern is an
even-length cycle of finite non-negative local-space intervals paired with one
finite local-space phase. Construction canonicalizes the phase modulo the
positive cycle; positive phase advances into the cycle, and the same phase
restarts at every contour. Source units, percentages, and authored odd-list
repetition resolve before this boundary; the node, rather than the dash
pattern, owns the transform. Path-length calibration remains inexpressible here
rather than being ignored or approximated. Geometry is rect, ellipse or path.
Constructs outside that — clips and groups as first-class nodes — are absent
rather than approximated, so a producer that meets one must refuse or declare
it rather than lower it into something this contract can hold.

Why this shape is chosen, and where the renderer joins it, is recorded in
[docs/wg/consolidation/n0-join-point.md](../../docs/wg/consolidation/n0-join-point.md).
