//! Contract tests for the admitted `cg` paint subset.

use cg::{BlendMode, CGColor, LinearGradientPaint, Paint, Paints, SolidPaint};
use rframe::{SolidPaintStack, SolidPaintStackError};

#[test]
fn invisible_paints_normalize_away_before_variant_validation() {
    let inactive_gradient = LinearGradientPaint {
        active: false,
        ..Default::default()
    };
    let mut transparent_blended = SolidPaint::new_color(CGColor::TRANSPARENT);
    transparent_blended.blend_mode = BlendMode::Multiply;

    let stack = SolidPaintStack::try_from_paints(Paints::new([
        Paint::LinearGradient(inactive_gradient),
        Paint::Solid(transparent_blended),
        Paint::Solid(SolidPaint::new_color(CGColor::RED)),
    ]))
    .expect("nonvisual values do not enter the resolved stack");

    let colors = stack.iter().map(|solid| solid.color).collect::<Vec<_>>();
    assert_eq!(colors, [CGColor::RED]);
    assert!(SolidPaintStack::solid(CGColor::TRANSPARENT).is_empty());
}

#[test]
fn visible_nonordinary_paint_is_rejected_at_construction() {
    assert_eq!(
        SolidPaintStack::try_from_paints(Paints::new([
            Paint::Solid(SolidPaint::new_color(CGColor::GREEN)),
            Paint::LinearGradient(LinearGradientPaint::default()),
        ])),
        Err(SolidPaintStackError { index: 1 })
    );
}
