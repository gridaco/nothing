//! Pure mappings from the shared per-element style record
//! ([`StyledElement`]) to the types the HTML importer's emitters consume.
//!
//! This is the importer's half of the htmlcss seam (gridaco/nothing#30):
//! `crate::htmlcss::collect::styled_of` extracts one element's resolved
//! style into a [`StyledElement`]; the functions here map that record
//! onto v1 node-record fields. Every function is pure and total — no
//! Stylo types, no DOM access.
//!
//! The mappings reproduce the importer's pinned behavior (the
//! `html_import_snapshot` golden corpus) exactly; where the record is
//! richer than what the importer historically consumed (percent radii,
//! `repeating` gradient flags, flex-basis, …), the extra information is
//! deliberately dropped, matching the old `ComputedValues` walk.

use crate::cg::prelude::*;
use crate::node::schema::*;

use crate::htmlcss::style::{
    BackgroundImage, BackgroundLayer, FilterFunction, GradientKeyword,
    GradientStop as CssGradientStop, LinearGradient as CssLinearGradient, StyleImage,
    StyledElement,
};
use crate::htmlcss::types::AlignItems as CssAlignItems;
use crate::htmlcss::types::{
    BorderStyle as CssBorderStyle, CssLength, Display, FlexDirection, FlexWrap, JustifyContent,
    Position,
};

use super::CSSMargin;

/// The importer's solid paint shape — default blend, active.
fn solid(color: CGColor) -> Paint {
    Paint::Solid(SolidPaint {
        color,
        blend_mode: BlendMode::default(),
        active: true,
    })
}

/// CSS `width`/`height`/`min-*`/`max-*` → [`LayoutDimensionStyle`].
///
/// Only definite px lengths are honored; `auto` clears the target
/// dimension (containers default to the factory's 100×100 otherwise)
/// and percentages/calc leave the field untouched — the importer has
/// never resolved them (no containing-block size at import time).
pub(super) fn dimensions_from(el: &StyledElement, dims: &mut LayoutDimensionStyle) {
    match el.width {
        CssLength::Px(v) => dims.layout_target_width = Some(v),
        CssLength::Auto => dims.layout_target_width = None,
        _ => {}
    }
    match el.height {
        CssLength::Px(v) => dims.layout_target_height = Some(v),
        CssLength::Auto => dims.layout_target_height = None,
        _ => {}
    }
    if let CssLength::Px(v) = el.min_width {
        if v > 0.0 {
            dims.layout_min_width = Some(v);
        }
    }
    if let CssLength::Px(v) = el.min_height {
        if v > 0.0 {
            dims.layout_min_height = Some(v);
        }
    }
    if let CssLength::Px(v) = el.max_width {
        dims.layout_max_width = Some(v);
    }
    if let CssLength::Px(v) = el.max_height {
        dims.layout_max_height = Some(v);
    }
}

/// CSS `width`/`height` → a leaf [`Size`] (Rectangle nodes).
///
/// `auto`, percentages, and calc all resolve to `0×0` — unlike the
/// design-tool convention (100×100 default), HTML leaf elements have no
/// intrinsic size.
pub(super) fn size_from(el: &StyledElement) -> Size {
    let px = |v: CssLength| -> f32 {
        match v {
            CssLength::Px(p) => p,
            _ => 0.0,
        }
    };
    Size {
        width: px(el.width),
        height: px(el.height),
    }
}

/// Flex-child properties (`flex-grow`, absolute positioning) →
/// [`LayoutChildStyle`]. `None` when all values are at their defaults
/// (grow = 0, position static/relative).
pub(super) fn layout_child_from(el: &StyledElement) -> Option<LayoutChildStyle> {
    let grow = el.flex_grow;
    // The record maps `position: fixed` to `Position::Absolute` at
    // extraction (Stylo's `is_absolutely_positioned` covers both), so
    // one variant check matches the old ComputedValues read.
    let is_absolute = el.position == Position::Absolute;
    if grow > 0.0 || is_absolute {
        Some(LayoutChildStyle {
            layout_grow: grow,
            layout_positioning: if is_absolute {
                LayoutPositioning::Absolute
            } else {
                LayoutPositioning::Auto
            },
        })
    } else {
        None
    }
}

