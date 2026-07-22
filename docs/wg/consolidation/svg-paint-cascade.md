---
title: "Finding: SVG paint in the shared cascade"
description: "Why Servo Stylo omitted SVG longhands, what official upstream now supplies, the owner-taken dependency-provenance decision, and the remaining ingress and capability gaps."
tags:
  - internal
  - wg
  - program
format: md
---

# Finding: SVG paint in the shared cascade

**Genre:** finding and decision evidence. Not a spec and not a plan. It records
what was established while building the [Web-first prototype](./web-first.md),
the upstream change that followed, and the scope of the resulting owner
decision.

**Status:** **D-L taken 2026-07-23** in the
[charter's registry](./charter.md). SVG paint uses Servo-capable support
maintained in official upstream Stylo. A published release containing the
required support is preferred; until one exists, the dependency is fixed to an
immutable official-upstream revision. A floating branch and a private source
fork are outside the decision.

## The crux

The Web-first direction requires HTML and SVG to share **one** browser-grade
cascade, so that a rule like `.mark { fill: … }` authored anywhere in the
document reaches an SVG descendant through the same cascade that styles HTML.
The prototype proves the cascade *crosses the boundary* — an HTML `<style>`
rule reaches an inline-SVG element — but only for properties the cascade
actually models. The released Stylo version used by that prototype did not
model SVG paint under Servo, so its SVG semantic compiler read only direct
`fill` presentation attributes outside the cascade and dropped an inline
`style` declaration for `fill` as an unknown longhand. Official upstream has
since enabled the basic paint set under Servo. That resolves dependency
provenance; it does not by itself supply production SVG/XML ingress, consume
the computed values, or land an SVG capability.

## Evidence

- **E1 — at the prototype baseline, 44 of the 46 longhands in Stylo's SVG
  style structs were absent under the compiled engine.** Stylo splits its
  property database by engine (servo vs
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

- **E3 — the gecko engine does not satisfy this program's build constraint.**
  Enabling Stylo's gecko engine (which registers all the SVG longhands) pulls
  `bindgen`, `mozbuild`, and `nsstring` — the Mozilla/Gecko build system and its
  C++ bindings. That is not a standalone Cargo/servo build; adopting it would
  import Gecko's build environment into the engine.

- **E4 — a minimal servo-only fork is build- and behavior-feasible for `fill`
  and `stroke`.** A bounded Stylo 0.16 spike enabled only these two inherited
  longhands under servo and classified changes to them as repaint damage. The
  generated layout guard exposed one additional pointer-sized computed-style
  slot: after recording the expected 224-to-232-byte change, a servo-only build
  passed without `bindgen`, `mozbuild`, or `nsstring`. The cascade then produced
  native typed SVG-paint representations for colors, `none`, and a URL with a
  color fallback; the initial values remained black and `none`. It also proved
  inheritance, syntactically-invalid fallback to a lower declaration, and the
  distinct invalid-at-computed-value behavior of an unresolved `var()` (unset,
  then inherited, without reconsidering the lower declaration). `currentColor`
  remained typed through computed style; invoking Stylo's color-resolution
  operation on the consuming child's style resolved it against that child's
  computed `color`. The paint-server case proves typed URL and fallback
  preservation, not resource binding. This evidence covers two longhands, not
  the other gecko-only SVG longhands or their servo glue.

- **E5 — the three independent intake gaps are technically closeable without a
  second cascade.** Under HTML foreign-content parsing, a bounded spike admitted
  SVG-namespace `style` elements in document order and synthesized an
  empty-namespace `fill` presentation attribute as exactly one declaration at
  presentation-hint precedence. Author normal over presentation, inline normal
  over presentation, author important over inline normal, and later equal-
  specificity SVG stylesheet order all passed. A semicolon-bearing attribute
  was rejected as one invalid paint value rather than expanded into additional
  declarations. A separate strict SVG/XML spike populated the same frozen
  semantic DOM and used the same Stylo cascade. It proved a direct SVG document
  root with no synthetic HTML, non-HTML document mode, `:root`, SVG and XLink
  namespaces, case-sensitive element and attribute selectors, SVG-namespace
  stylesheet intake, inheritance, rejection of malformed XML, and rejection of
  wrong-case or wrong-namespace SVG roots. The selected spike parser accepts
  UTF-8 input and rejects DTDs; a production entry still requires deliberate
  non-UTF-8 and DTD policies, plus base-URL and external-resource policies.
  These are isolated feasibility results, not
  capabilities landed in the workspace.

- **E6 — the current direct-attribute fallback is narrower than previously
  stated.** It reads a `fill` attribute and resolves `currentColor` through the
  computed `color`. It does not parse `fill` from an inline `style`, so only
  the direct-attribute case is in hand. This distinction matters because the
  status quo does not cover both authoring forms.

- **E7 — official upstream Stylo now exposes the required basic paint set
  under Servo.** The published
  [`0.19.0`](https://github.com/servo/stylo/releases/tag/v0.19.0) release
  predates the relevant merge and therefore does not contain it. Official upstream
  [servo/stylo#383](https://github.com/servo/stylo/pull/383) enables `fill`,
  `fill-opacity`, `fill-rule`, `stroke`, `stroke-width`, `stroke-linecap`,
  `stroke-linejoin`, `stroke-dasharray`, `stroke-dashoffset`,
  `stroke-miterlimit`, and `stroke-opacity` for Servo. The tested immutable
  official-upstream revision is that exact merge,
  [`a64923b5d5c67313c81c5056f5e30ec0babb04d6`](https://github.com/servo/stylo/commit/a64923b5d5c67313c81c5056f5e30ec0babb04d6).
  At that revision, 24 longhands in Stylo's SVG style structs remain
  Gecko-only: `-moz-context-properties`, `clip-rule`, `color-interpolation`,
  `color-interpolation-filters`, `cx`, `cy`, `d`, `flood-color`,
  `flood-opacity`, `lighting-color`, `marker-start`, `marker-mid`, `marker-end`,
  `paint-order`, `r`, `rx`, `ry`, `shape-rendering`, `stop-color`,
  `stop-opacity`, `text-anchor`, `vector-effect`, `x`, and `y`. Later official
  upstream [servo/stylo#427](https://github.com/servo/stylo/pull/427) enables
  the eight geometry properties, but geometry is not load-bearing for D-L and
  is deliberately outside this minimal paint pin. This proves official paint
  dependency availability, not production ingress or rendered capability.

## The options

| # | Option | What it buys | What it costs | Feasibility |
| --- | --- | --- | --- | --- |
| 1 | **Gecko-engine Stylo** | All 46 longhands in Stylo's enumerated SVG style structs, with native typed computed representations | A Gecko build environment | **Not viable** under the program's standalone Cargo/servo constraint (E3) |
| 2 | **Private fork/patch Stylo** — un-gate the required SVG longhands for Servo and complete the missing cascade intake | Native typed SVG paint in the shared cascade | Carrying a private dependency patch and its upgrade burden | The spike proved feasibility (E4–E5), but official upstream now contains the required basic paint support; a private fork is outside D-L |
| 3 | **Custom-property carrier** — rewrite `fill`/`stroke`/… to `--*` at stylesheet + presentation-attribute intake, cascade those, read them back | Stylo custom-property inheritance and specificity mechanics for carrier tokens | Rewriting author CSS (shorthands, `all`, specificity); a no-op presentation-hint stub to implement; **loses** SVG paint computed-value semantics — the compiler must re-resolve `currentColor`, paint servers, types outside Stylo | Viable for mechanics (E2), lossy on semantics |
| 4 | **Status quo** — read paint from a direct presentation attribute outside the cascade (what the prototype does) | Correct for the direct attributes exercised by the proving shell; honest and free | Inline style and stylesheet paint are dropped; presentation attributes do **not** participate in the shared cascade | In hand, deliberately narrow (E6) |
| 5 | **Official upstream Stylo** — use the first published release containing [servo/stylo#383](https://github.com/servo/stylo/pull/383); until then, use its exact tested merge revision | Native typed basic paint values without a private fork | A temporary exact revision pin until an eligible release exists; the remaining 24 Gecko-only longhands and production ingress are separate work | **Chosen** for paint dependency provenance (E7) |

A refinement of option 3: **registered** custom properties (`@property` with a
`syntax`, e.g. `<color>`) could recover typed computed values for the simple
color case — but not paint-server (`url(#…)`) or context-dependent paint
semantics, and servo `@property` support here is itself unverified. It narrows
the loss, it does not remove it, and it needs its own spike.

## Decision and scope

The owner took **D-L** on 2026-07-23:

> SVG paint enters the shared cascade through Servo-capable support maintained
> in official upstream Stylo. Prefer the first published release containing
> [servo/stylo#383](https://github.com/servo/stylo/pull/383); until one exists,
> use the tested immutable official-upstream revision
> [`a64923b5d5c67313c81c5056f5e30ec0babb04d6`](https://github.com/servo/stylo/commit/a64923b5d5c67313c81c5056f5e30ec0babb04d6).
> Do not use a floating upstream branch or a private source fork.

This decides **dependency provenance only**. It does not claim that the
production Web path admits SVG presentation attributes at presentation-hint
precedence, admits SVG-namespace stylesheets, has a conforming SVG/XML entry,
consumes the newly available computed values, supports the 24 remaining
Gecko-only SVG longhands, or passes an SVG-vector capability gate. E4–E5 remain
evidence that the missing ingress dimensions are technically closeable; E7
removes the need to own the basic-paint patch privately. The proving shell's
direct-attribute fallback remains deliberately narrow until those separate
capability steps are implemented and gated.
