---
title: HTML/CSS Font-Provider Seam
description: "Evidence for D-D: where host-owned font resources meet the shared HTML/CSS front end and the text oracle without leaking backend state into the pure core."
tags:
  - internal
  - wg
  - program
format: md
---

# HTML/CSS Font-Provider Seam

**Date:** 2026-07-21

**Status:** Evidence complete; registry decision **D-D remains open**
pending the owner's explicit GO.

**Genre:** program decision study. This document inventories the
current seams, states the constraints they prove, and supplies proposed
decision wording. It does not take D-D, select the text oracle, name an
API or crate, or prescribe an implementation. The domain contract is
the [universal shaped-text RFD](../feat-paragraph/text-layout.md), which
wins if this study conflicts with it.

## The decision

Phase 4 shares the HTML/CSS styled-tree front end and maps its output
onto the chassis. Text on both sides must continue through one
shaped-text artifact. The open question is therefore:

> At what boundary do authored font requirements cross to host-owned
> acquisition, and how do the closed results become explicit inputs to
> font-dependent CSS and shaped-text resolution?

D-D must answer that question without doing any of the following:

- importing the legacy runtime or a graphics backend into the pure
  model;
- treating an operating-system font lookup as declared engine state;
- moving authored CSS family choice into host policy;
- allowing measurement and paint to select or shape fonts separately;
  or
- choosing the production shaper, which is the later owner decision
  **D-H**.

The word “provider” in D-D names the host-facing acquisition problem.
It does not name one universal resolver. The text RFD's **resolution
environment** begins after authoring syntax and defaults are complete;
font-dependent CSS resolution necessarily precedes it. D-D must define
how those ordered stages receive the same declared resource facts
without folding their distinct policies together.

## Bedrock

These constraints are already settled outside D-D.

1. **A font request and a font environment are different inputs.** The
   requested family list, face attributes, features, spacing, language,
   and script belong to attributed source. Exact resources, aliases,
   fallback order, synthesis policy, and permitted missing-glyph
   behavior belong to the resolution environment.
2. **The environment is a manifest, not an ambient promise.** Exact
   font-content identity and face index are required. A family name,
   file name, URL, or operating-system handle alone is insufficient.
   Resolution also needs immutable access to every exact resource the
   manifest marks available; a manifest whose referenced bytes may
   change or disappear is not closed input.
3. **Text resolves once.** Measurement, surrounding layout, painting,
   bounds, damage, hit testing, editing geometry, and faithful export
   consume one immutable shaped-text artifact. None may choose a font,
   wrap, or shape again.
4. **Resource change means new resolution.** Adding or replacing a font,
   changing generic or fallback policy, or changing any other
   layout-affecting environmental fact produces a new environment
   identity and a new artifact. An old artifact is never patched.
5. **Incomplete resolution fails honestly.** A required missing,
   pending, invalid, or identity-mismatched resource, the absence of a
   permitted face or fallback, or unresolved glyph coverage under the
   declared policy produces a typed failure. An unused candidate need
   not fail resolution merely because it is unavailable. An interactive
   placeholder is host presentation, not a complete artifact.
6. **The chassis boundary stays backend-neutral.** `n0-model` is the
   skia-free document-and-resolution core. Backend-native font objects
   can exist behind a text-oracle adapter and in a private replay
   registry; they cannot be the shared contract.
7. **D-D and D-H are orthogonal.** D-D supplies explicit font facts to
   whichever oracle is selected. D-H decides the oracle's shaping and
   numeric policy at the Phase 5 boundary.
8. **Font-dependent CSS and shaped text are separate, ordered
   resolutions.** CSS metrics help complete the attributed source that
   the text oracle later consumes. Each stage has its own identified
   policy, while the declarations, exact resources, availability, and
   order shared by both stages must compare equal. D-D does not require
   one environment object or one selection algorithm.
