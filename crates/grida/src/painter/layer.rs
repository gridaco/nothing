use super::geometry::PainterShape;
use crate::cg::prelude::*;
use crate::node::id::NodeId;
use crate::vectornetwork::VectorNetwork;
use math2::{rect::Rectangle, transform::AffineTransform};

/// A Skia-friendly, cacheable picture layer for vector rendering.
///
/// `PainterPictureLayer` represents a flattened, self-contained unit of vector draw commands,
/// recorded as a Skia `SkPicture`. It is designed for reuse across multiple frames or draw passes,
/// enabling high-performance rendering via picture caching.
///
/// This is the first step of isolating draw content from rendering context (transform, opacity, blend),
/// allowing layers to be reused with different composite properties (see `LayerUsage`).
///
/// ## Characteristics
///
/// - Contains **pure draw content** (shape, paint, effects)
/// - Does **not** include transform, opacity, blend mode, clip
/// - Can be recorded once and reused as `SkPicture` or rendered live
/// - Effects like blur/shadow are baked into the picture if needed
///
/// ## Use Cases
///
/// - Caching static shape trees (e.g. icons, frames, symbols)
/// - Re-recording affected subtrees for dirty region rendering
/// - Serving as a source input for tile-based compositing
///
/// ## Typical Workflow
///
/// 1. Compile scene node(s) into a `PainterPictureLayer`
/// 2. Record its content into a `SkPicture`
/// 3. On each frame, draw the cached picture with:
///     - transform
///     - opacity
///     - blend mode
///
/// ## Example
///
/// ```rust,ignore
/// // Layer definition
/// let layer = PainterPictureLayer {
///     shape: shape,
///     fills: Paints::new([fill]),
///     strokes: Paints::new([stroke]),
///     effects: vec![],
/// };
///
/// // Record
/// let picture = record_to_sk_picture(&layer);
///
/// // Use
/// canvas.save();
/// canvas.concat(transform);
/// canvas.save_layer_alpha(...);
/// canvas.draw_picture(&picture, None, None);
/// canvas.restore();
/// canvas.restore();
/// ```
///
/// ## See Also
/// - [`LayerUsage`] — carries per-frame composite state (transform, opacity)
/// - [`RenderCommand`] — full rendering instruction with resolved state
/// - [`PainterShape`] — resolved shape geometry abstraction
#[derive(Debug, Clone)]
pub enum PainterPictureLayer {
    Shape(PainterPictureShapeLayer),
    Text(PainterPictureTextLayer),
    Vector(PainterPictureVectorLayer),
    MarkdownEmbed(PainterPictureMarkdownEmbedLayer),
    HtmlEmbed(PainterPictureHtmlEmbedLayer),
}

impl PainterPictureLayer {
    /// Mutable access to the layer's shared base (id, z-index, opacity,
    /// blend mode, transform, clip path).
    ///
    /// Used by partial-invalidation paths to patch a layer's cached
    /// world transform in place when only its geometry changed —
    /// avoids rebuilding the entire `LayerList`.
    #[inline]
    pub fn base_mut(&mut self) -> &mut PainterPictureLayerBase {
        match self {
            PainterPictureLayer::Shape(layer) => &mut layer.base,
            PainterPictureLayer::Text(layer) => &mut layer.base,
            PainterPictureLayer::Vector(layer) => &mut layer.base,
            PainterPictureLayer::MarkdownEmbed(layer) => &mut layer.base,
            PainterPictureLayer::HtmlEmbed(layer) => &mut layer.base,
        }
    }

