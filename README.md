<p align="center">
  <img src="./assets/logo.svg" alt="Nothing" width="474">
</p>

# n0

**Nothing but drawing.** A 2D graphics engine that speaks the Web — browser-grade
rendering, without a browser.

`n0` (pronounced "nothing") turns SVG and HTML/CSS into exact pixels: from a
command line, from a library, and eventually onto an editable realtime canvas.
No scripting, no navigation, no network, no user agent.

## The claim

n0 is the renderer half of a browser — embeddable, deterministic, and
eventually editable. There is no agent half:

| n0 refuses             | which buys                                                         |
| ---------------------- | ------------------------------------------------------------------ |
| scripting              | determinism — same input, same declared time, same bytes           |
| navigation & network   | one binary, no sandbox, no headless flags, no 200MB download       |
| the user-agent surface | a small attack surface and a core you can embed                    |
| an ambient clock       | frame-exact animation — dropped frames are structurally impossible |

## What it's for

- **Deterministic Web rendering.** SVG and HTML/CSS to exact pixels — in CI, in
  a container, on a machine that will never have a browser. Same input, same
  declared time, same bytes.
- **Animation as exact time.** A source compiles once and is sampled at a
  declared signed-nanosecond instant. An exported sequence has no frame to drop.
- **Realtime on heavy documents.** 60+ fps where an entire page is one frame
  inside an infinite canvas — panning, zooming, editing.
- **Layout we own.** Browser-grade layout is where general-purpose layout
  libraries stop. Taffy carries flex until it doesn't.
- **One engine, everywhere.** Native and WebAssembly, single-threaded — no
  worker threads to hide behind.

## The pipeline

One document, one cascade, one compiler, one contract, one kernel.

```text
source bytes  (.svg | .html)
    → one namespace-aware document        csscascade
    → one Stylo cascade                   csscascade
    → effective values: Base | Sample t   websem
    → rframe::Frame                       websem
    → resolve → drawlist → paint          n0
```

Each stage owns one decision. `csscascade` resolves values and decides no
meaning; `websem` decides what will and will not render, and never touches a
canvas; `rframe` is a vocabulary whose value is what it *cannot* express; `n0`
decides how to get pixels, never what they mean.

Two pieces are borrowed on purpose. The cascade is **Stylo** — Firefox's style
engine, pinned to an upstream revision, not a CSS subset written here. The
rasterizer is **Skia** — the same one Chromium rasterizes with, which is much of
why byte-exact comparison against a Chromium bake is a reasonable bar at all.
n0 is the architecture between them: it owns resolution, the drawlist, damage,
caching, and time. It owns no cascade and no rasterizer.

## Declared holes, never guessed pixels

The engine refuses loudly, or it names the hole. It never guesses.

```console
$ # best-effort (the default): render what is admitted, declare the rest by name
$ cargo run -q -p n0_cli --bin n0 -- fixtures/test-svg/probe/polygon-fill-probe.svg out.png 64x64
degraded: skipped svg/polygon[1]: unsupported element <polygon>
rendered fixtures/test-svg/probe/polygon-fill-probe.svg -> out.png (64x64, base-shared-frame, 1 degraded, 223 bytes)

$ # --strict: refuse on the first construct outside the slice
$ cargo run -q -p n0_cli --bin n0 -- fixtures/test-svg/probe/polygon-fill-probe.svg out.png 64x64 --strict
error: render failed: unsupported element <polygon>
```

Where nothing degrades, the two admissions are frame-identical, and a law
checks that across the whole corpus.

```sh
# a whole composition — containers, curves, strokes, one animated rect —
# rendered at its authored state, and at exactly one second
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg cub.png 96x96
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg cub-1s.png 96x96 \
  --time-ns 1000000000
```

## Where it is today

The Web path renders a standalone SVG, or an HTML document's first inline SVG:
`<rect>`, `<circle>`, `<ellipse>`, `<path>` and `<line>` — filled and stroked —
nested in `<g>` with the whole `transform` grammar; root sizing per SVG2 §8.2
with the full `preserveAspectRatio` grammar; and one exact-time `<animate>`.
[`crates/n0_cli/README.md`](./crates/n0_cli/README.md) is the statement of
record for that slice and what it refuses.

Its corpus is 77 Chromium-baked cells and 10 sampled frames, byte-exact except
six curved cells carrying a declared tolerance confined to the weighted
rational conic. That describes one enumerated corpus — **it is not a
conformance claim**, and no conformance score exists.