9. **Identity is semantic before it is representational.** Two inputs
   compare as the same environment only when every layout-affecting
   manifest fact is the same. A generation counter, revision, hash, or
   label may carry that identity, but does not define equality by
   itself.

## Patrol inventory

The patrol covers the font-bearing paths that a Phase 4 extraction or
adoption would touch. “Observed” below means present in the current
tree; implications are drawn in the following section.

### Shared HTML/CSS front end

The shared [front end](../../../crates/grida/src/htmlcss/frontend.rs)
parses HTML and runs the Stylo cascade. The
[styled-tree collector](../../../crates/grida/src/htmlcss/collect.rs)
then extracts plain Rust records and creates no Skia objects. The
[HTML importer architecture test](../../../crates/grida/tests/html_import_architecture.rs)
guards the importer from reading Stylo directly; it does not yet prove
that the whole front end and styled-tree boundary are graphics-backend-
free.

The cascade itself currently constructs a Stylo device with a fixed
`SimpleFontProvider` in
[csscascade](../../../crates/csscascade/src/cascade.rs). That provider
does not see `FontRepository`: it fabricates ascent, x-height,
cap-height, zero advance, and ideographic width as fixed multiples of
the requested size, and reports the same generic base size. The later
layout and paint stages therefore do not merely use a different API;
they resolve text against a different source of font facts.

The [font style record](../../../crates/grida/src/htmlcss/style.rs)
carries size, weight, italic posture, a family list, line height,
spacing, direction, decoration, shadow, whitespace, and related CSS
state. Two losses are explicit in that record:

- every CSS generic family in the renderer-facing list is collapsed to
  `system-ui`; and
- only the first generic's original identity is retained for the HTML
  importer.

The current importer then projects only the first family into the
legacy scene model. These are current compatibility choices, not
browser-grade family semantics to preserve.

### Legacy HTML layout and paint

The public [HTML/CSS renderer](../../../crates/grida/src/htmlcss/mod.rs)
accepts a concrete `FontRepository`. Layout obtains its Skia
`FontCollection` and creates paragraphs inside Taffy's measure
callback. [Paint](../../../crates/grida/src/htmlcss/paint.rs) later
creates and lays out new paragraphs from the styled runs and the same
collection.

Using one collection reduces one source of drift, but it is not the
text RFD's one-artifact rule: measurement and paint still shape
independently, and no exact selected-font identity crosses between
them.

The [HTML/CSS WPT host](../../../crates/grida_wpt/src/render.rs)
constructs an empty repository and enables system fallback. The
renderer side is therefore driven by the machine's font manager rather
than a checked-in font environment. That is useful for local browser-
like preview but is not a portable conformance input.

Raw HTML also has no font-resource handshake corresponding to
`collect_image_urls`. The engine does not yet turn the complete
stylesheet set and document base URL into normalized `@font-face`
requirements for a host to satisfy. Legacy scene loading discovers only
scene text families; it does not inspect CSS inside HTML or Markdown
embeds. Requested/missing tracking in `FontRepository` is therefore
useful legacy host know-how, but it is not currently wired to the HTML
path and cannot substitute for authored `@font-face` semantics.

### Legacy font repository and paragraph path

The legacy [font repository](../../../crates/grida/src/runtime/font_repository.rs)
contains several years of useful host behavior:

- explicit byte registration with more than one face per family;
- separate embedded, user, fallback, and optional system-font sources;
- caller-provided family aliases and ordered fallback families;
- requested, available, and missing-family tracking; and
- a generation counter used to invalidate Skia caches and to reject
  stale paragraph-cache entries after resolution-state mutation.

It also contains boundaries that cannot cross into the chassis: a
legacy `ByteStore`, Skia `TypefaceFontProvider` and `FontCollection`
objects, mutable request bookkeeping, and a process-local generation
number rather than a durable manifest of exact face contents.

