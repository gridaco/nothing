//! Construction laws for the source-neutral repeating-program paint fact.
//!
//! The producer has already resolved every source relation before this
//! contract is built. These tests pin what remains: finite tile geometry, an
//! invertible local mapping, immutable checked items, one exclusive paint
//! value, and bounded recursive programs.

use std::sync::Arc;

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    FrameItems, FrameNode, Geometry, Identity, MAX_PATTERN_DEPTH, PaintStack, PatternPaint,
    PatternPaintError, Provenance, VisualRef,
};

fn owner(value: u64) -> VisualRef {
    VisualRef::new(Identity::new(value), Provenance::new(value))
}

fn rect_items(paints: PaintStack) -> FrameItems {
    let rect = Rectangle::from_xywh(0.0, 0.0, 8.0, 8.0);
    FrameItems::from_nodes(vec![FrameNode {
        owner: owner(1),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect),
        bounds: rect,
        paints,
        stroke: None,
    }])
}

fn pattern(items: FrameItems) -> Result<PatternPaint, PatternPaintError> {
    PatternPaint::new(8.0, 8.0, AffineTransform::identity(), Arc::new(items), 1.0)
}

#[test]
fn one_checked_program_is_the_complete_paint_value() {
    let items = rect_items(PaintStack::solid(CGColor::RED));
    let resolved = pattern(items.clone()).expect("finite pattern");
    let stack = PaintStack::from_pattern(resolved.clone());

    assert_eq!(resolved.width(), 8.0);
    assert_eq!(resolved.height(), 8.0);
    assert_eq!(resolved.transform(), AffineTransform::identity());
    assert_eq!(resolved.items().as_ref(), &items);
    assert_eq!(resolved.opacity(), 1.0);
    assert_eq!(resolved.depth(), 1);
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.iter().count(), 0, "a pattern is not a cg leaf");
    assert_eq!(stack.pattern(), Some(&resolved));
}

#[test]
fn construction_rejects_unusable_tile_mapping_and_opacity() {
    let items = Arc::new(rect_items(PaintStack::solid(CGColor::RED)));
    for (width, height) in [
        (0.0, 8.0),
        (-1.0, 8.0),
        (8.0, f32::INFINITY),
        (f32::NAN, 8.0),
    ] {
        assert_eq!(
            PatternPaint::new(
                width,
                height,
                AffineTransform::identity(),
                Arc::clone(&items),
                1.0,
            ),
            Err(PatternPaintError::InvalidTile)
        );
    }

    let singular = AffineTransform::from_acebdf(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(
        PatternPaint::new(8.0, 8.0, singular, Arc::clone(&items), 1.0),
        Err(PatternPaintError::InvalidTransform)
    );
    for opacity in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        assert_eq!(
            PatternPaint::new(
                8.0,
                8.0,
                AffineTransform::identity(),
                Arc::clone(&items),
                opacity,
            ),
            Err(PatternPaintError::InvalidOpacity)
        );
    }
}

#[test]
fn recursive_programs_stop_at_the_contract_bound() {
    let mut current =
        pattern(rect_items(PaintStack::solid(CGColor::RED))).expect("depth-one pattern");
    assert_eq!(current.depth(), 1);

    for expected_depth in 2..=MAX_PATTERN_DEPTH {
        current = pattern(rect_items(PaintStack::from_pattern(current)))
            .expect("within the recursive bound");
        assert_eq!(current.depth(), expected_depth);
    }

    assert_eq!(
        pattern(rect_items(PaintStack::from_pattern(current))),
        Err(PatternPaintError::TooDeep)
    );
}

#[test]
fn a_zero_opacity_program_normalizes_to_the_empty_stack() {
    let resolved = PatternPaint::new(
        8.0,
        8.0,
        AffineTransform::identity(),
        Arc::new(rect_items(PaintStack::solid(CGColor::RED))),
        0.0,
    )
    .expect("zero is a valid checked opacity");
    assert!(PaintStack::from_pattern(resolved).is_empty());
}