/// Flex-container properties (`display`, direction, wrap, alignment,
/// gap) → [`LayoutContainerStyle`].
///
/// Non-flex displays (block, grid, table, …) map to a Flex column: the
/// IR's `LayoutMode::Normal` maps to taffy `Display::Block`, which
/// causes children of a flex parent to stretch to 100% width (block
/// intrinsic sizing). Using Flex column instead gives correct sizing
/// when these containers are nested inside flex parents.
pub(super) fn flex_container_from(el: &StyledElement, out: &mut LayoutContainerStyle) {
    out.layout_mode = LayoutMode::Flex;
    if el.display != Display::Flex {
        out.layout_direction = Axis::Vertical;
        return;
    }

    out.layout_direction = match el.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => Axis::Horizontal,
        FlexDirection::Column | FlexDirection::ColumnReverse => Axis::Vertical,
    };

    out.layout_wrap = Some(match el.flex_wrap {
        FlexWrap::Nowrap => LayoutWrap::NoWrap,
        FlexWrap::Wrap | FlexWrap::WrapReverse => LayoutWrap::Wrap,
    });

    // align-items → cross axis alignment. `None` is the record's
    // authored-`normal` (and unrecognized-keyword) case — the importer
    // leaves the field unset; `baseline` has no IR equivalent and joins
    // it, exactly like the old AlignFlags fall-through arm.
    out.layout_cross_axis_alignment = match el.align_items {
        Some(CssAlignItems::Center) => Some(CrossAxisAlignment::Center),
        Some(CssAlignItems::Start) => Some(CrossAxisAlignment::Start),
        Some(CssAlignItems::End) => Some(CrossAxisAlignment::End),
        Some(CssAlignItems::Stretch) => Some(CrossAxisAlignment::Stretch),
        Some(CssAlignItems::Baseline) | None => None,
    };

    // justify-content → main axis alignment. `None` (authored `normal`
    // or unrecognized) leaves the field unset, as before.
    out.layout_main_axis_alignment = match el.justify_content {
        Some(JustifyContent::Center) => Some(MainAxisAlignment::Center),
        Some(JustifyContent::Start) => Some(MainAxisAlignment::Start),
        Some(JustifyContent::End) => Some(MainAxisAlignment::End),
        Some(JustifyContent::SpaceBetween) => Some(MainAxisAlignment::SpaceBetween),
        Some(JustifyContent::SpaceAround) => Some(MainAxisAlignment::SpaceAround),
        Some(JustifyContent::SpaceEvenly) => Some(MainAxisAlignment::SpaceEvenly),
        Some(JustifyContent::Stretch) => Some(MainAxisAlignment::Stretch),
        None => None,
    };

    // Gap (flex containers only).
    // CSS column-gap = inline-axis gap, row-gap = block-axis gap.
    // For flex-direction: row, column-gap is the main-axis gap.
    // For flex-direction: column, row-gap is the main-axis gap.
    if el.row_gap != 0.0 || el.column_gap != 0.0 {
        let (main_gap, cross_gap) = match el.flex_direction {
            FlexDirection::Row | FlexDirection::RowReverse => (el.column_gap, el.row_gap),
            FlexDirection::Column | FlexDirection::ColumnReverse => (el.row_gap, el.column_gap),
        };
        out.layout_gap = Some(LayoutGap {
            main_axis_gap: main_gap,
            cross_axis_gap: cross_gap,
        });
    }
}

/// Record gradient stops → CG gradient stops.
///
/// - `currentcolor` stops and fully-transparent stops fold to
///   transparent black — the goldens pin that fold (the old
///   `css_color_to_cg(..).unwrap_or(clear)` behavior).
/// - px-positioned offsets fold to `0.0`: the importer has never
///   resolved absolute lengths along the gradient line (no geometry at
///   import time).
fn stops_from(stops: &[CssGradientStop]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|s| GradientStop {
            offset: if s.offset_is_px { 0.0 } else { s.offset },
            color: if s.color_is_currentcolor || s.color.a == 0 {
                CGColor::from_rgba(0, 0, 0, 0)
            } else {
                s.color
            },
        })
        .collect()
}

/// CSS linear-gradient direction → IR Alignment endpoints.
///
/// CSS gradient angles: 0deg = to top, 90deg = to right, 180deg = to bottom.
/// IR Alignment: (-1,-1) = top-left, (0,0) = center, (1,1) = bottom-right.
///
/// `to <side-or-corner>` keywords map to exact axis/corner constants;
/// angle directions go through trig.
fn linear_endpoints(g: &CssLinearGradient) -> (Alignment, Alignment) {
    match g.keyword {
        Some(GradientKeyword::ToTop) => (Alignment::BOTTOM_CENTER, Alignment::TOP_CENTER),
        Some(GradientKeyword::ToBottom) => (Alignment::TOP_CENTER, Alignment::BOTTOM_CENTER),
        Some(GradientKeyword::ToLeft) => (Alignment::CENTER_RIGHT, Alignment::CENTER_LEFT),
        Some(GradientKeyword::ToRight) => (Alignment::CENTER_LEFT, Alignment::CENTER_RIGHT),
        Some(GradientKeyword::Corner { right, bottom }) => {
            let x1 = if right { -1.0 } else { 1.0 };
            let y1 = if bottom { -1.0 } else { 1.0 };
            (Alignment(x1, y1), Alignment(-x1, -y1))
        }
        None => {
            // Replicate Stylo's computed `Angle::radians()` bit-for-bit:
            // the record's `angle_deg` is the stored f32 degrees; the
            // deg→rad conversion runs in f64 and narrows, exactly like
            // the old direct `radians()` read.
            let rad = ((g.angle_deg as f64) * (std::f64::consts::PI / 180.0))
                .min(f32::MAX as f64)
                .max(f32::MIN as f64) as f32;
            let sin = rad.sin();
            let cos = rad.cos();
            // CSS gradient line: from (-sin, cos) to (sin, -cos) in NDC
            (Alignment(-sin, cos), Alignment(sin, -cos))
        }
    }
}

