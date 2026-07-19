//! IRSVG paint attributes → runtime [`Paint`] projection (v1 adapter side).
//!
//! The IR types in [`crate::cg::svg`] are spec-faithful SVG values; this
//! module is the bridge into the runtime paint model: it bakes SVG
//! fill-/stroke-opacity into each paint entry and normalizes gradient
//! transforms into the painter's UV `[0,1]` convention. It lives with the
//! import adapter — not on the IR — so the vocabulary module stays free of
//! runtime-paint policy (see `tests/svg_import_architecture.rs`).
//!
//! Free functions rather than inherent/`From` impls: after the planned
//! extraction of `cg/` into its own crate, impls over those types from this
//! crate would violate the orphan rule.

use crate::cg::prelude::*;
use crate::cg::svg::{
    SVGFillAttributes, SVGGradientSpreadMethod, SVGLinearGradientPaint, SVGPaint,
    SVGRadialGradientPaint, SVGStrokeAttributes,
};
use math2::transform::AffineTransform;

/// Project an IR fill into a runtime [`Paint`], baking `fill-opacity`.
pub(crate) fn fill_paint(fill: &SVGFillAttributes, bounds: Option<(f32, f32, f32, f32)>) -> Paint {
    svg_paint_with_opacity(&fill.paint, fill.fill_opacity, bounds)
}

/// Project an IR stroke into a runtime [`Paint`], baking `stroke-opacity`.
pub(crate) fn stroke_paint(
    stroke: &SVGStrokeAttributes,
    bounds: Option<(f32, f32, f32, f32)>,
) -> Paint {
    svg_paint_with_opacity(&stroke.paint, stroke.stroke_opacity, bounds)
}

fn tile_mode_from_spread(spread_method: SVGGradientSpreadMethod) -> TileMode {
    match spread_method {
        SVGGradientSpreadMethod::Pad => TileMode::Clamp,
        SVGGradientSpreadMethod::Reflect => TileMode::Mirror,
        SVGGradientSpreadMethod::Repeat => TileMode::Repeated,
    }
}

/// Helper that converts SVG paint + separate opacity fields into the runtime
/// `Paint` model (which bakes opacity into each paint entry).
///
/// SVG allows opacity on fill/stroke independently of paints; our runtime stores
/// opacity within each `Paint`. This function is the bridging layer.
fn svg_paint_with_opacity(
    paint: &SVGPaint,
    opacity: f32,
    bounds: Option<(f32, f32, f32, f32)>,
) -> Paint {
    match paint {
        SVGPaint::Solid(solid) => Paint::Solid(SolidPaint {
            active: true,
            color: solid.color.with_multiplier(opacity),
            blend_mode: BlendMode::Normal,
        }),
        SVGPaint::LinearGradient(linear) => svg_linear_gradient_to_paint(linear, opacity, bounds),
        SVGPaint::RadialGradient(radial) => svg_radial_gradient_to_paint(radial, opacity, bounds),
    }
}

fn svg_linear_gradient_to_paint(
    linear: &SVGLinearGradientPaint,
    opacity: f32,
    bounds: Option<(f32, f32, f32, f32)>,
) -> Paint {
    let xy1 = Alignment::from_uv(Uv(linear.x1, linear.y1));
    let xy2 = Alignment::from_uv(Uv(linear.x2, linear.y2));
    let mut transform = AffineTransform::from(linear.transform);
    normalize_gradient_transform(&mut transform, bounds);

    Paint::LinearGradient(LinearGradientPaint {
        active: true,
        xy1,
        xy2,
        tile_mode: tile_mode_from_spread(linear.spread_method),
        transform,
        stops: linear.stops.clone(),
        opacity,
        blend_mode: BlendMode::Normal,
    })
}

