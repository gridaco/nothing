# websem

`websem` is the Web semantic compiler: it turns an SVG or HTML source into the
resolved render contract, and it decides what this engine will and will not
render. Everything upstream of it is parsing and cascading; everything
downstream is painting. The judgement about meaning happens here.

```text
source bytes
    -> one namespace-aware document      (csscascade)
    -> one Stylo cascade                 (csscascade)
    -> effective values: Base | Sample t (this crate)
    -> rframe::Frame                     (this crate)
    -> n0
```

Two entries, both retained. `from_standalone_svg` compiles an `.svg` document.
`from_html_inline_svg` compiles an HTML document's **first inline SVG** — when
that subtree is admitted the render succeeds and the rest of the page
contributes nothing, which is a pinned contract rather than an accident. A
retained session is compiled once and can then be read at Base or at an exact
signed-nanosecond sample without re-parsing.

| Module             | Ownership                                                                                                      |
| ------------------ | -------------------------------------------------------------------------------------------------------------- |
| `svg`              | the two entries, the element walk, the patrols, viewport mapping, shapes, paint                                |
| `svg_path`         | the `d` grammar, normalised to absolute commands                                                               |
| `svg_transform`    | the computed `transform` operation list, converted to one affine (the _attribute_ grammar lives in csscascade) |
| `svg_paint_server` | the gradient id table, href template chains, stops, and the fold of every gradient coordinate system into the contract's unit-box paints |
| `svg_animation`    | the closed exact-time sampling inventory                                                                       |
| `effective_values` | the Base-or-Sample view the compiler reads through                                                             |

## The admitted slice

Shapes `<rect>`, `<circle>`, `<ellipse>`, `<path>`, `<line>`, `<polygon>` and
`<polyline>`, each with a solid fill and a solid stroke (width, cap, join,
miter limit, and a path's fill rule — which the points shapes share).
Containers `<g>` with the whole `transform` grammar in both spellings — the
attribute enters the one cascade as a presentation hint of the CSS
`transform` property — flattened into a per-node affine. `<use>`/`<defs>`
same-document references, expanded into the one tree before the cascade
(csscascade's `svg_use`), rendered as containers of their shadow content.
`<linearGradient>`/`<radialGradient>` paint servers, resolved through a
whole-document first-id-wins table into the contract's gradient paints —
concentric radials only, stops from attributes, `gradientTransform` through
the one cascade as the transform property's hint on gradient elements.
Root sizing per SVG2 §8.2 with the full `preserveAspectRatio` grammar. One
exact-time `<animate attributeName="x">` on a top-level `<rect>`.

`crates/n0_cli/README.md` is the single statement of record for that slice and
what it refuses; this table is the compiler's map, not a second copy of it.

## Two admissions, one compiler

**Strict** refuses on the first construct outside the slice — the harness that
names the edge. **Best-effort**, the product default, compiles what it admits
and declares everything else as a named degradation: a subtree construct is
skipped by name at a stable path, a blocked dynamic surface that leaves Base
honest resolves every sample to Base, and a beyond-inventory animation element
— active at document load, so its target's authored state never honestly
renders — skips its target in every view, declared at the target's path.
Where nothing degrades the two are frame-identical, and a law checks that
over the whole corpus.

Neither mode ever guesses a pixel. That is the invariant the patrols exist for:
a construct the compiler cannot honour must refuse loudly or be declared, and a
patrol that over-refuses is preferred to one that lets a wrong pixel through.

## Anti-goals

- **Not a layout engine.** No box model, no flow, no intrinsic sizing. The HTML
  entry compiles an inline SVG subtree; it does not lay out the page.
- **Not a text shaper.** Text refuses; shaping, fonts and resources are the
  kernel's and are not admitted through here.
- **Not a painter.** It emits `rframe::Frame` and never touches a canvas.
- **Not a second cascade.** Property resolution belongs to Stylo via
  `csscascade`; this crate reads computed values and authored text, and adds no
  matcher of its own.
- **Not a coverage claim.** The slice above is a bounded enumeration with a
  stated edge, not a statement about SVG conformance.

## Boundaries worth knowing

The cascaded surface is patrolled for an enumerated property set; anything
beyond it is a named open boundary rather than a silent pass. A `<style>` sheet
is not attributable to one element without selector matching, so sheet-borne
findings are document-level: strict refuses the document, best-effort declares
once against the sheet. Lengths in units with no basis in this build (`vw`,
`ex`, and their family) refuse rather than resolve against the cascade's pinned
device.

Why this crate exists and what it succeeded is recorded in
[docs/wg/consolidation/svg-engine-of-record.md](../../docs/wg/consolidation/svg-engine-of-record.md).
