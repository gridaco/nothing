---
title: "Finding: SVG paint in the shared cascade"
description: "The servo-engine Stylo the workspace compiles omits the SVG paint longhands, so `fill`/`stroke` cannot come from the shared cascade. Enumerated evidence, the evaluated options, and the owner decision this gates."
tags:
  - internal
  - wg
  - program
format: md
---

# Finding: SVG paint in the shared cascade

**Genre:** finding — grounded evidence for an **open owner decision**. Not a
spec and not a plan. It records what was established while building the
[Web-first prototype](./web-first.md), so the decision it gates can be taken on
evidence rather than assumption.

**Status:** open as **D-L** in the
[charter's registry](./charter.md). No option below is chosen here.

## The crux

The Web-first direction requires HTML and SVG to share **one** browser-grade
cascade, so that a rule like `.mark { fill: … }` authored anywhere in the
document reaches an SVG descendant through the same cascade that styles HTML.
The prototype proves the cascade *crosses the boundary* — an HTML `<style>`
rule reaches an inline-SVG element — but only for properties the cascade
actually models. **SVG paint is not among them**, so today the SVG semantic
compiler reads only direct `fill` presentation attributes outside the cascade;
an inline `style` declaration for `fill` is dropped with the unknown longhand.
Closing that gap is a real cost with no free option; this finding lays the
options out.

## Evidence

- **E1 — the entire SVG paint/geometry property set is absent under the
  compiled engine.** Stylo splits its property database by engine (servo vs
  gecko). The workspace compiles the **servo** engine; property declarations
  marked gecko-only are never registered, so they do not exist in the computed
  style. Of the 46 SVG-struct longhands in Stylo 0.16, **44 are gecko-only** —
  the whole of `fill`, `fill-opacity`, `fill-rule`, `stroke`, `stroke-width`,
  `stroke-dasharray`, `stroke-linecap`/`-linejoin`/`-miterlimit`/`-opacity`/
  `-dashoffset`, `paint-order`, `marker-start`/`-mid`/`-end`, `text-anchor`,
  `clip-rule`, `color-interpolation`(`-filters`), `shape-rendering`,
  `vector-effect`, `stop-color`/`-opacity`, `flood-`/`lighting-color`, the SVG
  geometry presentation properties (`x`,`y`,`cx`,`cy`,`r`,`rx`,`ry`,`d`), and
  the SVG mask longhands. Only `clip-path` and `mask-image` — shared box
  properties, not SVG paint — survive under servo. A `fill` declaration in a
  stylesheet or inline `style` is therefore an unknown declaration, dropped at
  parse time; there is no computed value to read.

- **E2 — CSS custom properties do cascade and read back under servo, but as
  untyped token strings.** A custom property set on an HTML `<style>` rule
  (`.mark { --x: #16a34a }`) cascades to an inline-SVG descendant with full
  inheritance and specificity and is readable from the computed custom-property
  map — verified empirically. But the value returns as the **raw token string**
  (`"#16a34a"`): no computed-value processing, no `currentColor` resolution
  against `color`, no `url(#…)` paint-server binding, no type checking, and
  custom properties inherit by default with their own invalidation semantics.
  The carrier reproduces cascade *mechanics* for a paint value; it does not
  reproduce SVG paint *computed values*.

- **E3 — the gecko engine is not a viable build here.** Enabling Stylo's gecko
  engine (which registers all the SVG longhands) pulls `bindgen`, `mozbuild`,
  and `nsstring` — the Mozilla/Gecko build system and its C++ bindings. That is
  not buildable in this pure-Rust Cargo workspace.

- **E4 — the SVG value types themselves compile under servo.** The generic,
  specified, and computed `svg` value modules (e.g. the generic SVG-paint type)
  are compiled unconditionally, not gecko-gated. Only the property
  *declarations* are engine-gated. A fork that un-gated the SVG paint longhands
  for servo is therefore *plausible* — but unverified at build depth, and it
  means carrying a patch to a large, fast-moving dependency.

- **E5 — property availability is necessary but not sufficient.** The proving
  shell currently has three independent intake gaps: its standalone entry uses
  HTML foreign-content parsing rather than the SVG/XML grammar; author-style
  collection recognizes HTML-namespace `style` elements but not an SVG
  `style` element; and presentation hints are not synthesized into the
  cascade. Un-gating `fill` alone would therefore still miss standalone XML,
  SVG-authored stylesheets, and presentation-attribute precedence.

- **E6 — the current direct-attribute fallback is narrower than previously
  stated.** It reads a `fill` attribute and resolves `currentColor` through the
  computed `color`. It does not parse `fill` from an inline `style`, so only
  the direct-attribute case is in hand. This distinction matters because the
  status quo does not cover both authoring forms.

## The options

| # | Option | What it buys | What it costs | Feasibility |
| --- | --- | --- | --- | --- |
| 1 | **Gecko-engine Stylo** | Every SVG longhand, true computed values | A Gecko build environment | **Not viable** here (E3) |
| 2 | **Fork/patch Stylo** — un-gate the SVG paint longhands for servo and complete the missing cascade intake | Real SVG paint in the shared cascade, true computed values | Vendoring + carrying a patch on a fast-moving crate; build-verifying each longhand's servo glue; adding the SVG/XML, SVG-stylesheet, and presentation-hint ingress proved missing by E5 | Plausible, unverified (E4–E5); high maintenance |
| 3 | **Custom-property carrier** — rewrite `fill`/`stroke`/… to `--*` at stylesheet + presentation-attribute intake, cascade those, read them back | Correct cascade + inheritance + specificity for the *paint value* | Rewriting author CSS (shorthands, `all`, specificity); a no-op presentation-hint stub to implement; **loses** SVG paint computed-value semantics — the compiler must re-resolve `currentColor`, paint servers, types outside Stylo | Viable for mechanics (E2), lossy on semantics |
| 4 | **Status quo** — read paint from a direct presentation attribute outside the cascade (what the prototype does) | Correct for the direct attributes exercised by the proving shell; honest and free | Inline style and stylesheet paint are dropped; presentation attributes do **not** participate in the shared cascade | In hand, deliberately narrow (E6) |
| 5 | **Track upstream** — a future Stylo enabling SVG under servo | Eventually option 1's result with no fork | No signal it is planned; upstream servo SVG support is historically minimal | Not in 0.16 (E1) |

A refinement of option 3: **registered** custom properties (`@property` with a
`syntax`, e.g. `<color>`) could recover typed computed values for the simple
color case — but not paint-server (`url(#…)`) or context-dependent paint
semantics, and servo `@property` support here is itself unverified. It narrows
the loss, it does not remove it, and it needs its own spike.

## Recommendation (for the owner to decide)

No option is free, so this is a genuine registry decision. The evidence
suggests:

- **Keep option 4 only as the proving-shell posture.** It is correct for the
  direct attributes the prototype's fixtures actually use and is honest about
  what it does not do. It is not an entry into SVG-vector capability work.
- **Do not adopt option 3 as the cascade of record.** A carrier that reproduces
  cascade mechanics while discarding SVG paint semantics is, in spirit, another
  temporary shim — the same thing the [amendment](./web-first.md) rules out when
  it forbids promoting the temporary SVG-only matcher. It may have a place as a
  *narrow* bridge for the cascaded-rule case, but only behind an explicit
  decision, never as the default.
- **Scope option 2 as the path most faithful to "one browser-grade cascade."**
  It is the only option that yields real SVG paint computed values without
  Gecko. Before committing to it, a timeboxed fork-feasibility spike should
  confirm the servo build actually computes the un-gated longhands — E4 makes
  it plausible, not proven.

## The decision to file

**D-L** is registered in the [charter's decision registry](./charter.md): *how
SVG paint enters the shared cascade* — Gecko vs a servo-capable Stylo fork vs a
scoped custom-property carrier vs direct-attribute status quo. Its evidence bar
is this finding plus a bounded feasibility bundle that proves the full ingress,
not merely a longhand build: the SVG/XML grammar entry, SVG-namespace
stylesheet intake, presentation-hint precedence, minimal paint-longhand
computation, and precedence/inheritance/currentColor/invalid-value behavior.
Until the owner decides it, the Web-first path reads direct paint attributes
outside the cascade, says so, and does not accrete SVG-vector capability on
that scaffold.
