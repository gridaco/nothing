# Pinned gate fonts

Font bytes the Web-first suite's oracles depend on. A font here is an
**identity, not a family name**: the same name may resolve to different bytes,
so every entry is pinned by content hash, and a changed byte is a new identity
— never a silent re-bake. The method that consumes these fonts is
[the text-oracle brief](../../../docs/wg/consolidation/text-oracle.md).

| File | sha256 | Bytes | Source | License |
| --- | --- | --- | --- | --- |
| `ahem.ttf` | `b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448` | 21768 | [web-platform-tests/wpt `fonts/Ahem.ttf` @ `986881aa`](https://github.com/web-platform-tests/wpt/blob/986881aaf27ffc441f67dd9e5595e797141b1f40/fonts/Ahem.ttf) | Public domain |
| `ahem-a-acute-gap.ttf` | `5c5bae141120698a28040408774fecffbdae863791d7df870fc05c7f52daf12d` | 21280 | Deterministic T6 derivative of the pinned `ahem.ttf`; see below | Public domain |
| [`../../fonts/Allerta/Allerta-Regular.ttf`](../../fonts/Allerta/Allerta-Regular.ttf) | `16d6915227c7560725c037c9c93163cba5367c3ef4cf2ec12bf40b9eb2984a6b` | 16248 | [Google Fonts: Allerta](https://fonts.google.com/specimen/Allerta) | [SIL Open Font License 1.1](../../fonts/Allerta/OFL.txt), license bytes `f81e6bb77d9f3f302d6264545b8abcd09dda0827fb4b62cf811ed59d6fd3a968` |
| [`../../fonts/Bungee/Bungee-Regular.ttf`](../../fonts/Bungee/Bungee-Regular.ttf) | `b90c3ca443713b070cb1dec6a3bb1ef7572c2b565c431d9a85d74bbfa07e24cc` | 118080 | [Google Fonts: Bungee](https://fonts.google.com/specimen/Bungee) | [SIL Open Font License 1.1](../../fonts/Bungee/OFL.txt), license bytes `d5787a50dde5be6c6daecab9ed459939b1bc37cff0d0e00257eaf1ec2cf4c16c` |

[Ahem](https://web-platform-tests.org/writing-tests/ahem.html) is the font the
web platform's own test suite uses to make text deterministic: every visible
glyph is a solid em-square box (ascent 0.8em, descent 0.2em), so at integer
positions and a font size divisible by 5 a glyph rasterizes with no
anti-aliasing ambiguity at all. That property is what lets text enter the
byte-exact Chromium gate without weakening it.

`ahem-a-acute-gap.ttf` is the one purpose-built T6 fallback identity. The
committed bytes were produced with FontTools 4.60.2 from the pinned Ahem bytes,
with timestamp recalculation disabled and table order retained. After first
asserting both Unicode cmap tables mapped U+0041 to `A` and U+00C1 to
`Aacute`, the deterministic transform points U+0041 at Ahem's existing blank
`space` glyph and removes U+00C1 from both tables. No outlines, metrics, names,
or other mappings change. Repeating that transform produced the exact hash
above. The derivative keeps Ahem's public-domain license and exists only to
make primary-versus-fallback face identity visible on the exact pixel lattice.

Allerta is the first real-font Rung-B identity. It already lived in the shared
font fixtures, so the text suite references those exact bytes instead of
copying them. Its geometry oracle compares resolved artifact numbers with
Chromium and deliberately makes no browser-pixel claim.

Bungee first entered as a negative Rung-B boundary identity: its 50px metrics
are integral while its glyph boundaries miss Chromium's 1/64 query grid, so it
still guards that horizontal refusal independently from Allerta's
vertical-metric case. Oracle v4 also uses the same exact bytes for two positive
1000px geometry witnesses. Its U+0301 and U+030B glyphs remain separate from
ASCII `x`, advance by zero, and carry the measured attachment offsets needed
to prove both artifact axes. This is a two-mark admit-list, not general font or
Unicode coverage.

Oracle v5 additionally declares Bungee beside Ahem in the exact text pixel
suite, ahead of Ahem in environment order. The earlier positive cells select
Ahem by their computed requests; Bungee is the distinct alternate that makes
accidental environment-order selection visible. Oracle v8 uses the same bytes
to measure the opposite branch: a complete cluster the first selected face
cannot shape may move to Bungee, while canonical composition that the first
face can shape stays there. Bungee also keeps the real-font query-grid refusal
live after fallback selection.

Oracle v7 also declares aliases of the same pinned Allerta and Ahem bytes at
distinct static face tuples. Allerta is the visible losing face and Ahem the
selected face in the nearest-matching cell. Reusing the identities keeps the
test about stretch/style/weight choice rather than introducing another font;
each alias still states its complete tuple explicitly in the closed manifest.

This directory grows only when a rung's admitted slice requires a new pinned
identity, and never holds a corpus: breadth suites live untracked under
`fixtures/local/`.
