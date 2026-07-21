//! Probe + determinism for the kernel painter, independent of any producer.
//!
//! Builds a resolved [`Frame`] by hand (a green rectangle filling a 64×64
//! viewport), lowers it, rasterizes it, and asserts interior pixels — no
//! oracle needed (the fill color is the fixture input). Then re-rasterizes and
//! asserts byte-identical pixels.

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::frame::{Color, Frame, FrameNode, Geometry, NodeId, PaintStack};
use rframe::render;

const GREEN: [u8; 4] = [0x16, 0xa3, 0x4a, 0xff];

fn green_rect_frame() -> Frame {
    let rect = Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0);
    Frame {
        bounds: rect,
        nodes: vec![FrameNode {
            id: NodeId(0),
            transform: AffineTransform::identity(),
            geometry: Geometry::Rect(rect),
            bounds: rect,
            paints: PaintStack::solid(Color::opaque(0x16, 0xa3, 0x4a)),
            clip: None,
        }],
    }
}

#[test]
fn solid_rect_fills_the_viewport() {
    let raster = render(&green_rect_frame(), 64, 64);
    // Probe interior points (away from edges — no AA to worry about anyway).
    for (x, y) in [(1, 1), (32, 32), (62, 62), (10, 50)] {
        assert_eq!(raster.at(x, y), GREEN, "pixel ({x},{y}) should be #16a34a");
    }
}

#[test]
fn raster_is_deterministic() {
    let frame = green_rect_frame();
    let a = render(&frame, 64, 64);
    let b = render(&frame, 64, 64);
    assert_eq!(a.pixels, b.pixels, "two renders must be byte-identical");
}
