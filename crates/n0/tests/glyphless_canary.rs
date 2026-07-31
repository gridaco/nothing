//! The source-neutrality canary, on the one shared downstream.
//!
//! Real resolved facts from a non-Web producer — a minimal n0 authored
//! document run through n0's own resolver — lower into the neutral
//! [`rframe::Frame`] contract and reach pixels through the engine's one
//! private drawlist and painter. Formerly `crates/rframe/tests/n0_canary.rs`,
//! where the same facts rendered through rframe's temporary proving painter;
//! that duplicate downstream retired when the vector join was taken
//! (docs/wg/consolidation/n0-join-point.md).
//!
//! It is an invariant probe, not an n0 product milestone, and it adds no
//! n0 XML features (see the Web-First Amendment).

use n0::glyphless;
use n0::paint::PaintCtx;
use n0_model::math::Affine;
use n0_model::model::{
    Color as N0Color, DocBuilder, Header, Paint as N0Paint, Payload, ShapeDesc, SizeIntent,
};
use n0_model::resolve::{resolve, ResolveOptions};

use cg::CGColor;
use math2::transform::AffineTransform;
use math2::Rectangle;
use rframe::{Frame, FrameNode, Geometry, Identity, PaintStack, Provenance, VisualRef};

const GREEN: [u8; 4] = [0x16, 0xa3, 0x4a, 0xff];

/// n0's ARGB `Color(0xAARRGGBB)` → the neutral straight-alpha RGBA leaf.
fn to_rframe_color(c: N0Color) -> CGColor {
    let argb = c.argb();
    CGColor::from_rgba(
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
        ((argb >> 24) & 0xff) as u8,
    )
}

/// n0's `Affine { a, b, c, d, e, f }` (matrix `[[a,c,e],[b,d,f]]`) → math2's.
fn to_math2(a: Affine) -> AffineTransform {
    AffineTransform::from_acebdf(a.a, a.c, a.e, a.b, a.d, a.f)
}

/// The `[r, g, b, a]` at `(x, y)` of an RGBA8888 raster row-major buffer.
fn at(pixels: &[u8], width: i32, x: i32, y: i32) -> [u8; 4] {
    let height = pixels.len() as i32 / (width * 4);
    assert!(
        (0..width).contains(&x) && (0..height).contains(&y),
        "probe ({x}, {y}) outside {width}x{height}"
    );
    let offset = ((y * width + x) * 4) as usize;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

/// Compile an admitted contract frame and rasterize it through n0's one
/// private drawlist and painter.
fn raster(frame: &Frame, width: i32, height: i32) -> Vec<u8> {
    glyphless::compile(frame.clone())
        .expect("compile admitted canary frame")
        .raster_to_bytes(
            &AffineTransform::identity(),
            width,
            height,
            &PaintCtx::new(None),
        )
        .expect("resource-free glyphless raster")
}

#[test]
fn n0_rect_reaches_the_shared_downstream() {
    // 1. Build a minimal n0 document: one 64×64 rectangle, filled green.
    let mut b = DocBuilder::new();
    let rect = b.add(
        0,
        Header::new(SizeIntent::Fixed(64.0), SizeIntent::Fixed(64.0)),
        Payload::Shape {
            desc: ShapeDesc::Rect,
        },
    );
    b.node_mut(rect).fills = n0_model::model::Paints::solid(N0Color(0xFF16_A34A));
    let doc = b.build();

    // 2. Run n0's own resolver.
    let resolved = resolve(&doc, &ResolveOptions::default());
    let n0_box = resolved.box_of(rect);
    let n0_world = resolved.world_of(rect);

    // 3. Read the fill from n0's own model and lower into the neutral contract.
    let fill = match doc.get(rect).fills.as_slice() {
        [N0Paint::Solid(sp), ..] => to_rframe_color(sp.color),
        _ => panic!("expected a solid fill on the n0 rect"),
    };
    let geometry = Rectangle::from_xywh(n0_box.x, n0_box.y, n0_box.w, n0_box.h);
    let frame = Frame {
        owner: VisualRef::new(Identity::new(1), Provenance::new(1)),
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        nodes: vec![FrameNode {
            owner: VisualRef::new(Identity::new(2), Provenance::new(2)),
            transform: to_math2(n0_world),
            geometry: Geometry::Rect(geometry),
            bounds: geometry,
            paints: PaintStack::solid(fill),
            stroke: None,
        }],
    };

    // 4. Render through the same downstream the Web producers use.
    let pixels = raster(&frame, 64, 64);
    for (x, y) in [(1, 1), (32, 32), (62, 62)] {
        assert_eq!(
            at(&pixels, 64, x, y),
            GREEN,
            "n0-sourced pixel ({x},{y}) should be #16a34a through the shared kernel"
        );
    }
    assert_eq!(
        pixels,
        raster(&frame, 64, 64),
        "two renders must be byte-identical"
    );
}

#[test]
fn hand_built_frame_probes_and_re_raster_are_exact() {
    // From `crates/rframe/tests/paint_probe.rs`: the producer-independent
    // laws — a hand-built full-viewport solid rect probes exactly (the fill
    // color is the fixture input; no oracle needed) and re-rasters
    // byte-identically.
    let rect = Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0);
    let frame = Frame {
        owner: VisualRef::new(Identity::new(1), Provenance::new(1)),
        bounds: rect,
        nodes: vec![FrameNode {
            owner: VisualRef::new(Identity::new(2), Provenance::new(2)),
            transform: AffineTransform::identity(),
            geometry: Geometry::Rect(rect),
            bounds: rect,
            paints: PaintStack::solid(CGColor::from_rgb(0x16, 0xa3, 0x4a)),
            stroke: None,
        }],
    };

    let pixels = raster(&frame, 64, 64);
    for (x, y) in [(1, 1), (32, 32), (62, 62), (10, 50)] {
        assert_eq!(
            at(&pixels, 64, x, y),
            GREEN,
            "pixel ({x},{y}) should be #16a34a"
        );
    }
    assert_eq!(
        pixels,
        raster(&frame, 64, 64),
        "two rasters must be byte-identical"
    );
}

#[test]
fn hand_built_ellipse_fills_the_inscribed_oval_not_its_box_corners() {
    // The contract's ellipse geometry: the fill covers only the oval
    // inscribed in the local-space rectangle — the bounding-box center is
    // the fixture fill color, the box corner is not (the pixel fact that
    // distinguishes an oval from a rectangle) — and re-rasters
    // byte-identically.
    let bbox = Rectangle::from_xywh(8.0, 6.0, 20.0, 16.0);
    let frame = Frame {
        owner: VisualRef::new(Identity::new(1), Provenance::new(1)),
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 48.0),
        nodes: vec![FrameNode {
            owner: VisualRef::new(Identity::new(2), Provenance::new(2)),
            transform: AffineTransform::identity(),
            geometry: Geometry::Ellipse(bbox),
            bounds: bbox,
            paints: PaintStack::solid(CGColor::from_rgb(0x16, 0xa3, 0x4a)),
            stroke: None,
        }],
    };

    let pixels = raster(&frame, 64, 48);
    assert_eq!(
        at(&pixels, 64, 18, 14),
        GREEN,
        "pixel (18,14) at the oval's center should be #16a34a"
    );
    assert_ne!(
        at(&pixels, 64, 9, 7),
        GREEN,
        "pixel (9,7) at the bounding-box corner stays outside the oval"
    );
    assert_eq!(
        pixels,
        raster(&frame, 64, 48),
        "two rasters must be byte-identical"
    );
}