/// CSS background (color + gradient image layers) → fill paints.
///
/// The record stores image layers bottom-to-top (reverse source order)
/// for the renderer's paint loop; the importer emits the solid color
/// first, then gradients in source order — so image layers are
/// re-reversed here. Raster (`url()`) backgrounds are not imported.
pub(super) fn fills_from_background(el: &StyledElement) -> Paints {
    let mut paints: Vec<Paint> = Vec::new();
    let mut images: Vec<&BackgroundImage> = Vec::new();

    // 1. Background color (bottom layer). The record already omits it
    //    when fully transparent (the old a == 0 fold).
    for layer in &el.background {
        match layer {
            BackgroundLayer::Solid { color, .. } => paints.push(solid(*color)),
            BackgroundLayer::Image(img) => images.push(img),
        }
    }

    // 2. Gradient layers on top, in source order.
    for img in images.iter().rev() {
        match &img.source {
            StyleImage::LinearGradient(g) => {
                let (xy1, xy2) = linear_endpoints(g);
                paints.push(Paint::LinearGradient(LinearGradientPaint {
                    active: true,
                    xy1,
                    xy2,
                    stops: stops_from(&g.stops),
                    ..Default::default()
                }));
            }
            StyleImage::RadialGradient(g) => {
                paints.push(Paint::RadialGradient(RadialGradientPaint::from_stops(
                    stops_from(&g.stops),
                )));
            }
            StyleImage::ConicGradient(g) => {
                paints.push(Paint::SweepGradient(SweepGradientPaint {
                    active: true,
                    stops: stops_from(&g.stops),
                    ..Default::default()
                }));
            }
            StyleImage::Url(_) => {}
        }
    }

    if paints.is_empty() {
        Paints::default()
    } else {
        Paints::new(paints)
    }
}

/// Border radius → per-corner px radii. Percent components drop to
/// `0.0` — the importer has never resolved them (no box size at import
/// time); a calc radius carrying any percent term drops entirely,
/// matching the old `to_length()`-or-zero read.
pub(super) fn corner_radius_from(el: &StyledElement) -> RectangularCornerRadius {
    let r = &el.border_radius;
    let px = |v: f32, pct: f32| -> f32 {
        if pct != 0.0 {
            0.0
        } else {
            v
        }
    };
    RectangularCornerRadius {
        tl: Radius {
            rx: px(r.tl_x, r.tl_x_pct),
            ry: px(r.tl_y, r.tl_y_pct),
        },
        tr: Radius {
            rx: px(r.tr_x, r.tr_x_pct),
            ry: px(r.tr_y, r.tr_y_pct),
        },
        br: Radius {
            rx: px(r.br_x, r.br_x_pct),
            ry: px(r.br_y, r.br_y_pct),
        },
        bl: Radius {
            rx: px(r.bl_x, r.bl_x_pct),
            ry: px(r.bl_y, r.bl_y_pct),
        },
    }
}