    /// Returns true when the layer has no effects that would produce different
    /// `SkPicture` recordings for different `EffectQuality` levels.
    ///
    /// When effects are empty, the reduced-quality and full-quality render
    /// variants produce identical `SkPicture` byte streams. The picture cache
    /// can safely store such nodes under `variant_key = 0` (default store)
    /// regardless of the active render policy, avoiding redundant re-recording
    /// when switching between stable and unstable frames.
    #[inline]
    pub fn effects_empty(&self) -> bool {
        match self {
            PainterPictureLayer::Shape(s) => s.effects.is_empty(),
            PainterPictureLayer::Text(t) => t.effects.is_empty(),
            PainterPictureLayer::Vector(v) => v.effects.is_empty(),
            PainterPictureLayer::MarkdownEmbed(m) => m.effects.is_empty(),
            PainterPictureLayer::HtmlEmbed(h) => h.effects.is_empty(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum PainterRenderCommand {
    Draw(PainterPictureLayer),
    MaskGroup(PainterMaskGroup),
    /// A render surface: draws children into an offscreen buffer, then applies
    /// surface-level effects (shadows, blur) to the composited result.
    ///
    /// This is the core optimization from Phase 3 of the renderer rewrite.
    /// Instead of applying expensive effects per-child (N × 220µs), we draw
    /// all children as simple geometry into a surface, then apply the effect
    /// once (1 × 220µs).
    RenderSurface(PainterRenderSurface),
}

/// A render surface that composites children before applying effects.
///
/// Created during `LayerList::from_scene()` when the effect tree identifies
/// a container/group node that needs a render surface for effects.
#[derive(Debug, Clone)]
pub struct PainterRenderSurface {
    /// The node ID of the container/group that owns this surface.
    pub id: NodeId,
    /// World-space bounds of the surface (union of all children + effect expansion).
    pub bounds: Rectangle,
    /// World transform of the surface owner.
    pub transform: AffineTransform,
    /// Opacity of the surface (applied when compositing into parent).
    pub opacity: f32,
    /// Blend mode of the surface (applied when compositing into parent).
    pub blend_mode: LayerBlendMode,
    /// Effects to apply to the composited surface content.
    /// These are the container-level effects that triggered the render surface.
    pub effects: LayerEffects,
    /// Clip path from ancestor containers (if any).
    pub clip_path: Option<skia_safe::Path>,
    /// The container's own draw command (its background fills/strokes).
    /// Drawn first, before children, inside the render surface.
    pub own_layer: Option<PainterPictureLayer>,
    /// Child commands to draw into the surface before applying effects.
    pub children: Vec<PainterRenderCommand>,
}

#[derive(Debug, Clone)]
pub struct PainterMaskGroup {
    pub mask_type: LayerMaskType,
    pub mask_commands: Vec<PainterRenderCommand>,
    pub content_commands: Vec<PainterRenderCommand>,
}

pub trait Layer {
    fn id(&self) -> &NodeId;
    fn z_index(&self) -> usize;
    fn transform(&self) -> AffineTransform;
    fn shape(&self) -> &PainterShape;
}

impl Layer for PainterPictureLayer {
    fn id(&self) -> &NodeId {
        match self {
            PainterPictureLayer::Shape(layer) => &layer.base.id,
            PainterPictureLayer::Text(layer) => &layer.base.id,
            PainterPictureLayer::Vector(layer) => &layer.base.id,
            PainterPictureLayer::MarkdownEmbed(layer) => &layer.base.id,
            PainterPictureLayer::HtmlEmbed(layer) => &layer.base.id,
        }
    }

    fn z_index(&self) -> usize {
        match self {
            PainterPictureLayer::Shape(layer) => layer.base.z_index,
            PainterPictureLayer::Text(layer) => layer.base.z_index,
            PainterPictureLayer::Vector(layer) => layer.base.z_index,
            PainterPictureLayer::MarkdownEmbed(layer) => layer.base.z_index,
            PainterPictureLayer::HtmlEmbed(layer) => layer.base.z_index,
        }
    }

    fn transform(&self) -> AffineTransform {
        match self {
            PainterPictureLayer::Shape(layer) => layer.base.transform,
            PainterPictureLayer::Text(layer) => layer.base.transform,
            PainterPictureLayer::Vector(layer) => layer.base.transform,
            PainterPictureLayer::MarkdownEmbed(layer) => layer.base.transform,
            PainterPictureLayer::HtmlEmbed(layer) => layer.base.transform,
        }
    }

    fn shape(&self) -> &PainterShape {
        match self {
            PainterPictureLayer::Shape(layer) => &layer.shape,
            PainterPictureLayer::Vector(layer) => &layer.shape,
            PainterPictureLayer::Text(layer) => &layer.shape,
            PainterPictureLayer::MarkdownEmbed(layer) => &layer.shape,
            PainterPictureLayer::HtmlEmbed(layer) => &layer.shape,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PainterPictureLayerBase {
    pub id: NodeId,
    pub z_index: usize,
    pub opacity: f32,
    pub blend_mode: LayerBlendMode,
    pub transform: AffineTransform,
    pub clip_path: Option<skia_safe::Path>,
}

#[derive(Debug, Clone)]
pub struct PainterPictureShapeLayer {
    pub base: PainterPictureLayerBase,
    pub shape: PainterShape,
    pub effects: LayerEffects,
    pub strokes: Paints,
    pub fills: Paints,
    pub stroke_path: Option<skia_safe::Path>,
    /// Marker shape at the start endpoint (line nodes).
    pub marker_start_shape: StrokeMarkerPreset,
    /// Marker shape at the end endpoint (line nodes).
    pub marker_end_shape: StrokeMarkerPreset,
    /// Stroke width needed for decoration sizing.
    pub stroke_width: f32,
    /// Whether the stroke geometry overlaps the fill area (Inside or Center).
    /// When true AND both fills and strokes are present, the paint-alpha opacity
    /// folding fast path cannot be used because applying opacity independently
    /// to fill and stroke paints produces a visible compositing artifact in the
    /// overlap region (double-blending, up to 64 channel diff).
    ///
    /// Per the SVG/CSS spec (and Chromium's implementation), node-level opacity
    /// requires group isolation (save_layer): fill+stroke are drawn at full
    /// opacity into an offscreen surface, then composited at the node's opacity.
    /// Only Outside strokes have zero geometric overlap and can safely use
    /// per-paint-alpha.
    ///
    /// See `docs/wg/feat-2d/stroke-fill-opacity.md`.
    pub stroke_overlaps_fill: bool,
    /// Pre-computed fill path with stroke region subtracted (PathOp::Difference).
    ///
    /// When stroke overlaps fill (Inside/Center) and both fills and strokes are
    /// present, drawing this path instead of the full fill path eliminates the
    /// overlap region. This allows per-paint-alpha opacity folding (zero GPU
    /// surfaces) while producing output identical to `save_layer_alpha`.
    ///
    /// `None` when no overlap exists, or when PathOp fails on degenerate geometry.
    /// In the `None` + overlap case, the painter falls back to `save_layer_alpha`.
    pub non_overlapping_fill_path: Option<skia_safe::Path>,
}

#[derive(Debug, Clone)]
pub struct PainterPictureTextLayer {
    pub base: PainterPictureLayerBase,
    pub effects: LayerEffects,
    pub strokes: Paints,
    pub fills: Paints,
    pub stroke_width: f32,
    pub stroke_align: StrokeAlign,
    pub stroke_path: Option<skia_safe::Path>,
    pub shape: PainterShape,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub max_lines: Option<usize>,
    pub ellipsis: Option<String>,
    pub text: String,
    pub text_style: TextStyleRec,
    pub text_align: TextAlign,
    pub text_align_vertical: TextAlignVertical,
    pub id: NodeId,
    /// When set, the text is rendered using per-run attributed styling
    /// instead of the uniform `text_style` + node-level `fills`/`strokes`.
    pub attributed_string: Option<AttributedString>,
}

#[derive(Debug, Clone)]
pub struct PainterPictureVectorLayer {
    pub base: PainterPictureLayerBase,
    pub effects: LayerEffects,
    pub strokes: Paints,
    pub fills: Paints,
    pub shape: PainterShape,
    pub vector: VectorNetwork,
    pub stroke_width: f32,
    pub stroke_align: StrokeAlign,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub stroke_miter_limit: StrokeMiterLimit,
    pub stroke_width_profile: Option<crate::cg::varwidth::VarWidthProfile>,
    pub stroke_dash_array: Option<StrokeDashArray>,
    pub corner_radius: f32,
    /// Marker shape at the start endpoint (first vertex).
    pub marker_start_shape: StrokeMarkerPreset,
    /// Marker shape at the end endpoint (last vertex).
    pub marker_end_shape: StrokeMarkerPreset,
}

/// A painter layer for Markdown content rendered via the `htmlcss` pipeline.
///
/// The markdown source is carried here so the painter can convert it to
/// HTML+CSS via `htmlcss::markdown_to_styled_html()` and render at draw time.
#[derive(Debug, Clone)]
pub struct PainterPictureMarkdownEmbedLayer {
    pub base: PainterPictureLayerBase,
    pub effects: LayerEffects,
    pub shape: PainterShape,
    /// Background fills for the markdown container.
    pub fills: Paints,
    /// GFM markdown source text.
    pub markdown: String,
    /// Layout width for text wrapping.
    pub width: f32,
    /// Layout height for clipping.
    pub height: f32,
}

/// A painter layer for HTML+CSS content rendered directly to a Skia Picture.
///
/// The HTML source is carried here so the painter can call
/// `htmlcss::render()` at draw time and cache the result.
#[derive(Debug, Clone)]
pub struct PainterPictureHtmlEmbedLayer {
    pub base: PainterPictureLayerBase,
    pub effects: LayerEffects,
    pub shape: PainterShape,
    /// Background fills for the HTML embed container.
    pub fills: Paints,
    /// Raw HTML+CSS source text.
    pub html: String,
    /// Layout width for text wrapping.
    pub width: f32,
    /// Layout height for clipping.
    pub height: f32,
}

/// A layer with its associated node ID.
/// This pairs a layer with its source node ID, eliminating the need to store ID in the layer itself.
#[derive(Debug, Clone)]
pub struct LayerEntry {
    pub id: NodeId,
    pub layer: PainterPictureLayer,
}

/// Flat list of [`PainterPictureLayer`] entries with their IDs.
#[derive(Debug, Default, Clone)]
pub struct LayerList {
    pub layers: Vec<LayerEntry>,
    pub commands: Vec<PainterRenderCommand>,
}

impl LayerList {
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn filter(&self, filter: impl Fn(&PainterPictureLayer) -> bool) -> Self {
        let mut list = LayerList::default();
        for indexed in &self.layers {
            if filter(&indexed.layer) {
                list.layers.push(indexed.clone());
            }
        }
        list
    }
}
