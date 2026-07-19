---
name: io-grida
description: >
  Guides work on the Grida file format (.grida): the FlatBuffers schema and the
  Rust decoder that loads it into the canvas runtime. Use when working with
  .grida files, the FlatBuffers schema, codegen, or debugging format issues.
  (The TS reader/writer packages live in the grida product repo.)
---

# Grida I/O — `.grida` Format & Loading (engine side)

## Format Overview

Grida uses **FlatBuffers** as the canonical binary format. File identifier: `"GRID"`.

Two on-disk variants:

| Variant         | Detection             | Notes                                          |
| --------------- | --------------------- | ---------------------------------------------- |
| Raw FlatBuffers | `"GRID"` at bytes 4–7 | Bare document, no images                       |
| ZIP archive     | ZIP magic bytes       | `manifest.json` + `document.grida` + `images/` |

**Document model**: Flat node repository (not nested). Nodes reference parents via ID + fractional-index position strings. Multi-scene: each Figma page → a `SceneNode`.

## Key Locations

| Path                                     | Role                                                        |
| ---------------------------------------- | ----------------------------------------------------------- |
| `format/grida.fbs`                       | **Source of truth** — FlatBuffers schema                    |
| `format/AGENTS.md`                       | Schema evolution rules (append-only, never reuse field IDs) |
| `crates/grida/src/io/`                   | Rust decoder (FBS/ZIP/JSON → `Scene`)                       |
| `crates/grida/src/io/generated/grida.rs` | Auto-generated Rust FlatBuffers bindings (committed)        |
| `crates/grida/src/node/schema.rs`        | Rust runtime node schema                                    |

The **TS side** (reader/writer, archive pack, clipboard) lives in the grida
product repo: [`packages/grida-canvas-io`](https://github.com/gridaco/grida/tree/main/packages/grida-canvas-io)
and [`packages/grida-canvas-schema/grida.ts`](https://github.com/gridaco/grida/blob/main/packages/grida-canvas-schema/grida.ts).

## Rust Side — `crates/grida/src/io/`

| File               | Role                                                           |
| ------------------ | -------------------------------------------------------------- |
| `io_grida_file.rs` | Format detection + unified `decode_all(&bytes)` → `Vec<Scene>` |
| `io_grida_fbs.rs`  | FlatBuffers → Rust runtime (`GridaFile` → `Scene`)             |

## Code Generation

Schema → generated Rust code (committed; CI asserts freshness via
`.github/workflows/check-generated-fbs.yml`):

```sh
python3 bin/activate-flatc -- --rust -o crates/grida/src/io/generated format/grida.fbs
```

Uses pinned `flatc` v25.12.19 via `bin/activate-flatc`. The `--ts` mode also
works here — it is only needed when the grida repo deliberately re-snapshots
its **frozen** TS bindings (its generator was severed at the engine split).

## Verification

```sh
cargo test -p grida --test fbs_roundtrip
```

## Schema Changes

### Evolution (non-breaking, default)

- Add new **optional** fields only; never change or reuse field IDs
- Prefer `table` over `struct` (structs are immutable once defined)
- Append-only enums; new union variants at the end only
- PATCH bump only (e.g. `0.91.0` → `0.91.1`)
- Round-trip tests required

### Breaking changes

When you need to invalidate old files:

1. **Bump MINOR** (while MAJOR=0) in both places — **this is now a
   cross-REPO lockstep**:
   - Rust: `SCHEMA_VERSION` in `crates/grida/src/io/io_grida_fbs.rs` (here)
   - TS: `grida.program.document.SCHEMA_VERSION` in
     [`packages/grida-canvas-schema/grida.ts`](https://github.com/gridaco/grida/blob/main/packages/grida-canvas-schema/grida.ts)
     (grida repo — coordinate the change there)
2. Keep them **exactly in sync** — both writers must emit the same version string.
3. Old files will be **rejected** by the TS reader (`isSchemaCompatible()` throws on mismatch).

Format: `MAJOR.MINOR.PATCH-prerelease+build`. See `format/AGENTS.md` for the full checklist.

## Debugging FlatBuffers Issues

Use `flatbuffers::root::<fbs::GridaFile>(&bytes)` (not `root_unchecked`) in a
Rust test to run the FlatBuffers verifier. It reports the exact field chain
with the bad offset.

```rust
use grida::io::generated::grida::grida as fbs;
let result = flatbuffers::root::<fbs::GridaFile>(&bytes);
```

Note: the TS FlatBuffers decoder is more lenient than Rust — a TS-side
round-trip may pass even when the bytes are structurally invalid. Always
verify with the Rust verifier.

### Cross-boundary tests

The TS-encode → WASM-decode cross-boundary tests were **retired at the engine
split** (they reached across the repo seam). Boundary coverage now means:
Rust round-trip tests here, TS reader tests in the grida repo, and the shared
`fixtures/test-grida` corpus (canonical here; grida holds a frozen snapshot).
