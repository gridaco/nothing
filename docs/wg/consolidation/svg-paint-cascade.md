---
title: "Finding: SVG paint in the shared cascade"
description: "Why Servo Stylo omits most SVG longhands, what a bounded native paint and SVG/XML ingress spike proved, and the open owner decision it gates."
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

- **E1 — 44 of the 46 longhands in Stylo's SVG style structs are absent under
  the compiled engine.** Stylo splits its property database by engine (servo vs
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

## The options

| # | Option | What it buys | What it costs | Feasibility |
| --- | --- | --- | --- | --- |
| 1 | **Gecko-engine Stylo** | All 46 longhands in Stylo's enumerated SVG style structs, with native typed computed representations | A Gecko build environment | **Not viable** under the program's standalone Cargo/servo constraint (E3) |
| 2 | **Fork/patch Stylo** — un-gate the required SVG longhands for servo and complete the missing cascade intake | Native typed SVG paint in the shared cascade | Carrying a dependency patch; build-verifying each additional longhand's servo glue; productionizing the SVG/XML, SVG-stylesheet, and presentation-hint ingress proved feasible by E5 | Verified for native `fill`/`stroke` and the required ingress topology; remaining SVG-longhand breadth and upgrade cost are unmeasured (E4–E5) |
| 3 | **Custom-property carrier** — rewrite `fill`/`stroke`/… to `--*` at stylesheet + presentation-attribute intake, cascade those, read them back | Stylo custom-property inheritance and specificity mechanics for carrier tokens | Rewriting author CSS (shorthands, `all`, specificity); a no-op presentation-hint stub to implement; **loses** SVG paint computed-value semantics — the compiler must re-resolve `currentColor`, paint servers, types outside Stylo | Viable for mechanics (E2), lossy on semantics |
| 4 | **Status quo** — read paint from a direct presentation attribute outside the cascade (what the prototype does) | Correct for the direct attributes exercised by the proving shell; honest and free | Inline style and stylesheet paint are dropped; presentation attributes do **not** participate in the shared cascade | In hand, deliberately narrow (E6) |
| 5 | **Track upstream** — a future Stylo enabling SVG under servo | Eventually option 1's result with no fork | No schedule is established by this finding | Absent in 0.16; upstream schedule unknown (E1) |

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
  Among presently actionable options, it is the only path the bounded spike
  proves can yield native SVG-paint values through the shared Stylo cascade
  without Gecko. The isolated spikes demonstrate both document grammars feeding
  native `fill` and `stroke` into the same cascade implementation. They do not
  choose the fork, prove the remaining SVG longhands, measure upgrade ownership,
  or land the production SVG/XML entry.

## The decision to file

**D-L** is registered in the [charter's decision registry](./charter.md): *how
SVG paint enters the shared cascade* — Gecko vs a servo-capable Stylo fork vs a
scoped custom-property carrier vs direct-attribute status quo. Its evidence bar
is this finding plus a bounded feasibility bundle that covers the ingress
dimensions named by D-L, not merely a longhand build: the SVG/XML grammar
entry, SVG-namespace stylesheet intake, presentation-hint precedence, minimal
paint-longhand computation, and precedence, inheritance, `currentColor`, and
invalid-value behavior.

That bundle is now present in E4–E5, so D-L is ready for the owner to decide;
no option is ratified here. Until the owner decides it, the Web-first path reads
direct paint attributes outside the cascade, says so, and does not accrete
SVG-vector capability on that scaffold.
