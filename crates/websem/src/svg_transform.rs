//! The computed `transform` value: a typed operation list in, one affine out.
//!
//! Separated from the compiler because it is exactly that and nothing else —
//! no document, no attribute text, no compiler error type, no element. Since
//! the transform rung, the *attribute* grammar lives in csscascade
//! (`svg_transform::transform_attribute_to_css`), which rewrites the SVG
//! spelling into the one CSS `transform` property at presentation-hint level;
//! by the time this module runs, both spellings are the same computed
//! operation list and precedence has already been decided by the cascade.
//!
//! What this module refuses is the boundary of the admitted function set: an
//! operation outside the 2D affine family (the 3D forms, `perspective`) and a
//! `calc()` length both name themselves in a [`TransformRefusal`] rather than
//! flatten or approximate — Chromium applies 3D forms on SVG (measured:
//! `translate3d` moves a rect), so dropping one silently would move nothing
//! where Chromium moves.

use math2::transform::AffineTransform;
use style::values::computed::LengthPercentage;
use style::values::computed::transform::{Transform, TransformOperation};

/// Why a computed transform is outside the admitted slice, named for the
/// declared hole. The compiler owns the sentence; this module owns the fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransformRefusal {
    /// An operation outside the 2D affine set, in its CSS spelling.
    Function(&'static str),
    /// A `calc()` length, which needs resolution machinery this slice
    /// does not carry.
    Calc,
    /// A percentage translation in a context whose caller supplies no
    /// basis — a gradient's transform, where Chromium resolves the
    /// percentage against the viewport and then applies the raw number in
    /// fraction space (measured), an incoherence this slice refuses.
    Percentage,
}

/// Convert a computed transform list into one affine.
///
/// The list composes left to right — each operation applies inside the
/// mapping accumulated before it, which is [`AffineTransform::compose`]'s
/// "apply `other` after `self`" order. Percentages in translations resolve
/// against `basis` — the viewport's user-unit extent (the `viewBox` when one
/// maps the viewport), horizontal against its width, vertical against its
/// height: the transform reference box with the measured used origin `0 0`,
/// which is the local user-space origin (the transform rung's probe matrix;
/// Chromium ignores a `viewBox` min for this purpose).
pub(crate) fn computed_transform_to_affine(
    transform: &Transform,
    basis: Option<(f32, f32)>,
) -> Result<AffineTransform, TransformRefusal> {
    let (basis_width, basis_height) = match basis {
        Some((width, height)) => (Some(width), Some(height)),
        None => (None, None),
    };
    let mut composed = AffineTransform::identity();
    for operation in transform.0.iter() {
        let step = match operation {
            TransformOperation::Matrix(m) => {
                AffineTransform::from_acebdf(m.a, m.c, m.e, m.b, m.d, m.f)
            }
            TransformOperation::Translate(x, y) => {
                let tx = resolve_length(x, basis_width)?;
                let ty = resolve_length(y, basis_height)?;
                AffineTransform::from_acebdf(1.0, 0.0, tx, 0.0, 1.0, ty)
            }
            TransformOperation::TranslateX(x) => {
                let tx = resolve_length(x, basis_width)?;
                AffineTransform::from_acebdf(1.0, 0.0, tx, 0.0, 1.0, 0.0)
            }
            TransformOperation::TranslateY(y) => {
                let ty = resolve_length(y, basis_height)?;
                AffineTransform::from_acebdf(1.0, 0.0, 0.0, 0.0, 1.0, ty)
            }
            TransformOperation::Scale(sx, sy) => {
                AffineTransform::from_acebdf(*sx, 0.0, 0.0, 0.0, *sy, 0.0)
            }
            TransformOperation::ScaleX(sx) => {
                AffineTransform::from_acebdf(*sx, 0.0, 0.0, 0.0, 1.0, 0.0)
            }
            TransformOperation::ScaleY(sy) => {
                AffineTransform::from_acebdf(1.0, 0.0, 0.0, 0.0, *sy, 0.0)
            }
            TransformOperation::Rotate(angle) => rotate_transform(angle.degrees()),
            TransformOperation::Skew(ax, ay) => AffineTransform::from_acebdf(
                1.0,
                ax.degrees().to_radians().tan(),
                0.0,
                ay.degrees().to_radians().tan(),
                1.0,
                0.0,
            ),
            TransformOperation::SkewX(angle) => AffineTransform::from_acebdf(
                1.0,
                angle.degrees().to_radians().tan(),
                0.0,
                0.0,
                1.0,
                0.0,
            ),
            TransformOperation::SkewY(angle) => AffineTransform::from_acebdf(
                1.0,
                0.0,
                0.0,
                angle.degrees().to_radians().tan(),
                1.0,
                0.0,
            ),
            // The beyond-2D family refuses by CSS spelling. Chromium
            // composes each of these on SVG content (measured for
            // `translate3d`), so a silent drop would be a wrong pixel;
            // flattening them is a future rung's measured work.
            TransformOperation::Matrix3D(_) => {
                return Err(TransformRefusal::Function("matrix3d"));
            }
            TransformOperation::TranslateZ(_) => {
                return Err(TransformRefusal::Function("translateZ"));
            }
            TransformOperation::Translate3D(..) => {
                return Err(TransformRefusal::Function("translate3d"));
            }
            TransformOperation::ScaleZ(_) => {
                return Err(TransformRefusal::Function("scaleZ"));
            }
            TransformOperation::Scale3D(..) => {
                return Err(TransformRefusal::Function("scale3d"));
            }
            TransformOperation::RotateX(_) => {
                return Err(TransformRefusal::Function("rotateX"));
            }
            TransformOperation::RotateY(_) => {
                return Err(TransformRefusal::Function("rotateY"));
            }
            TransformOperation::RotateZ(_) => {
                return Err(TransformRefusal::Function("rotateZ"));
            }
            TransformOperation::Rotate3D(..) => {
                return Err(TransformRefusal::Function("rotate3d"));
            }
            TransformOperation::Perspective(_) => {
                return Err(TransformRefusal::Function("perspective"));
            }
            // Animation-composition intermediates cannot appear in a static
            // cascade; total matching keeps that a named fact rather than a
            // panic if one ever does.
            TransformOperation::InterpolateMatrix { .. } => {
                return Err(TransformRefusal::Function("interpolatematrix"));
            }
            TransformOperation::AccumulateMatrix { .. } => {
                return Err(TransformRefusal::Function("accumulatematrix"));
            }
        };
        composed = composed.compose(&step);
    }
    Ok(composed)
}

