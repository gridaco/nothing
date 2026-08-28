//! ENG-2.1 · the display list — a pure, diffable projection of resolved
//! geometry and authored paint intent. The engine frame boundary supplies the
//! exact fonts accompanying shaped text. [`build_glyphless_unchecked`] exists only for
//! deterministic lab and structural probes whose resolver deliberately emits
//! no glyphs. Neither path performs I/O or raster work. The camera is not baked
//! in, so one drawlist paints at any zoom.

use n0_model::math::Affine;
use n0_model::model::{
    CornerSmoothing, Document, NodeId, Paints, Payload, RectangularCornerRadius, ShapeDesc, Stroke,
};
use n0_model::path::ResolvedPathArtifact;
use n0_model::properties::ValueView;
use n0_model::resolve::Resolved;
use n0_model::text_layout::TextLayout;
use std::sync::Arc;

use crate::text_layout::TextFontRegistry;

/// One backend-neutral geometric contributor in a private clip program.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedClipGeometry {
    /// Geometry-local coordinates to frame space.
    pub world: Affine,
    pub kind: ResolvedClipGeometryKind,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedClipGeometryKind {
    Rect { x: f32, y: f32, w: f32, h: f32 },
    Oval { x: f32, y: f32, w: f32, h: f32 },
    Path(Arc<ResolvedPathArtifact>),
}

/// One path-operation union. Empty means a valid clip that admits no pixels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedClipLayer {
    pub geometries: Arc<[ResolvedClipGeometry]>,
}

/// The private projection of a resolved geometric clip. Layers intersect in
/// order; every layer first unions its contributors.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedClipPath {
    pub(crate) layers: Arc<[ResolvedClipLayer]>,
    /// Whether the backend should compute fractional edge coverage.
    pub(crate) anti_alias: bool,
}

/// Drawlist projection of a resolved mask-source interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedMaskMode {
    Alpha,
    Luminance,
}

/// Pixel interpolation space for one private resolved filter operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterColorSpace {
    Srgb,
    LinearRgb,
}

