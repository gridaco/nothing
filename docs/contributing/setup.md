# Contributing to nothing | Setup

What a fresh clone needs before the workspace builds. Everything here is
per-machine — none of it is checked in.

## Base

```sh
# 1. Rust toolchain — auto-pins via rust-toolchain.toml (rustfmt + clippy included)
cargo --version

# 2. ninja — required by skia-bindings
brew install ninja                      # macOS
# sudo apt-get install -y ninja-build   # Ubuntu/Debian
```

That covers `cargo test` and the `n0` binary. `skia-bindings` downloads a
prebuilt Skia for target/feature combinations that rust-skia publishes and
builds from source otherwise, which is what ninja is for — so the first
build on a combination without a prebuilt is long, and later ones are not.

## WebAssembly

Only needed to build `@grida/canvas-wasm`. Nothing else in the workspace
targets WASM — see the caveats in [AGENTS.md](../../AGENTS.md#current-state-and-caveats).

### Emscripten

The SDK is a submodule, installed at a pinned version by `bin/activate-emsdk`:

```sh
git submodule update --init third_party/externals/emsdk
python3 bin/activate-emsdk
```

Two things to know:

- **`bin/activate-emsdk` is the source of truth for the SDK version.** Do not
  run `emsdk install latest` instead — an unpinned SDK drifts away from the one
  CI builds with, and Emscripten is not ABI-stable across major versions.
- **The activator needs python >= 3.10**, an emsdk 6.0.4+ requirement. macOS
  ships 3.9, so `brew install python` first; Linux CI images are already past
  that floor.

### Build

The crate's [justfile](../../crates/grida-canvas-wasm/justfile) is the supported
path — it activates emsdk, builds, copies the artifacts into `lib/bin/`, and
packages:

```sh
brew install just               # or: cargo install just
cd crates/grida-canvas-wasm && just build
```

`just dev` is the same thing in debug mode (~100MB output rather than ~10MB).
Publishing is separate — see
[PUBLISHING.md](../../crates/grida-canvas-wasm/PUBLISHING.md).

### Building without the justfile

Follow this if `just` is unavailable, or when isolating an OS-specific failure.
These steps were tested on a fresh Ubuntu container and produce the same
`lib/bin/grida_canvas_wasm.wasm` and `lib/bin/grida-canvas-wasm.js`.

1. **Fetch submodules and install build tools**

   ```sh
   git submodule update --init --recursive
   rustup target add wasm32-unknown-emscripten
   pnpm install
   ```

2. **Install and activate Emscripten** — as above, from the repo root:

   ```sh
   python3 bin/activate-emsdk
   ```

3. **(Ubuntu only) provide missing locale headers.** Some Ubuntu images lack
   `xlocale.h`, which breaks the Skia build. A symlink fixes it:

   ```sh
   sudo ln -s /usr/include/locale.h /usr/include/xlocale.h
   ```

4. **Build the crate**

   ```sh
   cd crates/grida-canvas-wasm
   source ../../third_party/externals/emsdk/emsdk_env.sh
   export CC=emcc CXX=em++ AR=emar
   cargo build --release --target wasm32-unknown-emscripten
   ```

   If this fails inside the `skia-bindings` build script with
   `failed to prepare emscripten archive for linking: … lib<x>.wasm.a … No such
   file or directory`, you have hit a known defect in the published prebuilts
   (<https://github.com/rust-skia/rust-skia/issues/1310>). Seed the expected
   names and re-run the build — this is what the justfile does automatically:

   ```sh
   for f in ../../target/wasm32-unknown-emscripten/*/build/skia-bindings-*/out/skia/lib*.a; do
     [ -e "${f%.a}.wasm.a" ] || cp "$f" "${f%.a}.wasm.a"
   done
   ```

5. **Copy artifacts and package**

   ```sh
   mkdir -p lib/bin
   cp ../../target/wasm32-unknown-emscripten/release/*.js lib/bin/
   cp ../../target/wasm32-unknown-emscripten/release/*.wasm lib/bin/
   pnpm --filter @grida/canvas-wasm build
   ```

After these steps the compiled module is in `lib/bin/`, ready for consumption
or publishing.

## See also

- [`crates/grida-canvas-wasm/AGENTS.md`](../../crates/grida-canvas-wasm/AGENTS.md)
  — build system internals, bundler compatibility, output layout.
- [`crates/grida-canvas-wasm/PUBLISHING.md`](../../crates/grida-canvas-wasm/PUBLISHING.md)
  — cutting an npm release.
- [Web Platform Tests](./wpt.md) — the separate sibling-checkout setup that
  suite needs.