Its edge behavior is not uniformly contractual. Registration records
the requested family and advances the generation even when bytes are
absent or decoding fails; embedded faces are not added to the family
inventory used by missing checks; requested names are exact strings;
and the fallback helper removes the primary family but does not remove
duplicates within the fallback list. A migration must preserve the
capabilities without silently canonizing these failure modes.

The legacy scene-text path is further along than the HTML path. Its
[paragraph cache](../../../crates/grida/src/cache/paragraph.rs) lazily
rejects the artifact when font generation or width changes, while the
runtime's font-loaded event separately rebuilds affected scene caches.
Its
[resolved-text record](../../../crates/grida/src/text/resolved.rs)
stamps the SkParagraph oracle version and process-local environment.
That artifact and the invalidation law are know-how to adopt through
the text contract. The repository representation itself is not.

### SVG host context

The SVG renderer's
[host context](../../../crates/grida/src/htmlcss/svg/context.rs) proves
that a thin host can supply font behavior. Its `PreloadedFonts` accepts
bytes, family aliases, generic bindings, and a default family; the
SVG reftest host constructs a curated set and deliberately avoids
system fallback in
[the suite adapter](../../../crates/grida_dev/src/reftest/render.rs).

The same context also shows why that trait is not the shared answer:
it returns a Skia `Typeface`, exposes fallback as an unrecorded lookup,
has no environment identity or availability state, and defaults the
general renderer to ambient system fonts. SVG text then selects and
shapes inside its painter. Those are SVG implementation facts, not a
backend-neutral shaped-text contract.

### Chassis text seam

