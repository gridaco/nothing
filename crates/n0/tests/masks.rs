//! Byte-level laws for the source-neutral two-phase image-mask lowering.

use cg::CGColor;
use math2::transform::AffineTransform;
use math2::Rectangle;
use n0::glyphless;
use n0::paint::PaintCtx;
use rframe::{
    ClipGeometry, ClipLayer, ClipPath, Frame, FrameItem, FrameItems, FrameNode, Geometry, Identity,
    Mask, MaskMode, PaintStack, Provenance, VisualRef,
};

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id))
}

fn rect_node(id: u64, rect: Rectangle, color: CGColor) -> FrameItem {
    FrameItem::Node(FrameNode {
        owner: owner(id),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect),
        bounds: rect,
        paints: PaintStack::solid(color),
        stroke: None,
    })
}

fn mask(id: u64, mode: MaskMode, region: Rectangle) -> FrameItem {
    let geometry = ClipGeometry::new(AffineTransform::identity(), Geometry::Rect(region))
        .expect("finite mask region");
    let layer = ClipLayer::new(vec![geometry]).expect("one mask-region geometry");
    let region = ClipPath::new(vec![layer]).expect("one mask-region layer");
    FrameItem::MaskBegin(Mask::new(owner(id), mode, region))
}

fn empty_region_mask(id: u64, mode: MaskMode) -> FrameItem {
    let layer = ClipLayer::new(Vec::<ClipGeometry>::new()).expect("empty clip-all region layer");
    let region = ClipPath::new(vec![layer]).expect("one empty region layer");
    FrameItem::MaskBegin(Mask::new(owner(id), mode, region))
}

fn raster(items: Vec<FrameItem>) -> Vec<u8> {
    let bounds = Rectangle::from_xywh(0.0, 0.0, 32.0, 32.0);
    let frame = Frame {
        owner: owner(100),
        bounds,
        items: FrameItems::try_new(items).expect("checked mask stream"),
    };
    glyphless::compile(frame)
        .expect("admitted mask frame")
        .raster_to_bytes(&AffineTransform::identity(), 32, 32, &PaintCtx::new(None))
        .expect("resource-free mask raster")
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 32 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

#[test]
fn alpha_mask_uses_source_alpha_and_geometric_region() {
    let full = Rectangle::from_xywh(0.0, 0.0, 32.0, 32.0);
    let left = Rectangle::from_xywh(0.0, 0.0, 16.0, 32.0);
    let pixels = raster(vec![
        mask(1, MaskMode::Alpha, left),
        rect_node(2, full, CGColor::BLACK),
        FrameItem::MaskSource,
        rect_node(3, full, CGColor::from_rgba(255, 255, 255, 128)),
        FrameItem::MaskEnd,
    ]);

    assert_eq!(at(&pixels, 8, 16), [127, 127, 127, 255]);
    assert_eq!(at(&pixels, 24, 16), [255, 255, 255, 255]);
}

#[test]
fn luminance_mask_converts_color_before_multiplying_the_target() {
    let full = Rectangle::from_xywh(0.0, 0.0, 32.0, 32.0);
    let pixels = raster(vec![
        mask(1, MaskMode::Luminance, full),
        rect_node(2, full, CGColor::BLACK),
        FrameItem::MaskSource,
        rect_node(3, full, CGColor::RED),
        FrameItem::MaskEnd,
    ]);

    assert_eq!(at(&pixels, 16, 16), [201, 201, 201, 255]);
}

#[test]
fn empty_mask_source_is_transparent_black() {
    let full = Rectangle::from_xywh(0.0, 0.0, 32.0, 32.0);
    let pixels = raster(vec![
        mask(1, MaskMode::Alpha, full),
        rect_node(2, full, CGColor::BLACK),
        FrameItem::MaskSource,
        FrameItem::MaskEnd,
    ]);

    assert!(pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [255, 255, 255, 255]));
}

#[test]
fn empty_mask_region_erases_even_a_nonempty_source() {
    let full = Rectangle::from_xywh(0.0, 0.0, 32.0, 32.0);
    let pixels = raster(vec![
        empty_region_mask(1, MaskMode::Alpha),
        rect_node(2, full, CGColor::BLACK),
        FrameItem::MaskSource,
        rect_node(3, full, CGColor::WHITE),
        FrameItem::MaskEnd,
    ]);

    assert!(pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [255, 255, 255, 255]));
}
