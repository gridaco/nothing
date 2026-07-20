// Grida's own SVG Types (that with unique properties)

use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SVGTextAnchor {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "middle")]
    Middle,
    #[serde(rename = "end")]
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SVGPaint {
    #[serde(rename = "solid")]
    Solid(SVGSolidPaint),
    #[serde(rename = "linear-gradient")]
    LinearGradient(SVGLinearGradientPaint),
    #[serde(rename = "radial-gradient")]
    RadialGradient(SVGRadialGradientPaint),
}

impl SVGPaint {
    pub const TRANSPARENT: Self = Self::Solid(SVGSolidPaint {
        color: CGColor::TRANSPARENT,
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVGSolidPaint {
    pub color: CGColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVGLinearGradientPaint {
    pub id: String,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub transform: CGTransform2D,
    pub stops: Vec<GradientStop>,
    pub spread_method: SVGGradientSpreadMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVGRadialGradientPaint {
    pub id: String,
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub fx: f32,
    pub fy: f32,
    pub transform: CGTransform2D,
    pub stops: Vec<GradientStop>,
    pub spread_method: SVGGradientSpreadMethod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SVGGradientSpreadMethod {
    #[serde(rename = "pad")]
    Pad,
    #[serde(rename = "reflect")]
    Reflect,
    #[serde(rename = "repeat")]
    Repeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVGFillAttributes {
    /// [`fill`] property
    ///
    /// [`fill`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill
    pub paint: SVGPaint,
    /// [`fill-opacity`] property
    ///
    /// [`fill-opacity`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-opacity
    pub fill_opacity: f32,
    // [`fill-rule`] property
    ///
    /// [`fill-rule`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-rule
    pub fill_rule: FillRule,
}

/// SVG stroke, stroke-* attributes definition as-is, following the SVG spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SVGStrokeAttributes {
    /// [`stroke`] property
    ///
    /// [`stroke`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke
    pub paint: SVGPaint,
    /// [`stroke-width`] property
    ///
    /// [`stroke-width`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-width
    pub stroke_width: f32,
    /// [`stroke-linecap`] property
    ///
    /// [`stroke-linecap`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linecap
    pub stroke_linecap: StrokeCap,
    /// [`stroke-linejoin`] property
    ///
    /// [`stroke-linejoin`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linejoin
    pub stroke_linejoin: StrokeJoin,
    /// [`stroke-miterlimit`] property
    ///
    /// [`stroke-miterlimit`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-miterlimit
    pub stroke_miterlimit: StrokeMiterLimit,
    /// [`stroke-dasharray`] property
    ///
    /// [`stroke-dasharray`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dasharray
    pub stroke_dasharray: Option<StrokeDashArray>,
    /// [`stroke-opacity`] property
    ///
    /// [`stroke-opacity`]: https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-opacity
    pub stroke_opacity: f32,
}

impl Default for SVGStrokeAttributes {
    fn default() -> Self {
        Self {
            paint: SVGPaint::TRANSPARENT,
            stroke_width: 1.0,
            stroke_linecap: StrokeCap::default(),
            stroke_linejoin: StrokeJoin::default(),
            stroke_miterlimit: StrokeMiterLimit::default(),
            stroke_dasharray: None,
            stroke_opacity: 1.0,
        }
    }
}

// SVG Packed Scene is dedicated struct for archive / transport format of resolved SVG file.
// rules:
//   - size efficient: table-like structure similar to ttf
// pub struct SVGPackedScene {
//   images
//   paints
//   nodes
// }

/// Intermediate Representation of an SVG node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IRSVGChildNode {
    #[serde(rename = "group")]
    Group(IRSVGGroupNode),
    #[serde(rename = "text")]
    Text(IRSVGTextNode),
    #[serde(rename = "path")]
    Path(IRSVGPathNode),
    #[serde(rename = "image")]
    Image(IRSVGImageNode),
}

/// <svg> (root)
/// nested <svg> will be treated as <g> (IRSVGGroupNode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGInitialContainerNode {
    pub width: f32,
    pub height: f32,
    pub children: Vec<IRSVGChildNode>,
}

/// <g>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGGroupNode {
    pub transform: CGTransform2D,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub children: Vec<IRSVGChildNode>,
    // filters
}

/// <text>
///
/// SVG `<text>` element IR representation.
///
/// Contains one or more positioned chunks. Each chunk is either uniform
/// (single style) or attributed (per-span variation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGTextNode {
    pub transform: CGTransform2D,
    pub text_content: String,
    pub fill: Option<SVGFillAttributes>,
    pub stroke: Option<SVGStrokeAttributes>,
    pub chunks: Vec<IRSVGTextChunk>,
    pub bounds: CGRect,
}

/// A positioned text chunk — either uniform (single style) or attributed
/// (per-span style variation).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRSVGTextChunk {
    /// Single-style chunk → packs to `TextSpanNode`.
    Uniform(IRSVGTextSpanNode),
    /// Multi-style chunk → packs to `AttributedTextNode`.
    Attributed(IRSVGAttributedTextChunk),
}

/// `<tspan>` — a positioned text chunk with uniform styling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGTextSpanNode {
    pub transform: CGTransform2D,
    pub text: String,
    pub fill: Option<SVGFillAttributes>,
    pub stroke: Option<SVGStrokeAttributes>,
    pub font_size: Option<f32>,
    pub anchor: SVGTextAnchor,
}

/// A positioned text chunk with per-span style variation.
///
/// Produced when a `<text>` chunk contains multiple `<tspan>` children with
/// different font/fill/stroke attributes. Each span maps to a
/// [`IRSVGTextStyledRun`] with byte offsets into the chunk text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGAttributedTextChunk {
    pub transform: CGTransform2D,
    /// Full text content of the chunk (untrimmed from usvg).
    pub text: String,
    pub anchor: SVGTextAnchor,
    /// Per-span styled runs (byte offsets into `text`).
    pub runs: Vec<IRSVGTextStyledRun>,
}

/// A styled sub-range within a text chunk, derived from a usvg `TextSpan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGTextStyledRun {
    /// Byte offset (start, inclusive) into the parent chunk's text.
    pub start: usize,
    /// Byte offset (end, exclusive) into the parent chunk's text.
    pub end: usize,
    pub fill: Option<SVGFillAttributes>,
    pub stroke: Option<SVGStrokeAttributes>,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_style: SVGFontStyle,
    pub font_family: String,
    pub letter_spacing: f32,
    pub word_spacing: f32,
}

/// SVG font-style mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SVGFontStyle {
    Normal,
    Italic,
    Oblique,
}

/// <path>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGPathNode {
    pub transform: CGTransform2D,
    pub fill: Option<SVGFillAttributes>,
    pub stroke: Option<SVGStrokeAttributes>,
    pub d: String,
    #[serde(skip_serializing, default)]
    pub bounds: CGRect,
}

/// <image>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSVGImageNode {}
