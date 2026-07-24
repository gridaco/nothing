# n0 CLI

`n0_cli` builds the `n0` executable: the thin product host for file-to-output
rendering on the SVG engine of record
([D-N](../../docs/wg/consolidation/svg-engine-of-record.md)). The command owns
arguments, source I/O, raster surfaces, and encoding. It does not own source
semantics, layout, the drawlist, or an authored document model.

The route is one pipeline: the websem compiler lowers standalone SVG or
inline-HTML SVG from the retained document session to the shared
`rframe::Frame`, which the n0 engine compiles and paints on a CPU raster.
Static renders are the Base view; `--time-ns` renders one exact
signed-nanosecond Sample of the same compile:

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/svg-fill-named-rect.svg /tmp/rect.png 64x64

cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-rect-x-animation.svg /tmp/t1s.png 64x32 \
  --time-ns 1000000000
```

- Input: one UTF-8 `.html`, `.htm`, or `.svg` file.
- Output: one `.png` file at an explicit positive `WxH` size.
- Resources: self-contained input only; external images and stylesheets are
  not resolved.
- Capability: the admitted slice is deliberately narrow. Beyond-slice
  constructs refuse loudly with the construct named — never wrong pixels.
  The HTML entry compiles exactly the document's first inline SVG; when that
  subtree is admitted the render succeeds and the surrounding page
  contributes nothing (a pinned contract), and sampling inline HTML refuses.

The retired mature `htmlcss` route must not return silently (locked by this
crate's architecture test). The binary name does not imply that Web sources
are converted into the n0 authored model. n0 XML, directory input, resource
loading, and additional encoders enter only when their actual contracts are
implemented.

The governing topology is the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md); the
succession decision is
[svg-engine-of-record.md](../../docs/wg/consolidation/svg-engine-of-record.md);
the donor-mining map is
[web-renderer-adoption.md](../../docs/wg/consolidation/web-renderer-adoption.md).