The chassis already owns the correct structural seam.
[n0-model's text contract](../../../crates/n0-model/src/text_layout.rs)
defines a backend-neutral `TextLayoutOracle` and an immutable artifact
with oracle and environment labels, line geometry, exact-font run
identity, replay keys, positioned glyphs, bounds, and unresolved-glyph
count. Resolution stores one final-width artifact per text node.

The current [SkParagraph adapter](../../../crates/n0/src/text_layout.rs)
is deliberately a proving bridge:

- it receives one already-loaded host typeface;
- registers it under one internal family;
- disables fallback;
- labels both the environment and selected fonts with process-local
  Skia identities; and
- retains the exact backend fonts in a private drawlist registry so
  paint replays glyphs without reshaping.

The host [paint context](../../../crates/n0/src/paint.rs) separately
owns an opaque context-plus-revision identity spanning fonts and
images. A [frame product](../../../crates/n0/src/frame.rs) rejects
execution under a different revision. This is a sound whole-frame
integrity check, but it is not the RFD's portable font-manifest
identity.

Current gaps are stated honestly in the [n0 README](../../../crates/n0/README.md):
the font environment is process-local, fallback is disabled, paragraph
coverage is narrow, and unresolved glyphs produce a report while the
infallible oracle interface still returns the diagnostic artifact. A
font-backed replay also fails closed because the replay format carries
no reconstructible manifest.

The chassis authored text record currently exposes size, weight, and
italic posture but no requested family list. That is sufficient for
the proving bridge and Draft 0 n0 XML defaults; it is insufficient to
preserve authored CSS `font-family` through the Phase 4 adapter.

### Font foundation crate

The [fonts crate](../../../crates/fonts/src/lib.rs) is a backend-neutral
foundation for OpenType parsing, metadata, feature discovery, and face
selection. It neither owns shaping nor supplies the current engines'
runtime registry. This is the topology's intended separation between
font introspection and the versioned text oracle. Its future role is
evidence for D-H, not permission to make D-D choose an oracle early.

## Findings

### F1 — authored request must survive independently

A provider cannot repair an authored request that the styled-tree or
model adapter already discarded. Phase 4 must preserve the ordered CSS
family list—including each generic token's identity—and every supported
face-selection input in styled source. Before universal text resolution,
the HTML adapter applies the identified CSS generic-binding policy and
expands each generic token into ordered declared family or face
candidates. The token remains HTML-side source provenance and cache
input; the universal text contract sees concrete candidates and does
not reinterpret CSS syntax.

The existing collapse of all generics to `system-ui` is therefore a
named gap, not captured essence.

### F2 — authored declarations and host acquisition form a handshake

An HTML engine, not a host loader, owns interpretation of the complete
stylesheet set. That includes `@font-face` descriptor normalization,
source order, URL resolution against the document base, and the
production of resource requirements. A host owns transport, security,
licensing, persistence, byte acquisition, and reporting acquisition
state: bytes obtained, missing, pending, or transport failure, together
with any expected content identity.

The engine-owned boundary validates acquired content identity, font
structure, collection face index, and declaration consistency. Only
after that validation may the closed input classify a candidate as
available, invalid, or identity-mismatched. Each selected CSS or text
resolver separately validates policy and oracle support before claiming
a complete result. This distinction avoids blessing the legacy behavior
that can record a registration as available even when bytes are absent
or decoding fails.

This split prevents two opposite leaks. The host never parses CSS or
chooses a face in response to an authored family token, and the engine
never performs hidden network or platform I/O while resolving style or
text. Late arrival satisfies a requirement only for newly identified
inputs.

### F3 — cascade and shaping are separate, ordered resolutions

Today Stylo answers font-relative CSS metric queries from a fabricated
provider while SkParagraph later uses the host repository. This can
change computed lengths, layout, wrapping, and paint even if both paths
are internally deterministic.

The correct invariant is compatibility, not one shared resolver. CSS
font resolution computes font-dependent style under an explicitly
identified CSS-metrics policy over declared exact resources. The HTML
adapter then projects complete styled source into the language-neutral
inputs of the shaped-text RFD. The text oracle consumes its own complete
resolution environment and independently identified oracle policy.
Neither stage may consult unrelated ambient state, and a styled result
and shaped artifact with unequal font-resource identities cannot form
one render product.

### F4 — resolution receives closed inputs, not a mutable repository

For one resolution, every resource admitted as a candidate by the
declared family, generic, fallback, source-precedence, and replacement
policies has an explicit availability state. Resources not admitted by
those policies are outside the closed input. Every candidate marked
available has both an exact content identity and face index and
immutable access to that exact resource. Access may borrow or share
storage; it need not copy bytes.

The access path performs no discovery, network loading, ambient
fallback, or observation of later mutation. A URL, family name, mutable
manager, or process-local handle alone is not sufficient. Late arrival,
replacement, a changed declaration, or a changed layout-affecting policy
creates a newly identified input set before either resolution runs.

### F5 — a shared font-resource identity makes compatibility executable

For this candidate, the **font-resource identity** is the semantic
identity of the facts shared by the two ordered resolutions: authored
face declarations; exact content and face indexes; declared family
metadata and aliases; generic bindings and their expanded candidate
order; source-tier and candidate order; and availability. CSS-metrics
input and the text resolution environment are compatible exactly when
this identity is equal. Neither stage may rebind or reacquire a resource
between them.

Their complete identities are intentionally not equal. CSS policy and
metric behavior remain CSS-stage inputs. Text selection, fallback,
synthesis, replacement, language, scale, safety, and oracle version
remain resolution-environment or oracle inputs. Each stage accounts for
its own complete layout-affecting facts in addition to the shared
font-resource identity.

Two independently constructed manifests with the same semantic facts
therefore have the same font-resource identity. An opaque generation,
revision, label, or digest is useful provenance and may encode that
equality; it cannot make unequal manifests equal or equal manifests
unequal. A late resource arrival invalidates font-derived CSS results as
well as shaped text: cascade is recomputed first, then text is resolved
again, and mixed font-resource identities are refused.

### F6 — semantic identities and replay handles stay separate

These identities serve different constraints and must not be folded
together:

| Identity | What it answers | Lifetime |
|---|---|---|
| CSS font-policy version | Which declaration matching, generic expansion, and font-metric policy produced font-dependent style? | Versioned CSS policy |
| Font-resource identity | Which declarations, exact resources, availability, bindings, and order were shared by CSS and text? | Immutable shared projection |
| Text-oracle version | Which shaping, breaking, metrics, and numeric policy produced geometry? | Versioned policy |
| Resolution-environment identity | Which font-resource identity and text-only external policies were inputs? | Immutable semantic inputs |
| Resolved face identity | Which exact face and effective shaping state produced a glyph run? | Shaped artifact |
| Backend replay binding, when used | Where can this backend replay the already-selected face? | Private artifact/drawlist lifetime |

The broader paint-environment revision may continue to guard a whole
frame. It must not substitute for the narrower semantic identities
recorded by text.

### F7 — neither existing provider is a contract to promote

`FontRepository` carries the richest operational know-how, but it also
carries the legacy runtime, Skia, mutable caches, and incomplete
identity. The SVG `FontResolver` is already host-facing and easy to
curate, but it returns backend objects and makes fallback invisible.

Both are valid adapters on their present sides. Promoting either would
make the second consumer inherit the first consumer's backend and
failure semantics. The shared contract must be shaped by both HTML and
n0 instead.

### F8 — the shaped-text artifact is the only downstream font seam

After the oracle returns, layout and paint do not need a provider.
They need the one artifact. The exact backend font can accompany that
artifact behind an opaque replay key, as the chassis already proves.

Consequently, Phase 4 cannot preserve the HTML renderer's current
measure-then-reshape structure. Legacy byte equality is an extraction
gate, not permission to retain split text semantics in the end state.

### F9 — D-D does not choose D-H

Explicit resource and policy inputs work with the current SkParagraph
adapter, a future fonts-backed oracle, or another ratified oracle. D-D
does not choose the text oracle's face-matching algorithm, fallback
granularity, metric interpretation, shaping implementation, or native-
object construction topology. Those remain part of D-H and the selected
oracle version. CSS declaration and source matching, generic expansion,
and metric-query semantics remain the separately identified CSS policy
and are graded against Chromium in the HTML lane.

Backend-native objects may be constructed privately by host-acquisition,
CSS-metrics, oracle, or replay adapters. No such object crosses the
universal contract. A backend may retain a private replay binding, but
D-D does not require that topology. Exact selected-face identity remains
with the artifact, and no downstream consumer reselects or reconstructs
a face from authored style.

### F10 — extraction and capability need different proof

Making the current front end independently hostable is a zero-behavior
move and must preserve the declared 139-fixture output under one fixed
host configuration. Replacing fabricated metrics, preserving generic
families, loading authored web fonts, or changing fallback is a
capability move. Those changes require controlled Chromium conformance
fixtures under the scoreboard discipline; legacy output is evidence of
what was captured, not the oracle for new behavior.

## Candidate decision

Only one of the locally evidenced shapes satisfies every bedrock
constraint.

| Shape | Useful property | Blocking defect | Disposition for D-D |
|---|---|---|---|
| Pass legacy `FontRepository` into the shared path | Multi-face registration, missing tracking, fallback, invalidation | Legacy runtime and Skia cross the boundary; identity is process-local and mutable | Not eligible |
| Promote SVG `FontResolver` | Small host-facing resolver; curated and system implementations exist | Returns Skia typefaces; fallback and resource identity are implicit; text shapes in paint | Not eligible |
| Let the core query a live host provider | Host can load lazily and hide storage details | Resolution can observe mutable state or I/O; cache identity and typed completeness cannot be proven | Not eligible |
| Give cascade and shaping one shared resolution environment | One object appears to prevent drift | Text resolution starts after cascade; the stages own distinct policies and inputs | Not eligible |
| Engine issues authored requirements; host supplies acquisition results; engine validates closed exact resources for separate CSS and text resolutions | Preserves authored semantics, host-owned acquisition, deterministic closure, and D-H independence | Requires a real acquisition handshake, semantic identities, CSS-metrics policy, immutable resource access, and typed failure work not present today | **Candidate for owner GO** |

Proposed wording, if the owner takes D-D:

> **D-D — declared, closed font inputs.** The engine interprets authored
> font declarations and issues resource requirements. Hosts own external
> acquisition and return exact resource access or acquisition state; the
> engine validates those results and closes exact declared font resources.
> HTML style resolution and shaped-text resolution are separate, ordered
> consumers under separately identified policies. Neither consults
> ambient font state, and both carry the same font-resource identity.
> The HTML adapter preserves authored family intent, applies its identified
> CSS generic-binding policy, and projects ordered concrete candidates
> into language-neutral shaped-text inputs without exporting CSS syntax.
> The text oracle alone consumes the complete resolution environment
> defined by the shaped-text RFD, performs authoritative selection and
> shaping, and records exact environment and selected-face identities in
> one immutable artifact.
> Every admitted resource marked available provides immutable access to
> its exact content for that resolution; late arrival or replacement
> creates a newly identified input set. Backend-native types remain
> private, and downstream consumers replay the artifact without font
> selection or reshaping. D-H remains the independent choice of oracle
> policy and implementation.

This wording deliberately decides a semantic boundary, not a Rust
trait, field layout, crate name, or storage scheme.

## Minimum contract if accepted

The Phase 4 seam is an ordered handshake, not a provider callback that
selects fonts on demand.

### Authored requirements

The engine consumes the complete stylesheet set, each stylesheet's base
URL, and the document base URL. For each supported authored face it
preserves the declared family, weight, width and posture ranges, source
order, source kind and resolved resource key, plus any supported coverage
or loading-policy descriptor.
It also preserves named and generic family requests in the styled
source. CSS syntax stops at the HTML adapter: the universal text
contract neither parses nor enumerates CSS generic families.

The host receives resource requirements, not CSS family-selection
queries. It may satisfy them from embedded bytes, application assets,
network responses, or explicitly admitted system resources, but it
does not choose authored precedence, matching, fallback, or synthesis.
It reports exact acquired resource access, missing or pending
acquisition, transport failure, and any expected content identity; it
does not declare font structure semantically valid.

### Closed declared resources

Before either font-dependent resolution begins, every resource admitted
as a candidate by the declared source, family, generic, fallback, and
replacement policies has an explicit validated availability state. The
engine-owned boundary checks content identity, font structure,
collection face index, and declaration consistency before closing the
input. Every available candidate then supplies immutable access to exact
content and its collection face index. Access deterministically returns
that resource or a typed identity failure; it performs no discovery,
I/O, ambient fallback, or observation of later mutation during
resolution. Each semantic consumer separately rejects unsupported
policy or oracle capability before claiming a complete result.

The font-resource identity accounts for the shared facts: exact
resources, declarations and aliases, generic bindings and their expanded
candidates, source-tier and candidate order, and validated availability.
Independently constructed equal manifests have the same semantic
identity; a local generation or revision is only a provenance label.
The representation, hashing scheme, ownership model, and whether
immutable storage is shared or borrowed are not decided.

### Ordered semantic consumers

CSS font resolution uses an explicitly identified CSS-metrics and
selection policy over the closed declared resources. Every resulting
font-derived computed value is attributable to the exact selected face
and the font-resource identity, and the styled result retains that
identity.

The HTML adapter retains each authored generic token as HTML-side
provenance, expands it under the CSS generic-binding policy into ordered
declared candidates, and projects those candidates into complete
language-neutral attributed source or reports unsupported semantics.
The text oracle receives the resolution environment defined by the
shaped-text RFD, carrying the same font-resource identity. That
environment owns text-specific candidate interpretation, fallback and
synthesis, permitted missing-glyph or replacement behavior, exact
replacement faces, language-service versions, coordinate policy, and
safety limits. D-D requires explicit inputs and coherent invalidation;
D-H remains free to choose how the text oracle interprets them.

The boundary must refuse:

- a family, URL, or operating-system handle presented as exact font
  identity;
- an undeclared system fallback;
- host-side selection in response to an authored CSS family token;
- mutable resource state or I/O observed during one resolution;
- a complete artifact while a required resource remains pending;
- a styled result combined with a text environment derived from
  a different font-resource identity;
- a glyph run without exact face identity; and
- a painter attempting to reinterpret glyph IDs through another font
  registry.

System fonts are not forbidden. A preview host may acquire them before
resolution, admit their exact identities and order into the closed
resources, and accept that a different machine supplies different
input. What is forbidden is ambient fallback presented as the same
input.

## Captured essence

The following behavior must be re-homed before any provider path is
replaced or deleted:

| Existing essence | Provenance | Re-home destination |
|---|---|---|
| Multiple faces can share one declared family | Legacy `FontRepository` registration | Resolution-environment conformance fixture for face selection |
| Embedded, user, explicit fallback, and optional system sources remain distinguishable | Legacy repository source tiers | Host acquisition provenance and font-resource identity tests |
| Requested, available, missing, and late-arriving fonts are observable | Legacy runtime and wasm font APIs | Typed acquisition state; typed resolution failure; re-resolution from newly identified inputs |
| Font and fallback changes reject stale geometry and rebuild affected runtime caches | Legacy lazy generation check, font-loaded event, and chassis context revision | Font-resource identity replacement law covering cascade and text caches |
| Cascade-time font-relative metrics and shaped text must not use unrelated providers | Current Stylo metric stub versus later `FontRepository` | Separate policy fixtures carrying the same font-resource identity |
| Curated generic bindings and aliases make a corpus independent of system fonts | SVG `PreloadedFonts` and reftest adapter | Shared declared-resource fixtures for web conformance |
| One resolved glyph layout and its exact selected faces survive into paint | Chassis text oracle and drawlist registry | Shaped-text artifact consumption test; private registry integrity when a backend uses one |
| Oracle identity and environment identity are separate | Both engines' resolved-text records | Resolution identity and cache-key tests |

The patrol also names capability gaps that are not legacy essence:
authored web-font requirements, explicit collision precedence across
source tiers, and typed invalid or identity-mismatch validation.

These implementation details are deliberate non-adoptions:

- legacy `ByteStore` and Skia provider objects as public contract;
- a process-local generation or typeface ID as portable identity;
- default ambient system-font lookup;
- the fixed Stylo `SimpleFontProvider` as production font metrics;
- SVG's paint-time selection and shaping;
- HTML's generic-family collapse and layout/paint reshaping split;
- the absent authored `@font-face` requirement handshake and implicit
  source-tier collision behavior;
- registration success that cannot distinguish missing bytes or decode
  failure, and an inventory that omits embedded faces;
- exact-string missing tracking and duplicate fallback candidates as
  intended selection semantics;
- the current single-typeface chassis restriction; and
- returning a partial diagnostic artifact where the RFD requires a
  typed resolution failure.

No source, fixture, baseline, or runtime path is deleted by this study.

## Gates after owner GO

D-D becomes executable only when the following tests can be written in
producer-neutral terms.

### Requirement and closure gate

- A complete stylesheet set with its base URLs produces normalized
  authored face declarations and ordered resource requirements,
  including source order and supported face descriptors.
- Acquisition distinguishes exact resource access, missing, pending, and
  transport failure. Engine resource validation separately distinguishes
  invalid font structure, face index, declaration inconsistency, and
  content-identity mismatch. CSS and text resolution separately reject
  unsupported policy or oracle capability. Successful resource
  validation creates newly identified closed input; it does not mutate
  an active resolution.
- Every available candidate deterministically yields the exact declared
  content and face index. The resolution path performs no acquisition,
  network I/O, or ambient fallback.
- A required unavailable resource or exhausted permitted fallback fails
  with a typed diagnostic. An unavailable candidate that the declared
  policy does not require is not made fatal by the boundary itself.
- Authored web, embedded, application, fallback, and admitted system
  sources have explicit identity-bearing precedence. The exact selected
  face is observable in the result.

### Identity and invalidation gate

- Two adapters independently constructing the same shared manifest
  produce the same font-resource identity. Changing one exact resource,
  face index, declaration, alias, generic binding or expansion,
  candidate order, availability state, or source precedence produces a
  different font-resource identity.
- CSS and text are compatible exactly when that font-resource identity
  is equal. Their CSS-policy, resolution-environment, and oracle
  identities remain separate and need not compare equal.
- A local generation, revision, label, or digest cannot make unequal
  manifests equal or equal manifests unequal.
- Late arrival of a face used by font-relative CSS recomputes cascade
  before text resolution. A fixture using `1ex` or `1ch` changes both
  the attributable computed value and shaped artifact as required.
- A styled result and shaped-text environment with different
  font-resource identities cannot form one frame product.
- A system-font-backed preview records exact resources and cannot
  compare equal to a curated or wasm environment by family name alone.

### Architecture gate

- The shared HTML/CSS front end and styled-tree contract contain no
  graphics-backend types. They perform no acquisition or I/O, and no
  font lookup occurs outside the supplied immutable resources.
- CSS metric queries are pure over those resources, use an explicitly
  identified CSS policy, and record exact-face provenance for
  font-derived computed values.
- The chassis model and resolution-environment contract remain
  graphics-backend-free.
- Backend-native font objects are confined to host-acquisition,
  CSS-metrics, oracle/backend, and private replay adapters. D-H decides
  their construction topology; no native object crosses a universal
  contract.
- A glyph-bearing drawlist retains sufficient private backend state to
  replay the artifact's selected faces without re-selection. When replay
  keys are used, they are valid only with their originating registry.
- No layout or paint path shapes the same text independently of the
  artifact.

### Zero-behavior extraction gate

- Extracting the current styled-tree front end preserves byte-identical
  output across the declared 139-fixture golden corpus under one fixed,
  recorded host configuration.
- The gate does not replace fabricated CSS metrics, generic collapse,
  fallback, or split HTML shaping; those are named gaps, not hidden
  extraction changes.

### Capability and conformance gate

- The styled tree preserves supported CSS family requests without the
  current generic collapse.
- The HTML adapter deterministically expands each preserved generic token
  into ordered declared candidates under its identified CSS policy; the
  universal text contract receives the candidates, not the CSS token.
- Checked-in `@font-face` fixtures cover a loaded named face, weight and
  posture matching, source and cross-script fallback, and font-relative
  metrics. The same controlled resources are supplied to Chromium and
  the engine so a difference is renderer behavior, not machine state.
- Generic-family preservation and deterministic binding have structural
  tests. Chromium becomes the generic-family oracle only when the
  harness controls or has audited its platform generic mapping.
- Missing and pending resources, late arrival, source-tier collisions,
  and identity replacement have producer-neutral state-machine tests.
  The current text-hidden exact subset is not treated as evidence for
  visible font behavior.
- The chassis `from_styled` adapter is a capability grant and therefore
  enters the scoreboard only after the Phase 2 flip rule is ratified;
  unsupported typography remains explicit coverage, never a fallback
  shim.
- Native preview, wasm, CLI export, and headless hosts honor the same
  requirement, closed-resource, identity, and artifact contracts even
  when their acquisition and backend adapters differ.

The study itself does not authorize an extraction, adapter, score run,
or baseline change.

## Out of scope

- Selecting or implementing the production text oracle (D-H).
- Naming a crate, trait, or public Rust type before two consumers have
  shaped it.
- Defining external resource discovery, network transport, licensing,
  packaging, or cache storage. Interpreting authored declarations and
  issuing requirements is part of the seam.
- Adding `font-family` to the n0 XML Draft 0 grammar.
- Migrating SVG text onto the universal shaped-text artifact.
- Expanding CSS Fonts coverage beyond what the HTML lane's conformance
  corpus justifies.
