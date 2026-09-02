# Pinned gate fonts

Font bytes the Web-first suite's oracles depend on. A font here is an
**identity, not a family name**: the same name may resolve to different bytes,
so every entry is pinned by content hash, and a changed byte is a new identity
— never a silent re-bake. The method that consumes these fonts is
[the text-oracle brief](../../../docs/wg/consolidation/text-oracle.md).

| File | sha256 | Bytes | Source | License |
| --- | --- | --- | --- | --- |
| `ahem.ttf` | `b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448` | 21768 | [web-platform-tests/wpt `fonts/Ahem.ttf` @ `986881aa`](https://github.com/web-platform-tests/wpt/blob/986881aaf27ffc441f67dd9e5595e797141b1f40/fonts/Ahem.ttf) | Public domain |
| [`../../fonts/Allerta/Allerta-Regular.ttf`](../../fonts/Allerta/Allerta-Regular.ttf) | `16d6915227c7560725c037c9c93163cba5367c3ef4cf2ec12bf40b9eb2984a6b` | 16248 | [Google Fonts: Allerta](https://fonts.google.com/specimen/Allerta) | [SIL Open Font License 1.1](../../fonts/Allerta/OFL.txt), license bytes `f81e6bb77d9f3f302d6264545b8abcd09dda0827fb4b62cf811ed59d6fd3a968` |
| [`../../fonts/Bungee/Bungee-Regular.ttf`](../../fonts/Bungee/Bungee-Regular.ttf) | `b90c3ca443713b070cb1dec6a3bb1ef7572c2b565c431d9a85d74bbfa07e24cc` | 118080 | [Google Fonts: Bungee](https://fonts.google.com/specimen/Bungee) | [SIL Open Font License 1.1](../../fonts/Bungee/OFL.txt), license bytes `d5787a50dde5be6c6daecab9ed459939b1bc37cff0d0e00257eaf1ec2cf4c16c` |

[Ahem](https://web-platform-tests.org/writing-tests/ahem.html) is the font the
web platform's own test suite uses to make text deterministic: every visible
glyph is a solid em-square box (ascent 0.8em, descent 0.2em), so at integer
positions and a font size divisible by 5 a glyph rasterizes with no
anti-aliasing ambiguity at all. That property is what lets text enter the
byte-exact Chromium gate without weakening it.

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

This directory grows only when a rung's admitted slice requires a new pinned
identity, and never holds a corpus: breadth suites live untracked under
`fixtures/local/`.
