//! Reftest — the Web path's render vs. a committed **Chromium** oracle.
//!
//! Compiles the standalone SVG fixture into the neutral contract, renders it
//! through the shared `rframe` downstream, and probes interior pixels against
//! the committed Chromium bake (`fixtures/web-first/chromium/…`, baked at
//! deviceScaleFactor=1 by `bake_chromium.ts`). The fill color is the fixture
//! input; Chromium is the independent oracle. No similarity score is computed
//! and the sealed scoreboard is never invoked.

use rframe::{decode_png, render};
use websem::compile_standalone_svg;

const SVG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/svg-currentcolor-rect.svg"
));
const ORACLE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/chromium/svg-currentcolor-rect.png"
));

const GREEN: [u8; 4] = [0x16, 0xa3, 0x4a, 0xff];
const PROBES: &[(i32, i32)] = &[(0, 0), (1, 1), (32, 32), (62, 62), (63, 63), (10, 50)];

#[test]
fn standalone_svg_matches_committed_chromium_oracle() {
    let frame = compile_standalone_svg(SVG).expect("compile standalone SVG");
    let actual = render(&frame, 64, 64);
    let oracle = decode_png(ORACLE_PNG).expect("decode committed Chromium oracle");
    assert_eq!(
        (oracle.width, oracle.height),
        (64, 64),
        "oracle must be 64x64"
    );

    for &(x, y) in PROBES {
        let a = actual.at(x, y);
        let o = oracle.at(x, y);
        assert_eq!(
            a, o,
            "pixel ({x},{y}): rframe {a:?} != Chromium oracle {o:?}"
        );
        assert_eq!(a, GREEN, "pixel ({x},{y}) should be #16a34a");
    }
}

#[test]
fn render_is_deterministic() {
    let frame = compile_standalone_svg(SVG).expect("compile standalone SVG");
    let a = render(&frame, 64, 64);
    let b = render(&frame, 64, 64);
    assert_eq!(a.pixels, b.pixels, "two renders must be byte-identical");
}
