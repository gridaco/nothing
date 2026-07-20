// NOTE: This module only contains conversion utilities (Into/From) between usvg
// primitives and our SVG IR / core CG types. Scene construction lives elsewhere.

use crate::cg::prelude::*;

/// Convert a usvg value into the corresponding canvas-graphics vocabulary.
///
/// The trait lives on the importer side so the skia-free `cg` crate does not
/// acquire an SVG parser dependency.
pub trait IntoCg<T> {
    fn into_cg(self) -> T;
}

impl IntoCg<CGColor> for usvg::Color {
    fn into_cg(self) -> CGColor {
        let color = self;
        CGColor::from_rgba(color.red, color.green, color.blue, 255)
    }
}

impl IntoCg<CGRect> for usvg::Rect {
    fn into_cg(self) -> CGRect {
        let rect = self;
        CGRect {
            x: rect.x(),
            y: rect.y(),
            width: rect.width(),
            height: rect.height(),
        }
    }
}

impl IntoCg<BlendMode> for usvg::BlendMode {
    fn into_cg(self) -> BlendMode {
        let blend_mode = self;
        match blend_mode {
            usvg::BlendMode::Normal => BlendMode::Normal,
            usvg::BlendMode::Multiply => BlendMode::Multiply,
            usvg::BlendMode::Screen => BlendMode::Screen,
            usvg::BlendMode::Overlay => BlendMode::Overlay,
            usvg::BlendMode::Darken => BlendMode::Darken,
            usvg::BlendMode::Lighten => BlendMode::Lighten,
            usvg::BlendMode::ColorDodge => BlendMode::ColorDodge,
            usvg::BlendMode::ColorBurn => BlendMode::ColorBurn,
            usvg::BlendMode::HardLight => BlendMode::HardLight,
            usvg::BlendMode::SoftLight => BlendMode::SoftLight,
            usvg::BlendMode::Difference => BlendMode::Difference,
            usvg::BlendMode::Exclusion => BlendMode::Exclusion,
            usvg::BlendMode::Hue => BlendMode::Hue,
            usvg::BlendMode::Saturation => BlendMode::Saturation,
            usvg::BlendMode::Color => BlendMode::Color,
            usvg::BlendMode::Luminosity => BlendMode::Luminosity,
        }
    }
}

impl IntoCg<ImageMaskType> for usvg::MaskType {
    fn into_cg(self) -> ImageMaskType {
        let mask_type = self;
        match mask_type {
            usvg::MaskType::Luminance => ImageMaskType::Luminance,
            usvg::MaskType::Alpha => ImageMaskType::Alpha,
        }
    }
}

impl IntoCg<FillRule> for usvg::FillRule {
    fn into_cg(self) -> FillRule {
        let fill_rule = self;
        match fill_rule {
            usvg::FillRule::NonZero => FillRule::NonZero,
            usvg::FillRule::EvenOdd => FillRule::EvenOdd,
        }
    }
}

impl IntoCg<SVGTextAnchor> for usvg::TextAnchor {
    fn into_cg(self) -> SVGTextAnchor {
        let text_anchor = self;
        match text_anchor {
            usvg::TextAnchor::Start => SVGTextAnchor::Start,
            usvg::TextAnchor::Middle => SVGTextAnchor::Middle,
            usvg::TextAnchor::End => SVGTextAnchor::End,
        }
    }
}

impl IntoCg<StrokeMiterLimit> for usvg::StrokeMiterlimit {
    fn into_cg(self) -> StrokeMiterLimit {
        let miterlimit = self;
        StrokeMiterLimit::new(miterlimit.get())
    }
}

impl IntoCg<StrokeWidth> for usvg::StrokeWidth {
    fn into_cg(self) -> StrokeWidth {
        let stroke_width = self;
        StrokeWidth::Uniform(stroke_width.get())
    }
}

impl IntoCg<StrokeCap> for usvg::LineCap {
    fn into_cg(self) -> StrokeCap {
        let line_cap = self;
        match line_cap {
            usvg::LineCap::Butt => StrokeCap::Butt,
            usvg::LineCap::Round => StrokeCap::Round,
            usvg::LineCap::Square => StrokeCap::Square,
        }
    }
}

impl IntoCg<StrokeJoin> for usvg::LineJoin {
    fn into_cg(self) -> StrokeJoin {
        let line_join = self;
        match line_join {
            usvg::LineJoin::Miter => StrokeJoin::Miter,
            usvg::LineJoin::Round => StrokeJoin::Round,
            usvg::LineJoin::Bevel => StrokeJoin::Bevel,
            // [MODEL_MISMATCH]
            usvg::LineJoin::MiterClip => StrokeJoin::Miter,
        }
    }
}

