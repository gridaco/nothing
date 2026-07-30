# n0 CLI

`n0_cli` builds the `n0` executable: the thin product host for file-to-output
rendering on the SVG engine of record
([the SVG engine of record](../../docs/wg/consolidation/svg-engine-of-record.md)).
The command owns
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

# the cub: a whole composition (viewBox-only, containers, curves, strokes,
# a <line>) with one animated rect — the same file, static and at 1s
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub.png 96x96
cargo run -p n0_cli --bin n0 -- \
  fixtures/web-first/animation/svg-scene-cub-animation.svg /tmp/cub-1s.png 96x96 \
  --time-ns 1000000000

# dev harness: refuse on the first beyond-slice construct instead of
# rendering best-effort with declared degradations (the default)
cargo run -p n0_cli --bin n0 -- \
  fixtures/test-svg/probe/polygon-fill-probe.svg /tmp/probe.png 64x64 --strict
```

- Input: one UTF-8 `.html`, `.htm`, or `.svg` file.
- Output: one `.png` file at an explicit positive `WxH` size. For a
  standalone SVG, `WxH` is also the **initial viewport** (SVG2 §8.2) — the
  window the document is loaded into: explicit root `width`/`height` win, a
  missing dimension is `auto` and resolves to 100% of `WxH`, and `viewBox`
  maps user units into the viewport under the full `preserveAspectRatio`
  grammar. A viewBox-only SVG therefore renders at the requested raster.
- Resources: self-contained input only; external images and stylesheets are
  not resolved.
- Capability: the admitted slice is deliberately narrow — solid-filled and
  solid-stroked `<rect>`, `<circle>`, `<ellipse>`, `<path>` (the path-data
  grammar except the elliptical arc, with `fill-rule`) and `<line>`, nested in
  `<g>` containers with the SVG `transform` grammar, under the outer `<svg>`.
  A stroke is centred, its width is a cascaded length, and its cap, join and
  miter limit come from the one cascade; `stroke-opacity` and dashing do not.
  The
  default admission is **best-effort**: the admitted subset renders and
  every beyond-slice construct is declared on stderr with its node path and
  reason (`degraded: skipped svg/polygon[1]: unsupported element <polygon>`);
  a beyond-inventory dynamic surface that leaves the Base view honest (an
  event handler, a CSS animation carrier) samples as the Base view, while a
  beyond-inventory _animation element_ — active at document load in
  Chromium, so its target's authored state never honestly renders — skips
  its target in every view, declared at the target's path (one that cannot
  be attributed to a skippable element, an `href` retarget or a
  root-`<svg>` target, refuses in both admissions like `<script>`). Declared
  holes, never guessed pixels — the patrol is per attribute and per
  cascaded property, so an admitted element carrying a rendering attribute
  or stylesheet value the slice does not consume becomes a declared hole,
  not a wrong paint (cascaded properties beyond the enumerated patrol are
  a named open boundary; see the websem compiler doc). `--strict` refuses
  loudly on the first beyond-slice construct instead — the harness that names
  the slice's edge (`--best-effort` is the explicit spelling of the default).
  Document-level contracts (no `<svg>` root, malformed standalone XML, a
  script-suspended standalone parse, the outer viewport grammar —
  percentage root dimensions, malformed `preserveAspectRatio`,
  CSS-cascaded root sizing, missing dimensions on the HTML entry — and root
  patrols) refuse in both admissions. A stylesheet declaring a property the
  cascade cannot represent is document-level too, but only `--strict`
  refuses it: the default declares it against the sheet and renders, since
  a sheet is not attributable to one element without selector matching.
- The HTML entry compiles exactly the document's first inline SVG; when that
  subtree is admitted the render succeeds and the surrounding page
  contributes nothing (a pinned contract). Sampling inline HTML refuses
  under `--strict` and samples as the Base view (declared) by default.

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