/// CSS borders → strokes, stroke width, and stroke style.
///
/// The importer's border model: one stroke paint (the first
/// non-transparent side color in top→right→bottom→left order), uniform
/// or per-side widths, and a dash pattern from the top side's line
/// style. Side widths are already zeroed for `none`/`hidden` styles at
/// extraction.
pub(super) fn strokes_from_border(el: &StyledElement) -> (Paints, StrokeWidth, StrokeStyle) {
    let b = &el.border;
    let (top_w, right_w, bottom_w, left_w) =
        (b.top.width, b.right.width, b.bottom.width, b.left.width);

    let has_border = top_w > 0.0 || right_w > 0.0 || bottom_w > 0.0 || left_w > 0.0;
    if !has_border {
        return (Paints::default(), StrokeWidth::None, StrokeStyle::default());
    }

    // Use the top border color as the primary stroke color (most common
    // single-color case); for per-side colors, the first visible border
    // side. `currentcolor` sides (the initial value — carried by every
    // side whose color was never authored) and fully-transparent sides
    // are skipped, reproducing the old `css_color_to_cg` fold.
    let border_color = [&b.top, &b.right, &b.bottom, &b.left]
        .into_iter()
        .find(|s| !s.color_is_currentcolor && s.color.a != 0)
        .map(|s| s.color);

    let strokes = match border_color {
        Some(color) => Paints::new([solid(color)]),
        None => return (Paints::default(), StrokeWidth::None, StrokeStyle::default()),
    };

    // Stroke width: use rectangular if sides differ, uniform otherwise
    let stroke_width = if top_w == right_w && right_w == bottom_w && bottom_w == left_w {
        StrokeWidth::Uniform(top_w)
    } else {
        StrokeWidth::Rectangular(RectangularStrokeWidth {
            stroke_top_width: top_w,
            stroke_right_width: right_w,
            stroke_bottom_width: bottom_w,
            stroke_left_width: left_w,
        })
    };

    // Stroke style: map border-style to dash array
    let dash_array = match b.top.style {
        CssBorderStyle::Dashed => Some(StrokeDashArray(vec![4.0, 4.0])),
        CssBorderStyle::Dotted => Some(StrokeDashArray(vec![1.0, 1.0])),
        _ => None,
    };

    let stroke_style = StrokeStyle {
        stroke_align: StrokeAlign::Inside,
        stroke_cap: StrokeCap::Butt,
        stroke_join: StrokeJoin::Miter,
        stroke_miter_limit: StrokeMiterLimit::default(),
        stroke_dash_array: dash_array,
    };

    (strokes, stroke_width, stroke_style)
}

/// CSS effects (box-shadow, filter, backdrop-filter) → [`LayerEffects`].
///
/// Box shadows first (source order), then `filter` drop-shadows; the
/// last `filter: blur()` and the last `backdrop-filter: blur()` win.
/// Color-matrix filter functions (brightness, contrast, …) have no
/// layer-effect equivalent and are dropped, as before.
pub(super) fn effects_from(el: &StyledElement) -> LayerEffects {
    let mut shadows = Vec::new();
    let mut blur = None;
    let mut backdrop_blur = None;

    for s in &el.box_shadow {
        let fe = FeShadow {
            dx: s.offset_x,
            dy: s.offset_y,
            blur: s.blur,
            spread: s.spread,
            color: s.color,
            active: true,
        };
        shadows.push(if s.inset {
            FilterShadowEffect::InnerShadow(fe)
        } else {
            FilterShadowEffect::DropShadow(fe)
        });
    }

    for f in &el.filter {
        match f {
            FilterFunction::Blur(px) => blur = Some(FeLayerBlur::from(*px)),
            FilterFunction::DropShadow {
                offset_x,
                offset_y,
                blur: shadow_blur,
                color,
            } => {
                shadows.push(FilterShadowEffect::DropShadow(FeShadow {
                    dx: *offset_x,
                    dy: *offset_y,
                    blur: *shadow_blur,
                    spread: 0.0,
                    color: *color,
                    active: true,
                }));
            }
            _ => {}
        }
    }

    for f in &el.backdrop_filter {
        if let FilterFunction::Blur(px) = f {
            backdrop_blur = Some(FeBackdropBlur::from(*px));
        }
    }

    LayerEffects {
        blur,
        backdrop_blur,
        shadows,
        glass: None,
        noises: Vec::new(),
    }
}

/// CSS `mix-blend-mode` → [`LayerBlendMode`]. `normal` is
/// `PassThrough`, everything else wraps 1:1.
pub(super) fn blend_mode_from(el: &StyledElement) -> LayerBlendMode {
    match el.blend_mode {
        BlendMode::Normal => LayerBlendMode::PassThrough,
        mode => LayerBlendMode::Blend(mode),
    }
}

/// CSS margin → the importer's margin-surgery input. The record stores
/// each side as `Auto` or a resolved px length (percent margins resolve
/// to 0 at extraction, as the old walk did).
pub(super) fn margin_from(el: &StyledElement) -> CSSMargin {
    let side = |v: CssLength| -> (f32, bool) {
        match v {
            CssLength::Auto => (0.0, true),
            CssLength::Px(p) => (p, false),
            _ => (0.0, false),
        }
    };
    let (top, top_auto) = side(el.margin.top);
    let (right, right_auto) = side(el.margin.right);
    let (bottom, bottom_auto) = side(el.margin.bottom);
    let (left, left_auto) = side(el.margin.left);
    CSSMargin {
        top,
        right,
        bottom,
        left,
        top_auto,
        right_auto,
        bottom_auto,
        left_auto,
    }
}
