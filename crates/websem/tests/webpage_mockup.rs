//! Data test (no render) over the webpage-design fixture.
//!
//! The mockup is a webpage *layout* expressed as inline-SVG rectangles inside
//! an HTML document. This asserts the resolved facts directly: the fixture
//! compiles to many rects, and the brand color authored once in the HTML
//! `<style>` cascades into the inline SVG via `fill="currentColor"` — the
//! Web-first cross-boundary cascade, in a richer shape than the single-rect
//! fixtures.

use rframe::frame::Paint;
use websem::compile_html_inline_svg;

const HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/html-webpage-mockup.html"
));

fn solid_colors(frame: &rframe::Frame) -> Vec<[u8; 4]> {
    frame
        .nodes
        .iter()
        .flat_map(|n| n.paints.paints.iter())
        .map(|p| match p {
            Paint::Solid(c) => [c.r, c.g, c.b, c.a],
        })
        .collect()
}

#[test]
fn webpage_mockup_cascades_brand_color_into_inline_svg() {
    let frame = compile_html_inline_svg(HTML).expect("compile webpage mockup");
    assert!(
        frame.nodes.len() > 10,
        "the mockup is a multi-rect layout, got {} nodes",
        frame.nodes.len()
    );
    let colors = solid_colors(&frame);
    // #4a3aa7 is authored only in the HTML `<style> .brand` rule; its presence
    // on an inline-SVG rect proves the cascade crossed the HTML→SVG boundary.
    assert!(
        colors.contains(&[0x4a, 0x3a, 0xa7, 0xff]),
        "brand purple must cascade from the HTML <style> into the inline SVG"
    );
    // A directly-authored hex fill resolves too.
    assert!(
        colors.contains(&[0x16, 0xa3, 0x4a, 0xff]),
        "the green button fill must resolve"
    );
}
