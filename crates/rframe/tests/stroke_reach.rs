//! Producer-only contract laws for finite stroke reach.

use cg::CGColor;
use rframe::{PaintStack, Stroke, StrokeCap, StrokeJoin};

fn black() -> PaintStack {
    PaintStack::solid(CGColor::from_rgb(0, 0, 0))
}

/// Reach is derived from the self-contained stroke and takes no spatial or
/// execution context.
#[test]
fn outset_is_a_stroke_only_finite_derivation() {
    let derive: fn(&Stroke) -> f64 = Stroke::outset;
    let stroke = Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Round, 4.0)
        .expect("every member is valid")
        .expect("the stroke paints");

    assert!(derive(&stroke).is_finite());
}

/// Aggregate arithmetic cannot reject independently representable facts.
#[test]
fn maximum_width_with_a_default_miter_is_carried_exactly() {
    let paints = black();
    let stroke = Stroke::new(
        paints.clone(),
        f32::MAX,
        StrokeCap::Butt,
        StrokeJoin::Miter,
        4.0,
    )
    .expect("every member is valid")
    .expect("the stroke paints");

    assert_eq!(stroke.paints(), &paints);
    assert_eq!(stroke.width(), f32::MAX);
    assert_eq!(stroke.cap(), StrokeCap::Butt);
    assert_eq!(stroke.join(), StrokeJoin::Miter);
    assert_eq!(stroke.miter_limit(), 4.0);
    assert_eq!(stroke.dash_intervals(), None);

    let expected = (f64::from(f32::MAX) / 2.0) * 4.0;
    assert_eq!(stroke.outset(), expected);
    assert!(stroke.outset().is_finite());
}

/// The square-cap irrational bound is represented outward, never inward.
#[test]
fn maximum_width_with_a_square_cap_has_a_finite_conservative_outset() {
    let stroke = Stroke::new(black(), f32::MAX, StrokeCap::Square, StrokeJoin::Round, 4.0)
        .expect("every member is valid")
        .expect("the stroke paints");

    assert_eq!(stroke.width(), f32::MAX);
    assert_eq!(stroke.cap(), StrokeCap::Square);

    let nearest = (f64::from(f32::MAX) / 2.0) * std::f64::consts::SQRT_2;
    assert_eq!(stroke.outset(), nearest.next_up());
    assert!(stroke.outset().is_finite());
}

/// Every pair of finite `f32` members has ample exponent range in `f64`.
#[test]
fn maximum_width_and_miter_limit_have_a_finite_exact_outset() {
    let stroke = Stroke::new(
        black(),
        f32::MAX,
        StrokeCap::Butt,
        StrokeJoin::Miter,
        f32::MAX,
    )
    .expect("every member is valid")
    .expect("the stroke paints");

    let expected = (f64::from(f32::MAX) / 2.0) * f64::from(f32::MAX);
    assert_eq!(stroke.outset(), expected);
    assert!(stroke.outset().is_finite());
}

/// Widening happens before arithmetic, so the least positive width retains a
/// positive half-width instead of underflowing in the carried representation.
#[test]
fn the_least_positive_width_keeps_its_derived_reach() {
    let width = f32::from_bits(1);
    let stroke = Stroke::new(black(), width, StrokeCap::Butt, StrokeJoin::Round, 4.0)
        .expect("every member is valid")
        .expect("the stroke paints");

    assert_eq!(stroke.width(), width);
    assert_eq!(stroke.outset(), f64::from(width) / 2.0);
    assert!(stroke.outset() > 0.0);
}
