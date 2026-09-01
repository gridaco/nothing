# Hi robots, welcome to n0 ("nothing") — the Grida graphics engine.

[README.md](./README.md) says what this engine is and why. This file is the
working map: the laws that bind every change, the commands, the layout, and the
caveats that are true right now.

`n0` is a 2D graphics engine and a **Rust-first Cargo workspace** (resolver 3;
members in the root `Cargo.toml`). The Grida product monorepo — editor,
packages, services — is [gridaco/grida](https://github.com/gridaco/grida); it
consumes this repo **only** as the published `@grida/canvas-wasm` npm artifact.
Do not add product/editor code here.

## The laws (bind every change)

These are what keep two duty cycles, three source languages, and an editor from
becoming three engines. Each has an enforcing mechanism — a law without one is
a comment.

- **Never a silent wrong pixel.** A construct the compiler cannot honour must
  refuse loudly (`--strict`) or be declared by name at a stable node path
  (best-effort). A patrol that over-refuses beats one that lets a wrong pixel
  through. _Enforced by:_ the websem patrols, the refusal corpus, and the law
  that both admissions are frame-identical where nothing degrades.

- **One meaning, many policies.** Render modes may differ in _when and at what
  quality_ they paint — never in _what things mean_. Static is not a second
  renderer; it is the same pipeline with an empty temporal input set (no camera
  delta, no previous frame, no dirty set). _Tripwire:_ a mode, budget, or
  quality flag must never become readable from `websem`, `rframe`, or resolve.
  The moment one is, two pipelines have started growing.

- **Realtime in structure, static in policy.** The architecture that makes
  realtime possible ships from day one, because retrofitting it _is_ the
  rebuild. The optimizations realtime needs ship only once measured, because an
  optimization is a relative claim — _same as X, faster_ — and a cache built
  over an unverified X becomes a second place that believes the wrong answer.
  _In tree:_ `DirtyClass` is fully classified in `n0-model/src/ops.rs` and
  referenced **zero** times in `crates/n0/src`. The socket is shaped, the policy
  is deliberately absent. Do not "finish" it without the gate below.

- **Reuse ≡ fresh.** Any frame produced with reuse must be byte-identical to the
  same frame produced from scratch. _Enforced by:_ `crates/n0/tests/cache.rs` —
  the `*_matches_fresh` naming _is_ the law. Every new cache, damage path, or
  incremental stage adds its own instance.

- **A module's identity is what it refuses**, and the refusal has a guarding
  test. `rframe` cannot express a paint that references a resource — a
  pattern, an image. `animation-sampling` owns no clock.
  `csscascade` adds no matcher of its own. _Enforced by:_ the architecture tests
  — `rframe`'s backend-free lock, the model tier's skia-free lock, `n0_cli`'s
  lock against the retired `htmlcss` route. Before adding to a module, state
  what it refuses; if the addition violates no refusal, the name is too loose.

- **The oracle is external.** Chromium or declared consensus grades pixels —
  never the other engine in this tree. Stylo bounds what can be _supported_: a
  gap there is a declared hole, not a wrong pixel, and never a reason to add a
  second matcher.

- **Patrol before drop.** No deletion, replacement, or conflict resolution
  without a triage pass and a captured-essence ledger first. Load-bearing
  caveats are re-homed before the deletion merges; deliberate drops are named in
  the commit message.

## Setup

[`docs/contributing/setup.md`](./docs/contributing/setup.md) is the statement of
record — Rust toolchain and `ninja` for the base, the pinned emsdk for a WASM
build. Do not restate it here.

## Commands

```sh
# check (each crate must pass independently)
cargo check -p htmlcss -p grida -p grida-canvas-wasm -p grida_dev -p n0 -p n0-model \
  -p websem -p rframe -p animation-sampling -p textlayout -p n0_cli

# tests
cargo test -p grida     # legacy engine tests
cargo test -p htmlcss   # extracted Web renderer tests
cargo test -p n0-model -p n0   # v2 engine tests (model is skia-free, fast)
cargo test -p n0_cli    # thin product command host + HTML/SVG render probes
cargo test              # all

# lint / format (enforced)
cargo clippy --no-deps  # skia deps make full clippy expensive
cargo fmt --all

# WASM build + npm package (crate-local justfile; see its PUBLISHING.md)
cd crates/grida-canvas-wasm && just build

# FlatBuffers codegen (pinned flatc; CI asserts freshness of grida.rs)
python3 bin/activate-flatc -- --rust -o crates/grida/src/io/generated format/grida.fbs \
  && mv crates/grida/src/io/generated/grida_generated.rs crates/grida/src/io/generated/grida.rs
```

## Project Structure

| directory                   | notes                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------ |
| `crates/grida`              | legacy engine compatibility consumer (node model, io, text, import/export, editor-era runtime) |
| `crates/cg`                 | the backend-neutral canvas-graphics vocabulary                                             |
| `crates/htmlcss`            | extracted mature static HTML/CSS/SVG renderer; transitional direct-Skia Web implementation |
| `crates/grida_editor`       | editor core — document working copy, invertible mutations, history, commands               |
| `crates/grida-canvas-wasm`  | WASM bindings + the `@grida/canvas-wasm` npm package (see its `PUBLISHING.md`)             |
| `crates/math2` · `csscascade` · `fonts` | foundations                                                                    |
| `crates/grida_dev`          | dev CLI, benchmarks, reftest tooling                                                       |
| `crates/grida_wpt`          | web-platform-tests harness                                                                 |
| `crates/n0` · `n0-model` · `n0_dev` | the v2 engine family (the `anchor` model): skia-free model crate, resolve→drawlist→paint engine, winit/egui dev shell — promoted from the `model-v2-anchor` branch (gridaco/nothing#9) |
| `crates/websem`             | the Web semantic compiler: an SVG or HTML source → one namespace-aware document → one Stylo cascade → `rframe::Frame`. Owns no document parser and no painter; decodes admitted SVG attribute-value grammars while deciding what the engine will and will not render |
| `crates/rframe`             | the resolved render contract (`Frame`): the visual facts a producer states after resolving its source. Contract-only and backend-free — no document, no cascade, no paint call |
| `crates/animation-sampling` | the time axis: Base or one exact signed-nanosecond Sample, with no ambient clock                        |
| `crates/textlayout`         | the Web family's text resolution oracle ([the text-layout RFD](./docs/wg/feat-paragraph/text-layout.md) at its v2 profile): attributed text + a declared font environment → one immutable resolved layout with explicit source/glyph cluster ranges, or a typed refusal. Owns no font discovery, no render contract, no clock — and is *not* an engine-wide text service **by decision**: the D-M text stage joined low (2026-08-05), so each engine keeps its own text artifact and `rframe` carries no text fact ([the text-stage evidence](./docs/wg/consolidation/n0-join-point.md#the-text-stage-evidence)) |
| `crates/n0_cli`             | thin `n0` file-render command on the SVG engine of record (`websem → rframe → n0`): Base and exact-time SVG/HTML renders at WxH-as-initial-viewport — best-effort by default (constructs outside the admitted slice declared on stderr), `--strict` refuses loudly. Its README is the statement of record for that slice |
| `archive/model-v2/`                 | the frozen v2 workbench archive (phase papers, experiment verdicts, demo pages); paths inside the frozen papers refer to the pre-promotion layout — see its README's map |
| `format/`                   | the FlatBuffers schema (`grida.fbs`) — **source of truth**; see `format/AGENTS.md`         |
| `docs/wg/`                  | the engine's normative working-group specs (canvas, format, research, feat-*) — same-repo  |
| `fixtures/`                 | test corpora (see the `fixtures` skill); **`fixtures/local/` is untracked** — large suites (resvg, W3C SVG 1.1, oxygen-icons, perf, refig) are downloaded per-machine |
| `packages/grida-reftest`    | the reftest diff/score/report npm tooling (run via `pnpm -C packages/grida-reftest exec …`) |
| `third_party/`              | vendored usvg (reference source) + emsdk submodule                                         |
| `bin/`                      | `activate-flatc`, `activate-emsdk` — pinned tool activators                                |

## Current state and caveats

What is true right now, so a session does not infer it from ambition.

- **Two engines are live, on purpose.** `crates/grida` still depends on
  `crates/htmlcss` and still renders Web sources; D-N permitted breaking that
  and the permission was never used. The n0 path (`websem → rframe → n0`) is the
  SVG engine of record for **new** work — `htmlcss` is a frozen semantics donor
  that evolution rungs mine, never extend.
- **Nothing on the n0 path is publishable.** `n0_cli`, `n0-model`, `websem`, and
  `rframe` are all `publish = false`, and there are no releases. The only shipped
  artifact in the tree is the frozen v1 wasm package.
- **There is no n0 WebAssembly target.** `grida-canvas-wasm` binds
  `crates/grida` only. The v2 port is priced work, not an assumption.
- **Taffy is the layout engine — where layout exists.** It sits in `htmlcss`,
  `grida`, and `n0-model` only; the Web-first render path
  (`websem → rframe → n0`) runs no layout of any kind today, and `n0-model`'s
  resolve tier is never called on it. A house-built layout engine is a stated
  goal, not a current fact. The wall that would force it is browser-grade
  intrinsic sizing across a namespace-aware tree — not flex.
- **No conformance score may be produced or inspected.** The FLIP rule is
  unratified ([gridaco/nothing#49](https://github.com/gridaco/nothing/issues/49)).
  A corpus may be described; results may not be scored, aggregated, or presented
  as conformance.
- **The admitted slice has one statement of record** —
  [`crates/n0_cli/README.md`](./crates/n0_cli/README.md). Do not restate it
  elsewhere; link it. The same holds for the v1 capability inventory
  ([`crates/grida/README.md`](./crates/grida/README.md)) and the realtime
  optimization estate ([`docs/wg/feat-2d/optimization.md`](./docs/wg/feat-2d/optimization.md)).
- **Plans are `*.plan.md`** — gitignored scratch, never committed knowledge.
  Durable knowledge lands in `docs/wg/` or a crate README; work items land in
  issues.

## Skills

Agent skills live in `.agents/skills/` (`.claude/skills` symlinks to it):
engine loops and doctrine — `render-perf`, `render-reftest`, `io-svg`,
`io-grida`, `dev-render-htmlcss-feature`, `dev-render-htmlcss-svg-feature`,
`research`, `fixtures`, `docs-wg` — plus craft doctrine carried from grida
(`naming`, `sdk-design`, `sdk-seam`, `etiology`, `pedantic`, `links`,
`oss-standards`, `vision`).

## Link discipline (see the `links` skill)

Engine paths (`crates/`, `format/`, `docs/wg/**`, `fixtures/`) → same-repo
relative. grida-side references → absolute
`https://github.com/gridaco/grida/blob/main/<path>` or `https://grida.co/...`.
**Never** author `https://grida.co/docs/wg/...` links for docs that live here —
grida.co does not publish this repo's wg tree. (This repo's own `www/` docs
app does publish `docs/wg`; `.md`-suffixed relative links resolve there and
on GitHub alike.) `main` only, no SHA pins.

## The freeze contract (v1)

gridaco/grida is frozen on the published `@grida/canvas-wasm@0.91.0-canary.22`.
This repo owns publishing and must never unpublish/deprecate that version.
The `v1-freeze` branch pins the tree that built it, for emergency `canary.N+1`
cuts.

## Provenance

The engine migrated from [gridaco/grida](https://github.com/gridaco/grida) with
its full history (2025→); Grida remains the product monorepo. The v2 family was
promoted from the `model-v2-anchor` research branch
([gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)); its frozen
workbench record lives in [`archive/model-v2/`](./archive/model-v2/README.md),
and paths inside those frozen papers refer to the pre-promotion layout.

How two engines become one is the **consolidation program** —
[`docs/wg/consolidation/`](./docs/wg/consolidation/index.md). It owns the phases,
the gates, and the owner decision registry (D-C, D-L, D-M, D-N taken; FLIP,
NAME, D-D, D6, D-H, D-G(b), D-J, D-K, D-E/D-I open). Read
[its index](./docs/wg/consolidation/index.md) before program work; the charter
records the *route*, and the current *position* is tracked on
[gridaco/nothing#43](https://github.com/gridaco/nothing/issues/43).

## Where work gets filed

- **This repo (gridaco/nothing)**: engine rendering, the node/document model,
  `.grida` format/schema, engine text/SVG/HTML import, reftests and engine perf,
  `@grida/canvas-wasm` publishing, engine WG specs.
- **[gridaco/grida](https://github.com/gridaco/grida)**: the editor/product, desktop,
  forms/database, SVG editor (TS), platform/billing, and everything user-facing.
- When unsure: file where the fix would land. Cross-repo references are always
  full `gridaco/<repo>#N` form — never bare `#N`.
