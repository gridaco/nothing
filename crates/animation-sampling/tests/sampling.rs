use animation_sampling::{
    CubicBezier, CubicBezierError, CubicControl, Easing, FillMode, KeyframeOffset,
    KeyframeOffsetError, SampleTime, ScalarCurve, ScalarCurveError, ScalarKeyframe, ScalarSegment,
    Timing, TimingError,
};
use std::collections::BTreeMap;

fn offset(numerator: u64, denominator: u64) -> KeyframeOffset {
    KeyframeOffset::new(numerator, denominator).unwrap()
}

fn keyframe(numerator: u64, denominator: u64, value: f32) -> ScalarKeyframe {
    ScalarKeyframe::new(offset(numerator, denominator), value).unwrap()
}

fn segment(easing: Easing, numerator: u64, denominator: u64, value: f32) -> ScalarSegment {
    ScalarSegment::new(easing, keyframe(numerator, denominator, value))
}

fn sample_bits(curve: &ScalarCurve, timing: Timing, fill: FillMode, time: i64) -> Option<u32> {
    timing
        .contribution(SampleTime::from_nanoseconds(time), fill)
        .map(|contribution| curve.sample(contribution).value().to_bits())
}

#[test]
fn sample_time_and_timing_are_checked_integer_nanoseconds() {
    let negative = SampleTime::from_nanoseconds(-17);
    assert_eq!(negative.nanoseconds(), -17);
    assert_eq!(
        negative.checked_add_nanoseconds(20),
        Some(SampleTime::from_nanoseconds(3))
    );
    assert_eq!(
        SampleTime::from_nanoseconds(i64::MAX).checked_add_nanoseconds(1),
        None
    );
    assert_eq!(
        SampleTime::from_nanoseconds(i64::MIN).checked_sub_nanoseconds(1),
        None
    );
    assert!(SampleTime::try_from(i128::from(i64::MAX) + 1).is_err());

    assert_eq!(Timing::new(0, 0, 1), Err(TimingError::ZeroDuration));
    assert_eq!(Timing::new(0, 1, 0), Err(TimingError::ZeroRepeatCount));
    assert!(matches!(
        Timing::new(i64::MAX, 1, 1),
        Err(TimingError::ActiveEndOverflow { .. })
    ));

    let timing = Timing::new(20, 7, 3).unwrap();
    assert_eq!(timing.begin(), SampleTime::from_nanoseconds(20));
    assert_eq!(timing.duration_nanoseconds(), 7);
    assert_eq!(timing.repeat_count(), 3);
    assert_eq!(timing.active_end(), SampleTime::from_nanoseconds(41));

    let negative = Timing::new(-20, 7, 3).unwrap();
    assert_eq!(negative.begin(), SampleTime::from_nanoseconds(-20));
    assert_eq!(negative.active_end(), SampleTime::from_nanoseconds(1));

    let full_signed_span = Timing::new(i64::MIN, u64::MAX, 1).unwrap();
    assert_eq!(
        full_signed_span.active_end(),
        SampleTime::from_nanoseconds(i64::MAX)
    );
    assert!(matches!(
        Timing::new(i64::MIN, u64::MAX, 2),
        Err(TimingError::ActiveEndOverflow { .. })
    ));
}

#[test]
fn contribution_boundaries_encode_absence_active_repeats_and_freeze() {
    let timing = Timing::new(-2, 4, 3).unwrap();
    assert_eq!(
        timing.contribution(SampleTime::from_nanoseconds(-3), FillMode::Freeze),
        None
    );

    let begin = timing
        .contribution(SampleTime::from_nanoseconds(-2), FillMode::Remove)
        .unwrap();
    assert_eq!(begin.repeat_index(), 0);
    assert!(!begin.is_terminal());

    let repeat = timing
        .contribution(SampleTime::from_nanoseconds(2), FillMode::Remove)
        .unwrap();
    assert_eq!(repeat.repeat_index(), 1);
    assert!(!repeat.is_terminal());

    assert_eq!(
        timing.contribution(SampleTime::from_nanoseconds(10), FillMode::Remove),
        None
    );
    let frozen = timing
        .contribution(SampleTime::from_nanoseconds(10), FillMode::Freeze)
        .unwrap();
    assert_eq!(frozen.repeat_index(), 2);
    assert!(frozen.is_terminal());
}

