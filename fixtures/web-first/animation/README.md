# Web-first SVG animation oracle

This directory isolates one behavior: explicit-time sampling of an SVG
`<animate>` element that changes a rectangle's `x` value.

- `svg-rect-x-animation.svg` is the animated input. Its authored `x` is `4`;
  the animation moves it from `20` to `44` over two seconds and freezes there.
- `svg-rect-x-base.svg` is the static Base projection: the same authored scene
  with the animation element removed.
- `cases.json` closes the admitted sample set and records the shuffled retained
  seek order.
- `chromium/*.png` are independent Chromium oracle pixels.
- `bake_chromium.ts` verifies the DOM animation value and bounding box before
  each capture. It double-captures every case on fresh pages, then seeks a
  retained document in shuffled order and requires exact decoded RGBA equality.
- `oracle-bake.json` records the Chromium version, environment, inputs, outputs,
  hashes, and capture policy.

Chromium does not expose the engine's Base policy for an animated document.
The Base oracle is therefore deliberately a static authoring projection, not
an observation at time zero. `Sample(0ns)` is separate and resolves to `x=20`.

The fixture is fully opaque, integer-aligned, font-free, and network-free.
There is no similarity threshold and this harness produces no score or report.

Run from the repository root:

```sh
pnpm -C packages/grida-reftest exec tsx \
  "$(pwd)/fixtures/web-first/animation/bake_chromium.ts"
```
