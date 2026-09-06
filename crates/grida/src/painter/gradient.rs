use crate::{
    backends::skia::{sk_matrix, IntoSkia},
    cg::prelude::*,
};
use skia_safe::gradient::{Colors as GradientColors, Gradient, Interpolation};

fn build_gradient_stops(
    stops: &[GradientStop],
    opacity: f32,
) -> (Vec<skia_safe::Color4f>, Vec<f32>) {
    let mut colors = Vec::with_capacity(stops.len());
    let mut positions = Vec::with_capacity(stops.len());

    for stop in stops {
        // The legacy engine's gradients are authored in bytes; narrowing here
        // keeps this painter's byte staging (and its pixels) exactly as it
        // was before the stop leaf widened.
        let CGColor { r, g, b, a } = stop.color.to_rgba8();
        let alpha = (a as f32 * opacity).round().clamp(0.0, 255.0) as u8;
        colors.push(skia_safe::Color4f::from(skia_safe::Color::from_argb(
            alpha, r, g, b,
        )));
        positions.push(stop.offset);
    }

    (colors, positions)
}

fn make_gradient<'a>(
    colors: &'a [skia_safe::Color4f],
    positions: &'a [f32],
    tile_mode: skia_safe::TileMode,
) -> Gradient<'a> {
    Gradient::new(
        GradientColors::new(colors, Some(positions), tile_mode, None),
        Interpolation::default(),
    )
}

pub fn gradient_paint(paint: &GradientPaint, size: (f32, f32)) -> skia_safe::Paint {
    match paint {
        GradientPaint::Linear(gradient) => linear_gradient_paint(gradient, size),
        GradientPaint::Radial(gradient) => radial_gradient_paint(gradient, size),
        GradientPaint::Sweep(gradient) => sweep_gradient_paint(gradient, size),
        GradientPaint::Diamond(gradient) => diamond_gradient_paint(gradient, size),
    }
}

pub fn linear_gradient_paint(
    gradient: &LinearGradientPaint,
    (x, y): (f32, f32),
) -> skia_safe::Paint {
    let mut paint = skia_safe::Paint::default();
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let uv1 = gradient.xy1.to_uv();
    let uv2 = gradient.xy2.to_uv();
    let p1 = skia_safe::Point::new(uv1.u(), uv1.v());
    let p2 = skia_safe::Point::new(uv2.u(), uv2.v());

    let grad = make_gradient(&colors, &positions, gradient.tile_mode.into_skia());
    if let Some(shader) = skia_safe::shaders::linear_gradient((p1, p2), &grad, Some(&matrix)) {
        paint.set_shader(shader);
    }

    paint.set_alpha_f(gradient.opacity);
    paint.set_anti_alias(true);
    paint
}

pub fn linear_gradient_shader(
    gradient: &LinearGradientPaint,
    (x, y): (f32, f32),
) -> Option<skia_safe::Shader> {
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let start_uv = gradient.xy1.to_uv();
    let end_uv = gradient.xy2.to_uv();
    let start_point = skia_safe::Point::new(start_uv.u(), start_uv.v());
    let end_point = skia_safe::Point::new(end_uv.u(), end_uv.v());

    let grad = make_gradient(&colors, &positions, gradient.tile_mode.into_skia());
    let shader =
        skia_safe::shaders::linear_gradient((start_point, end_point), &grad, Some(&matrix))?;

    if gradient.opacity < 1.0 {
        let opacity_color =
            skia_safe::Color::from_argb((gradient.opacity * 255.0) as u8, 255, 255, 255);
        let opacity_shader = skia_safe::shaders::color(opacity_color);
        Some(skia_safe::shaders::blend(
            skia_safe::BlendMode::DstIn,
            shader,
            opacity_shader,
        ))
    } else {
        Some(shader)
    }
}

/// Build the legacy paint while preserving explicitly ordered radial circles.
///
/// # Panics
/// Panics if explicit circles are invalid or cannot construct a backend
/// shader. This legacy API has no typed error channel; it must not replace
/// unsupported explicit geometry with an unshaded paint.
pub fn radial_gradient_paint(
    gradient: &RadialGradientPaint,
    (x, y): (f32, f32),
) -> skia_safe::Paint {
    let mut paint = skia_safe::Paint::default();
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let grad = make_gradient(&colors, &positions, gradient.tile_mode.into_skia());
    if let Some(shader) = radial_geometry_shader(gradient.geometry, &grad, &matrix) {
        paint.set_shader(shader);
    }

    paint.set_alpha_f(gradient.opacity);
    paint.set_anti_alias(true);
    paint
}