#[test]
fn offsets_are_exact_reduced_and_curve_structure_is_canonical() {
    let half = KeyframeOffset::new(2, 4).unwrap();
    assert_eq!(half.numerator(), 1);
    assert_eq!(half.denominator(), 2);
    assert_eq!(
        KeyframeOffset::new(0, u64::MAX).unwrap(),
        KeyframeOffset::ZERO
    );
    assert_eq!(
        KeyframeOffset::new(u64::MAX, u64::MAX).unwrap(),
        KeyframeOffset::ONE
    );
    assert_eq!(
        KeyframeOffset::new(1, 0),
        Err(KeyframeOffsetError::ZeroDenominator { numerator: 1 })
    );
    assert_eq!(
        KeyframeOffset::new(3, 2),
        Err(KeyframeOffsetError::OutsideUnitInterval {
            numerator: 3,
            denominator: 2,
        })
    );

    assert!(matches!(
        ScalarCurve::new(
            ScalarKeyframe::new(half, 0.0).unwrap(),
            vec![segment(Easing::Linear, 1, 1, 1.0)]
        ),
        Err(ScalarCurveError::FirstOffsetMustBeZero { actual }) if actual == half
    ));
    assert!(matches!(
        ScalarCurve::new(
            keyframe(0, 1, 0.0),
            vec![segment(Easing::Linear, 1, 2, 1.0)]
        ),
        Err(ScalarCurveError::LastOffsetMustBeOne { actual }) if actual == half
    ));
    assert!(matches!(
        ScalarCurve::new(
            keyframe(0, 1, 0.0),
            vec![
                segment(Easing::Linear, 1, 2, 1.0),
                segment(Easing::Linear, 2, 4, 2.0),
                segment(Easing::Linear, 1, 1, 3.0),
            ]
        ),
        Err(ScalarCurveError::OffsetsNotStrictlyIncreasing {
            previous_index: 1,
            current_index: 2,
            previous,
            current,
        }) if previous == half && current == half
    ));

    let constant = ScalarCurve::new(ScalarKeyframe::new(half, 7.0).unwrap(), vec![]).unwrap();
    assert_eq!(constant, ScalarCurve::constant(7.0).unwrap());
    assert_eq!(constant.first().offset(), KeyframeOffset::ZERO);
    assert_eq!(constant.keyframe_count(), 1);
}

#[test]
fn non_finite_values_are_rejected_before_a_scalar_curve_can_exist() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            ScalarKeyframe::new(KeyframeOffset::ZERO, value),
            Err(ScalarCurveError::NonFiniteValue { value_bits })
                if value_bits == value.to_bits()
        ));
        assert!(matches!(
            ScalarCurve::constant(value),
            Err(ScalarCurveError::NonFiniteValue { value_bits })
                if value_bits == value.to_bits()
        ));
        assert!(matches!(
            ScalarCurve::linear(0.0, value),
            Err(ScalarCurveError::NonFiniteValue { value_bits })
                if value_bits == value.to_bits()
        ));
        assert!(matches!(
            ScalarCurve::linear(value, 0.0),
            Err(ScalarCurveError::NonFiniteValue { value_bits })
                if value_bits == value.to_bits()
        ));
    }

    assert!(ScalarCurve::linear(-f32::MAX, f32::MAX).is_ok());
    assert!(ScalarCurve::constant(-0.0).is_ok());
}

#[test]
fn scalar_sampling_preserves_endpoints_repeats_offsets_and_zero_signs() {
    let curve = ScalarCurve::new(
        keyframe(0, 1, -0.0),
        vec![
            segment(Easing::Linear, 1, 3, 3.0),
            segment(Easing::Linear, 1, 1, 12.0),
        ],
    )
    .unwrap();
    let timing = Timing::new(0, 12, 2).unwrap();

    assert_eq!(
        sample_bits(&curve, timing, FillMode::Freeze, 0),
        Some((-0.0_f32).to_bits())
    );
    assert_eq!(
        sample_bits(&curve, timing, FillMode::Freeze, 4),
        Some(3.0_f32.to_bits())
    );
    assert_eq!(
        sample_bits(&curve, timing, FillMode::Freeze, 12),
        Some((-0.0_f32).to_bits()),
        "an exact repeat boundary restarts at the first keyframe"
    );
    assert_eq!(sample_bits(&curve, timing, FillMode::Remove, 24), None);
    assert_eq!(
        sample_bits(&curve, timing, FillMode::Freeze, 24),
        Some(12.0_f32.to_bits())
    );
}

