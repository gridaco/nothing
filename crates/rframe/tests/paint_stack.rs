//! Contract tests for the admitted `cg` paint subset: solids, linear
//! gradients, and radial gradients, all normal-blend. Sweep and diamond
//! gradients, image paints, and non-normal blends are rejected at
//! construction — a producer refuses or declares them instead.

use cg::{
    BlendMode, CGColor, DiamondGradientPaint, GradientStop, ImagePaint, LinearGradientPaint, Paint,
    Paints, RadialGradientPaint, SolidPaint, SweepGradientPaint,
};
use rframe::{PaintAlphaFactor, PaintStack, PaintStackError, Stroke, StrokeCap, StrokeJoin};

fn ramp() -> Vec<GradientStop> {
    vec![
        GradientStop {
            offset: 0.0,
            color: CGColor::RED.into(),
        },
        GradientStop {
            offset: 1.0,
            color: CGColor::BLUE.into(),
        },
    ]
}

#[test]
fn invisible_paints_normalize_away_before_variant_validation() {
    let inactive_gradient = LinearGradientPaint {
        active: false,
        ..Default::default()
    };
    let mut transparent_blended = SolidPaint::new_color(CGColor::TRANSPARENT);
    transparent_blended.blend_mode = BlendMode::Multiply;

    let stack = PaintStack::try_from_paints(Paints::new([
        Paint::LinearGradient(inactive_gradient),
        Paint::Solid(transparent_blended),
        Paint::Solid(SolidPaint::new_color(CGColor::RED)),
    ]))
    .expect("nonvisual values do not enter the resolved stack");

    let colors = stack
        .iter()
        .map(|paint| match paint {
            Paint::Solid(solid) => solid.color,
            _ => panic!("only the visible solid survives"),
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, [CGColor::RED]);
    assert!(PaintStack::solid(CGColor::TRANSPARENT).is_empty());
}

#[test]
fn visible_gradients_are_admitted_with_their_fields_intact() {
    let linear = LinearGradientPaint {
        stops: ramp(),
        ..Default::default()
    };
    let radial = RadialGradientPaint {
        geometry: Some(cg::RadialGradientGeometry {
            start: cg::RadialGradientCircle {
                center: (-0.25, 0.375),
                radius: 0.75,
            },
            end: cg::RadialGradientCircle {
                center: (0.5, 0.625),
                radius: 0.0,
            },
        }),
        stops: ramp(),
        ..Default::default()
    };

    let stack = PaintStack::try_from_paints(Paints::new([
        Paint::LinearGradient(linear.clone()),
        Paint::RadialGradient(radial.clone()),
    ]))
    .expect("normal-blend gradients are inside the admitted scope");

    let paints = stack.iter().cloned().collect::<Vec<_>>();
    assert_eq!(
        paints,
        [Paint::LinearGradient(linear), Paint::RadialGradient(radial)]
    );
}

#[test]
fn visible_paints_outside_the_admitted_set_are_rejected_at_construction() {
    let beyond: [Paint; 3] = [
        Paint::SweepGradient(SweepGradientPaint {
            stops: ramp(),
            ..Default::default()
        }),
        Paint::DiamondGradient(DiamondGradientPaint {
            stops: ramp(),
            ..Default::default()
        }),
        Paint::Image(ImagePaint {
            active: true,
            image: cg::ResourceRef::RID("fixture://paint-stack".into()),
            quarter_turns: 0,
            alignement: cg::Alignment::CENTER,
            fit: cg::ImagePaintFit::Fit(math2::box_fit::BoxFit::Cover),
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            filters: Default::default(),
        }),
    ];
    for paint in beyond {
        assert_eq!(
            PaintStack::try_from_paints(Paints::new([
                Paint::Solid(SolidPaint::new_color(CGColor::GREEN)),
                paint,
            ])),
            Err(PaintStackError { index: 1 })
        );
    }
}

#[test]
fn a_visible_nonnormal_blend_is_rejected_even_on_an_admitted_variant() {
    let mut blended = LinearGradientPaint {
        stops: ramp(),
        ..Default::default()
    };
    blended.blend_mode = BlendMode::Multiply;
    assert_eq!(
        PaintStack::try_from_paints(Paints::new([Paint::LinearGradient(blended)])),
        Err(PaintStackError { index: 0 })
    );
}

#[test]
fn alpha_factor_admits_exactly_the_closed_unit_interval() {
    for admitted in [0.0, -0.0, f32::MIN_POSITIVE, 0.5, 1.0] {
        let factor = PaintAlphaFactor::new(admitted).expect("a finite unit factor");
        assert_eq!(factor.get(), if admitted == 0.0 { 0.0 } else { admitted });
    }
    for refused in [-0.25, 1.25, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(
            PaintAlphaFactor::new(refused).is_err(),
            "{refused} is not a finite unit factor"
        );
    }
}

#[test]
fn alpha_factor_is_independent_of_intrinsic_alpha_and_paint_order() {
    let linear = LinearGradientPaint {
        stops: ramp(),
        opacity: 0.25,
        ..Default::default()
    };
    let radial = RadialGradientPaint {
        stops: ramp(),
        opacity: 0.75,
        ..Default::default()
    };
    let ordinary = PaintStack::try_from_paints(Paints::new([
        Paint::LinearGradient(linear.clone()),
        Paint::RadialGradient(radial.clone()),
    ]))
    .expect("admitted gradients");
    let factored = ordinary
        .clone()
        .with_alpha_factor(PaintAlphaFactor::new(0.5).expect("half alpha"));

    assert_eq!(ordinary.alpha_factor(), PaintAlphaFactor::IDENTITY);
    assert_eq!(factored.alpha_factor().get(), 0.5);
    assert_ne!(factored, ordinary, "the second alpha operation is material");
    assert_eq!(
        factored.iter().cloned().collect::<Vec<_>>(),
        [Paint::LinearGradient(linear), Paint::RadialGradient(radial)],
        "the factor neither folds into intrinsic opacity nor changes paint order"
    );
}

#[test]
fn zero_and_empty_stacks_canonicalize_to_empty_identity() {
    let half = PaintAlphaFactor::new(0.5).expect("half alpha");
    let zero = PaintAlphaFactor::new(0.0).expect("zero is checked before normalization");

    let empty = PaintStack::empty().with_alpha_factor(half);
    assert!(empty.is_empty());
    assert_eq!(empty.alpha_factor(), PaintAlphaFactor::IDENTITY);

    let zeroed = PaintStack::solid(CGColor::RED).with_alpha_factor(zero);
    assert!(zeroed.is_empty());
    assert_eq!(zeroed.alpha_factor(), PaintAlphaFactor::IDENTITY);
}

#[test]
fn stroke_carries_the_same_factored_stack_without_a_second_field() {
    let factored =
        PaintStack::try_from_paints(Paints::new([Paint::LinearGradient(LinearGradientPaint {
            stops: ramp(),
            opacity: 0.25,
            ..Default::default()
        })]))
        .expect("admitted gradient")
        .with_alpha_factor(PaintAlphaFactor::new(0.5).expect("half alpha"));
    let stroke = Stroke::new(
        factored.clone(),
        8.0,
        StrokeCap::Round,
        StrokeJoin::Round,
        4.0,
    )
    .expect("valid stroke")
    .expect("visible stroke");

    assert_eq!(stroke.paints(), &factored);
}