Not admitted yet: text, gradients, clips, masks, filters, and opacity. Not built
yet: any layout engine, any editor host, any WebAssembly target.

**Nothing here is published.** There are no releases, and the only shipped
artifact in the tree is the frozen v1 wasm package. Run it from a clone —
[setup](./AGENTS.md#setup).

## One pipeline

2D has no perceptual slack: a 40px icon is either right or visibly wrong, and no
level-of-detail trick hides it. So there is no fast renderer beside an exact one.
Static is the same pipeline with an empty temporal input set — no camera delta,
no previous frame, no dirty set. Realtime is the same pipeline allowed to reuse.

> Modes may differ in **when and at what quality** they paint — never in **what
> things mean**.

The architecture that makes realtime possible ships from day one; the
optimizations it needs ship only once measured. So the sockets ship empty:
`DirtyClass` classifies every operation's invalidation in `n0-model`, and the
engine references it exactly zero times. It full-resolves every frame until
correctness has earned the right to reuse.

## The laws

- **Never a silent wrong pixel.** Refuse, or name the hole.
- **A module's identity is what it refuses**, and the refusal has a guarding
  test. `rframe` cannot express a gradient. `animation-sampling` owns no clock.
  `n0_cli` is architecturally forbidden from calling the renderer it replaced.
- **Reuse ≡ fresh.** Any frame produced with reuse is byte-identical to the same
  frame produced from scratch.
- **The oracle is external.** Chromium, or declared consensus. Never this
  engine's other half. Stylo bounds what can be *supported*; a gap there is a
  declared hole, never a wrong pixel.

## Why the Web first

The destination is n0's own source language — SVG with layout, reimagined. It
is deliberately not being built yet.

The Web is the only 2D graphics specification that is frozen, adversarially
tested, and ships with a free executable oracle. A custom format has none of
that. On the Web path every correctness question has an answer before anyone has
an opinion — and the cascade, layout, paint, text, and animation built to satisfy
it are the same ones the format will stand on.

The format is not deprioritized. It is waiting for a bar that can grade it.

## Also in this tree

**A second engine.** [`crates/grida`](./crates/grida/README.md) is the mature
v1: 19 node types, gradients, image filters, shadows, masks, PDF and SVG export,
and a realtime estate benchmarked to 135K-node documents. It ships
as `@grida/canvas-wasm` and is frozen there. It is being succeeded, not
extended — its know-how migrates into n0 through contracts, never by copying,
and the measurements in
[`docs/wg/feat-2d/optimization.md`](./docs/wg/feat-2d/optimization.md) are the
best existing description of what a realtime 2D engine actually needs.

**Two custom formats, parked.** `.grida` (the v1 FlatBuffers binary) is frozen
and read-only. `.n0.xml` (the authored source language) builds and is tested,
and is deliberately not being expanded. Both are waiting on the same thing: a
foundation proven against an oracle that can grade it.

## Workspace

The Web path — `csscascade` → `websem` → `rframe` → `n0`:

| crate                                                         |                                                                    |
| ------------------------------------------------------------- | ------------------------------------------------------------------ |
| [`csscascade`](./crates/csscascade/README.md)                 | the Stylo bridge — one namespace-aware document, one cascade       |
| [`websem`](./crates/websem/README.md)                         | the Web semantic compiler — what will and will not render          |
| [`rframe`](./crates/rframe/README.md)                         | the resolved render contract — backend-free, provisional           |
| [`animation-sampling`](./crates/animation-sampling/README.md) | the time axis — no ambient clock                                   |
| [`n0`](./crates/n0/README.md) · [`n0-model`](./crates/n0-model/README.md) | the engine, and its skia-free model                     |
| [`n0_cli`](./crates/n0_cli/README.md)                         | the `n0` binary                                                     |

The v1 engine and its satellites: `grida`, `htmlcss`, `cg`, `grida_editor`,
`grida-canvas-wasm`, `grida_dev`, `grida_wpt`. Foundations: `math2`, `fonts`.
The full map is in [AGENTS.md](./AGENTS.md).

## Docs

[`docs/wg/`](./docs/wg/index.md) is the engine's working group — normative
specifications, domain studies, and research, including 41 documents on
Chromium's rendering architecture. Start at
[the consolidation program](./docs/wg/consolidation/index.md) for how the two
engines become one.

Rust-first Cargo workspace. Licensed under [MIT](./LICENSE-MIT) or
[Apache-2.0](./LICENSE-APACHE), at your option.