#[test]
fn scalar_interpolation_rounds_once_to_binary32_ties_even() {
    let midpoint = |from, to| {
        sample_bits(
            &ScalarCurve::linear(from, to).unwrap(),
            Timing::new(0, 2, 1).unwrap(),
            FillMode::Freeze,
            1,
        )
        .unwrap()
    };

    let even = f32::from_bits(0x3f00_0000);
    let odd = f32::from_bits(0x3f00_0001);
    let next_even = f32::from_bits(0x3f00_0002);
    assert_eq!(midpoint(even, odd), even.to_bits());
    assert_eq!(midpoint(odd, next_even), next_even.to_bits());

    let min_subnormal = f32::from_bits(1);
    let even_subnormal = f32::from_bits(2);
    assert_eq!(midpoint(0.0, min_subnormal), 0.0_f32.to_bits());
    assert_eq!(
        midpoint(min_subnormal, even_subnormal),
        even_subnormal.to_bits()
    );
    assert_eq!(
        midpoint(f32::from_bits(0x007f_ffff), f32::from_bits(0x0080_0000)),
        0x0080_0000
    );
    assert_eq!(
        midpoint(f32::from_bits(0xbf80_0000), f32::from_bits(0xbf80_0001)),
        0xbf80_0000
    );
    assert_eq!(midpoint(-min_subnormal, -0.0), (-0.0_f32).to_bits());
}

#[test]
fn cubic_easing_is_checked_and_matches_pinned_exact_samples() {
    assert_eq!(
        CubicBezier::new(f32::NAN, 0.0, 1.0, 1.0),
        Err(CubicBezierError::NotFinite {
            control: CubicControl::X1,
        })
    );
    assert_eq!(
        CubicBezier::new(0.0, 0.0, 1.1, 1.0),
        Err(CubicBezierError::XOutsideUnitInterval {
            control: CubicControl::X2,
        })
    );

    let timing = Timing::new(0, 14, 1).unwrap();
    let identity = ScalarCurve::new(
        keyframe(0, 1, 0.0),
        vec![segment(
            Easing::CubicBezier(CubicBezier::new(0.25, 0.25, 0.75, 0.75).unwrap()),
            1,
            1,
            1.0,
        )],
    )
    .unwrap();
    let linear = ScalarCurve::linear(0.0, 1.0).unwrap();
    for time in 0..=14 {
        assert_eq!(
            sample_bits(&identity, timing, FillMode::Freeze, time),
            sample_bits(&linear, timing, FillMode::Freeze, time)
        );
    }

    let exact_hit = ScalarCurve::new(
        keyframe(0, 1, 0.0),
        vec![segment(
            Easing::CubicBezier(CubicBezier::new(0.0, 1.0, 1.0, 1.0).unwrap()),
            1,
            1,
            1.0,
        )],
    )
    .unwrap();
    assert_eq!(
        sample_bits(
            &exact_hit,
            Timing::new(0, 2, 1).unwrap(),
            FillMode::Freeze,
            1
        ),
        Some(0.875_f32.to_bits())
    );

    let css_ease = ScalarCurve::new(
        keyframe(0, 1, 0.0),
        vec![segment(
            Easing::CubicBezier(CubicBezier::new(0.25, 0.1, 0.25, 1.0).unwrap()),
            1,
            1,
            1.0,
        )],
    )
    .unwrap();
    assert_eq!(
        sample_bits(
            &css_ease,
            Timing::new(0, 3, 1).unwrap(),
            FillMode::Freeze,
            1
        ),
        Some(0x3f13_6bb6)
    );
}

#[test]
fn sampling_is_stateless_and_independent_of_query_order() {
    let curve = ScalarCurve::new(
        keyframe(0, 1, -4.0),
        vec![
            segment(Easing::Linear, 2, 5, 8.0),
            segment(
                Easing::CubicBezier(CubicBezier::new(0.25, 0.1, 0.25, 1.0).unwrap()),
                1,
                1,
                20.0,
            ),
        ],
    )
    .unwrap();
    let timing = Timing::new(-7, 19, 3).unwrap();
    let ordered = (-10..=55)
        .map(|time| (time, sample_bits(&curve, timing, FillMode::Freeze, time)))
        .collect::<BTreeMap<_, _>>();

    let shuffled = [
        55, -10, 4, 17, -7, 36, 0, 12, 49, -3, 18, 11, 35, 54, 1, 5, 20, 9,
    ];
    for time in shuffled {
        assert_eq!(
            sample_bits(&curve, timing, FillMode::Freeze, time),
            ordered[&time]
        );
    }
}