/// Build the legacy radial shader with the same explicit-circle boundary as
/// [`radial_gradient_paint`].
///
/// # Panics
/// Panics if explicit circles are invalid or the backend rejects the shader.
pub fn radial_gradient_shader(
    gradient: &RadialGradientPaint,
    (x, y): (f32, f32),
) -> Option<skia_safe::Shader> {
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let grad = make_gradient(&colors, &positions, gradient.tile_mode.into_skia());
    let shader = radial_geometry_shader(gradient.geometry, &grad, &matrix)?;

    if gradient.opacity < 1.0 {
        let opacity_color =
            skia_safe::Color::from_argb((gradient.opacity * 255.0) as u8, 255, 255, 255);
        let opacity_shader = skia_safe::shaders::color(opacity_color);
        Some(skia_safe::shaders::blend(
            skia_safe::BlendMode::DstIn,
            shader,
            opacity_shader,
        ))
    } else {
        Some(shader)
    }
}

/// The legacy callers have no typed paint-error channel. Preserve their old
/// absent-geometry path, but fail loudly if explicit circles are invalid or
/// cannot be lowered; returning None would let callers paint a fallback.
fn radial_geometry_shader(
    geometry: Option<RadialGradientGeometry>,
    gradient: &Gradient<'_>,
    matrix: &skia_safe::Matrix,
) -> Option<skia_safe::Shader> {
    let Some(geometry) = geometry else {
        return skia_safe::shaders::radial_gradient(((0.5, 0.5), 0.5), gradient, Some(matrix));
    };
    for circle in [geometry.start, geometry.end] {
        assert!(
            circle.center.0.is_finite()
                && circle.center.1.is_finite()
                && circle.radius.is_finite()
                && circle.radius >= 0.0,
            "invalid explicit radial gradient circle at the legacy painter boundary"
        );
    }
    Some(
        skia_safe::shaders::two_point_conical_gradient(
            (geometry.start.center, geometry.start.radius),
            (geometry.end.center, geometry.end.radius),
            gradient,
            Some(matrix),
        )
        .expect("explicit radial gradient circles cannot be lowered by the legacy painter"),
    )
}

#[cfg(test)]
mod radial_circle_tests {
    use super::*;

    fn radial(geometry: Option<RadialGradientGeometry>) -> RadialGradientPaint {
        RadialGradientPaint {
            geometry,
            ..RadialGradientPaint::from_colors(vec![CGColor::RED, CGColor::BLUE])
        }
    }

    fn pixels(gradient: &RadialGradientPaint) -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).unwrap();
        surface.canvas().clear(skia_safe::Color::GREEN);
        surface.canvas().draw_rect(
            skia_safe::Rect::from_wh(64.0, 64.0),
            &radial_gradient_paint(gradient, (64.0, 64.0)),
        );
        let info = skia_safe::ImageInfo::new(
            (64, 64),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        );
        let mut bytes = vec![0; 64 * 64 * 4];
        assert!(surface.read_pixels(&info, &mut bytes, 64 * 4, (0, 0)));
        bytes
    }

    #[test]
    fn legacy_radial_painter_keeps_ordered_circles_and_the_unpainted_domain() {
        let geometry = RadialGradientGeometry {
            start: RadialGradientCircle {
                center: (-0.25, 0.5),
                radius: 0.125,
            },
            end: RadialGradientCircle {
                center: (0.5, 0.5),
                radius: 0.5,
            },
        };
        let plain = pixels(&radial(None));
        let painted = pixels(&radial(Some(geometry)));
        assert_ne!(plain, painted, "the new geometry cannot be ignored");
        assert!(
            painted
                .chunks_exact(4)
                .any(|pixel| pixel == [0, 255, 0, 255]),
            "no-solution exterior preserves the backdrop"
        );
        assert_ne!(
            painted,
            pixels(&radial(Some(RadialGradientGeometry {
                start: geometry.end,
                end: geometry.start
            }))),
            "circle order is semantic"
        );
        assert!(radial_gradient_shader(&radial(Some(geometry)), (64.0, 64.0)).is_some());
    }

    #[test]
    #[should_panic(
        expected = "invalid explicit radial gradient circle at the legacy painter boundary"
    )]
    fn legacy_radial_painter_cannot_turn_an_invalid_circle_into_a_fallback() {
        let geometry = RadialGradientGeometry {
            start: RadialGradientCircle {
                center: (f32::NAN, 0.5),
                radius: 0.0,
            },
            end: RadialGradientCircle {
                center: (0.5, 0.5),
                radius: 0.5,
            },
        };
        let _ = radial_gradient_paint(&radial(Some(geometry)), (64.0, 64.0));
    }
}

