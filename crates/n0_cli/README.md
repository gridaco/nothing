# n0 CLI

`n0_cli` builds the `n0` executable: the thin product host for file-to-output
rendering. The command owns arguments, source and asset I/O, host providers,
raster surfaces, and encoding. It does not own source semantics, layout, the
drawlist, or an authored document model.

The currently admitted surface is deliberately narrow:

```sh
cargo run -p n0_cli --bin n0 -- \
  fixtures/test-svg/L0/basic-shapes.svg /tmp/shapes.png 500x500

cargo run -p n0_cli --bin n0 -- \
  fixtures/test-html/L0/svg-inline-basic.html /tmp/page.png 800x600
```

- Input: one UTF-8 `.html`, `.htm`, or `.svg` file.
- Output: one `.png` file at an explicit positive `WxH` size.
- Resources: self-contained input only; external images and stylesheets are
  not resolved yet.
- Fonts: the ambient system font manager, with fallback enabled.

The route in this cut is `n0` host → `htmlcss` → direct Skia CPU raster → PNG.
That direct backend seam is transitional and named; moving the executable does
not claim that the mature renderer already lowers through the provisional
`rframe` chassis. The binary name likewise does not imply that Web sources are
converted into the n0 authored model. n0 XML, directory input, resource loading,
and additional encoders enter only when their actual contracts are implemented.

The governing topology is the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md); the physical
renderer-adoption ledger is
[web-renderer-adoption.md](../../docs/wg/consolidation/web-renderer-adoption.md).
