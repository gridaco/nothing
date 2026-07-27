# htmlcss

`htmlcss` is the internal, unpublished home of the mature static
HTML/CSS/SVG renderer extracted from `grida`. It preserves the existing
direct-Skia implementation while the Web semantic family is adopted into the
engine chassis through proved contracts.

The crate owns HTML parsing and Stylo cascade extraction, Taffy-backed layout,
the direct HTML painter, and the broad in-tree SVG renderer. Its public host
inputs are pre-resolved image/CSS/font resources; it performs no filesystem or
network I/O.

## Anti-goals

- Not the source-neutral resolved render contract, private drawlist, or engine
  kernel.
- Not an n0 model adapter and not a legacy node/import/format package.
- Not permission to promote the temporary SVG-only matcher as the cascade of
  record.
- Not a cleanup of the known global Stylo slot, inline-SVG serialize/reparse,
  direct-Skia paint, or ambient system-font behavior. Those are preserved here
  as compatibility evidence for separate gated cuts.

The governing topology and extraction patrol are recorded in the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md) and
[Web renderer adoption finding](../../docs/wg/consolidation/web-renderer-adoption.md).
