# Pinned gate fonts

Font bytes the Web-first suite's oracles depend on. A font here is an
**identity, not a family name**: the same name may resolve to different bytes,
so every entry is pinned by content hash, and a changed byte is a new identity
— never a silent re-bake. The method that consumes these fonts is
[the text-oracle brief](../../../docs/wg/consolidation/text-oracle.md).

| File | sha256 | Bytes | Source | License |
| --- | --- | --- | --- | --- |
| `ahem.ttf` | `b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448` | 21768 | [web-platform-tests/wpt `fonts/Ahem.ttf` @ `986881aa`](https://github.com/web-platform-tests/wpt/blob/986881aaf27ffc441f67dd9e5595e797141b1f40/fonts/Ahem.ttf) | Public domain |

[Ahem](https://web-platform-tests.org/writing-tests/ahem.html) is the font the
web platform's own test suite uses to make text deterministic: every visible
glyph is a solid em-square box (ascent 0.8em, descent 0.2em), so at integer
positions and a font size divisible by 5 a glyph rasterizes with no
anti-aliasing ambiguity at all. That property is what lets text enter the
byte-exact Chromium gate without weakening it.

This directory grows only when a rung's admitted slice requires a new pinned
identity, and never holds a corpus: breadth suites live untracked under
`fixtures/local/`.
