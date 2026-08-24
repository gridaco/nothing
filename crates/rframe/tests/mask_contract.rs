//! Contract laws for resolved two-phase image masks.

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    ClipGeometry, ClipLayer, ClipPath, FrameItem, FrameItems, FrameItemsError, FrameNode, Geometry,
    Identity, Mask, MaskMode, PaintStack, Provenance, Scope, ScopeEffect, ScopeOpacity, VisualRef,
};

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id))
}

fn node(id: u64) -> FrameItem {
    let rect = Rectangle::from_xywh(8.0, 8.0, 24.0, 24.0);
    FrameItem::Node(FrameNode {
        owner: owner(id),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect),
        bounds: rect,
        paints: PaintStack::solid(CGColor::WHITE),
        stroke: None,
    })
}

fn mask(id: u64, mode: MaskMode) -> FrameItem {
    let rect = Rectangle::from_xywh(4.0, 4.0, 32.0, 32.0);
    let geometry = ClipGeometry::new(AffineTransform::identity(), Geometry::Rect(rect))
        .expect("finite mask region");
    let layer = ClipLayer::new(vec![geometry]).expect("one region geometry");
    let region = ClipPath::new(vec![layer]).expect("one region layer");
    FrameItem::MaskBegin(Mask::new(owner(id), mode, region))
}

fn scope(id: u64) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect: ScopeEffect::Opacity(ScopeOpacity::new(0.5).expect("0.5 is a scope fact")),
    })
}

#[test]
fn alpha_and_luminance_masks_carry_only_resolved_facts() {
    for mode in [MaskMode::Alpha, MaskMode::Luminance] {
        let FrameItem::MaskBegin(mask) = mask(1, mode) else {
            unreachable!()
        };
        assert_eq!(mask.mode(), mode);
        assert_eq!(mask.region().layers().len(), 1);
    }
}

#[test]
fn a_balanced_mask_admits_an_empty_source_as_transparent_black() {
    FrameItems::try_new(vec![
        mask(1, MaskMode::Luminance),
        node(2),
        FrameItem::MaskSource,
        FrameItem::MaskEnd,
    ])
    .expect("a valid empty mask source hides its non-empty target");
}

#[test]
fn mask_target_content_is_required() {
    assert_eq!(
        FrameItems::try_new(vec![
            mask(1, MaskMode::Alpha),
            FrameItem::MaskSource,
            FrameItem::MaskEnd,
        ]),
        Err(FrameItemsError::EmptyMaskTarget { index: 0 })
    );
}

#[test]
fn mask_phase_markers_cannot_escape_or_cross_other_scopes() {
    assert_eq!(
        FrameItems::try_new(vec![FrameItem::MaskSource]),
        Err(FrameItemsError::UnexpectedMaskSource { index: 0 })
    );
    assert_eq!(
        FrameItems::try_new(vec![FrameItem::MaskEnd]),
        Err(FrameItemsError::UnexpectedMaskEnd { index: 0 })
    );
    assert_eq!(
        FrameItems::try_new(vec![mask(1, MaskMode::Alpha), node(2)]),
        Err(FrameItemsError::UnclosedMask { index: 0 })
    );
    assert_eq!(
        FrameItems::try_new(vec![
            mask(1, MaskMode::Alpha),
            scope(9),
            node(2),
            FrameItem::MaskSource,
        ]),
        Err(FrameItemsError::UnexpectedMaskSource { index: 3 })
    );
    assert_eq!(
        FrameItems::try_new(vec![
            mask(1, MaskMode::Alpha),
            node(2),
            FrameItem::MaskSource,
            scope(9),
            node(3),
            FrameItem::MaskEnd,
        ]),
        Err(FrameItemsError::UnexpectedMaskEnd { index: 5 })
    );
    assert_eq!(
        FrameItems::try_new(vec![
            scope(9),
            mask(1, MaskMode::Alpha),
            node(2),
            FrameItem::ScopeEnd,
        ]),
        Err(FrameItemsError::UnopenedScopeEnd { index: 3 })
    );
}

#[test]
fn nested_masks_are_a_checked_painter_order() {
    FrameItems::try_new(vec![
        mask(1, MaskMode::Alpha),
        mask(2, MaskMode::Luminance),
        node(3),
        FrameItem::MaskSource,
        node(4),
        FrameItem::MaskEnd,
        FrameItem::MaskSource,
        node(5),
        FrameItem::MaskEnd,
    ])
    .expect("a nested target mask closes before the outer source phase");
}