pub fn sweep_gradient_paint(gradient: &SweepGradientPaint, (x, y): (f32, f32)) -> skia_safe::Paint {
    let mut paint = skia_safe::Paint::default();
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let grad = make_gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    if let Some(shader) = skia_safe::shaders::sweep_gradient(
        (0.5_f32, 0.5_f32),
        (0.0_f32, 360.0_f32),
        &grad,
        Some(&matrix),
    ) {
        paint.set_shader(shader);
    }

    paint.set_alpha_f(gradient.opacity);
    paint.set_anti_alias(true);
    paint
}

pub fn sweep_gradient_shader(
    gradient: &SweepGradientPaint,
    (x, y): (f32, f32),
) -> Option<skia_safe::Shader> {
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let grad = make_gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    let shader = skia_safe::shaders::sweep_gradient(
        (0.5_f32, 0.5_f32),
        (0.0_f32, 360.0_f32),
        &grad,
        Some(&matrix),
    )?;

    if gradient.opacity < 1.0 {
        let opacity_color =
            skia_safe::Color::from_argb((gradient.opacity * 255.0) as u8, 255, 255, 255);
        let opacity_shader = skia_safe::shaders::color(opacity_color);
        Some(skia_safe::shaders::blend(
            skia_safe::BlendMode::DstIn,
            shader,
            opacity_shader,
        ))
    } else {
        Some(shader)
    }
}

pub fn diamond_gradient_paint(
    gradient: &DiamondGradientPaint,
    (x, y): (f32, f32),
) -> skia_safe::Paint {
    let mut paint = skia_safe::Paint::default();

    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let grad = make_gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    let base =
        skia_safe::shaders::linear_gradient(((0.0_f32, 0.0_f32), (1.0_f32, 0.0_f32)), &grad, None);

    if let Some(base_shader) = base {
        const SKSL: &str = r#"
            uniform shader gradient;
            half4 main(float2 coord) {
                float2 p = coord - float2(0.5, 0.5);
                float t = (abs(p.x) + abs(p.y)) * 2.0;
                t = clamp(t, 0.0, 1.0);
                return gradient.eval(float2(t, 0.0));
            }
        "#;

        if let Ok(effect) = skia_safe::RuntimeEffect::make_for_shader(SKSL, None) {
            let mut matrix = skia_safe::Matrix::scale((x, y));
            matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

            if let Some(shader) = effect.make_shader(
                skia_safe::Data::new_copy(&[]),
                &[base_shader.into()],
                Some(&matrix),
            ) {
                paint.set_shader(shader);
            }
        }
    }

    paint.set_alpha_f(gradient.opacity);
    paint.set_anti_alias(true);
    paint
}

pub fn diamond_gradient_shader(
    gradient: &DiamondGradientPaint,
    (x, y): (f32, f32),
) -> Option<skia_safe::Shader> {
    let (colors, positions) = build_gradient_stops(&gradient.stops, 1.0);

    let grad = make_gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    let base =
        skia_safe::shaders::linear_gradient(((0.0_f32, 0.0_f32), (1.0_f32, 0.0_f32)), &grad, None)?;

    const SKSL: &str = r#"
        uniform shader gradient;
        half4 main(float2 coord) {
            float2 p = coord - float2(0.5, 0.5);
            float t = (abs(p.x) + abs(p.y)) * 2.0;
            t = clamp(t, 0.0, 1.0);
            return gradient.eval(float2(t, 0.0));
        }
    "#;

    let effect = skia_safe::RuntimeEffect::make_for_shader(SKSL, None).ok()?;

    let mut matrix = skia_safe::Matrix::scale((x, y));
    matrix.pre_concat(&sk_matrix(gradient.transform.matrix));

    let shader = effect.make_shader(
        skia_safe::Data::new_copy(&[]),
        &[base.into()],
        Some(&matrix),
    )?;

    if gradient.opacity < 1.0 {
        let opacity_color =
            skia_safe::Color::from_argb((gradient.opacity * 255.0) as u8, 255, 255, 255);
        let opacity_shader = skia_safe::shaders::color(opacity_color);
        Some(skia_safe::shaders::blend(
            skia_safe::BlendMode::DstIn,
            shader,
            opacity_shader,
        ))
    } else {
        Some(shader)
    }
}
