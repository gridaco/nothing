//! Contract tests for the admitted `cg` paint subset: solids, linear
//! gradients, and radial gradients, all normal-blend. Sweep and diamond
//! gradients, image paints, and non-normal blends are rejected at
//! construction — a producer refuses or declares them instead.

use cg::{
    BlendMode, CGColor, DiamondGradientPaint, GradientStop, ImagePaint, LinearGradientPaint, Paint,
    Paints, RadialGradientPaint, SolidPaint, SweepGradientPaint,
};
use rframe::{PaintStack, PaintStackError};

fn ramp() -> Vec<GradientStop> {
    vec![
        GradientStop {
            offset: 0.0,
            color: CGColor::RED,
        },
        GradientStop {
            offset: 1.0,
            color: CGColor::BLUE,
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
