//! Equivalence test (NOT a reftest — neither side is an external oracle).
//!
//! The inline-SVG-in-HTML grammar entry and the standalone-SVG grammar entry
//! go through the same document machinery, the same Stylo cascade, and the
//! same SVG semantic compiler. Their SVG-local resolved frames must be
//! byte-identical — the strongest evidence that "both reach the same
//! provisional source-neutral resolved representation" (Web-First Amendment).
//!
//! Both carry the green fill via a cascaded `color` + `fill="currentColor"`;
//! the inline entry sources `color` from the surrounding HTML `<style>` rule,
//! the standalone entry from an inline `style` attribute — different sources,
//! one shared cascade, one identical resolved frame.

use websem::{compile_html_inline_svg, compile_standalone_svg};

const HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/html-inline-svg-currentcolor-rect.html"
));
const SVG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/svg-currentcolor-rect.svg"
));

#[test]
fn inline_and_standalone_resolve_to_the_same_frame() {
    let inline = compile_html_inline_svg(HTML).expect("compile inline-SVG-in-HTML");
    let standalone = compile_standalone_svg(SVG).expect("compile standalone SVG");
    assert_eq!(
        inline, standalone,
        "inline and standalone SVG must reach the same SVG-local resolved frame"
    );
}
