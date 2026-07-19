# `format/`

This directory contains **canonical file formats and schemas** used across Grida.

## FlatBuffers

- **Schema**: `format/grida.fbs`
- **File identifier**: `"GRID"`
- **File extension**: `"grida"`
- **Docs**: [FlatBuffers documentation](https://flatbuffers.dev/)

### Install `flatc` locally (developer workflow)

Most developers will use an OS-installed `flatc`.

- macOS (Homebrew):

```sh
brew install flatbuffers
```

### Validate / compile schema

```sh
# Compiles the schema to a binary schema file (.bfbs)
flatc --schema --binary -o /tmp/grida-fbs-check format/grida.fbs
ls -la /tmp/grida-fbs-check
```

### Generate bindings (ad-hoc)

```sh
# TypeScript
flatc --ts --ts-no-import-ext -o /tmp/grida-fbs-gen/ts format/grida.fbs

# Rust
flatc --rust -o /tmp/grida-fbs-gen/rust format/grida.fbs
```

### Also available: `bin/activate-flatc` (the repo's pinned flatc)

The repo script `bin/activate-flatc` downloads and caches a **pinned** `flatc`
release binary (currently **v25.12.19**) and runs it — the same binary CI uses,
so regenerated output is byte-stable.

```sh
# Compiles the schema to a binary schema file (.bfbs)
python3 bin/activate-flatc -- --schema --binary -o /tmp/grida-fbs-check format/grida.fbs

# TypeScript
python3 bin/activate-flatc -- --ts --ts-no-import-ext -o /tmp/grida-fbs-gen/ts format/grida.fbs

# Rust
python3 bin/activate-flatc -- --rust -o /tmp/grida-fbs-gen/rust format/grida.fbs
```

> Generated code — where it lives post-split:
>
> - **Rust (this repo)**: `crates/grida/src/io/generated/grida.rs` — **committed**; CI asserts freshness (`.github/workflows/check-generated-fbs.yml`). Changes to it are expected in PRs that modify `grida.fbs`.
> - **TypeScript (gridaco/grida)**: the product repo holds a **frozen tombstone** of the TS bindings (`packages/grida-format/src`, generator severed at the split) — it follows schema changes only by a deliberate re-snapshot there.
>
> **Contributor workflow**: after editing `grida.fbs`, regenerate and commit the Rust bindings:
>
> ```sh
> python3 bin/activate-flatc -- --rust -o crates/grida/src/io/generated format/grida.fbs \
>   && mv crates/grida/src/io/generated/grida_generated.rs crates/grida/src/io/generated/grida.rs
> ```

## Format & Import Mapping Docs

For tracking how CSS, HTML, and SVG map into the Grida IR (and which properties are still missing), see the working group docs:

- **[docs/wg/format/](../docs/wg/format/)** — index page with links to:
  - [Grida IR reference](../docs/wg/format/grida.md) — canonical IR node types, paint, layout, effects
  - [CSS mapping](../docs/wg/format/css.md) — CSS → Grida IR property mapping and TODO tracker
  - [HTML mapping](../docs/wg/format/html.md) — HTML element → Grida IR node mapping
  - [SVG mapping](../docs/wg/format/svg.md) — SVG → usvg → Grida IR mapping tracker

## References

- [Adobe Photoshop File Format Specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/) — PSD/PSB structure, image resources, layer and mask info; useful when comparing or aligning design-tool format concepts.
- [Figma .fig (Kiwi) format](https://github.com/gridaco/grida/tree/main/.ref/figma) — extracted Kiwi schema (`fig.kiwi`) and tooling for Figma’s binary `.fig` format (reference material, kept in the product repo).

## Changelog

See [CHANGELOG.md](./CHANGELOG.md).