/// One translation length: absolute px, or a percentage of its axis basis.
fn resolve_length(length: &LengthPercentage, basis: Option<f32>) -> Result<f32, TransformRefusal> {
    if let Some(absolute) = length.to_length() {
        return Ok(absolute.px());
    }
    if let Some(percentage) = length.to_percentage() {
        let Some(basis) = basis else {
            return Err(TransformRefusal::Percentage);
        };
        return Ok(percentage.0 * basis);
    }
    Err(TransformRefusal::Calc)
}

/// A rotation about the origin.
///
/// A quarter turn is produced from its integer matrix rather than from
/// `sin`/`cos`: in f32 the cosine of a right angle is `-4.37e-8`, not
/// zero, so the generic path shears and shifts the shape by a fraction of
/// a unit where the exact matrix does not. This applies to both spellings
/// alike — the attribute's `rotate(90)` reaches here as the computed
/// `rotate(90deg)`.
///
/// Two guards keep the shortcut honest. The multiple-of-90 test uses `%`,
/// which is exact in f32, rather than comparing a quotient to its
/// truncation — past `90 * 2^23` every quotient is integral by
/// construction, so a quotient test would snap arbitrary large angles onto
/// one of four exact matrices. The magnitude bound then keeps the quadrant
/// index meaningful, since reducing a huge quotient mod 4 is not.
fn rotate_transform(degrees: f32) -> AffineTransform {
    /// Well past any authored angle, and far below where f32 spacing makes
    /// the quadrant reduction lossy.
    const EXACT_QUARTER_TURN_LIMIT: f32 = 360.0 * 1024.0;
    if degrees.abs() <= EXACT_QUARTER_TURN_LIMIT && degrees % 90.0 == 0.0 {
        let (sin, cos) = match (degrees / 90.0).rem_euclid(4.0) as i32 {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        };
        return AffineTransform::from_acebdf(cos, -sin, 0.0, sin, cos, 0.0);
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    AffineTransform::from_acebdf(cos, -sin, 0.0, sin, cos, 0.0)
}
