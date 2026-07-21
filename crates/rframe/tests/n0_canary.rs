//! The source-neutrality canary.
//!
//! A **non-Web** producer — the n0 engine — reaches the same downstream kernel.
//! This builds a minimal `n0-model` document (one green rectangle), runs n0's
//! own resolver, lowers the *resolved* facts into the neutral [`rframe::Frame`]
//! contract, and renders through the same drawlist + painter the Web path uses.
//!
//! It proves the shared downstream is source-neutral. It is an invariant probe,
//! not an n0 product milestone, and it adds no n0 XML features (see the
//! Web-First Amendment). `rframe` has no dependency on n0 — this coupling is
//! test-only.

use n0_model::math::Affine;
use n0_model::model::{
    Color as N0Color, DocBuilder, Header, Paint as N0Paint, Payload, ShapeDesc, SizeIntent,
};
use n0_model::resolve::{ResolveOptions, resolve};

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::frame::{Color, Frame, FrameNode, Geometry, NodeId, PaintStack};
use rframe::render;

/// n0's ARGB `Color(0xAARRGGBB)` → the neutral straight-alpha RGBA leaf.
fn to_rframe_color(c: N0Color) -> Color {
    let argb = c.argb();
    Color::rgba(
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
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        nodes: vec![FrameNode {
            id: NodeId(0),
            transform: to_math2(n0_world),
            geometry: Geometry::Rect(geometry),
            bounds: geometry,
            paints: PaintStack::solid(fill),
            clip: None,
        }],
    };

    // 4. Render through the same downstream the Web producers use.
    let raster = render(&frame, 64, 64);
    for (x, y) in [(1, 1), (32, 32), (62, 62)] {
        assert_eq!(
            raster.at(x, y),
            [0x16, 0xa3, 0x4a, 0xff],
            "n0-sourced pixel ({x},{y}) should be #16a34a through the shared kernel"
        );
    }
}