/// One source-neutral input in a private resolved filter program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterInput {
    Source,
    SourceAlpha,
    Node(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ResolvedFilterComposite {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Lighter,
    Arithmetic { k1: f32, k2: f32, k3: f32, k4: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterBlend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterMorphology {
    Erode,
    Dilate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterTurbulenceKind {
    Turbulence,
    FractalNoise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterDisplacementChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFilterConvolveEdgeMode {
    Duplicate,
    Wrap,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ResolvedFilterLightSource {
    Distant {
        direction: [f32; 3],
    },
    Point {
        location: [f32; 3],
    },
    Spot {
        location: [f32; 3],
        target: [f32; 3],
        falloff_exponent: f32,
        cutoff_angle: f32,
    },
}

/// The private filter-operation vocabulary admitted by the painter.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedFilterPrimitive {
    GaussianBlur {
        sigma_x: f32,
        sigma_y: f32,
    },
    Offset {
        dx: f32,
        dy: f32,
    },
    SolidColor {
        color: n0_model::model::Color32F,
    },
    Composite {
        operator: ResolvedFilterComposite,
    },
    Blend {
        mode: ResolvedFilterBlend,
    },
    DropShadow {
        dx: f32,
        dy: f32,
        sigma_x: f32,
        sigma_y: f32,
        color: n0_model::model::Color32F,
    },
    ColorMatrix {
        matrix: [f32; 20],
    },
    ComponentTransfer {
        /// R, G, B, then A. The source-neutral frame contract names this
        /// order before projection into the private drawlist.
        tables: Arc<[[u8; 256]; 4]>,
    },
    Morphology {
        operator: ResolvedFilterMorphology,
        radius_x: f32,
        radius_y: f32,
    },
    Turbulence {
        kind: ResolvedFilterTurbulenceKind,
        base_frequency_x: f32,
        base_frequency_y: f32,
        num_octaves: u8,
        seed: f32,
        stitch_tiles: bool,
    },
    DisplacementMap {
        scale: f32,
        x_channel: ResolvedFilterDisplacementChannel,
        y_channel: ResolvedFilterDisplacementChannel,
    },
    ConvolveMatrix {
        order_x: u16,
        order_y: u16,
        kernel: Arc<[f32]>,
        gain: f32,
        bias: f32,
        target_x: u16,
        target_y: u16,
        edge_mode: ResolvedFilterConvolveEdgeMode,
        preserve_alpha: bool,
    },
    DiffuseLighting {
        surface_scale: f32,
        diffuse_constant: f32,
        color: n0_model::model::Color,
        light: ResolvedFilterLightSource,
    },
    Merge,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedFilterNode {
    pub inputs: Arc<[ResolvedFilterInput]>,
    pub region: n0_model::math::RectF,
    pub color_space: ResolvedFilterColorSpace,
    pub primitive: ResolvedFilterPrimitive,
}

/// One checked filter program and its hard local effect region.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFilter {
    pub(crate) region: n0_model::math::RectF,
    pub(crate) nodes: Arc<[ResolvedFilterNode]>,
    /// Whether the graph can create output from a fully transparent source.
    /// Such a graph needs one explicit transparent source raster even when
    /// its scope contains no draw item, or a lazy layer restore is skipped.
    pub(crate) may_paint_transparent_input: bool,
    /// The invocation's isolated source is known to be fully transparent.
    /// Source references are materialized explicitly so generated/additive
    /// graph output is not skipped by a lazy backend layer.
    pub(crate) source_is_transparent: bool,
}

/// Product-local owner slot for a source-neutral glyphless frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GlyphlessOwnerSlot(u32);

impl GlyphlessOwnerSlot {
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One private source-neutral repeating vector program.
///
/// The nested drawlist is already compiled and preflighted. Its frame clip is
/// the tile cell `(0, 0, width, height)`; `transform` maps that tile-local
/// coordinate system into the consuming geometry's local space before both
/// axes repeat.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPattern {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) transform: Affine,
    pub(crate) program: Arc<DrawList<GlyphlessOwnerSlot>>,
    pub(crate) opacity: f32,
}

/// Absolute geometry in the consuming node's local coordinate system.
/// Pattern items keep that origin in geometry instead of folding it into
/// `world`, because the pattern mapping is stated in the same local space.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedPatternGeometry {
    Rect { x: f32, y: f32, w: f32, h: f32 },
    Oval { x: f32, y: f32, w: f32, h: f32 },
    Path(Arc<ResolvedPathArtifact>),
}

/// One canonical, source-neutral dash phase carried by private stroke
/// material.
///
/// The producer that resolves a nonzero phase owns normalization into its
/// positive interval cycle. The painter consumes this scalar verbatim: it
/// neither parses source syntax nor applies a second modulo. Ordinary
/// [`n0_model`] strokes have no phase vocabulary and therefore project as
/// [`StrokeDashPhase::ZERO`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeDashPhase(f32);

impl StrokeDashPhase {
    pub const ZERO: Self = Self(0.0);

    /// Wrap one already-canonical phase without reinterpreting it.
    pub(crate) fn from_canonical(value: f32) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!(value >= 0.0);
        Self(value)
    }

    /// The local-space offset into the paired dash interval cycle.
    pub const fn value(self) -> f32 {
        self.0
    }
}

/// One checked alpha stage applied after each paint's intrinsic alpha has
/// materialized.
///
/// This is leaf-paint material, not a compositing scope: every paint in the
/// item receives the factor independently after its shader and intrinsic paint
/// alpha materialize, but before coverage and compositing. Ordinary
/// [`n0_model`] producers have no such fact and therefore project as
/// [`PostPaintOpacity::IDENTITY`], which is a true no-op rather than an extra
/// quantization stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostPaintOpacity(f32);

impl PostPaintOpacity {
    pub const IDENTITY: Self = Self(1.0);

    /// Copy one producer-checked factor without re-resolving or quantizing it.
    pub(crate) fn from_resolved(value: f32) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!((0.0..=1.0).contains(&value));
        Self(value)
    }

    /// The post-materialization alpha factor.
    pub const fn value(self) -> f32 {
        self.0
    }
}

/// One paint primitive, carrying the world transform copied verbatim from its
/// resolved producer (never recomputed — pixel identity depends on it) and the
/// geometry in the visual's own local space.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<K = NodeId> {
    /// Diagnostic ownership is generic and is never read by the painter.
    ///
    /// Ordinary lists retain their authored [`NodeId`]. An engine-private
    /// source-neutral product uses a product-local slot instead, without
    /// fabricating an n0 node or changing the draw-item vocabulary.
    pub node: K,
    pub world: Affine,
    pub kind: ItemKind,
}

/// Paint ownership for one resolved text layout.
///
/// Shaping ignores paint values, but attributed run boundaries remain indexed
/// in the resolved glyph runs. An outer `None` denotes uniform text, while an
/// attributed entry of `None` inherits node paints and `Some([])` means
/// explicit no ink. Keeping those states distinct lets invalid attributed run
/// ownership fail closed instead of being mistaken for uniform-text fallback.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPaints {
    pub node: Paints,
    pub runs: Option<Vec<Option<Paints>>>,
}