impl IntoCg<CGTransform2D> for usvg::Transform {
    fn into_cg(self) -> CGTransform2D {
        let transform = self;
        CGTransform2D::new(
            transform.sx,
            transform.kx,
            transform.tx,
            transform.ky,
            transform.sy,
            transform.ty,
        )
    }
}

impl IntoCg<GradientStop> for usvg::Stop {
    fn into_cg(self) -> GradientStop {
        let value = self;
        GradientStop {
            offset: value.offset().get(),
            color: value.color().into_cg(),
            // [MODEL_MISMATCH]
            // opacity: value.opacity().get(),
        }
    }
}

struct UsvgStops<'a>(&'a [usvg::Stop]);

impl From<UsvgStops<'_>> for Vec<GradientStop> {
    fn from(stops: UsvgStops<'_>) -> Self {
        stops.0.iter().cloned().map(IntoCg::into_cg).collect()
    }
}

impl IntoCg<SVGLinearGradientPaint> for &usvg::LinearGradient {
    fn into_cg(self) -> SVGLinearGradientPaint {
        let gradient = self;
        SVGLinearGradientPaint {
            id: gradient.id().to_string(),
            x1: gradient.x1(),
            y1: gradient.y1(),
            x2: gradient.x2(),
            y2: gradient.y2(),
            transform: gradient.transform().into_cg(),
            stops: Vec::<GradientStop>::from(UsvgStops(gradient.stops())),
            spread_method: gradient.spread_method().into_cg(),
        }
    }
}

impl IntoCg<SVGRadialGradientPaint> for &usvg::RadialGradient {
    fn into_cg(self) -> SVGRadialGradientPaint {
        let gradient = self;
        SVGRadialGradientPaint {
            id: gradient.id().to_string(),
            cx: gradient.cx(),
            cy: gradient.cy(),
            r: gradient.r().get(),
            fx: gradient.fx(),
            fy: gradient.fy(),
            transform: gradient.transform().into_cg(),
            stops: Vec::<GradientStop>::from(UsvgStops(gradient.stops())),
            spread_method: gradient.spread_method().into_cg(),
        }
    }
}

impl IntoCg<SVGPaint> for &usvg::Paint {
    fn into_cg(self) -> SVGPaint {
        let paint = self;
        match paint {
            usvg::Paint::Color(color) => SVGPaint::Solid(SVGSolidPaint {
                color: (*color).into_cg(),
            }),
            usvg::Paint::LinearGradient(gradient) => {
                SVGPaint::LinearGradient(gradient.as_ref().into_cg())
            }
            usvg::Paint::RadialGradient(gradient) => {
                SVGPaint::RadialGradient(gradient.as_ref().into_cg())
            }
            // [MODEL_MISMATCH]
            // fallback to solid paint
            usvg::Paint::Pattern(_pattern) => SVGPaint::TRANSPARENT,
        }
    }
}

impl IntoCg<SVGStrokeAttributes> for usvg::Stroke {
    fn into_cg(self) -> SVGStrokeAttributes {
        let stroke = self;
        SVGStrokeAttributes {
            paint: stroke.paint().into_cg(),
            stroke_opacity: stroke.opacity().get(),
            stroke_width: stroke.width().get(),
            stroke_linecap: stroke.linecap().into_cg(),
            stroke_linejoin: stroke.linejoin().into_cg(),
            stroke_miterlimit: stroke.miterlimit().into_cg(),
            stroke_dasharray: stroke
                .dasharray()
                .map(|slice| StrokeDashArray(slice.to_vec())),
        }
    }
}

impl IntoCg<SVGFillAttributes> for &usvg::Fill {
    fn into_cg(self) -> SVGFillAttributes {
        let fill = self;
        SVGFillAttributes {
            paint: fill.paint().into_cg(),
            fill_opacity: fill.opacity().get(),
            fill_rule: fill.rule().into_cg(),
        }
    }
}

impl IntoCg<SVGStrokeAttributes> for &usvg::Stroke {
    fn into_cg(self) -> SVGStrokeAttributes {
        let stroke = self;
        stroke.clone().into_cg()
    }
}

impl IntoCg<SVGGradientSpreadMethod> for usvg::SpreadMethod {
    fn into_cg(self) -> SVGGradientSpreadMethod {
        let method = self;
        match method {
            usvg::SpreadMethod::Pad => SVGGradientSpreadMethod::Pad,
            usvg::SpreadMethod::Reflect => SVGGradientSpreadMethod::Reflect,
            usvg::SpreadMethod::Repeat => SVGGradientSpreadMethod::Repeat,
        }
    }
}

// [MODEL_MISMATCH]
// impl From<usvg::Fill> for Fill {
//     fn from(fill: usvg::Fill) -> Self {
//         // - fill.opacity
//         // - fill.rule
//         // - fill.paint
//     }
// }

// [MODEL_MISMATCH]
// impl From<usvg::ClipPath> for ??
