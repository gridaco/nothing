//! The n0 projection and replay laws for source-neutral pattern programs.
//!
//! These are producer-independent contract fixtures. Chromium remains the
//! external pixel oracle for Web meaning; here the asserted colors are the
//! literal hand-built inputs and therefore pin projection, repetition,
//! recursive replay, deterministic freshness, and damage ownership.

use std::sync::Arc;

use cg::CGColor;
use math2::transform::AffineTransform;
use math2::Rectangle;
use n0::glyphless::{compile, diff_frame};
use n0::paint::PaintCtx;
use rframe::{
    Frame, FrameItems, FrameNode, Geometry, Identity, PaintStack, PatternPaint, Provenance,
    VisualRef,
};

fn owner(value: u64) -> VisualRef {
    VisualRef::new(Identity::new(value), Provenance::new(value))
}

fn node(value: u64, rect: Rectangle, paints: PaintStack) -> FrameNode {
    FrameNode {
        owner: owner(value),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect),
        bounds: rect,
        paints,
        stroke: None,
    }
}

fn stripe_pattern(left: CGColor, right: CGColor) -> PatternPaint {
    PatternPaint::new(
        8.0,
        8.0,
        AffineTransform::identity(),
        Arc::new(FrameItems::from_nodes(vec![
            node(
                1,
                Rectangle::from_xywh(0.0, 0.0, 4.0, 8.0),
                PaintStack::solid(left),
            ),
            node(
                2,
                Rectangle::from_xywh(4.0, 0.0, 4.0, 8.0),
                PaintStack::solid(right),
            ),
        ])),
        1.0,
    )
    .expect("checked stripe program")
}

fn frame(pattern: PatternPaint) -> Frame {
    let target = Rectangle::from_xywh(0.0, 0.0, 32.0, 16.0);
    Frame {
        owner: owner(100),
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        items: FrameItems::from_nodes(vec![node(101, target, PaintStack::from_pattern(pattern))]),
    }
}

fn raster(frame: &Frame) -> Vec<u8> {
    compile(frame.clone())
        .expect("compile checked pattern frame")
        .raster_to_bytes(&AffineTransform::identity(), 64, 64, &PaintCtx::new(None))
        .expect("preflight and replay pattern")
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 64 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

#[test]
fn a_vector_tile_repeats_and_fresh_replay_is_identical() {
    let resolved = frame(stripe_pattern(
        CGColor::from_rgb(0xef, 0x44, 0x44),
        CGColor::from_rgb(0x22, 0xc5, 0x5e),
    ));
    let pixels = raster(&resolved);

    assert_eq!(at(&pixels, 1, 4), [0xef, 0x44, 0x44, 0xff]);
    assert_eq!(at(&pixels, 5, 4), [0x22, 0xc5, 0x5e, 0xff]);
    assert_eq!(at(&pixels, 9, 4), [0xef, 0x44, 0x44, 0xff]);
    assert_eq!(at(&pixels, 29, 12), [0x22, 0xc5, 0x5e, 0xff]);
    assert_eq!(
        pixels,
        raster(&resolved),
        "a fresh picture program and shader produce the same bytes"
    );
}

#[test]
fn a_nested_pattern_reenters_the_same_projection_and_replay() {
    let inner = stripe_pattern(
        CGColor::from_rgb(0xef, 0x44, 0x44),
        CGColor::from_rgb(0x22, 0xc5, 0x5e),
    );
    let outer = PatternPaint::new(
        16.0,
        16.0,
        AffineTransform::identity(),
        Arc::new(FrameItems::from_nodes(vec![node(
            3,
            Rectangle::from_xywh(0.0, 0.0, 16.0, 16.0),
            PaintStack::from_pattern(inner),
        )])),
        1.0,
    )
    .expect("bounded nested program");

    let pixels = raster(&frame(outer));
    assert_eq!(at(&pixels, 1, 4), [0xef, 0x44, 0x44, 0xff]);
    assert_eq!(at(&pixels, 5, 4), [0x22, 0xc5, 0x5e, 0xff]);
    assert_eq!(at(&pixels, 17, 12), [0xef, 0x44, 0x44, 0xff]);
}

#[test]
fn changed_tile_content_damages_the_outer_client_only() {
    let before = compile(frame(stripe_pattern(
        CGColor::from_rgb(0xef, 0x44, 0x44),
        CGColor::from_rgb(0x22, 0xc5, 0x5e),
    )))
    .expect("before product");
    let after = compile(frame(stripe_pattern(
        CGColor::from_rgb(0xef, 0x44, 0x44),
        CGColor::from_rgb(0x25, 0x63, 0xeb),
    )))
    .expect("after product");

    let damage = diff_frame(&before, &after);
    assert_eq!(damage.changed, [owner(101)]);
    assert_eq!(
        damage.union_frame,
        Some(Rectangle::from_xywh(0.0, 0.0, 32.0, 16.0)),
        "the repeated source has no independent scene damage owner"
    );
}
