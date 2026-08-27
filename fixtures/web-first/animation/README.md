# Web-first SVG sampling oracle

This directory isolates one behavior: **explicit-time sampling**. Each fixture
is a pair — an animated document and the static Base projection of the same
authored scene — captured from Chromium at a paused timeline, so the engine's
Base view and its exact-nanosecond samples are both gated against browser
pixels.

The admitted animation slice is deliberately one target and value vocabulary:
a single `<animate>` on a top-level `<rect>`'s `x`, with
`from`/`to`/`dur`/`fill="freeze"`. Everything else in a fixture is ordinary
static vocabulary. The rectangle may use the admitted repeating-pattern paint
and pattern/filter composition profile; the pattern source and tile geometry
remain static.

| Fixture | What it is |
| --- | --- |
| `svg-rect-x-animation` | The minimal case: one black rect on a white backdrop. Authored `x` is `4`; the animation moves it from `20` to `44` over two seconds and freezes there. |
| `svg-scene-cub` | The same slice over a **whole composition** — see below. Authored `x` is `12`; the block slides from `6` to `38` over two seconds and freezes. |
| `svg-pattern-client-animation` | The same animated client carrying a templated repeating pattern, a color-matrix filter inside the tile, and another color-matrix filter around the target. Its quarter-second sample proves a non-endpoint exact time as the rect moves from `8` to `24`. |

- `<id>-base.svg` is the static Base projection: the same authored scene with
  the animation element removed.
- `<id>-animation.svg` is the animated input.
- `cases.json` closes the admitted sample set per fixture, records the shuffled
  retained seek order, and declares the resolved frame each fixture must
  compile to (`frame.node_count`, `frame.animated_node_index` — read by
  `crates/websem/tests/svg_animation_x.rs`, not by the baker).
- `chromium/<id>/*.png` are independent Chromium oracle pixels.
- `bake_chromium.ts` verifies the DOM animation value and bounding box before
  each capture. It double-captures every case on fresh pages, then seeks a
  retained document in shuffled order and requires exact decoded RGBA equality.
- The baker and primitive-cell harness import the same hash-pinned
  `../chromium_capture.ts`; the animation manifest records that module's hash
  as well as the baker and suite hashes.
- `oracle-bake.json` records the Chromium version, environment, inputs, outputs,
  hashes, and capture policy.

The three fixtures contain 16 committed oracle frames: three Base projections
and thirteen exact-time samples. They do not change the separately counted
primitive-cell corpus.

Chromium does not expose the engine's Base policy for an animated document.
The Base oracle is therefore deliberately a static authoring projection, not
an observation at time zero. `Sample(0ns)` is separate, and each fixture's
authored value differs from its first sample so the two can never be confused —
the baker refuses a suite where they coincide.

Every fixture is fully opaque, integer-aligned, font-free, and network-free.
There is no similarity threshold and this harness produces no score or report.

## `svg-scene-cub` — the tiger, simplified and ours

The Ghostscript tiger is the de-facto SVG smoke test, but it is AGPL and so
lives untracked in `fixtures/local/` (see the
[engine-of-record register](../../../docs/wg/consolidation/svg-engine-of-record.md)). The cub
is an original drawing built to cover the same *features* in a fraction of the
markup — 17 materialized nodes instead of 240 — so a committed fixture can
exercise the whole admitted slice at once, statically and under sampling:

- a **viewBox-only root** (`viewBox="0 0 48 48"`, no `width`/`height`), so the
  scene is sized by the initial viewport and baked at 96x96 — a uniform 2x
  viewport mapping, the tiger's own shape;
- a **container** carrying a `translate`, with a nested `<g>` inside it, and
  `fill` / `stroke` / `stroke-width` / `stroke-linejoin` / `stroke-linecap`
  **inherited** through both — again the tiger's exact pattern;
- **path curves**: cubics, a mix of absolute and *relative* commands (`c`, `m`),
  an `h` shorthand, and a `Q` quadratic;
- **multi-subpath paths**: the mouth, the forehead stripes and the cheek
  stripes are one `<path>` each, with two, three and four subpaths;
- **strokes** on open contours with round caps and joins, a per-element
  `stroke-width` override, `fill="none"` and `stroke="none"`;
- a `<line>` (the floor), and `<rect>`s inside a group (the pupils).

Two absences are not oversights but consequences of the admitted slice, and the
fixture would stop sampling under `--strict` if it grew either:

- the animated element must be a **top-level `<rect>`** — the inventory admits
  an `<animate>` only on a materialized direct child of the root — so the
  sliding block sits beside the figure rather than inside its group;
- there is **no `<style>` element, no `style=` attribute, and no `color=`**. The
  first two are dynamic-inventory blockers (a CSS animation surface the slice
  does not own). `color` is a presentation attribute Chromium honors and this
  engine does not consume yet, so it is *declared* rather than painted —
  `currentColor` waits for its own rung, and until then a sampled scene cannot
  reach it by either door.

The drawing is deliberately curve-heavy *without* a `<circle>` or `<ellipse>`:
its near-elliptical muzzle, eyes and ear tips are cubic approximations. Filled
and stroked cubics bake byte-exact against Chromium while a true rational conic
does not (see the tolerance note in [`../README.md`](../README.md)) — so the
whole scene gates at zero differing pixels, in every frame.

Run from the repository root:

```sh
pnpm -C packages/grida-reftest exec tsx \
  "$(pwd)/fixtures/web-first/animation/bake_chromium.ts"
```

See it, statically and at an exact time:

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub.png 96x96 --strict
```

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub-1s.png 96x96 \
  --strict --time-ns 1000000000
```
