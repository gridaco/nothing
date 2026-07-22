# fixtures/web-first/pages — real-world webpage harness

Genuine, self-contained real-world webpages that serve as the **harness** the
Web-first engine is measured against — the *target*, not a demonstration of
current capability. They are deliberately rich (real semantic HTML, real text
content, images, real CSS), not minimized: "minimal, one concept per file" is
the rule for *unit* fixtures, not for a real-world page corpus.

> **The Web-first engine renders none of this yet.** The prototype
> (`crates/websem` + `crates/rframe`) handles solid-fill SVG `<rect>` only. These
> pages are the ground the engine grows toward; the ground truth is Chromium
> (see references below). This is the same relationship
> [`fixtures/test-html/`](../../test-html/README.md) has with the legacy
> renderer — an input corpus plus a browser oracle, not a pass today.

## The corpus

| File | Kind | Exercises |
| --- | --- | --- |
| `article.html` | Blog / magazine article | sticky header, hero SVG illustration, byline + avatar, drop-cap, `<h2>`/`<h3>` prose, blockquote, list, inline figure, syntax-highlighted `<pre><code>`, data table, callout, author card, related-posts grid, multi-column footer |
| `landing.html` | SaaS product landing page | sticky nav + CTA, hero with SVG product-dashboard mock, trusted-by logo strip, feature-card grid, 3-step + stats band, testimonial, 3-tier pricing (highlighted), gradient CTA band, multi-column footer with social icons |
| `docs.html` | Documentation page | three-column layout (left nav / content / TOC rail), breadcrumb, search box, version + GitHub controls, info & warning callouts, `<pre><code>` blocks, ordered steps, an API `<table>`, prev/next pager |

All imagery is **inline `<svg>`** (logos, avatars, illustrations, icons, chart
mocks). All CSS is inline in one `<style>`. Fonts are system stacks. There are
**zero** external resources — verified by baking with the network aborted (0
external requests blocked; a page that needed a CDN or webfont would render
broken).

## Ground truth (references)

`bake_reference.ts` renders each page in headless Chromium at
`deviceScaleFactor=1`, 1200px-wide viewport, full-page, with all `http(s)`
requests aborted. It writes `reference/<name>.png` + `reference-bake.json`
(provenance: browser version, per-file sha256, dimensions).

```sh
pnpm -C packages/grida-reftest exec tsx "$(pwd)/fixtures/web-first/pages/bake_reference.ts"
```

The `reference/` renders are **regenerable and environment-pinned** (system-font
text differs across OS/browser versions), so they are gitignored rather than
committed — the same discipline the refbrowser suites use. Baked with Chromium
149 the pages measure 1200×6561 (article), 1200×4621 (landing), 1200×3929
(docs).
