<p align="center">
  <img src="./assets/logo.svg" alt="Nothing" width="474">
</p>

# n0 ("nothing")

Nothing but drawing. An engine for everything drawable.

`n0` (pronounced "nothing") is a 2D graphics engine.

## Status

The graphics engine lives here. It migrated from the
[Grida repository](https://github.com/gridaco/grida) with its full history
(2025→) carried over; Grida remains the service/editor monorepo and consumes
the engine only through published artifacts.

The `n0` crate name stays reserved: the migrated crates keep their working
names, and the nothing-local topology (what becomes `n0`) is a separate,
deliberate task.

## Workspace

- [`crates/grida`](./crates/grida) — the canvas/rendering engine core
- [`crates/grida_editor`](./crates/grida_editor) — the editor core (document, history, commands)
- [`crates/grida-canvas-wasm`](./crates/grida-canvas-wasm) — WASM bindings (`@grida/canvas-wasm`)
- [`crates/math2`](./crates/math2) · [`crates/csscascade`](./crates/csscascade) · [`crates/fonts`](./crates/fonts) — foundations
- [`crates/grida_dev`](./crates/grida_dev) · [`crates/grida_wpt`](./crates/grida_wpt) — dev tools, benchmarks, reftests
- [`crates/n0`](./crates/n0) — the reserved public `n0` crate
- [`format/`](./format) — the FlatBuffers schema (source of truth)
- [`docs/wg/`](./docs/wg) — the engine's normative working-group specs

The repository is a Rust-first Cargo workspace.
