# Publishing `@grida/canvas-wasm`

**The publish flow is MANUAL.** No CI workflow publishes this package — the historical
grida-side workflows never actually published anything (one was a stub, one had zero
runs; both stayed behind at the engine split). Every published version — including
`0.91.0-canary.22` (2026-07-01) — was pushed by a maintainer from a local machine.
This document is that flow. (An approval-gated workflow is tracked in
[#8](https://github.com/gridaco/nothing/issues/8).)

## Prerequisites

- npm auth as an `@grida` scope member with publish rights (2FA)
- emsdk submodule initialized: `git submodule update --init third_party/externals/emsdk`
- Rust toolchain per `rust-toolchain.toml` (+ `wasm32-unknown-emscripten` target)
- `pnpm install` at the repo root (tsdown comes from the workspace)

## Flow

```sh
# 1. build the wasm + package the npm bundle (from this crate's directory;
#    its justfile activates emsdk via <repo-root>/bin/activate-emsdk).
#    `just build` compiles the wasm into lib/bin/ and runs tsdown → dist/.
cd crates/grida-canvas-wasm
just build

# 2. sanity: dist exists and the .wasm is real (≈17.6 MB, not a 130-byte LFS pointer)
ls -la dist/

# 3. bump "version" in package.json — THIS directory's manifest is the npm
#    package (name @grida/canvas-wasm; canary scheme: 0.91.0-canary.N)

# 4. publish (from this same directory; `prepack` re-runs `just build`,
#    `prepublishOnly` sanity-checks the dist size)
npm publish --access public --tag latest   # dist-tag policy: `latest` tracks the freeze pin
```

## Post-publish

- Commit the version bump.
- If the published version is (or becomes) the freeze pin consumed by `gridaco/grida`, coordinate
  per the freeze contract (see the root `AGENTS.md`) before moving any dist-tag — the pinned
  version must never be unpublished or deprecated.