impl TextPaints {
    /// Resolve paints only when glyph ownership matches the authored topology.
    pub fn for_source_run(&self, source_run: Option<usize>) -> Option<&Paints> {
        match (&self.runs, source_run) {
            (None, None) => Some(&self.node),
            (Some(runs), Some(index)) => runs
                .get(index)
                .map(|paints| paints.as_ref().unwrap_or(&self.node)),
            _ => None,
        }
    }

    fn has_visible_ink(&self) -> bool {
        match &self.runs {
            None => !self.node.is_empty(),
            Some(runs) => runs
                .iter()
                .any(|paints| !paints.as_ref().unwrap_or(&self.node).is_empty()),
        }
    }
}

/// The display-list vocabulary. Scope commands are explicit so subtree opacity
/// is composited as a group and a container's content clip can end before its
/// own strokes are painted. Geometry and paint coordinates stay in node-local
/// space; the executor applies `world` and the host view.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// The model's node opacity: the group layer is initialized with the
    /// backdrop so descendant paint blend modes see the accumulated result,
    /// and the restore blends `opacity·group + (1−opacity)·backdrop`
    /// arithmetically.
    BeginOpacity {
        opacity: f32,
    },
    /// The Web's group opacity: an **isolated** layer that starts empty, so
    /// contents blend only among themselves, and the restore composites the
    /// layer source-over at this opacity. Byte-distinct from
    /// [`ItemKind::BeginOpacity`] (measured: the two quantize one code value
    /// apart against the Chromium oracle), which is why it is a second
    /// meaning and not a flag.
    BeginIsolatedOpacity {
        opacity: f32,
    },
    /// Closes the innermost opacity scope, of either meaning.
    EndOpacity,
    BeginClipRect {
        w: f32,
        h: f32,
        corner_radius: RectangularCornerRadius,
        corner_smoothing: CornerSmoothing,
    },
    /// A source-neutral path-strategy clip, already resolved into frame-space
    /// geometry and bounded union/intersection layers.
    BeginClipPath {
        clip: Arc<ResolvedClipPath>,
    },
    EndClip,
    /// Open the isolated target composite of a resolved image mask.
    BeginMaskContent,
    /// Switch from target painting to the isolated mask-source image. The
    /// source is clipped to `region`, then restored through DstIn; luminance
    /// mode first converts source color to alpha.
    BeginMaskSource {
        mode: ResolvedMaskMode,
        region: Arc<ResolvedClipPath>,
    },
    EndMaskSource,
    EndMaskContent,
    /// Open an isolated group whose composite is evaluated by one checked
    /// image-filter program. `world` maps the program's local operation space
    /// into world space.
    BeginFilter {
        filter: Arc<ResolvedFilter>,
    },
    EndFilter,
    /// Fill absolute local geometry through one checked repeat program.
    PatternFill {
        geometry: ResolvedPatternGeometry,
        pattern: Arc<ResolvedPattern>,
        post_paint_opacity: PostPaintOpacity,
    },
    /// Stroke absolute local geometry through one checked repeat program.
    PatternStroke {
        geometry: ResolvedPatternGeometry,
        pattern: Arc<ResolvedPattern>,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    RectFill {
        w: f32,
        h: f32,
        corner_radius: RectangularCornerRadius,
        corner_smoothing: CornerSmoothing,
        paints: Paints,
        post_paint_opacity: PostPaintOpacity,
    },
    OvalFill {
        w: f32,
        h: f32,
        paints: Paints,
        post_paint_opacity: PostPaintOpacity,
    },
    PathFill {
        w: f32,
        h: f32,
        path: Arc<ResolvedPathArtifact>,
        paints: Paints,
        post_paint_opacity: PostPaintOpacity,
    },
    TextFill {
        layout: Arc<TextLayout>,
        paints: TextPaints,
        paint_w: f32,
        paint_h: f32,
        post_paint_opacity: PostPaintOpacity,
    },
    RectStroke {
        w: f32,
        h: f32,
        corner_radius: RectangularCornerRadius,
        corner_smoothing: CornerSmoothing,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    OvalStroke {
        w: f32,
        h: f32,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    /// A dashed resolved-frame ellipse whose absolute local coordinates must
    /// survive until Skia constructs the oval path effect. Folding `(x, y)`
    /// into `world` first is algebraically equal but not f32-equivalent.
    AbsoluteDashedOvalStroke {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    LineStroke {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        paint_w: f32,
        paint_h: f32,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    PathStroke {
        w: f32,
        h: f32,
        path: Arc<ResolvedPathArtifact>,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
    TextStroke {
        layout: Arc<TextLayout>,
        paint_w: f32,
        paint_h: f32,
        stroke: Stroke,
        dash_phase: StrokeDashPhase,
        post_paint_opacity: PostPaintOpacity,
    },
}

/// The whole scene as an ordered primitive stream, in paint order
/// (node fill, clipped children, then authored strokes). Diffable by `==`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawList<K = NodeId> {
    pub items: Vec<Item<K>>,
    /// Exact fonts referenced by glyph-bearing text items. Kept opaque outside
    /// the engine: display-list consumers replay keys only through this list.
    text_fonts: Option<Arc<TextFontRegistry>>,
}

impl<K> DrawList<K> {
    /// Mint a list with no glyph replay registry.
    pub(crate) fn from_items(items: Vec<Item<K>>) -> Self {
        Self {
            items,
            text_fonts: None,
        }
    }

    pub(crate) fn text_fonts(&self) -> &TextFontRegistry {
        self.text_fonts
            .as_deref()
            .expect("glyph-bearing drawlist has no text font registry")
    }

    pub(crate) fn same_text_fonts<L>(&self, other: &DrawList<L>) -> bool {
        self.text_fonts == other.text_fonts
    }

    /// Test-only raster identity used by the preview-cache join probe.
    ///
    /// Every paint-consumed field and the private font replay registry
    /// participate. Only `Item::node` is ignored because replay does not read
    /// it. Reuse therefore retains the cached list's diagnostic node slots;
    /// gradient and image preflight errors can still report those retained
    /// slots. Promotion beyond this bounded probe requires an explicit
    /// provenance policy for those diagnostics.
    ///
    /// The exhaustive destructuring is deliberate: adding a field to either
    /// `DrawList` or `Item` must fail compilation here and force a decision
    /// about whether the field affects raster identity.
    #[cfg(test)]
    pub(crate) fn raster_eq<L>(&self, other: &DrawList<L>) -> bool {
        let DrawList {
            items: left_items,
            text_fonts: left_text_fonts,
        } = self;
        let DrawList {
            items: right_items,
            text_fonts: right_text_fonts,
        } = other;

        left_text_fonts == right_text_fonts
            && left_items.len() == right_items.len()
            && left_items.iter().zip(right_items).all(|(left, right)| {
                let Item {
                    node: _,
                    world: left_world,
                    kind: left_kind,
                } = left;
                let Item {
                    node: _,
                    world: right_world,
                    kind: right_kind,
                } = right;
                left_world == right_world && left_kind == right_kind
            })
    }
}

/// Materialize only paints that can contribute pixels. The authored stack is
/// already bottom-to-top; filtering preserves that relative order.
fn visible_paints(paints: &Paints) -> Paints {
    Paints::new(paints.iter().filter(|paint| paint.visible()).cloned())
}

fn visible_stroke(
    stroke: &Stroke,
    payload: &Payload,
    corner_smoothing: CornerSmoothing,
) -> Option<Stroke> {
    if !stroke.renderable_for(payload, corner_smoothing) {
        return None;
    }
    let paints = visible_paints(&stroke.paints);
    if paints.is_empty() {
        return None;
    }
    let mut stroke = stroke.clone();
    stroke.paints = paints;
    Some(stroke)
}

fn push(items: &mut Vec<Item>, node: NodeId, world: Affine, kind: ItemKind) {
    items.push(Item { node, world, kind });
}

fn materialize_text_paints(payload: &Payload, node_fills: &Paints) -> TextPaints {
    let text = payload
        .as_text()
        .expect("text paint materialization requires text");
    let runs = text.runs.map(|runs| {
        runs.iter()
            .map(|run| run.fills.as_ref().map(visible_paints))
            .collect()
    });
    TextPaints {
        node: visible_paints(node_fills),
        runs,
    }
}

/// Project a glyphless lab-resolved tier into an ordered primitive stream.
///
/// Real rendering must enter through [`crate::frame::resolve_and_build`] or
/// [`crate::frame::render`]. Traversal is exactly the spike painter's
/// (`paint_node`): a hidden subtree (`world_opt == None`) prunes; the root and
/// derived kinds (group/lens) emit no ink but their children are still visited.
/// Node opacity scopes fill, descendants, and strokes. A frame's clip scopes
/// descendants only. Geometry is local-space, positioned by `world` — copied
/// verbatim, never recomputed. The camera is applied later by the executor.
pub fn build_glyphless_unchecked(doc: &Document, resolved: &Resolved) -> DrawList {
    build_inner(doc, resolved, None)
}

/// Project the exact authored-plus-effective-value view that produced
/// `resolved`. This is the value-aware structural probe counterpart to
/// [`crate::frame::resolve_and_build_view`].
pub fn build_glyphless_view_unchecked(view: &ValueView<'_>, resolved: &Resolved) -> DrawList {
    build_inner(view, resolved, None)
}

/// Project a resolved tier produced by the engine text oracle, retaining the
/// exact registry that minted every [`n0_model::text_layout::TextFontKey`].
pub(crate) fn build_with_text_fonts(
    doc: &Document,
    resolved: &Resolved,
    text_fonts: Arc<TextFontRegistry>,
) -> DrawList {
    build_inner(doc, resolved, Some(text_fonts))
}

/// Effective-value counterpart to [`build_with_text_fonts`]. Both paths use
/// one monomorphized projection below; the authored path reads the document
/// directly instead of paying a dynamic registry lookup at every paint item.
pub(crate) fn build_with_text_fonts_view(
    view: &ValueView<'_>,
    resolved: &Resolved,
    text_fonts: Arc<TextFontRegistry>,
) -> DrawList {
    build_inner(view, resolved, Some(text_fonts))
}

/// The drawlist's complete authored/effective read surface. Keeping this
/// private makes the optimization non-semantic: there is one projection
/// algorithm and no second public paint contract.
trait DrawValues {
    fn document(&self) -> &Document;
    fn opacity(&self, id: NodeId) -> f32;
    fn clips_content(&self, id: NodeId) -> bool;
    fn corner_radius(&self, id: NodeId) -> RectangularCornerRadius;
    fn corner_smoothing(&self, id: NodeId) -> CornerSmoothing;
    fn fills(&self, id: NodeId) -> &Paints;
    fn strokes(&self, id: NodeId) -> &[Stroke];
}

impl DrawValues for Document {
    #[inline]
    fn document(&self) -> &Document {
        self
    }

    #[inline]
    fn opacity(&self, id: NodeId) -> f32 {
        self.get(id).header.opacity
    }

    #[inline]
    fn clips_content(&self, id: NodeId) -> bool {
        match self.get(id).payload {
            Payload::Frame { clips_content, .. } => clips_content,
            _ => unreachable!("drawlist requests clips-content only for frames"),
        }
    }

    #[inline]
    fn corner_radius(&self, id: NodeId) -> RectangularCornerRadius {
        self.get(id).corner_radius
    }

    #[inline]
    fn corner_smoothing(&self, id: NodeId) -> CornerSmoothing {
        self.get(id).corner_smoothing
    }

    #[inline]
    fn fills(&self, id: NodeId) -> &Paints {
        &self.get(id).fills
    }

    #[inline]
    fn strokes(&self, id: NodeId) -> &[Stroke] {
        &self.get(id).strokes
    }
}

impl DrawValues for ValueView<'_> {
    #[inline]
    fn document(&self) -> &Document {
        ValueView::document(self)
    }

    #[inline]
    fn opacity(&self, id: NodeId) -> f32 {
        ValueView::opacity(self, id)
    }

    #[inline]
    fn clips_content(&self, id: NodeId) -> bool {
        ValueView::clips_content(self, id)
    }

    #[inline]
    fn corner_radius(&self, id: NodeId) -> RectangularCornerRadius {
        ValueView::corner_radius(self, id)
    }

    #[inline]
    fn corner_smoothing(&self, id: NodeId) -> CornerSmoothing {
        ValueView::corner_smoothing(self, id)
    }

    #[inline]
    fn fills(&self, id: NodeId) -> &Paints {
        ValueView::fills(self, id)
    }

    #[inline]
    fn strokes(&self, id: NodeId) -> &[Stroke] {
        ValueView::strokes(self, id)
    }
}

fn build_inner<V: DrawValues + ?Sized>(
    values: &V,
    resolved: &Resolved,
    text_fonts: Option<Arc<TextFontRegistry>>,
) -> DrawList {
    let doc = values.document();
    let mut items = Vec::new();
    emit(values, resolved, doc.root, &mut items);
    let has_glyphs = items.iter().any(|item| match &item.kind {
        ItemKind::TextFill { layout, .. } | ItemKind::TextStroke { layout, .. } => {
            !layout.glyph_runs.is_empty()
        }
        _ => false,
    });
    assert!(
        !has_glyphs || text_fonts.is_some(),
        "glyph-bearing resolved text requires its exact font registry"
    );
    DrawList {
        items,
        text_fonts: if has_glyphs { text_fonts } else { None },
    }
}

fn emit<V: DrawValues + ?Sized>(
    values: &V,
    resolved: &Resolved,
    id: NodeId,
    items: &mut Vec<Item>,
) {
    let Some(world) = resolved.world_opt(id) else {
        return; // hidden subtree — pruned, children not visited
    };
    let doc = values.document();
    let node = doc.get(id);
    let b = resolved.box_of(id);
    let text_layout = node
        .payload
        .as_text()
        .map(|_| Arc::clone(resolved.text_layout_of(id)));
    let text_paints = node
        .payload
        .as_text()
        .map(|_| materialize_text_paints(&node.payload, values.fills(id)));

    let opacity = values.opacity(id);
    let opacity_scope = opacity != 1.0;
    if opacity_scope {
        push(items, id, world, ItemKind::BeginOpacity { opacity });
    }

    // Root is the backdrop; derived kinds have no ink. Both still recurse and
    // may establish an opacity scope around their descendants.
    if id != doc.root && !node.payload.box_is_derived() {
        match &node.payload {
            Payload::Frame { .. } => {
                let paints = visible_paints(values.fills(id));
                if !paints.is_empty() {
                    let corner_radius = values.corner_radius(id);
                    let corner_smoothing = values.corner_smoothing(id);
                    push(
                        items,
                        id,
                        world,
                        ItemKind::RectFill {
                            w: b.w,
                            h: b.h,
                            corner_radius,
                            corner_smoothing,
                            paints,
                            post_paint_opacity: PostPaintOpacity::IDENTITY,
                        },
                    );
                }
            }
            // Lines have no fill channel, and `Fills` is intentionally
            // inapplicable to them in the closed registry. Do not perform a
            // speculative fill read before selecting the shape variant.
            Payload::Shape {
                desc: ShapeDesc::Line,
            } => {}
            Payload::Shape { desc } => {
                let paints = visible_paints(values.fills(id));
                if !paints.is_empty() {
                    let kind = match desc {
                        ShapeDesc::Rect => Some(ItemKind::RectFill {
                            w: b.w,
                            h: b.h,
                            corner_radius: values.corner_radius(id),
                            corner_smoothing: values.corner_smoothing(id),
                            paints,
                            post_paint_opacity: PostPaintOpacity::IDENTITY,
                        }),
                        ShapeDesc::Ellipse => Some(ItemKind::OvalFill {
                            w: b.w,
                            h: b.h,
                            paints,
                            post_paint_opacity: PostPaintOpacity::IDENTITY,
                        }),
                        ShapeDesc::Path(_) => {
                            resolved
                                .resolved_path_opt(id)
                                .map(|path| ItemKind::PathFill {
                                    w: b.w,
                                    h: b.h,
                                    path: Arc::clone(path),
                                    paints,
                                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                                })
                        }
                        ShapeDesc::Line => unreachable!("line matched before fill lookup"),
                    };
                    if let Some(kind) = kind {
                        push(items, id, world, kind);
                    }
                }
            }
            Payload::Text { .. } | Payload::AttributedText { .. } => {
                let paints = text_paints
                    .as_ref()
                    .expect("text fill has a paint fallback table");
                if paints.has_visible_ink() {
                    push(
                        items,
                        id,
                        world,
                        ItemKind::TextFill {
                            layout: text_layout
                                .as_ref()
                                .expect("text fill has resolved layout")
                                .clone(),
                            paints: paints.clone(),
                            paint_w: b.w,
                            paint_h: b.h,
                            post_paint_opacity: PostPaintOpacity::IDENTITY,
                        },
                    );
                }
            }
            // Excluded by the box_is_derived guard above.
            Payload::Group | Payload::Lens { .. } => unreachable!(),
        }
    }

    let clip_scope = matches!(node.payload, Payload::Frame { .. }) && values.clips_content(id);
    if clip_scope {
        let corner_radius = values.corner_radius(id);
        let corner_smoothing = values.corner_smoothing(id);
        push(
            items,
            id,
            world,
            ItemKind::BeginClipRect {
                w: b.w,
                h: b.h,
                corner_radius,
                corner_smoothing,
            },
        );
    }

    for &c in &node.children {
        emit(values, resolved, c, items);
    }

    if clip_scope {
        push(items, id, world, ItemKind::EndClip);
    }

    if id != doc.root && !node.payload.box_is_derived() {
        let corner_smoothing = match &node.payload {
            Payload::Frame { .. }
            | Payload::Shape {
                desc: ShapeDesc::Rect,
            } => values.corner_smoothing(id),
            _ => CornerSmoothing::default(),
        };
        for stroke in values
            .strokes(id)
            .iter()
            .filter_map(|stroke| visible_stroke(stroke, &node.payload, corner_smoothing))
        {
            if matches!(
                &node.payload,
                Payload::Shape {
                    desc: ShapeDesc::Path(_)
                }
            ) && resolved.resolved_path_opt(id).is_none()
            {
                continue;
            }
            let kind = match &node.payload {
                Payload::Frame { .. }
                | Payload::Shape {
                    desc: ShapeDesc::Rect,
                } => ItemKind::RectStroke {
                    w: b.w,
                    h: b.h,
                    corner_radius: values.corner_radius(id),
                    corner_smoothing,
                    stroke,
                    dash_phase: StrokeDashPhase::ZERO,
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                },
                Payload::Shape {
                    desc: ShapeDesc::Ellipse,
                } => ItemKind::OvalStroke {
                    w: b.w,
                    h: b.h,
                    stroke,
                    dash_phase: StrokeDashPhase::ZERO,
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                },
                Payload::Shape {
                    desc: ShapeDesc::Line,
                } => ItemKind::LineStroke {
                    x1: 0.0,
                    y1: 0.0,
                    x2: b.w,
                    y2: 0.0,
                    paint_w: b.w,
                    paint_h: b.h,
                    stroke,
                    dash_phase: StrokeDashPhase::ZERO,
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                },
                Payload::Shape {
                    desc: ShapeDesc::Path(_),
                } => ItemKind::PathStroke {
                    w: b.w,
                    h: b.h,
                    path: Arc::clone(resolved.resolved_path_of(id)),
                    stroke,
                    dash_phase: StrokeDashPhase::ZERO,
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                },
                Payload::Text { .. } | Payload::AttributedText { .. } => ItemKind::TextStroke {
                    layout: text_layout
                        .as_ref()
                        .expect("visible text stroke has resolved layout")
                        .clone(),
                    paint_w: b.w,
                    paint_h: b.h,
                    stroke,
                    dash_phase: StrokeDashPhase::ZERO,
                    post_paint_opacity: PostPaintOpacity::IDENTITY,
                },
                Payload::Group | Payload::Lens { .. } => unreachable!(),
            };
            push(items, id, world, kind);
        }
    }

    if opacity_scope {
        push(items, id, world, ItemKind::EndOpacity);
    }
}

#[cfg(test)]
mod text_paint_tests {
    use super::TextPaints;
    use n0_model::model::{Color, Paints};

    #[test]
    fn uniform_text_uses_node_paints_only_for_uniform_ownership() {
        let node = Paints::solid(Color::BLACK);
        let paints = TextPaints {
            node: node.clone(),
            runs: None,
        };

        assert_eq!(paints.for_source_run(None), Some(&node));
        assert_eq!(paints.for_source_run(Some(0)), None);
    }

    #[test]
    fn attributed_text_fails_closed_without_valid_run_ownership() {
        let node = Paints::solid(Color::BLACK);
        let override_paints = Paints::solid("#FF0000".into());
        let paints = TextPaints {
            node: node.clone(),
            runs: Some(vec![
                None,
                Some(override_paints.clone()),
                Some(Paints::default()),
            ]),
        };

        assert_eq!(paints.for_source_run(Some(0)), Some(&node));
        assert_eq!(paints.for_source_run(Some(1)), Some(&override_paints));
        assert!(paints
            .for_source_run(Some(2))
            .expect("valid explicit-empty run")
            .is_empty());
        assert_eq!(paints.for_source_run(None), None);
        assert_eq!(paints.for_source_run(Some(3)), None);
    }
}

#[cfg(test)]
#[path = "drawlist_vector_join_spike.rs"]
mod vector_join_spike;

#[cfg(test)]
#[path = "text_join_spike.rs"]
mod text_join_spike;
