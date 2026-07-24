//! Resolved-fact laws for the SVG-namespace stylesheet intake as websem
//! observes it today.
//!
//! The shared cascade now collects SVG's own `<style>` element. websem's
//! compiler does not yet consume typed `fill` from the cascade, but it does
//! resolve `fill="currentColor"` against the cascaded computed `color` — so
//! stylesheet-fed `color` is already visible in resolved frames. These laws
//! pin that live path at the resolved-fact level; the Chromium pixel bake
//! for style-bearing SVG joins the typed-paint capability step.

use websem::{compile_html_inline_svg, compile_standalone_svg};

fn sole_paint(frame: &rframe::Frame) -> [u8; 4] {
    let node = frame.nodes.first().expect("one compiled rect");
    let solid = node.paints.iter().next().expect("one solid paint");
    [solid.color.r, solid.color.g, solid.color.b, solid.color.a]
}

#[test]
fn standalone_svg_stylesheet_color_feeds_current_color() {
    // The `<style>` sits inside the rect: outside the compiled child walk,
    // inside the one document cascade. Its `color` rule reaches the rect's
    // `currentColor` fill.
    let frame = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="currentColor"><style>svg { color: #ef4444 }</style></rect></svg>"##,
    )
    .expect("style-bearing subtree compiles for Base");
    assert_eq!(
        sole_paint(&frame),
        [0xef, 0x44, 0x44, 0xff],
        "SVG's own <style> must feed the cascaded color behind currentColor"
    );
}

#[test]
fn sibling_svg_stylesheet_feeds_the_shared_document_cascade() {
    // One HTML document, two inline SVGs: the second carries the <style>,
    // the first is compiled. Document-wide CSS is one cascade — the same
    // mechanism that lets a <head> stylesheet color inline SVG.
    let frame = compile_html_inline_svg(
        r##"<html><body>
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="currentColor"/></svg>
<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><style>svg { color: #ef4444 }</style></svg>
</body></html>"##,
    )
    .expect("compile the first inline SVG");
    assert_eq!(
        sole_paint(&frame),
        [0xef, 0x44, 0x44, 0xff],
        "a sibling SVG's <style> participates in the one document cascade"
    );
}

#[test]
fn sampling_still_refuses_style_bearing_subtrees() {
    // The dynamic inventory is unchanged: a <style> inside the compiled
    // subtree still refuses Sample explicitly, whatever Base resolves.
    let source = websem::SvgFrameSource::from_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="currentColor"><style>svg { color: #ef4444 }</style></rect></svg>"##,
    )
    .expect("Base materializes");
    let error = source
        .sample_frame(animation_sampling::SampleTime::ZERO)
        .expect_err("style-bearing subtree must refuse Sample");
    assert!(error.reason().contains("<style>"));
}