fn svg_radial_gradient_to_paint(
    radial: &SVGRadialGradientPaint,
    opacity: f32,
    bounds: Option<(f32, f32, f32, f32)>,
) -> Paint {
    if (radial.fx - radial.cx).abs() > f32::EPSILON || (radial.fy - radial.cy).abs() > f32::EPSILON
    {
        return unsupported_svg_gradient("radial focal point (fx/fy)");
    }

    let mut gradient_transform = AffineTransform::from(radial.transform);
    normalize_gradient_transform(&mut gradient_transform, bounds);
    let alignment = radial_gradient_alignment_transform((radial.cx, radial.cy), radial.r);

    Paint::RadialGradient(RadialGradientPaint {
        active: true,
        transform: gradient_transform.compose(&alignment),
        stops: radial.stops.clone(),
        opacity,
        blend_mode: BlendMode::Normal,
        tile_mode: tile_mode_from_spread(radial.spread_method),
    })
}

fn unsupported_svg_gradient(reason: &str) -> Paint {
    // TODO: Implement support for unsupported SVG gradient features:
    // - radial gradient focal points (fx/fy different from cx/cy)
    // For now, we ignore these gradients by returning an inactive paint.
    let _ = reason;
    Paint::Solid(SolidPaint {
        active: false,
        color: CGColor::TRANSPARENT,
        blend_mode: BlendMode::Normal,
    })
}

fn radial_gradient_alignment_transform(center: (f32, f32), radius: f32) -> AffineTransform {
    if radius <= f32::EPSILON {
        return AffineTransform::identity();
    }

    let translate = translation(center.0, center.1);
    let scale = scale(radius * 2.0, radius * 2.0);
    let baseline = translation(-0.5, -0.5);

    translate.compose(&scale).compose(&baseline)
}

/// Undo the `from_bbox` transform that usvg bakes into gradient transforms
/// via `to_user_coordinates`.
///
/// usvg resolves `objectBoundingBox` gradients by post-concatenating
/// `from_bbox(rect)` = `[width, 0, x, 0, height, y]` onto the
/// `gradientTransform`. Our paint model expects the gradient transform
/// in UV [0,1] space (the painter applies `scale(shape_w, shape_h)`
/// at render time), so we multiply by `from_bbox_inv` to undo it:
///
///   from_bbox_inv = [1/w, 0, -x/w, 0, 1/h, -y/h]
///
/// The previous implementation only divided by (w, h), ignoring the
/// bbox origin (x, y). This caused gradients to be offset when the
/// shape had a non-zero position.
fn normalize_gradient_transform(
    transform: &mut AffineTransform,
    bounds: Option<(f32, f32, f32, f32)>,
) {
    if let Some((x, y, width, height)) = bounds {
        if width > f32::EPSILON && height > f32::EPSILON {
            // Apply from_bbox_inv = [1/w, 0, -x/w, 0, 1/h, -y/h]
            // as a left-multiply: result = from_bbox_inv * transform
            let inv_w = 1.0 / width;
            let inv_h = 1.0 / height;
            let m = &transform.matrix;
            let new_m00 = inv_w * m[0][0];
            let new_m01 = inv_w * m[0][1];
            let new_m02 = inv_w * m[0][2] - x * inv_w;
            let new_m10 = inv_h * m[1][0];
            let new_m11 = inv_h * m[1][1];
            let new_m12 = inv_h * m[1][2] - y * inv_h;
            transform.matrix[0][0] = new_m00;
            transform.matrix[0][1] = new_m01;
            transform.matrix[0][2] = new_m02;
            transform.matrix[1][0] = new_m10;
            transform.matrix[1][1] = new_m11;
            transform.matrix[1][2] = new_m12;
        }
    }
}

fn translation(tx: f32, ty: f32) -> AffineTransform {
    AffineTransform::from_acebdf(1.0, 0.0, tx, 0.0, 1.0, ty)
}

fn scale(sx: f32, sy: f32) -> AffineTransform {
    AffineTransform::from_acebdf(sx, 0.0, 0.0, 0.0, sy, 0.0)
}
