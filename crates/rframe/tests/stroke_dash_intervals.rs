//! Producer-only contract tests for resolved stroke dash intervals.

use cg::CGColor;
use rframe::{
    PaintStack, Stroke, StrokeCap, StrokeDashIntervals, StrokeDashIntervalsError, StrokeJoin,
};

fn black() -> PaintStack {
    PaintStack::solid(CGColor::from_rgb(0, 0, 0))
}

fn checked(intervals: Vec<f32>) -> StrokeDashIntervals {
    StrokeDashIntervals::new(intervals)
        .expect("valid interval cycle")
        .expect("present interval cycle")
}

fn dashed(cap: StrokeCap, intervals: StrokeDashIntervals) -> Option<Stroke> {
    Stroke::new_with_dash_intervals(black(), 8.0, cap, StrokeJoin::Miter, 4.0, Some(intervals))
        .expect("valid stroke")
}

/// Zero distances are resolved facts, not invalid sentinels: a later positive
/// member makes the whole cycle present.
#[test]
fn an_even_positive_cycle_including_zero_intervals_is_checked_and_readable() {
    let intervals = checked(vec![0.0, 8.0, 3.0, 0.0]);
    assert_eq!(intervals.as_slice(), &[0.0, 8.0, 3.0, 0.0]);

    let stroke = dashed(StrokeCap::Round, intervals);
    let exposed: Option<&StrokeDashIntervals> = stroke
        .as_ref()
        .expect("round caps paint the zero-length dash")
        .dash_intervals();
    assert_eq!(
        exposed.expect("dashed stroke").as_slice(),
        &[0.0, 8.0, 3.0, 0.0]
    );
}

/// A consumer receives complete painted-gap pairs and never has to repeat an
/// odd sequence or guess whether an empty present value means solid.
#[test]
fn empty_and_odd_cycles_refuse_by_name() {
    assert_eq!(
        StrokeDashIntervals::new(vec![]),
        Err(StrokeDashIntervalsError::Empty)
    );
    assert_eq!(
        StrokeDashIntervals::new(vec![4.0]),
        Err(StrokeDashIntervalsError::OddIntervalCount { count: 1 })
    );
    assert_eq!(
        StrokeDashIntervals::new(vec![4.0, 2.0, 1.0]),
        Err(StrokeDashIntervalsError::OddIntervalCount { count: 3 })
    );
}

/// Every member is a usable local-space distance before a cycle can cross the
/// contract; the error points at the member the producer failed to resolve.
#[test]
fn negative_and_non_finite_intervals_refuse_by_name() {
    assert_eq!(
        StrokeDashIntervals::new(vec![4.0, -1.0]),
        Err(StrokeDashIntervalsError::NegativeInterval { index: 1 })
    );
    assert_eq!(
        StrokeDashIntervals::new(vec![f32::NAN, 1.0]),
        Err(StrokeDashIntervalsError::NonFiniteInterval { index: 0 })
    );
    assert_eq!(
        StrokeDashIntervals::new(vec![1.0, f32::INFINITY]),
        Err(StrokeDashIntervalsError::NonFiniteInterval { index: 1 })
    );
    assert_eq!(
        StrokeDashIntervals::new(vec![1.0, f32::NEG_INFINITY]),
        Err(StrokeDashIntervalsError::NonFiniteInterval { index: 1 })
    );
}

/// Individually valid intervals can still overflow the repeating cycle. The
/// contract refuses that before a consumer can silently erase the stroke.
#[test]
fn a_cycle_whose_f32_sum_is_not_finite_refuses() {
    assert_eq!(
        StrokeDashIntervals::new(vec![f32::MAX, f32::MAX]),
        Err(StrokeDashIntervalsError::UnrepresentableCycleLength)
    );
}

/// Dash absence is the sole solid spelling; an all-zero authored result does
/// not create an empty present pattern that downstreams would reinterpret.
#[test]
fn an_all_zero_cycle_normalizes_to_solid_absence() {
    assert_eq!(
        StrokeDashIntervals::new(vec![0.0, -0.0, 0.0, 0.0]),
        Ok(None)
    );

    let solid = Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Miter, 4.0)
        .expect("valid stroke")
        .expect("visible stroke");
    assert_eq!(solid.dash_intervals(), None);
}

/// Zero-length painted slots are invisible only with butt caps. Round and
/// square caps paint at those slots, so their otherwise identical strokes
/// remain present.
#[test]
fn all_zero_painted_intervals_normalize_according_to_cap() {
    let intervals = checked(vec![0.0, 8.0, 0.0, 2.0]);
    assert_eq!(dashed(StrokeCap::Butt, intervals.clone()), None);
    assert!(dashed(StrokeCap::Round, intervals.clone()).is_some());
    assert!(dashed(StrokeCap::Square, intervals).is_some());

    let one_visible_dash = checked(vec![0.0, 8.0, 2.0, 8.0]);
    assert!(
        dashed(StrokeCap::Butt, one_visible_dash).is_some(),
        "one positive painted interval makes a butt-capped stroke visible"
    );
}

/// A dash cycle changes along-path coverage, not the conservative reach away
/// from the geometry, so damage and culling keep the existing outset law.
#[test]
fn dash_intervals_change_neither_outset_nor_other_invisible_normalization() {
    let solid = Stroke::new(black(), 8.0, StrokeCap::Square, StrokeJoin::Miter, 4.0)
        .expect("valid stroke")
        .expect("visible stroke");
    let intervals = checked(vec![4.0, 2.0]);
    let dashed = Stroke::new_with_dash_intervals(
        black(),
        8.0,
        StrokeCap::Square,
        StrokeJoin::Miter,
        4.0,
        Some(intervals.clone()),
    )
    .expect("valid stroke")
    .expect("visible stroke");
    assert_eq!(dashed.outset(), solid.outset());

    assert_eq!(
        Stroke::new_with_dash_intervals(
            black(),
            0.0,
            StrokeCap::Square,
            StrokeJoin::Miter,
            4.0,
            Some(intervals.clone()),
        ),
        Ok(None),
        "zero width still paints nothing"
    );
    assert_eq!(
        Stroke::new_with_dash_intervals(
            PaintStack::empty(),
            8.0,
            StrokeCap::Square,
            StrokeJoin::Miter,
            4.0,
            Some(intervals),
        ),
        Ok(None),
        "no paint still paints nothing"
    );
}
