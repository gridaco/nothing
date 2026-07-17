# Hi robots, welcome to nothing — the Grida graphics engine.

`n0` ("nothing") is the 2D graphics engine. This is a **Rust-first Cargo
workspace** (resolver 3; members in the root `Cargo.toml`). The Grida product
monorepo — editor, packages, services — is
[gridaco/grida](https://github.com/gridaco/grida); it consumes this repo
**only** as the published `@grida/canvas-wasm` npm artifact. Do not add
product/editor code here.

## Setup

```sh
# 1. emsdk submodule (needed for WASM builds only)
git submodule update --init

# 2. Rust toolchain auto-pins via rust-toolchain.toml (rustfmt + clippy included)
cargo --version

# 3. ninja is required for skia-bindings
brew install ninja            # macOS
# sudo apt-get install -y ninja-build   # Ubuntu/Debian
```

## Commands

```sh
# check (each crate must pass independently)
cargo check -p grida -p grida-canvas-wasm -p grida_dev

# tests
cargo test -p grida     # engine tests
cargo test              # all

# lint / format (enforced)
cargo clippy --no-deps  # skia deps make full clippy expensive
cargo fmt --all

# WASM build + npm package (crate-local justfile; see its PUBLISHING.md)
cd crates/grida-canvas-wasm && just build

# FlatBuffers codegen (pinned flatc; CI asserts freshness)
python3 bin/activate-flatc -- --rust -o crates/grida/src/io/generated format/grida.fbs
```

## Project Structure

| directory                   | notes                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------ |
| `crates/grida`              | the engine core (rendering, node model, io, text, svg/html import)                         |
| `crates/grida_editor`       | editor core — document working copy, invertible mutations, history, commands               |
| `crates/grida-canvas-wasm`  | WASM bindings + the `@grida/canvas-wasm` npm package (see its `PUBLISHING.md`)             |
| `crates/math2` · `csscascade` · `fonts` | foundations                                                                    |
| `crates/grida_dev`          | dev CLI, benchmarks, reftest tooling                                                       |
| `crates/grida_wpt`          | web-platform-tests harness                                                                 |
| `crates/n0`                 | the reserved public `n0` crate (future topology — a separate, deliberate task)             |
| `format/`                   | the FlatBuffers schema (`grida.fbs`) — **source of truth**; see `format/AGENTS.md`         |
| `docs/wg/`                  | the engine's normative working-group specs (canvas, format, research, feat-*) — same-repo  |
| `fixtures/`                 | test corpora (see the `fixtures` skill); **`fixtures/local/` is untracked** — large suites (resvg, W3C SVG 1.1, oxygen-icons, perf, refig) are downloaded per-machine |
| `packages/grida-reftest`    | the reftest diff/score/report npm tooling (run via `pnpm -C packages/grida-reftest exec …`) |
| `third_party/`              | vendored usvg (reference source) + emsdk submodule                                         |
| `bin/`                      | `activate-flatc`, `activate-emsdk` — pinned tool activators                                |

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
the docs site does not publish this repo's wg tree. `main` only, no SHA pins.

## The freeze contract (v1)

gridaco/grida is frozen on the published `@grida/canvas-wasm@0.91.0-canary.22`.
This repo owns publishing and must never unpublish/deprecate that version.
The `v1-freeze` branch pins the tree that built it, for emergency `canary.N+1`
cuts.
