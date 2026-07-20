use crate::cg::prelude::*;
use skia_safe;

/// Convert a backend-neutral canvas-graphics value into its Skia counterpart.
///
/// This local trait keeps backend mappings on the consumer side of the
/// skia-free `cg` crate boundary.
pub trait IntoSkia<T> {
    fn into_skia(self) -> T;
}

impl IntoSkia<skia_safe::Color> for CGColor {
    fn into_skia(self) -> skia_safe::Color {
        let color = self;
        skia_safe::Color::from_argb(color.a(), color.r(), color.g(), color.b())
    }
}

impl IntoSkia<skia_safe::Rect> for CGRect {
    fn into_skia(self) -> skia_safe::Rect {
        let rect = self;
        skia_safe::Rect::from_xywh(rect.x, rect.y, rect.width, rect.height)
    }
}

impl IntoSkia<skia_safe::PathOp> for BooleanPathOperation {
    fn into_skia(self) -> skia_safe::PathOp {
        let op = self;
        match op {
            BooleanPathOperation::Union => skia_safe::PathOp::Union,
            BooleanPathOperation::Intersection => skia_safe::PathOp::Intersect,
            BooleanPathOperation::Difference => skia_safe::PathOp::Difference,
            BooleanPathOperation::Xor => skia_safe::PathOp::XOR,
        }
    }
}

impl IntoSkia<skia_safe::TileMode> for TileMode {
    fn into_skia(self) -> skia_safe::TileMode {
        let tile_mode = self;
        match tile_mode {
            TileMode::Clamp => skia_safe::TileMode::Clamp,
            TileMode::Repeated => skia_safe::TileMode::Repeat,
            TileMode::Mirror => skia_safe::TileMode::Mirror,
            TileMode::Decal => skia_safe::TileMode::Decal,
        }
    }
}

impl IntoSkia<skia_safe::Blender> for BlendMode {
    fn into_skia(self) -> skia_safe::Blender {
        let val = self;
        use skia_safe::BlendMode::*;
        let sk_blend_mode = match val {
            BlendMode::Normal => SrcOver,
            BlendMode::Multiply => Multiply,
            BlendMode::Screen => Screen,
            BlendMode::Overlay => Overlay,
            BlendMode::Darken => Darken,
            BlendMode::Lighten => Lighten,
            BlendMode::ColorDodge => ColorDodge,
            BlendMode::ColorBurn => ColorBurn,
            BlendMode::HardLight => HardLight,
            BlendMode::SoftLight => SoftLight,
            BlendMode::Difference => Difference,
            BlendMode::Exclusion => Exclusion,
            BlendMode::Hue => Hue,
            BlendMode::Saturation => Saturation,
            BlendMode::Color => Color,
            BlendMode::Luminosity => Luminosity,
        };
        skia_safe::Blender::mode(sk_blend_mode)
    }
}

impl IntoSkia<skia_safe::BlendMode> for BlendMode {
    fn into_skia(self) -> skia_safe::BlendMode {
        let mode = self;
        use skia_safe::BlendMode::*;
        match mode {
            BlendMode::Normal => SrcOver,
            BlendMode::Multiply => Multiply,
            BlendMode::Screen => Screen,
            BlendMode::Overlay => Overlay,
            BlendMode::Darken => Darken,
            BlendMode::Lighten => Lighten,
            BlendMode::ColorDodge => ColorDodge,
            BlendMode::ColorBurn => ColorBurn,
            BlendMode::HardLight => HardLight,
            BlendMode::SoftLight => SoftLight,
            BlendMode::Difference => Difference,
            BlendMode::Exclusion => Exclusion,
            BlendMode::Hue => Hue,
            BlendMode::Saturation => Saturation,
            BlendMode::Color => Color,
            BlendMode::Luminosity => Luminosity,
        }
    }
}

impl IntoSkia<skia_safe::PaintCap> for StrokeCap {
    fn into_skia(self) -> skia_safe::PaintCap {
        let val = self;
        match val {
            StrokeCap::Butt => skia_safe::PaintCap::Butt,
            StrokeCap::Round => skia_safe::PaintCap::Round,
            StrokeCap::Square => skia_safe::PaintCap::Square,
        }
    }
}

impl IntoSkia<skia_safe::PaintJoin> for StrokeJoin {
    fn into_skia(self) -> skia_safe::PaintJoin {
        let val = self;
        match val {
            StrokeJoin::Miter => skia_safe::PaintJoin::Miter,
            StrokeJoin::Round => skia_safe::PaintJoin::Round,
            StrokeJoin::Bevel => skia_safe::PaintJoin::Bevel,
        }
    }
}

impl IntoSkia<skia_safe::textlayout::TextDecoration> for TextDecorationLine {
    fn into_skia(self) -> skia_safe::textlayout::TextDecoration {
        let mode = self;
        match mode {
            TextDecorationLine::None => skia_safe::textlayout::TextDecoration::NO_DECORATION,
            TextDecorationLine::Underline => skia_safe::textlayout::TextDecoration::UNDERLINE,
            TextDecorationLine::Overline => skia_safe::textlayout::TextDecoration::OVERLINE,
            TextDecorationLine::LineThrough => skia_safe::textlayout::TextDecoration::LINE_THROUGH,
        }
    }
}

impl IntoSkia<skia_safe::textlayout::TextDecorationStyle> for TextDecorationStyle {
    fn into_skia(self) -> skia_safe::textlayout::TextDecorationStyle {
        let mode = self;
        match mode {
            TextDecorationStyle::Solid => skia_safe::textlayout::TextDecorationStyle::Solid,
            TextDecorationStyle::Double => skia_safe::textlayout::TextDecorationStyle::Double,
            TextDecorationStyle::Dotted => skia_safe::textlayout::TextDecorationStyle::Dotted,
            TextDecorationStyle::Dashed => skia_safe::textlayout::TextDecorationStyle::Dashed,
            TextDecorationStyle::Wavy => skia_safe::textlayout::TextDecorationStyle::Wavy,
        }
    }
}

impl IntoSkia<skia_safe::textlayout::TextAlign> for TextAlign {
    fn into_skia(self) -> skia_safe::textlayout::TextAlign {
        let mode = self;
        use skia_safe::textlayout::TextAlign::*;
        match mode {
            TextAlign::Left => Left,
            TextAlign::Right => Right,
            TextAlign::Center => Center,
            TextAlign::Justify => Justify,
        }
    }
}

impl IntoSkia<skia_safe::textlayout::Decoration> for TextDecoration {
    fn into_skia(self) -> skia_safe::textlayout::Decoration {
        let decoration = self;
        skia_safe::textlayout::Decoration {
            ty: decoration.text_decoration_line.into_skia(),
            // Set the decoration mode based on skip_ink setting
            // Gaps: decoration skips over descenders (g, p, q, etc.)
            // Through: decoration goes through all characters including descenders
            // FIXME: the `Gaps` mode will make non-skipping underlines to completely not draw the underline.
            // see https://github.com/rust-skia/rust-skia/issues/1187
            // this might be a bug with skia-safe
            mode: skia_safe::textlayout::TextDecorationMode::Through,
            // mode: if decoration.text_decoration_skip_ink {
            //     skia_safe::textlayout::TextDecorationMode::Gaps
            // } else {
            //     skia_safe::textlayout::TextDecorationMode::Through
            // },
            color: decoration.text_decoration_color.into_skia(),
            style: decoration.text_decoration_style.into_skia(),
            thickness_multiplier: decoration.text_decoration_thickness,
        }
    }
}
