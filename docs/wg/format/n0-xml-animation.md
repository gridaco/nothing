---
title: "n0 XML animation"
description: "Decision RFD deferring native .n0.xml animation syntax while SVG proves the shared animation contract."
keywords:
  - grida xml
  - animation
  - declarative animation
  - svg animation
tags:
  - internal
  - wg
  - format-schema
  - canvas
  - authoring
  - rendering
  - svg
format: md
---

# n0 XML animation

**Status:** Decided — native syntax is deferred and unreserved.

`.n0.xml` remains a static authored scene language. Its strict reader must
reject animation elements and animation-only attributes rather than infer a
timing model. No native animation tag, attribute, namespace, or structural
position is reserved by this decision.

Actual SVG source is the first authored animation frontend. Its cumulative
contracts run from the [SVG Animation Profile
0](../feat-svg/animation.md) baseline through the current cumulative [Profile
6](../feat-svg/animation-path-geometry.md). SVG source is not n0 XML, and this
decision does not make `<animate>` or any other SVG animation element valid
inside `.n0.xml`.

## Decision

1. Actual SVG animation markup is the first admitted authored input.
2. SVG compiles to a source-neutral, typed animation program before sampling.
3. Sampling produces the existing immutable effective-property values; it does
   not mutate authored source or introduce a second value channel.
4. Native n0 XML animation grammar will be reconsidered only after SVG and
   at least one materially different producer have exercised that program.
5. Until then, the n0 XML grammar remains entirely static and reserves
   nothing on speculation.
6. An external runtime may continue supplying effective values to a n0 XML
   scene. That is runtime control, not native animation source semantics.

The source-neutral boundary begins after a frontend has resolved its own
targets, timing vocabulary, value syntax, and diagnostics. Authored SVG remains
SVG-shaped; a future native language need not retain SVG's spelling or its full
timing model.

## Why defer the grammar?

Animation syntax commits the language to more than values changing over time:

- the distinction between authored base and sampled effective values;
- timeline ownership and exact boundary behavior;
- durable property targets through copies and component occurrences;
- typed interpolation and the animatable-property registry;
- effect ordering and composition on one property;
- layout, paint, query, damage, and resource consequences; and
- static fallback, seeking, export, and trust policy.

The [durable-addressing RFD](./n0-xml-addressing.md) already supplies stable
owner/member identities, typed property targets, and the immutable sparse
effective-value boundary. It deliberately does not decide timing,
interpolation, or composition.

Choosing XML spelling before the animation program survives two frontends
would make frontend-specific assumptions look universal. It could also create
two canonical representations: an early native grammar and the later model it
failed to express. Leaving the namespace unreserved is the more compatible
choice; a future proposal must justify every new construct when its semantics
are known.

SVG is a useful first producer because it brings established concepts—base and
animated values, explicit timing, interpolation, repeat, post-interval fill,
and multiple effects—without requiring a new Grida syntax. Profile 0 narrows
that vocabulary to a tractable linear slice. Profiles 1–6 then prove that
keyframes, easing, ordered composition, live underlying values, typed
transforms, solid paints, typed path geometry, and discrete complete-value
selection remain source-neutral without making the rest of SMIL part of the
shared contract.

## Current static behavior

With no externally supplied effective values, a `.n0.xml` document renders
its authored scene. There is no implicit time zero, autoplay, poster frame, or
hidden timeline.

Examples such as these remain invalid:

```xml
<rect x="0" width="80" height="80">
  <animate property="x" from="0" to="240" dur="600ms"/>
</rect>
```

```xml
<animations>
  <animation target="card" property="x" duration="600ms"/>
</animations>
```

A processor must not preserve them as unknown inert children and later claim
they have defined n0 XML meaning. Rejecting them keeps source authoring and
runtime control distinguishable.

## Requirements for a future native proposal

A future `.n0.xml` animation RFD must arrive with a second-frontend
comparison and answer, normatively:

- which timelines exist and how exact sample time is represented;
- which typed properties are animatable and how each interpolates;
- how effects target nodes, component occurrences, and nested value members;
- how several effects on one property compose;
- how source values, sampled values, and editor inspection remain distinct;
- which changes rerun layout, repaint, alter queries, or touch resources;
- what static consumers and exporters produce without a sample time;
- which features may react to events, initiate I/O, or cross a trust boundary;
- how unsupported constructs are diagnosed and preserved; and
- which conformance corpus lets another implementation reproduce the same
  sampled scene.

It must also show that a concise common case remains easy for people and
language models to author without creating a shorthand that competes with a
second canonical form.

## Related contracts

- [SVG Animation Profile 0](../feat-svg/animation.md) — accepted first authored
  frontend baseline.
- [SVG Animation Profile 6](../feat-svg/animation-path-geometry.md) — current
  cumulative source contract.
- [n0 XML durable addressing](./n0-xml-addressing.md) — source identities,
  typed property targets, and effective values.
- [n0 XML](./n0-xml.md) — static authored-language contract.
- [Chromium SVG animation research](../research/chromium/svg/animation-and-smil.md)
  — browser implementation precedent, not Grida semantics.
