//! ENG-2.1 · the paint executor — the only module that touches Skia's raster
//! API. [`crate::text_layout`] uses Skia Paragraph strictly as the shaping
//! oracle. `execute_unchecked(canvas, drawlist, view, ctx)` replays a
//! [`DrawList`](crate::drawlist::DrawList) onto a skia `Canvas`,
//! composing `view.then(&item.world)` per item in the exact mathematical
//! form the current spike painter uses — pixel identity is a property of
//! doing the same float ops in the same order, not a tolerance. Complete
//! [`FrameProduct`](crate::frame::FrameProduct) rendering enters through its
//! checked execution method; the raw function here is for glyphless structural
//! probes and internal retained-list replay only.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use n0_model::math::Affine;
use n0_model::model::{
    Alignment, BlendMode, BoxFit, DiamondGradientPaint, GradientStop, ImageFilters, ImagePaint,
    ImagePaintFit, LinearGradientPaint, Paint as ModelPaint, Paints, RadialGradientPaint,
    RectangularCornerRadius, RectangularStrokeWidth, ResourceRef, Stroke, StrokeAlign, StrokeCap,
    StrokeJoin, StrokeWidth, SweepGradientPaint, TileMode,
};
use n0_model::path::{FillRule, PathCommand, ResolvedPathArtifact};
use n0_model::renderability::{self, RenderabilityError};
use n0_model::rounded_box::smooth_corner_params;
use skia_safe::canvas::{SaveLayerFlags, SaveLayerRec};
use skia_safe::gradient::{Colors as GradientColors, Gradient, Interpolation};
use skia_safe::{
    image::CachingHint, path_effect::PathEffect, shaders, stroke_rec::InitStyle, Blender, Canvas,
    ClipOp, Color, Color4f, ColorChannel, ColorMatrix, ColorSpace, CubicResampler, Data,
    FilterMode, Font, ISize, Image, ImageFilter, ImageInfo, Matrix, OpBuilder, Paint, PaintCap,
    PaintJoin, PaintStyle, Path, PathBuilder, PathDirection, PathFillType, PathOp, PictureRecorder,
    Point, Point3, RRect, Rect, SamplingOptions, Shader, StrokeRec,
};

use crate::drawlist::{
    DrawList, ItemKind, PostPaintOpacity, ResolvedClipGeometry, ResolvedClipGeometryKind,
    ResolvedClipLayer, ResolvedClipPath, ResolvedFilter, ResolvedFilterBlend,
    ResolvedFilterColorSpace, ResolvedFilterComposite, ResolvedFilterConvolveEdgeMode,
    ResolvedFilterDisplacementChannel, ResolvedFilterInput, ResolvedFilterLightSource,
    ResolvedFilterMorphology, ResolvedFilterPrimitive, ResolvedFilterTurbulenceKind,
    ResolvedMaskMode, ResolvedPattern, ResolvedPatternGeometry, StrokeDashPhase, StrokeSpace,
};

/// The gradient family whose local matrix could not be represented by the
/// raster backend for one resolved paint box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    Radial,
    Sweep,
    Diamond,
}

impl std::fmt::Display for GradientKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GradientKind::Linear => "linear",
            GradientKind::Radial => "radial",
            GradientKind::Sweep => "sweep",
            GradientKind::Diamond => "diamond",
        })
    }
}

/// Semantic use of a paint stack in the exact draw item that failed backend
/// capability validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintUseContext {
    Fill,
    Stroke,
    TextRun { source_run: Option<usize> },
}

impl std::fmt::Display for PaintUseContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaintUseContext::Fill => f.write_str("fill"),
            PaintUseContext::Stroke => f.write_str("stroke"),
            PaintUseContext::TextRun {
                source_run: Some(run),
            } => write!(f, "text source run {run}"),
            PaintUseContext::TextRun { source_run: None } => f.write_str("uniform text run"),
        }
    }
}

/// One exact drawlist gradient has no invertible local matrix in the pinned
/// backend. The paint index is explicitly post-visibility-filtering; authored
/// inactive or zero-opacity entries are absent from the drawlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradientPreflightReason {
    InvalidPaint(RenderabilityError),
    BackendMatrixNotInvertible,
    BackendShaderConstructionFailed,
}

/// One exact drawlist gradient failed capability validation. The paint index
/// is explicitly post-visibility-filtering; authored inactive or zero-opacity
/// entries are absent from the drawlist. Ownership is generic like the
/// drawlist's: the native route reports the authored [`NodeId`], while the
/// glyphless route reports its product-local slot and maps it back to the
/// contract's opaque owner before the error leaves the engine.
///
/// [`NodeId`]: n0_model::model::NodeId
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientPreflightError<K = n0_model::model::NodeId> {
    pub node: K,
    pub gradient: GradientKind,
    pub context: PaintUseContext,
    pub draw_item: usize,
    pub visible_paint_index: usize,
    pub reason: GradientPreflightReason,
}

impl std::fmt::Display for GradientPreflightReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradientPreflightReason::InvalidPaint(error) => error.fmt(f),
            GradientPreflightReason::BackendMatrixNotInvertible => {
                f.write_str("no invertible backend matrix exists for its resolved paint box")
            }
            GradientPreflightReason::BackendShaderConstructionFailed => {
                f.write_str("the backend could not construct the gradient shader")
            }
        }
    }
}

impl<K: std::fmt::Display> std::fmt::Display for GradientPreflightError<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} gradient on node {} {} visible paint {} in draw item {}: {}",
            self.gradient,
            self.node,
            self.context,
            self.visible_paint_index,
            self.draw_item,
            self.reason
        )
    }
}

impl<K: std::fmt::Display + std::fmt::Debug> std::error::Error for GradientPreflightError<K> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePreflightReason {
    MissingResource,
    UnsupportedModelState,
    TotalMatrixNotInvertible,
    ShaderConstructionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePreflightError {
    pub node: n0_model::model::NodeId,
    pub context: PaintUseContext,
    pub rid: String,
    pub draw_item: usize,
    pub visible_paint_index: usize,
    pub reason: ImagePreflightReason,
}

impl std::fmt::Display for ImagePreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "image `{}` on node {} {} visible paint {} in draw item {}: ",
            self.rid, self.node, self.context, self.visible_paint_index, self.draw_item
        )?;
        f.write_str(match self.reason {
            ImagePreflightReason::MissingResource => "resource is not loaded in this paint context",
            ImagePreflightReason::UnsupportedModelState => {
                "image paint state is unsupported by the proving renderer"
            }
            ImagePreflightReason::TotalMatrixNotInvertible => {
                "view, world, and image-fit matrices do not compose to an invertible backend matrix"
            }
            ImagePreflightReason::ShaderConstructionFailed => {
                "the backend could not construct the image shader"
            }
        })
    }
}

impl std::error::Error for ImagePreflightError {}

/// A checked vector-pattern program could not be recorded into the backend's
/// repeat shader before replay began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternPreflightError {
    pub draw_item: usize,
}

impl std::fmt::Display for PatternPreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "vector pattern in draw item {} could not construct its repeat shader",
            self.draw_item
        )
    }
}

impl std::error::Error for PatternPreflightError {}

static NEXT_PAINT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one complete host paint environment.
///
/// The context incarnation distinguishes independent hosts; the checked
/// revision changes whenever fonts or decoded images change, including an
/// overwrite under the same logical resource id. Damage and caches compare
/// this value without depending on [`PaintCtx`] itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaintEnvironmentKey {
    context: u64,
    revision: u64,
}

fn fresh_paint_context_id() -> u64 {
    NEXT_PAINT_CONTEXT_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .expect("paint context identity space exhausted")
}

/// Host-supplied resources: the typeface offered to text resolution and decoded
/// images used at paint time. Exact shaped fonts live with the drawlist; images
/// stay keyed by the model's logical RID so authored-source resolution remains
/// a host concern. A retained source program may lower an authored RID to an
/// origin-aware runtime RID before this boundary.
pub struct PaintCtx {
    id: u64,
    revision: u64,
    font: Option<skia_safe::Typeface>,
    images: BTreeMap<String, Image>,
}

impl PaintCtx {
    pub fn new(font: Option<skia_safe::Typeface>) -> Self {
        Self {
            id: fresh_paint_context_id(),
            revision: 0,
            font,
            images: BTreeMap::new(),
        }
    }

    /// Snapshot the current opaque environment identity for cache and damage
    /// comparison. It carries no resource contents and performs no I/O.
    pub fn environment_key(&self) -> PaintEnvironmentKey {
        PaintEnvironmentKey {
            context: self.id,
            revision: self.revision,
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("paint context revision exhausted");
    }

    /// Typeface offered to text resolution. The value is immutable through a
    /// shared reference so a cache cannot miss an environment change.
    pub fn font(&self) -> Option<&skia_safe::Typeface> {
        self.font.as_ref()
    }

    /// Replace the host typeface and invalidate every cache keyed by this
    /// context. Existing drawlists keep their exact resolved fonts.
    pub fn set_font(&mut self, font: Option<skia_safe::Typeface>) {
        // Exhaustion must fail before the environment changes; otherwise the
        // same key could name two different resource states.
        self.bump_revision();
        self.font = font;
    }

    /// Register an already-decoded image under the exact model resource id.
    pub fn insert_image(&mut self, rid: impl Into<String>, image: Image) {
        let image = image.with_default_mipmaps().unwrap_or(image);
        let rid = rid.into();
        // BTreeMap insertion has no recoverable failure after this point, so
        // reserve the new environment identity before mutating the map.
        self.bump_revision();
        self.images.insert(rid, image);
    }

    /// Eagerly decode encoded PNG/JPEG/WebP bytes and register them under `rid`.
    pub fn insert_encoded(&mut self, rid: impl Into<String>, bytes: &[u8]) -> Result<(), String> {
        let rid = rid.into();
        let image = Image::from_encoded(Data::new_copy(bytes))
            .ok_or_else(|| format!("could not decode image resource `{rid}`"))?;
        let image = image
            .make_raster_image(None, CachingHint::Allow)
            .ok_or_else(|| format!("could not decode image resource `{rid}`"))?;
        self.insert_image(rid, image);
        Ok(())
    }

    pub fn contains_image(&self, rid: &str) -> bool {
        self.images.contains_key(rid)
    }

    fn image(&self, rid: &str) -> Option<&Image> {
        self.images.get(rid)
    }
}

impl Default for PaintCtx {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod paint_ctx_tests {
    use super::{paint_box_matrix, sk_paint, PaintBox, PaintCtx};
    use crate::drawlist::PostPaintOpacity;
    use n0_model::math::Affine;
    use n0_model::model::{
        Color as ModelColor, GradientStop, LinearGradientPaint, Paint as ModelPaint,
        RadialGradientPaint, SolidPaint,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn positioned_paint_box_preserves_its_origin_without_unit_space_round_trip() {
        let x = 13.25_f32;
        let y = -7.75_f32;
        let w = 201.5_f32;
        let h = 89.125_f32;

        // The old compensation divided the origin by the extent in the
        // producer and multiplied it back here. That algebra changes this
        // ordinary finite coordinate by one binary32 step.
        assert_ne!((w * (x / w)).to_bits(), x.to_bits());

        let matrix = paint_box_matrix(PaintBox::from_xywh(x, y, w, h), &Affine::IDENTITY);
        assert_eq!(matrix[0].to_bits(), w.to_bits());
        assert_eq!(matrix[2].to_bits(), x.to_bits());
        assert_eq!(matrix[4].to_bits(), h.to_bits());
        assert_eq!(matrix[5].to_bits(), y.to_bits());
    }

    #[test]
    fn encoded_resources_are_eager_raster_images() {
        const IMAGE: &[u8] = include_bytes!("../../../fixtures/images/border-diamonds.png");
        let mut ctx = PaintCtx::new(None);
        ctx.insert_encoded("fixture.png", IMAGE).unwrap();
        let image = ctx.image("fixture.png").unwrap();
        assert!(
            !image.is_lazy_generated(),
            "resource registration must finish pixel decode before rendering"
        );
    }

    #[test]
    fn revision_exhaustion_cannot_mutate_the_environment() {
        const IMAGE: &[u8] = include_bytes!("../../../fixtures/images/border-diamonds.png");
        const FONT: &[u8] =
            include_bytes!("../../../fixtures/fonts/Inter/Inter-VariableFont_opsz,wght.ttf");

        let mut images = PaintCtx::new(None);
        images.revision = u64::MAX;
        let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(IMAGE)).unwrap();
        let image_key = images.environment_key();
        assert!(catch_unwind(AssertUnwindSafe(|| images.insert_image("new", image))).is_err());
        assert_eq!(images.environment_key(), image_key);
        assert!(!images.contains_image("new"));

        let mut fonts = PaintCtx::new(None);
        fonts.revision = u64::MAX;
        let typeface = skia_safe::FontMgr::new()
            .new_from_data(FONT, None)
            .expect("bundled Inter typeface");
        let font_key = fonts.environment_key();
        assert!(catch_unwind(AssertUnwindSafe(|| fonts.set_font(Some(typeface)))).is_err());
        assert_eq!(fonts.environment_key(), font_key);
        assert!(fonts.font().is_none());
    }

    #[test]
    fn shader_paint_opacity_folds_into_stop_colors_without_quantizing() {
        let opacity = 0.123_456_7;
        let gradient = LinearGradientPaint {
            opacity,
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: ModelColor::BLACK.into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: ModelColor(0xFFFF_FFFF).into(),
                },
            ],
            ..Default::default()
        };
        let model = ModelPaint::LinearGradient(gradient);
        let paint = sk_paint(
            &model,
            PaintBox::from_size(10.0, 10.0),
            &PaintCtx::new(None),
            PostPaintOpacity::IDENTITY,
        )
        .expect("valid gradient paint");
        // The paint alpha carries the opacity at the backend's own 8-bit
        // step — the measured Chromium quantization (one round to 255ths,
        // then the backend's float fold into the stops).
        let expected = (opacity * 255.0f32).round() / 255.0;
        assert_eq!(paint.alpha_f().to_bits(), expected.to_bits());
    }

    #[test]
    fn post_paint_opacity_folds_after_solid_and_gradient_intrinsic_alpha() {
        let intrinsic = 0.7_f32;
        let factor = 0.6_f32;
        let post_paint_opacity = PostPaintOpacity::from_resolved(factor);
        let stops = vec![
            GradientStop {
                offset: 0.0,
                color: ModelColor::BLACK.into(),
            },
            GradientStop {
                offset: 1.0,
                color: ModelColor(0xFFFF_FFFF).into(),
            },
        ];
        let models = [
            (
                ModelPaint::Solid(SolidPaint::new(ModelColor(0x4D12_3456))),
                77.0_f32 / 255.0,
            ),
            (
                ModelPaint::LinearGradient(LinearGradientPaint {
                    opacity: intrinsic,
                    stops: stops.clone(),
                    ..Default::default()
                }),
                (intrinsic * 255.0).round() / 255.0,
            ),
            (
                ModelPaint::RadialGradient(RadialGradientPaint {
                    opacity: intrinsic,
                    stops,
                    ..Default::default()
                }),
                (intrinsic * 255.0).round() / 255.0,
            ),
        ];

        for (model, intrinsic_alpha) in models {
            let paint = sk_paint(
                &model,
                PaintBox::from_size(10.0, 10.0),
                &PaintCtx::new(None),
                post_paint_opacity,
            )
            .expect("valid paint");
            let expected = intrinsic_alpha * factor;
            assert_eq!(paint.alpha_f().to_bits(), expected.to_bits());
            assert!(paint.color_filter().is_none());
        }

        let identity = sk_paint(
            &ModelPaint::LinearGradient(LinearGradientPaint {
                opacity: intrinsic,
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: ModelColor::BLACK.into(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: ModelColor(0xFFFF_FFFF).into(),
                    },
                ],
                ..Default::default()
            }),
            PaintBox::from_size(10.0, 10.0),
            &PaintCtx::new(None),
            PostPaintOpacity::IDENTITY,
        )
        .expect("valid gradient paint");
        assert_eq!(
            identity.alpha_f().to_bits(),
            ((intrinsic * 255.0).round() / 255.0).to_bits()
        );
        assert!(identity.color_filter().is_none());

        let folded = ((intrinsic * 255.0).round() / 255.0) * factor;
        let requantized = (folded * 255.0).round() / 255.0;
        assert_ne!(
            folded.to_bits(),
            requantized.to_bits(),
            "the probe values distinguish float folding from a second 8-bit alpha step"
        );
    }
}

/// Row-major `Affine` -> skia `Matrix`, byte-identical to the spike
/// painter's `skia_matrix` (SVG a b c d e f order).
fn skia_matrix(t: &Affine) -> Matrix {
    Matrix::new_all(t.a, t.c, t.e, t.b, t.d, t.f, 0.0, 0.0, 1.0)
}

fn with_local_transform(canvas: &Canvas, view: &Affine, world: &Affine, draw: impl FnOnce()) {
    let total = view.then(world);
    canvas.save();
    canvas.set_matrix(&skia_matrix(&total).into());
    draw();
    canvas.restore();
}

fn sk_blend_mode(mode: BlendMode) -> skia_safe::BlendMode {
    match mode {
        BlendMode::Normal => skia_safe::BlendMode::SrcOver,
        BlendMode::Multiply => skia_safe::BlendMode::Multiply,
        BlendMode::Screen => skia_safe::BlendMode::Screen,
        BlendMode::Overlay => skia_safe::BlendMode::Overlay,
        BlendMode::Darken => skia_safe::BlendMode::Darken,
        BlendMode::Lighten => skia_safe::BlendMode::Lighten,
        BlendMode::ColorDodge => skia_safe::BlendMode::ColorDodge,
        BlendMode::ColorBurn => skia_safe::BlendMode::ColorBurn,
        BlendMode::HardLight => skia_safe::BlendMode::HardLight,
        BlendMode::SoftLight => skia_safe::BlendMode::SoftLight,
        BlendMode::Difference => skia_safe::BlendMode::Difference,
        BlendMode::Exclusion => skia_safe::BlendMode::Exclusion,
        BlendMode::Hue => skia_safe::BlendMode::Hue,
        BlendMode::Saturation => skia_safe::BlendMode::Saturation,
        BlendMode::Color => skia_safe::BlendMode::Color,
        BlendMode::Luminosity => skia_safe::BlendMode::Luminosity,
    }
}

fn sk_filter_blend_mode(mode: ResolvedFilterBlend) -> skia_safe::BlendMode {
    match mode {
        ResolvedFilterBlend::Normal => skia_safe::BlendMode::SrcOver,
        ResolvedFilterBlend::Multiply => skia_safe::BlendMode::Multiply,
        ResolvedFilterBlend::Screen => skia_safe::BlendMode::Screen,
        ResolvedFilterBlend::Overlay => skia_safe::BlendMode::Overlay,
        ResolvedFilterBlend::Darken => skia_safe::BlendMode::Darken,
        ResolvedFilterBlend::Lighten => skia_safe::BlendMode::Lighten,
        ResolvedFilterBlend::ColorDodge => skia_safe::BlendMode::ColorDodge,
        ResolvedFilterBlend::ColorBurn => skia_safe::BlendMode::ColorBurn,
        ResolvedFilterBlend::HardLight => skia_safe::BlendMode::HardLight,
        ResolvedFilterBlend::SoftLight => skia_safe::BlendMode::SoftLight,
        ResolvedFilterBlend::Difference => skia_safe::BlendMode::Difference,
        ResolvedFilterBlend::Exclusion => skia_safe::BlendMode::Exclusion,
        ResolvedFilterBlend::Hue => skia_safe::BlendMode::Hue,
        ResolvedFilterBlend::Saturation => skia_safe::BlendMode::Saturation,
        ResolvedFilterBlend::Color => skia_safe::BlendMode::Color,
        ResolvedFilterBlend::Luminosity => skia_safe::BlendMode::Luminosity,
    }
}

fn sk_tile_mode(mode: TileMode) -> skia_safe::TileMode {
    match mode {
        TileMode::Clamp => skia_safe::TileMode::Clamp,
        TileMode::Repeated => skia_safe::TileMode::Repeat,
        TileMode::Mirror => skia_safe::TileMode::Mirror,
        TileMode::Decal => skia_safe::TileMode::Decal,
    }
}

fn gradient_stops(stops: &[GradientStop]) -> (Vec<skia_safe::Color4f>, Vec<f32>) {
    // The stop's checked unit components are already what `Color4f` holds:
    // a byte-authored stop reaches these exact bits through the widening
    // multiply `SkColor4f::FromColor` itself performs, and a stop whose alpha
    // was resolved by multiplication keeps the value the ramp interpolates,
    // instead of the neighbouring byte a narrowing would substitute.
    let colors = stops
        .iter()
        .map(|stop| {
            skia_safe::Color4f::new(
                stop.color.r(),
                stop.color.g(),
                stop.color.b(),
                stop.color.a(),
            )
        })
        .collect();
    let positions = stops.iter().map(|stop| stop.offset).collect();
    (colors, positions)
}

fn gradient<'a>(
    colors: &'a [skia_safe::Color4f],
    positions: &'a [f32],
    tile_mode: skia_safe::TileMode,
) -> Gradient<'a> {
    Gradient::new(
        GradientColors::new(colors, Some(positions), tile_mode, None),
        Interpolation::default(),
    )
}

/// Node-local box used by every non-solid paint. A degenerate axis becomes a
/// centered one-pixel interval so line and zero-axis paints retain a stable,
/// finite unit-space mapping.
#[derive(Debug, Clone, Copy)]
struct PaintBox {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl PaintBox {
    fn from_size(w: f32, h: f32) -> Self {
        Self::from_xywh(0.0, 0.0, w, h)
    }

    fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        let (x, w) = if w == 0.0 { (x - 0.5, 1.0) } else { (x, w) };
        let (y, h) = if h == 0.0 { (y - 0.5, 1.0) } else { (y, h) };
        PaintBox { x, y, w, h }
    }
}

fn paint_box_matrix(paint_box: PaintBox, transform: &Affine) -> Matrix {
    let mut matrix = Matrix::new_all(
        paint_box.w,
        0.0,
        paint_box.x,
        0.0,
        paint_box.h,
        paint_box.y,
        0.0,
        0.0,
        1.0,
    );
    matrix.pre_concat(&skia_matrix(transform));
    matrix
}

/// Map an existing local paint coordinate system into the coordinates in
/// which geometry is submitted to the canvas. Identity deliberately performs
/// no matrix write so every ordinary draw retains its established f32 route.
fn mapped_paint_box_matrix(
    paint_box: PaintBox,
    transform: &Affine,
    paint_to_canvas: &Affine,
) -> Matrix {
    let mut matrix = paint_box_matrix(paint_box, transform);
    if *paint_to_canvas != Affine::IDENTITY {
        // SkMatrix::postConcat is `other * self`: unit -> paint box -> model
        // paint transform -> frame, matching the transformed centerline.
        matrix.post_concat(&skia_matrix(paint_to_canvas));
    }
    matrix
}

/// Translation does not change stroke construction. Keeping that exact case
/// on the established local draw route avoids an otherwise observable f32
/// cancellation when large source coordinates meet an opposite translation.
/// The resolved contract remains frame-space; this is only an equivalent
/// backend execution route.
fn effective_stroke_space(space: StrokeSpace, world: &Affine) -> StrokeSpace {
    if space == StrokeSpace::Frame
        && world.a == 1.0
        && world.b == 0.0
        && world.c == 0.0
        && world.d == 1.0
    {
        StrokeSpace::Local
    } else {
        space
    }
}

fn gradient_transform(model: &ModelPaint) -> Option<(GradientKind, &Affine)> {
    match model {
        ModelPaint::LinearGradient(gradient) => Some((GradientKind::Linear, &gradient.transform)),
        ModelPaint::RadialGradient(gradient) => Some((GradientKind::Radial, &gradient.transform)),
        ModelPaint::SweepGradient(gradient) => Some((GradientKind::Sweep, &gradient.transform)),
        ModelPaint::DiamondGradient(gradient) => Some((GradientKind::Diamond, &gradient.transform)),
        ModelPaint::Solid(_) | ModelPaint::Image(_) => None,
    }
}

fn preflight_paints<K: Copy>(
    node: K,
    draw_item: usize,
    context: PaintUseContext,
    paints: &Paints,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
) -> Result<(), GradientPreflightError<K>> {
    for (visible_paint_index, model) in paints.iter().enumerate() {
        let Some((gradient, transform)) = gradient_transform(model) else {
            continue;
        };
        if let Err(error) = renderability::validate_paint(model) {
            return Err(GradientPreflightError {
                node,
                gradient,
                context,
                draw_item,
                visible_paint_index,
                reason: GradientPreflightReason::InvalidPaint(error),
            });
        }
        if mapped_paint_box_matrix(paint_box, transform, paint_to_canvas)
            .invert()
            .is_none()
        {
            return Err(GradientPreflightError {
                node,
                gradient,
                context,
                draw_item,
                visible_paint_index,
                reason: GradientPreflightReason::BackendMatrixNotInvertible,
            });
        }
        let shader_exists = match model {
            ModelPaint::LinearGradient(model) => {
                linear_gradient_shader_mapped(model, paint_box, paint_to_canvas).is_some()
            }
            ModelPaint::RadialGradient(model) => {
                radial_gradient_shader_mapped(model, paint_box, paint_to_canvas).is_some()
            }
            ModelPaint::SweepGradient(model) => {
                sweep_gradient_shader_mapped(model, paint_box, paint_to_canvas).is_some()
            }
            ModelPaint::DiamondGradient(model) => {
                diamond_gradient_shader_mapped(model, paint_box, paint_to_canvas).is_some()
            }
            ModelPaint::Solid(_) | ModelPaint::Image(_) => unreachable!(),
        };
        if !shader_exists {
            return Err(GradientPreflightError {
                node,
                gradient,
                context,
                draw_item,
                visible_paint_index,
                reason: GradientPreflightReason::BackendShaderConstructionFailed,
            });
        }
    }
    Ok(())
}

/// Prove that every gradient which can be evaluated by this exact drawlist has
/// an invertible backend-local matrix for its resolved paint box. This is
/// deliberately later than authored paint validation: multiplying a finite,
/// mathematically invertible transform by a concrete box can rescue or defeat
/// binary32 backend representability.
pub(crate) fn preflight_gradients<K: Copy>(
    list: &DrawList<K>,
) -> Result<(), GradientPreflightError<K>> {
    for (draw_item, item) in list.items.iter().enumerate() {
        match &item.kind {
            ItemKind::RectFill { w, h, paints, .. }
            | ItemKind::OvalFill { w, h, paints, .. }
            | ItemKind::PathFill { w, h, paints, .. } => preflight_paints(
                item.node,
                draw_item,
                PaintUseContext::Fill,
                paints,
                PaintBox::from_size(*w, *h),
                &Affine::IDENTITY,
            )?,
            ItemKind::TextFill {
                layout,
                paints,
                paint_w,
                paint_h,
                ..
            } => {
                let paint_box = PaintBox::from_size(*paint_w, *paint_h);
                for run in &layout.glyph_runs {
                    let Some(run_paints) = paints.for_source_run(run.source_run) else {
                        continue;
                    };
                    preflight_paints(
                        item.node,
                        draw_item,
                        PaintUseContext::TextRun {
                            source_run: run.source_run,
                        },
                        run_paints,
                        paint_box,
                        &Affine::IDENTITY,
                    )?;
                }
            }
            ItemKind::RectStroke {
                w,
                h,
                stroke,
                space,
                ..
            }
            | ItemKind::OvalStroke {
                w,
                h,
                stroke,
                space,
                ..
            }
            | ItemKind::PathStroke {
                w,
                h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_paints(
                    item.node,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_size(*w, *h),
                    paint_to_canvas,
                )?;
            }
            ItemKind::AbsoluteDashedOvalStroke {
                x,
                y,
                w,
                h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_paints(
                    item.node,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_xywh(*x, *y, *w, *h),
                    paint_to_canvas,
                )?;
            }
            ItemKind::LineStroke {
                paint_w,
                paint_h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_paints(
                    item.node,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_size(*paint_w, *paint_h),
                    paint_to_canvas,
                )?;
            }
            ItemKind::TextStroke {
                layout,
                paint_w,
                paint_h,
                stroke,
                ..
            } => {
                if !layout.glyph_runs.is_empty() {
                    preflight_paints(
                        item.node,
                        draw_item,
                        PaintUseContext::Stroke,
                        &stroke.paints,
                        PaintBox::from_size(*paint_w, *paint_h),
                        &Affine::IDENTITY,
                    )?;
                }
            }
            ItemKind::PatternFill { .. }
            | ItemKind::PatternStroke { .. }
            | ItemKind::BeginOpacity { .. }
            | ItemKind::BeginIsolatedOpacity { .. }
            | ItemKind::EndOpacity
            | ItemKind::BeginClipRect { .. }
            | ItemKind::BeginClipPath { .. }
            | ItemKind::EndClip
            | ItemKind::BeginMaskContent
            | ItemKind::BeginMaskSource { .. }
            | ItemKind::EndMaskSource
            | ItemKind::EndMaskContent
            | ItemKind::BeginFilter { .. }
            | ItemKind::EndFilter => {}
        }
    }
    Ok(())
}

fn image_rid(model: &ImagePaint) -> &str {
    match &model.image {
        ResourceRef::Rid(rid) | ResourceRef::Hash(rid) => rid,
    }
}

fn image_model_supported(model: &ImagePaint) -> bool {
    model.quarter_turns == 0
        && model.alignment == Alignment::CENTER
        && model.filters == ImageFilters::default()
        && matches!(model.fit, ImagePaintFit::Fit(_))
}

fn preflight_image_paints(
    item: &crate::drawlist::Item,
    draw_item: usize,
    context: PaintUseContext,
    paints: &Paints,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
    view: &Affine,
    ctx: &PaintCtx,
) -> Result<(), ImagePreflightError> {
    for (visible_paint_index, paint) in paints.iter().enumerate() {
        let ModelPaint::Image(model) = paint else {
            continue;
        };
        let rid = image_rid(model);
        let fail = |reason| ImagePreflightError {
            node: item.node,
            context,
            rid: rid.to_owned(),
            draw_item,
            visible_paint_index,
            reason,
        };
        if !image_model_supported(model) {
            return Err(fail(ImagePreflightReason::UnsupportedModelState));
        }
        let image = ctx
            .image(rid)
            .ok_or_else(|| fail(ImagePreflightReason::MissingResource))?;
        let local = image_fit_matrix(
            image,
            paint_box,
            match model.fit {
                ImagePaintFit::Fit(fit) => fit,
                ImagePaintFit::Transform(_) | ImagePaintFit::Tile(_) => unreachable!(),
            },
        );
        let geometry = view.then(&item.world);
        let geometry_is_finite = [
            geometry.a, geometry.b, geometry.c, geometry.d, geometry.e, geometry.f,
        ]
        .iter()
        .all(|value| value.is_finite());
        let determinant = f64::from(geometry.a) * f64::from(geometry.d)
            - f64::from(geometry.b) * f64::from(geometry.c);
        let ctm = skia_matrix(&geometry);
        if !geometry_is_finite
            || (determinant != 0.0 && Matrix::concat(&ctm, &local).invert().is_none())
        {
            return Err(fail(ImagePreflightReason::TotalMatrixNotInvertible));
        }
        if image_shader_mapped(model, paint_box, ctx, paint_to_canvas).is_none() {
            return Err(fail(ImagePreflightReason::ShaderConstructionFailed));
        }
    }
    Ok(())
}

/// Checked-execution image capability fence. Unlike gradient-local preflight,
/// image sampling depends on the final CTM, so this runs for the requested
/// view immediately before replay.
pub(crate) fn preflight_images(
    list: &DrawList,
    view: &Affine,
    ctx: &PaintCtx,
) -> Result<(), ImagePreflightError> {
    for (draw_item, item) in list.items.iter().enumerate() {
        match &item.kind {
            ItemKind::RectFill { w, h, paints, .. }
            | ItemKind::OvalFill { w, h, paints, .. }
            | ItemKind::PathFill { w, h, paints, .. } => preflight_image_paints(
                item,
                draw_item,
                PaintUseContext::Fill,
                paints,
                PaintBox::from_size(*w, *h),
                &Affine::IDENTITY,
                view,
                ctx,
            )?,
            ItemKind::TextFill {
                layout,
                paints,
                paint_w,
                paint_h,
                ..
            } => {
                let paint_box = PaintBox::from_size(*paint_w, *paint_h);
                for run in &layout.glyph_runs {
                    if let Some(run_paints) = paints.for_source_run(run.source_run) {
                        preflight_image_paints(
                            item,
                            draw_item,
                            PaintUseContext::TextRun {
                                source_run: run.source_run,
                            },
                            run_paints,
                            paint_box,
                            &Affine::IDENTITY,
                            view,
                            ctx,
                        )?;
                    }
                }
            }
            ItemKind::RectStroke {
                w,
                h,
                stroke,
                space,
                ..
            }
            | ItemKind::OvalStroke {
                w,
                h,
                stroke,
                space,
                ..
            }
            | ItemKind::PathStroke {
                w,
                h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_image_paints(
                    item,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_size(*w, *h),
                    paint_to_canvas,
                    view,
                    ctx,
                )?;
            }
            ItemKind::AbsoluteDashedOvalStroke {
                x,
                y,
                w,
                h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_image_paints(
                    item,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_xywh(*x, *y, *w, *h),
                    paint_to_canvas,
                    view,
                    ctx,
                )?;
            }
            ItemKind::LineStroke {
                paint_w,
                paint_h,
                stroke,
                space,
                ..
            } => {
                let paint_to_canvas = match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                };
                preflight_image_paints(
                    item,
                    draw_item,
                    PaintUseContext::Stroke,
                    &stroke.paints,
                    PaintBox::from_size(*paint_w, *paint_h),
                    paint_to_canvas,
                    view,
                    ctx,
                )?;
            }
            ItemKind::TextStroke {
                layout,
                paint_w,
                paint_h,
                stroke,
                ..
            } => {
                if !layout.glyph_runs.is_empty() {
                    preflight_image_paints(
                        item,
                        draw_item,
                        PaintUseContext::Stroke,
                        &stroke.paints,
                        PaintBox::from_size(*paint_w, *paint_h),
                        &Affine::IDENTITY,
                        view,
                        ctx,
                    )?;
                }
            }
            ItemKind::PatternFill { .. }
            | ItemKind::PatternStroke { .. }
            | ItemKind::BeginOpacity { .. }
            | ItemKind::BeginIsolatedOpacity { .. }
            | ItemKind::EndOpacity
            | ItemKind::BeginClipRect { .. }
            | ItemKind::BeginClipPath { .. }
            | ItemKind::EndClip
            | ItemKind::BeginMaskContent
            | ItemKind::BeginMaskSource { .. }
            | ItemKind::EndMaskSource
            | ItemKind::EndMaskContent
            | ItemKind::BeginFilter { .. }
            | ItemKind::EndFilter => {}
        }
    }
    Ok(())
}

/// Convert the model's centered-normalized gradient point to UV for Skia.
/// The f64 intermediate avoids overflowing or needlessly rounding finite f32
/// model values before the result returns to Skia's f32 coordinate space.
fn alignment_uv(alignment: Alignment) -> (f32, f32) {
    (
        ((f64::from(alignment.0) + 1.0) * 0.5) as f32,
        ((f64::from(alignment.1) + 1.0) * 0.5) as f32,
    )
}

fn linear_gradient_shader_mapped(
    paint: &LinearGradientPaint,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    let (colors, positions) = gradient_stops(&paint.stops);
    let from = alignment_uv(paint.xy1);
    let to = alignment_uv(paint.xy2);
    let stops = gradient(&colors, &positions, sk_tile_mode(paint.tile_mode));
    let matrix = mapped_paint_box_matrix(paint_box, &paint.transform, paint_to_canvas);
    shaders::linear_gradient((from, to), &stops, Some(&matrix))
}

fn radial_gradient_shader_mapped(
    paint: &RadialGradientPaint,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    let (colors, positions) = gradient_stops(&paint.stops);
    let stops = gradient(&colors, &positions, sk_tile_mode(paint.tile_mode));
    let matrix = mapped_paint_box_matrix(paint_box, &paint.transform, paint_to_canvas);
    match paint.geometry {
        None => shaders::radial_gradient(((0.5, 0.5), 0.5), &stops, Some(&matrix)),
        Some(geometry) => shaders::two_point_conical_gradient(
            (geometry.start.center, geometry.start.radius),
            (geometry.end.center, geometry.end.radius),
            &stops,
            Some(&matrix),
        ),
    }
}

fn sweep_gradient_shader_mapped(
    paint: &SweepGradientPaint,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    let (colors, positions) = gradient_stops(&paint.stops);
    let stops = gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    let matrix = mapped_paint_box_matrix(paint_box, &paint.transform, paint_to_canvas);
    shaders::sweep_gradient((0.5, 0.5), (0.0, 360.0), &stops, Some(&matrix))
}

fn diamond_gradient_shader_mapped(
    paint: &DiamondGradientPaint,
    paint_box: PaintBox,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    let (colors, positions) = gradient_stops(&paint.stops);
    let stops = gradient(&colors, &positions, skia_safe::TileMode::Clamp);
    let ramp = shaders::linear_gradient(((0.0, 0.0), (1.0, 0.0)), &stops, None)?;
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
    let matrix = mapped_paint_box_matrix(paint_box, &paint.transform, paint_to_canvas);
    effect.make_shader(Data::new_copy(&[]), &[ramp.into()], Some(&matrix))
}

fn image_fit_matrix(image: &Image, paint_box: PaintBox, fit: BoxFit) -> Matrix {
    let iw = image.width() as f32;
    let ih = image.height() as f32;
    let w = paint_box.w;
    let h = paint_box.h;
    let (sx, sy) = match fit {
        BoxFit::Contain => {
            let scale = (w / iw).min(h / ih);
            (scale, scale)
        }
        BoxFit::Cover => {
            let scale = (w / iw).max(h / ih);
            (scale, scale)
        }
        BoxFit::Fill => (w / iw, h / ih),
        BoxFit::None => (1.0, 1.0),
    };
    let tx = paint_box.x + (w - iw * sx) * 0.5;
    let ty = paint_box.y + (h - ih * sy) * 0.5;
    Matrix::new_all(sx, 0.0, tx, 0.0, sy, ty, 0.0, 0.0, 1.0)
}

fn image_shader_mapped(
    paint: &ImagePaint,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    if paint.quarter_turns != 0
        || paint.alignment != n0_model::model::Alignment::CENTER
        || paint.filters != ImageFilters::default()
    {
        return None;
    }
    let ImagePaintFit::Fit(fit) = paint.fit else {
        return None;
    };
    let rid = match &paint.image {
        ResourceRef::Rid(rid) | ResourceRef::Hash(rid) => rid,
    };
    let image = ctx.image(rid)?;
    let mut matrix = image_fit_matrix(image, paint_box, fit);
    if *paint_to_canvas != Affine::IDENTITY {
        matrix.post_concat(&skia_matrix(paint_to_canvas));
    }
    let sampling = SamplingOptions::from(CubicResampler::mitchell());
    let shader = image.to_shader(
        Some((skia_safe::TileMode::Decal, skia_safe::TileMode::Decal)),
        sampling,
        Some(&matrix),
    )?;
    Some(shader)
}

/// Record one already-compiled tile program and expose it as an infinitely
/// repeating shader. The nested list starts with its own hard frame clip, so
/// content outside `(0, 0, width, height)` cannot leak into a neighbouring
/// tile before repetition.
fn pattern_shader_mapped(
    pattern: &ResolvedPattern,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
) -> Option<Shader> {
    let tile = Rect::from_wh(pattern.width, pattern.height);
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(tile, false);
    execute_unchecked(canvas, &pattern.program, &Affine::IDENTITY, ctx);
    let picture = recorder.finish_recording_as_picture(Some(&tile))?;
    let mut matrix = skia_matrix(&pattern.transform);
    if *paint_to_canvas != Affine::IDENTITY {
        matrix.post_concat(&skia_matrix(paint_to_canvas));
    }
    Some(picture.to_shader(
        Some((skia_safe::TileMode::Repeat, skia_safe::TileMode::Repeat)),
        FilterMode::Linear,
        Some(&matrix),
        Some(&tile),
    ))
}

fn pattern_paint(
    pattern: &ResolvedPattern,
    post_paint_opacity: PostPaintOpacity,
    ctx: &PaintCtx,
) -> Option<Paint> {
    pattern_paint_mapped(pattern, post_paint_opacity, ctx, &Affine::IDENTITY)
}

fn pattern_paint_mapped(
    pattern: &ResolvedPattern,
    post_paint_opacity: PostPaintOpacity,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
) -> Option<Paint> {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_shader(pattern_shader_mapped(pattern, ctx, paint_to_canvas)?);
    // Pattern paint opacity follows the same byte-alpha materialization as a
    // gradient shader. A one-draw element-opacity fold then multiplies that
    // materialized alpha without another quantization.
    let opacity = pattern.opacity.clamp(0.0, 1.0);
    paint.set_alpha_f((opacity * 255.0).round() / 255.0);
    let factor = post_paint_opacity.value();
    if factor != 1.0 {
        paint.set_alpha_f(paint.alpha_f() * factor);
    }
    Some(paint)
}

fn pattern_stroke_paint(
    pattern: &ResolvedPattern,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    ctx: &PaintCtx,
) -> Option<Paint> {
    pattern_stroke_paint_mapped(
        pattern,
        stroke,
        dash_phase,
        post_paint_opacity,
        ctx,
        &Affine::IDENTITY,
    )
}

fn pattern_stroke_paint_mapped(
    pattern: &ResolvedPattern,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
) -> Option<Paint> {
    let width = uniform_stroke_width(stroke)?;
    let mut paint = pattern_paint_mapped(pattern, post_paint_opacity, ctx, paint_to_canvas)?;
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width);
    paint.set_stroke_cap(sk_stroke_cap(stroke.cap));
    paint.set_stroke_join(sk_stroke_join(stroke.join));
    paint.set_stroke_miter(stroke.miter_limit);
    if let Some(values) = stroke.dash_array.as_deref() {
        if !values.is_empty() {
            let intervals = normalized_dash_array(values)?;
            paint.set_path_effect(PathEffect::dash(&intervals, dash_phase.value())?);
        }
    }
    Some(paint)
}

fn preflight_pattern(pattern: &ResolvedPattern, ctx: &PaintCtx, paint_to_canvas: &Affine) -> bool {
    if preflight_patterns(&pattern.program, ctx).is_err() {
        return false;
    }
    pattern_shader_mapped(pattern, ctx, paint_to_canvas).is_some()
}

/// Prove every nested picture/repeat shader before the first target draw.
/// This keeps backend refusal outside replay: an unavailable shader cannot
/// silently turn a valid pattern into transparent paint.
pub(crate) fn preflight_patterns<K>(
    list: &DrawList<K>,
    ctx: &PaintCtx,
) -> Result<(), PatternPreflightError> {
    for (draw_item, item) in list.items.iter().enumerate() {
        let (pattern, paint_to_canvas) = match &item.kind {
            ItemKind::PatternFill { pattern, .. } => (pattern, &Affine::IDENTITY),
            ItemKind::PatternStroke { pattern, space, .. } => (
                pattern,
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => &Affine::IDENTITY,
                    StrokeSpace::Frame => &item.world,
                },
            ),
            _ => continue,
        };
        if !preflight_pattern(pattern, ctx, paint_to_canvas) {
            return Err(PatternPreflightError { draw_item });
        }
    }
    Ok(())
}

/// Materialize one model paint. The caller draws these in list order instead
/// of precomposing a stack: each entry's blend mode must see the actual canvas
/// result of the paints below it, including the scene backdrop.
fn sk_paint(
    model: &ModelPaint,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    post_paint_opacity: PostPaintOpacity,
) -> Option<Paint> {
    sk_paint_mapped(model, paint_box, ctx, post_paint_opacity, &Affine::IDENTITY)
}

fn sk_paint_mapped(
    model: &ModelPaint,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    post_paint_opacity: PostPaintOpacity,
    paint_to_canvas: &Affine,
) -> Option<Paint> {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_blend_mode(sk_blend_mode(model.blend_mode()));
    // Gradient ramps dither: the backend's ordered dither breaks the banding
    // an 8-bit quantized ramp would show, and Chromium's rasterizer dithers
    // its gradients the same way — the gradient cells are byte-exact *with*
    // it and one-off per ramp pixel without it.
    if matches!(
        model,
        ModelPaint::LinearGradient(_)
            | ModelPaint::RadialGradient(_)
            | ModelPaint::SweepGradient(_)
            | ModelPaint::DiamondGradient(_)
    ) {
        paint.set_dither(true);
    }
    match model {
        ModelPaint::Solid(solid) => {
            paint.set_color(Color::new(solid.color.argb()));
        }
        ModelPaint::LinearGradient(model) => {
            paint.set_shader(
                linear_gradient_shader_mapped(model, paint_box, paint_to_canvas)
                    .expect("preflighted linear gradient shader construction failed"),
            );
        }
        ModelPaint::RadialGradient(model) => {
            paint.set_shader(
                radial_gradient_shader_mapped(model, paint_box, paint_to_canvas)
                    .expect("preflighted radial gradient shader construction failed"),
            );
        }
        ModelPaint::SweepGradient(model) => {
            paint.set_shader(
                sweep_gradient_shader_mapped(model, paint_box, paint_to_canvas)
                    .expect("preflighted sweep gradient shader construction failed"),
            );
        }
        ModelPaint::DiamondGradient(model) => {
            paint.set_shader(
                diamond_gradient_shader_mapped(model, paint_box, paint_to_canvas)
                    .expect("preflighted diamond gradient shader construction failed"),
            );
        }
        ModelPaint::Image(model) => {
            paint.set_shader(image_shader_mapped(model, paint_box, ctx, paint_to_canvas)?);
        }
    }
    // Solids store opacity in their RGBA8 color. An image's opacity stays the
    // paint's float alpha modulation. A gradient's opacity quantizes to the
    // paint's 8-bit alpha step once and then scales the shader's float stops
    // by that step — Chromium reaches a translucent gradient through the
    // same 8-bit paint alpha (measured: a 0.5 `fill-opacity` ramp is the
    // full-opacity ramp times 128/255, not times 0.5), and the gradient
    // cells pin the product byte-exactly.
    match model {
        ModelPaint::Solid(_) => {}
        ModelPaint::Image(_) => {
            paint.set_alpha_f(model.opacity().clamp(0.0, 1.0));
        }
        _ => {
            let opacity = model.opacity().clamp(0.0, 1.0);
            paint.set_alpha_f((opacity * 255.0).round() / 255.0);
        }
    }
    // Chromium represents element opacity as a SaveLayerAlpha effect, then
    // folds a one-draw SrcOver effect into that draw by multiplying its float
    // paint alpha (PaintOpBuffer::PlaybackFoldingIterator and
    // ScopedRasterFlags). Preserve that exact order: intrinsic paint alpha
    // materializes first, then this factor multiplies it without another
    // 8-bit quantization. Identity performs no write at all.
    let factor = post_paint_opacity.value();
    if factor != 1.0 {
        paint.set_alpha_f(paint.alpha_f() * factor);
    }
    Some(paint)
}

fn sk_stroke_cap(cap: StrokeCap) -> PaintCap {
    match cap {
        StrokeCap::Butt => PaintCap::Butt,
        StrokeCap::Round => PaintCap::Round,
        StrokeCap::Square => PaintCap::Square,
    }
}

fn sk_stroke_join(join: StrokeJoin) -> PaintJoin {
    match join {
        StrokeJoin::Miter => PaintJoin::Miter,
        StrokeJoin::Round => PaintJoin::Round,
        StrokeJoin::Bevel => PaintJoin::Bevel,
    }
}

/// Normalize an authored dash array to the even-length form Skia requires.
/// Invalid/non-finite or all-zero programmatic values produce no geometry;
/// the XML boundary rejects those values before they reach this stage.
fn normalized_dash_array(values: &[f32]) -> Option<Vec<f32>> {
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || values.iter().all(|value| *value == 0.0)
    {
        return None;
    }
    let mut normalized = values.to_vec();
    if normalized.len() % 2 == 1 {
        normalized.extend_from_slice(values);
    }
    Some(normalized)
}

fn uniform_stroke_width(stroke: &Stroke) -> Option<f32> {
    match stroke.width.normalized() {
        StrokeWidth::None => None,
        StrokeWidth::Uniform(width) => Some(width),
        StrokeWidth::Rectangular(_) => None,
    }
}

/// Convert a stroke application into filled geometry so every existing paint
/// variant (including images and gradients) follows the same ordered painter
/// path. Open contours are necessarily centered; inside/outside are defined
/// only for closed outlines.
fn stroke_geometry(source: &Path, stroke: &Stroke, dash_phase: StrokeDashPhase) -> Path {
    let Some(width) = uniform_stroke_width(stroke) else {
        return Path::new();
    };
    let align = if source.is_last_contour_closed() {
        stroke.align
    } else {
        StrokeAlign::Center
    };
    let stroke_width = match align {
        StrokeAlign::Center => width,
        StrokeAlign::Inside | StrokeAlign::Outside => width * 2.0,
    };

    let mut path_to_stroke = source.clone();
    if let Some(values) = stroke.dash_array.as_deref() {
        if !values.is_empty() {
            let Some(intervals) = normalized_dash_array(values) else {
                return Path::new();
            };
            let Some(effect) = PathEffect::dash(&intervals, dash_phase.value()) else {
                return Path::new();
            };
            let filter_rec = StrokeRec::new(InitStyle::Hairline);
            let Some((dashed, _)) = effect.filter_path(source, &filter_rec, source.bounds()) else {
                return Path::new();
            };
            path_to_stroke = dashed.snapshot();
        }
    }

    let mut record = StrokeRec::new(InitStyle::Hairline);
    record.set_stroke_style(stroke_width, false);
    record.set_stroke_params(
        sk_stroke_cap(stroke.cap),
        sk_stroke_join(stroke.join),
        stroke.miter_limit,
    );
    let mut builder = PathBuilder::new();
    if !record.apply_to_path(&mut builder, &path_to_stroke) {
        return Path::new();
    }
    let outline = builder.snapshot();
    match align {
        StrokeAlign::Center => outline,
        StrokeAlign::Inside => {
            skia_safe::op(&outline, source, PathOp::Intersect).unwrap_or_default()
        }
        StrokeAlign::Outside => {
            skia_safe::op(&outline, source, PathOp::Difference).unwrap_or_default()
        }
    }
}

fn rect_path(w: f32, h: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.add_rect(Rect::from_wh(w, h), Some(PathDirection::CW), Some(0));
    builder.snapshot()
}

fn ordinary_rrect_path_at(rect: Rect, radius: &RectangularCornerRadius) -> Path {
    let rrect = RRect::new_rect_radii(
        rect,
        &[
            (radius.tl.rx, radius.tl.ry).into(),
            (radius.tr.rx, radius.tr.ry).into(),
            (radius.br.rx, radius.br.ry).into(),
            (radius.bl.rx, radius.bl.ry).into(),
        ],
    );
    // Point 0 is where the top-left curve joins the top edge. Keeping the
    // authored clockwise origin makes dash traversal deterministic.
    Path::rrect_with_start_index(rrect, PathDirection::CW, 0)
}

fn ordinary_rrect_path(w: f32, h: f32, radius: &RectangularCornerRadius) -> Path {
    ordinary_rrect_path_at(Rect::from_wh(w, h), radius)
}

/// Mirror the production engine's orthogonal smooth-corner construction.
///
/// That construction is circular-only today, so the XML boundary rejects
/// nonzero smoothing when any authored `rx != ry`. The defensive `min` here
/// retains production behavior for programmatically-built documents that do
/// not pass through the XML validator.
fn smooth_rrect_path(w: f32, h: f32, radius: &RectangularCornerRadius, smoothing: f32) -> Path {
    let shortest_side = w.min(h);
    let tl = smooth_corner_params(radius.tl.rx.min(radius.tl.ry), smoothing, shortest_side);
    let tr = smooth_corner_params(radius.tr.rx.min(radius.tr.ry), smoothing, shortest_side);
    let br = smooth_corner_params(radius.br.rx.min(radius.br.ry), smoothing, shortest_side);
    let bl = smooth_corner_params(radius.bl.rx.min(radius.bl.ry), smoothing, shortest_side);
    let mut builder = PathBuilder::new();

    // Start where the top-left curve joins the top edge, then wind clockwise.
    // This preserves the documented dash origin while tracing the same curve
    // as the production path, which starts midway along that top edge.
    builder.move_to((tl.extent.min(w / 2.0), 0.0));
    builder.line_to(((w - tr.extent).max(w / 2.0), 0.0));

    if tr.radius > 0.0 {
        builder.cubic_to(
            (w - (tr.extent - tr.a), 0.0),
            (w - (tr.extent - tr.a - tr.b), 0.0),
            (w - (tr.extent - tr.a - tr.b - tr.c), tr.d),
        );
        builder.arc_to(
            Rect::from_xywh(w - tr.radius * 2.0, 0.0, tr.radius * 2.0, tr.radius * 2.0),
            270.0 + tr.bezier_angle,
            90.0 - 2.0 * tr.bezier_angle,
            false,
        );
        builder.cubic_to(
            (w, tr.extent - tr.a - tr.b),
            (w, tr.extent - tr.a),
            (w, tr.extent.min(h / 2.0)),
        );
    }

    builder.line_to((w, (h - br.extent).max(h / 2.0)));
    if br.radius > 0.0 {
        builder.cubic_to(
            (w, h - (br.extent - br.a)),
            (w, h - (br.extent - br.a - br.b)),
            (w - br.d, h - (br.extent - br.a - br.b - br.c)),
        );
        builder.arc_to(
            Rect::from_xywh(
                w - br.radius * 2.0,
                h - br.radius * 2.0,
                br.radius * 2.0,
                br.radius * 2.0,
            ),
            br.bezier_angle,
            90.0 - 2.0 * br.bezier_angle,
            false,
        );
        builder.cubic_to(
            (w - (br.extent - br.a - br.b), h),
            (w - (br.extent - br.a), h),
            ((w - br.extent).max(w / 2.0), h),
        );
    }

    builder.line_to((bl.extent.min(w / 2.0), h));
    if bl.radius > 0.0 {
        builder.cubic_to(
            (bl.extent - bl.a, h),
            (bl.extent - bl.a - bl.b, h),
            (bl.extent - bl.a - bl.b - bl.c, h - bl.d),
        );
        builder.arc_to(
            Rect::from_xywh(0.0, h - bl.radius * 2.0, bl.radius * 2.0, bl.radius * 2.0),
            90.0 + bl.bezier_angle,
            90.0 - 2.0 * bl.bezier_angle,
            false,
        );
        builder.cubic_to(
            (0.0, h - (bl.extent - bl.a - bl.b)),
            (0.0, h - (bl.extent - bl.a)),
            (0.0, (h - bl.extent).max(h / 2.0)),
        );
    }

    builder.line_to((0.0, tl.extent.min(h / 2.0)));
    if tl.radius > 0.0 {
        builder.cubic_to(
            (0.0, tl.extent - tl.a),
            (0.0, tl.extent - tl.a - tl.b),
            (tl.d, tl.extent - tl.a - tl.b - tl.c),
        );
        builder.arc_to(
            Rect::from_xywh(0.0, 0.0, tl.radius * 2.0, tl.radius * 2.0),
            180.0 + tl.bezier_angle,
            90.0 - 2.0 * tl.bezier_angle,
            false,
        );
        builder.cubic_to(
            (tl.extent - tl.a - tl.b, 0.0),
            (tl.extent - tl.a, 0.0),
            (tl.extent.min(w / 2.0), 0.0),
        );
    }

    builder.close();
    builder.snapshot()
}

fn rounded_rect_path(w: f32, h: f32, radius: &RectangularCornerRadius, smoothing: f32) -> Path {
    if radius.is_zero() {
        rect_path(w, h)
    } else if smoothing == 0.0 {
        ordinary_rrect_path(w, h, radius)
    } else {
        smooth_rrect_path(w, h, radius, smoothing)
    }
}

fn expand_rect_by_widths(rect: Rect, widths: RectangularStrokeWidth, fraction: f32) -> Rect {
    Rect::from_ltrb(
        rect.left - widths.stroke_left_width * fraction,
        rect.top - widths.stroke_top_width * fraction,
        rect.right + widths.stroke_right_width * fraction,
        rect.bottom + widths.stroke_bottom_width * fraction,
    )
}

/// Insets without allowing Skia's `Rect::from_ltrb` normalization to turn an
/// overconsumed inner box inside out. Once either axis is exhausted the inner
/// contour is empty and the ring saturates to its outer contour.
fn inset_rect_by_widths(rect: Rect, widths: RectangularStrokeWidth, fraction: f32) -> Option<Rect> {
    let left = rect.left + widths.stroke_left_width * fraction;
    let top = rect.top + widths.stroke_top_width * fraction;
    let right = rect.right - widths.stroke_right_width * fraction;
    let bottom = rect.bottom - widths.stroke_bottom_width * fraction;
    (left < right && top < bottom).then(|| Rect::from_ltrb(left, top, right, bottom))
}

fn offset_radii_by_widths(
    radius: &RectangularCornerRadius,
    widths: RectangularStrokeWidth,
    fraction: f32,
) -> RectangularCornerRadius {
    let mut adjusted = *radius;
    adjusted.tl.rx = (adjusted.tl.rx + widths.stroke_left_width * fraction).max(0.0);
    adjusted.tl.ry = (adjusted.tl.ry + widths.stroke_top_width * fraction).max(0.0);
    adjusted.tr.rx = (adjusted.tr.rx + widths.stroke_right_width * fraction).max(0.0);
    adjusted.tr.ry = (adjusted.tr.ry + widths.stroke_top_width * fraction).max(0.0);
    adjusted.br.rx = (adjusted.br.rx + widths.stroke_right_width * fraction).max(0.0);
    adjusted.br.ry = (adjusted.br.ry + widths.stroke_bottom_width * fraction).max(0.0);
    adjusted.bl.rx = (adjusted.bl.rx + widths.stroke_left_width * fraction).max(0.0);
    adjusted.bl.ry = (adjusted.bl.ry + widths.stroke_bottom_width * fraction).max(0.0);
    adjusted
}

fn rectangular_stroke_contours(
    w: f32,
    h: f32,
    widths: RectangularStrokeWidth,
    radius: &RectangularCornerRadius,
    align: StrokeAlign,
) -> (Path, Option<Path>) {
    let base = Rect::from_wh(w, h);
    let (outward, inward) = match align {
        StrokeAlign::Inside => (0.0, 1.0),
        StrokeAlign::Center => (0.5, 0.5),
        StrokeAlign::Outside => (1.0, 0.0),
    };
    let outer_rect = expand_rect_by_widths(base, widths, outward);
    let outer_radius = offset_radii_by_widths(radius, widths, outward);
    let outer = ordinary_rrect_path_at(outer_rect, &outer_radius);
    let inner = inset_rect_by_widths(base, widths, inward).map(|inner_rect| {
        let inner_radius = offset_radii_by_widths(radius, widths, -inward);
        ordinary_rrect_path_at(inner_rect, &inner_radius)
    });
    (outer, inner)
}

fn rectangular_stroke_ring(outer: &Path, inner: Option<&Path>) -> Path {
    match inner {
        Some(inner) => skia_safe::op(outer, inner, PathOp::Difference).unwrap_or_default(),
        None => outer.clone(),
    }
}

fn rectangular_stroke_centerline(
    w: f32,
    h: f32,
    widths: RectangularStrokeWidth,
    radius: &RectangularCornerRadius,
    align: StrokeAlign,
) -> Option<Path> {
    let base = Rect::from_wh(w, h);
    let (rect, radius) = match align {
        StrokeAlign::Inside => (
            inset_rect_by_widths(base, widths, 0.5)?,
            offset_radii_by_widths(radius, widths, -0.5),
        ),
        StrokeAlign::Center => (base, *radius),
        StrokeAlign::Outside => (
            expand_rect_by_widths(base, widths, 0.5),
            offset_radii_by_widths(radius, widths, 0.5),
        ),
    };
    Some(ordinary_rrect_path_at(rect, &radius))
}

/// Project Grida's rectangular stroke-width union into one filled ring.
/// Solid strokes use the exact outer-minus-inner ring. Dashed strokes advance
/// once around a shared centerline at the maximum side width, then intersect
/// that outline with the ring so zero/thin sides suppress coverage without
/// resetting dash phase.
fn rectangular_stroke_geometry(
    w: f32,
    h: f32,
    radius: &RectangularCornerRadius,
    widths: RectangularStrokeWidth,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
) -> Path {
    if widths.is_none() {
        return Path::new();
    }
    let (outer, inner) = rectangular_stroke_contours(w, h, widths, radius, stroke.align);
    let ring = rectangular_stroke_ring(&outer, inner.as_ref());
    let Some(values) = stroke.dash_array.as_deref() else {
        return ring;
    };
    if values.is_empty() {
        return ring;
    }
    let Some(intervals) = normalized_dash_array(values) else {
        return Path::new();
    };
    let centerline =
        rectangular_stroke_centerline(w, h, widths, radius, stroke.align).unwrap_or(outer);
    let Some(effect) = PathEffect::dash(&intervals, dash_phase.value()) else {
        return Path::new();
    };
    let filter_rec = StrokeRec::new(InitStyle::Hairline);
    let Some((dashed, _)) = effect.filter_path(&centerline, &filter_rec, centerline.bounds())
    else {
        return Path::new();
    };
    let mut record = StrokeRec::new(InitStyle::Hairline);
    record.set_stroke_style(widths.max(), false);
    record.set_stroke_params(PaintCap::Butt, PaintJoin::Miter, stroke.miter_limit);
    let mut builder = PathBuilder::new();
    if !record.apply_to_path(&mut builder, &dashed.snapshot()) {
        return Path::new();
    }
    skia_safe::op(&builder.snapshot(), &ring, PathOp::Intersect).unwrap_or_default()
}

fn oval_path(w: f32, h: f32) -> Path {
    let mut builder = PathBuilder::new();
    // Explicit start index keeps dash origin at the rightmost point across
    // Skia versions (the library default changed historically).
    builder.add_oval(Rect::from_wh(w, h), Some(PathDirection::CW), Some(1));
    builder.snapshot()
}

fn line_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.add_line((x1, y1), (x2, y2));
    builder.snapshot()
}

/// The cap a solid *closed* contour must be stroked with, whatever the author
/// wrote. A dashed contour keeps the authored cap because every dash has ends.
///
/// A closed contour has no ends, so SVG's cap is inert on it — and Chromium
/// agrees: its butt, round and square captures of one are byte-identical to
/// each other at every width measured. Skia's stroker stops agreeing once the
/// device-space width falls to about one pixel; it paints the cap where the
/// contour rejoins. Measured against Chromium 149 on a 48x48 canvas, per
/// geometry and per cap, in differing pixels of 2304:
///
/// | device width | closed path | oval | rect | line (open) |
/// | --- | --- | --- | --- | --- |
/// | 0.5 · 1 | 84–95 | 84–95 | **0** | **0** |
/// | 1.25 and above | 0 | 0 | 0 | 0 |
///
/// Three facts decide where this is applied. It tracks the *device* width, not
/// the authored one — the same document at 2x diverges at an authored 0.5 and
/// agrees at an authored 1. Butt is byte-exact at every width, so butt is the
/// answer. And the divergence is per *arm*, not per closed contour: `draw_rect`
/// does not take the thin-stroke path, so [`ItemKind::RectStroke`] needs no
/// normalisation, while `draw_oval` and `draw_path` do.
///
/// Dashing changes that premise: even on a closed contour, every dash is an
/// open segment whose authored cap is visible. In particular, a zero-length
/// painted interval under a round or square cap is a dot; replacing that cap
/// with butt erases it. This helper therefore leaves dashed strokes unchanged.
///
/// `LineStroke` must never use the solid normalization: a line is open, its
/// caps are real, and Chromium's own captures of one are *not* cap-invariant.
/// `TextStroke` strokes glyph outlines, which are closed, but no admitted
/// source reaches it — the Web text slice refuses stroked text — so it remains
/// outside this measured normalization.
///
/// The caller applies this only when **every** contour is closed. A path that
/// mixes them cannot be served by one paint, and serving it by two draws is
/// worse: measured on a closed contour crossed by an open one, splitting is
/// byte-exact below a device pixel and then diverges by 32 to 47 pixels at 1.25
/// and 2, because the two runs' anti-aliased edges composite twice where they
/// overlap. So the mixed case refuses upstream instead, under its own name.
fn stroke_cap_for_closed_contours(stroke: &Stroke) -> Stroke {
    Stroke {
        cap: if stroke
            .dash_array
            .as_ref()
            .is_some_and(|intervals| !intervals.is_empty())
        {
            stroke.cap
        } else {
            StrokeCap::Butt
        },
        ..stroke.clone()
    }
}

#[cfg(test)]
mod closed_contour_cap_tests {
    use super::stroke_cap_for_closed_contours;
    use n0_model::model::{Paints, Stroke, StrokeAlign, StrokeCap, StrokeJoin, StrokeWidth};

    fn round_stroke(dash_array: Option<Vec<f32>>) -> Stroke {
        Stroke {
            paints: Paints::default(),
            width: StrokeWidth::Uniform(1.0),
            align: StrokeAlign::Center,
            cap: StrokeCap::Round,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
            dash_array,
        }
    }

    #[test]
    fn only_a_nonempty_dash_cycle_preserves_the_authored_cap() {
        assert_eq!(
            stroke_cap_for_closed_contours(&round_stroke(None)).cap,
            StrokeCap::Butt
        );
        assert_eq!(
            stroke_cap_for_closed_contours(&round_stroke(Some(Vec::new()))).cap,
            StrokeCap::Butt,
            "the private vocabulary treats an empty present array as solid"
        );
        assert_eq!(
            stroke_cap_for_closed_contours(&round_stroke(Some(vec![0.0, 8.0]))).cap,
            StrokeCap::Round,
            "an active dash cycle has cap-bearing segment ends"
        );
    }
}

#[cfg(test)]
mod dash_phase_route_tests {
    use super::*;
    use n0_model::model::{
        Color as ModelColor, Paints, Stroke, StrokeAlign, StrokeCap, StrokeJoin, StrokeWidth,
    };

    const W: i32 = 64;
    const H: i32 = 48;

    fn phase(value: f32) -> StrokeDashPhase {
        StrokeDashPhase::from_canonical(value)
    }

    fn dashed(width: StrokeWidth, align: StrokeAlign, cap: StrokeCap) -> Stroke {
        Stroke {
            paints: Paints::solid(ModelColor::BLACK),
            width,
            align,
            cap,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
            dash_array: Some(vec![8.0, 4.0]),
        }
    }

    fn raster(draw: impl FnOnce(&Canvas)) -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((W, H)).unwrap();
        {
            let canvas = surface.canvas();
            canvas.clear(Color::WHITE);
            draw(canvas);
        }
        read_pixels(&mut surface, W, H)
    }

    fn draw_black_path(canvas: &Canvas, path: &Path) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(Color::BLACK);
        canvas.draw_path(path, &paint);
    }

    fn outline_raster(cap: StrokeCap, dash_phase: StrokeDashPhase) -> Vec<u8> {
        let mut builder = PathBuilder::new();
        builder.add_rect(
            Rect::from_xywh(8.0, 8.0, 48.0, 32.0),
            Some(PathDirection::CW),
            Some(0),
        );
        let source = builder.snapshot();
        let stroke = dashed(StrokeWidth::Uniform(6.0), StrokeAlign::Outside, cap);
        let geometry = stroke_geometry(&source, &stroke, dash_phase);
        raster(|canvas| draw_black_path(canvas, &geometry))
    }

    fn rectangular_raster(dash_phase: StrokeDashPhase) -> Vec<u8> {
        let widths = RectangularStrokeWidth {
            stroke_top_width: 8.0,
            stroke_right_width: 4.0,
            stroke_bottom_width: 6.0,
            stroke_left_width: 2.0,
        };
        let stroke = dashed(
            StrokeWidth::Rectangular(widths),
            StrokeAlign::Center,
            StrokeCap::Butt,
        );
        let geometry = rectangular_stroke_geometry(
            48.0,
            32.0,
            &RectangularCornerRadius::default(),
            widths,
            &stroke,
            dash_phase,
        );
        raster(|canvas| {
            canvas.save();
            canvas.translate((8.0, 8.0));
            draw_black_path(canvas, &geometry);
            canvas.restore();
        })
    }

    fn native_multi_contour_raster(cap: StrokeCap, dash_phase: StrokeDashPhase) -> Vec<u8> {
        let mut builder = PathBuilder::new();
        builder.move_to((8.0, 14.0));
        builder.line_to((56.0, 14.0));
        builder.move_to((8.0, 34.0));
        builder.line_to((56.0, 34.0));
        let path = builder.snapshot();
        let stroke = dashed(StrokeWidth::Uniform(6.0), StrokeAlign::Center, cap);
        raster(|canvas| {
            let model = stroke.paints.iter().next().expect("one solid paint");
            let paint = native_stroke_paint(
                model,
                &stroke,
                dash_phase,
                PostPaintOpacity::IDENTITY,
                PaintBox::from_size(W as f32, H as f32),
                &PaintCtx::new(None),
            )
            .expect("finite checked dash material");
            canvas.draw_path(&path, &paint);
        })
    }

    fn row_band(pixels: &[u8], y0: usize, y1: usize) -> &[u8] {
        let row_bytes = W as usize * 4;
        &pixels[y0 * row_bytes..y1 * row_bytes]
    }

    #[test]
    fn canonical_phase_reaches_outline_and_rectangular_geometry_routes() {
        for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
            assert_ne!(
                outline_raster(cap, phase(0.0)),
                outline_raster(cap, phase(3.0))
            );
        }
        assert_ne!(
            rectangular_raster(phase(0.0)),
            rectangular_raster(phase(3.0))
        );
    }

    #[test]
    fn canonical_phase_reaches_native_caps_and_restarts_for_each_contour() {
        for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
            let zero = native_multi_contour_raster(cap, phase(0.0));
            let shifted = native_multi_contour_raster(cap, phase(3.0));
            assert_ne!(zero, shifted, "phase must affect the {cap:?} native route");
            assert_eq!(
                row_band(&shifted, 8, 21),
                row_band(&shifted, 28, 41),
                "each equal contour must restart at the same {cap:?} phase"
            );
        }
    }
}

/// Whether any contour may have no extent — a subpath that closes on the point
/// it opened at.
///
/// This is the exception to "a closed contour has no ends". A zero-length
/// subpath degenerates to a point, and SVG2 §13.2 makes the cap the *only*
/// thing that renders it: `M44 32 Z` under a square cap paints a dot Chromium
/// paints too, and normalising that cap to butt erases it. The corpus caught
/// exactly that.
///
/// Only on-curve endpoints are compared, never control points, so a curve whose
/// ends coincide reads as degenerate even where its hull bulges. That is the
/// safe direction: a false positive only declines to normalise a cap, while a
/// false negative would erase a dot the browser draws.
fn any_contour_may_be_degenerate(path: &ResolvedPathArtifact) -> bool {
    // `open` holds the current contour's start point, or `None` between
    // contours; `moved` is whether an endpoint has left that start.
    let mut open: Option<(f32, f32)> = None;
    let mut moved = false;
    for command in path.commands.iter() {
        match *command {
            PathCommand::MoveTo { x, y } => {
                if open.is_some() && !moved {
                    return true;
                }
                open = Some((x, y));
                moved = false;
            }
            PathCommand::Close => {
                if !moved {
                    return true;
                }
                open = None;
                moved = false;
            }
            PathCommand::LineTo { x, y }
            | PathCommand::QuadTo { x, y, .. }
            | PathCommand::CubicTo { x, y, .. }
            | PathCommand::ConicTo { x, y, .. } => moved |= open != Some((x, y)),
        }
    }
    open.is_some() && !moved
}

/// Project the already box-mapped, backend-independent command stream into
/// Skia. Resolution performed the only coordinate mapping, so bounds and
/// rasterization consume bit-identical f32 geometry.
fn backend_path(path: &ResolvedPathArtifact) -> Path {
    let fill_type = match path.fill_rule {
        FillRule::NonZero => PathFillType::Winding,
        FillRule::EvenOdd => PathFillType::EvenOdd,
    };
    let mut builder = PathBuilder::new_with_fill_type(fill_type);
    emit_commands(&mut builder, &path.commands);
    builder.snapshot()
}

/// Build one pre-resolved path-strategy clip with the same operation shape as
/// Chromium: each layer is an `SkOpBuilder` union, then chained layers use
/// pairwise intersection. `None` is a backend path-operation failure, never an
/// authored empty clip (an empty layer returns an empty `Path`).
fn backend_clip_path(clip: &ResolvedClipPath) -> Option<Path> {
    let mut layers = clip.layers.iter();
    let mut result = backend_clip_layer(layers.next().expect("resolved clip has a layer"))?;
    for layer in layers {
        let next = backend_clip_layer(layer)?;
        result = skia_safe::op(&result, &next, PathOp::Intersect)?;
    }
    Some(result)
}

fn backend_clip_layer(layer: &ResolvedClipLayer) -> Option<Path> {
    // Match Blink's operation shape, including empty contributors. The first
    // non-empty path is retained directly; only a later contributor promotes
    // the layer to `SkOpBuilder`. This is raster-observable at anti-aliased
    // edges, so `geometries.len() > 1` is not an equivalent shortcut.
    let mut resolved = Path::new();
    let mut builder: Option<OpBuilder> = None;
    for geometry in layer.geometries.iter() {
        let path = backend_clip_geometry(geometry);
        if let Some(builder) = &mut builder {
            builder.add(&path, PathOp::Union);
        } else if resolved.is_empty() {
            resolved = path;
        } else {
            let mut promoted = OpBuilder::default();
            promoted.add(&resolved, PathOp::Union);
            promoted.add(&path, PathOp::Union);
            builder = Some(promoted);
        }
    }
    match builder {
        Some(mut builder) => builder.resolve(),
        None => Some(resolved),
    }
}

fn backend_clip_geometry(geometry: &ResolvedClipGeometry) -> Path {
    let path = match &geometry.kind {
        ResolvedClipGeometryKind::Rect { x, y, w, h } => {
            let mut builder = PathBuilder::new();
            // Blink's PathBuilder starts rectangles at the upper-left and
            // walks clockwise. Keep those contour details explicit because
            // path operations and edge AA can observe them.
            builder.add_rect(
                Rect::from_xywh(*x, *y, *w, *h),
                Some(PathDirection::CW),
                Some(0),
            );
            builder.snapshot()
        }
        ResolvedClipGeometryKind::Oval { x, y, w, h } => {
            let mut builder = PathBuilder::new();
            // Blink's PathBuilder starts ellipses at 3 o'clock and walks
            // clockwise (`addOval(..., kCW, 1)`). Skia's implicit start index
            // is not that contract and differs at a few anti-aliased pixels.
            builder.add_oval(
                Rect::from_xywh(*x, *y, *w, *h),
                Some(PathDirection::CW),
                Some(1),
            );
            builder.snapshot()
        }
        ResolvedClipGeometryKind::Path(path) => backend_path(path),
    };
    path.make_transform(&skia_matrix(&geometry.world))
}

/// Product-build preflight for a geometric clip. Replay repeats the same pure
/// path operations and may therefore treat success as proven.
pub(crate) fn preflight_clip_path(clip: &ResolvedClipPath) -> bool {
    backend_clip_path(clip).is_some()
}

#[derive(Clone)]
struct BuiltFilterResult {
    /// `None` is the backend's input-image sentinel: the isolated scope's
    /// original composite, not a construction failure.
    image_filter: Option<ImageFilter>,
    color_space: ResolvedFilterColorSpace,
    /// Whether this result samples the isolated source image. Source-derived
    /// coverage must remain floating through Porter-Duff math; graphs made
    /// only from generated filter sources follow Chromium's unorm8 rounding.
    source_dependent: bool,
    /// Whether the result must cross the outer layer boundary with Chromium's
    /// exact byte-domain SrcOver. Native sRGB shadows reach Skia's lowp 8888
    /// restore, whose default division rounds differently on NEON and x86.
    requires_exact_restore: bool,
    /// Table and matrix color operations expose a measured final-layer
    /// boundary in the pinned backend. Source-derived output needs an explicit
    /// floating SrcOver; generated-only output needs the backend default.
    color_restore: Option<ColorRestore>,
    /// Chromium snapshots source-derived table, matrix, and active morphology
    /// input through one additional source-image boundary. Preserve that
    /// measured boundary through later graph nodes.
    source_preflatten: bool,
    /// An upstream procedural shader needs procedural-scoped blend arithmetic
    /// and final restore policy even after an intermediate sRGB result
    /// materializes. The next field tracks that narrower blend-domain state.
    procedural_provenance: bool,
    /// A direct sRGB procedural shader still enters Skia's byte-domain blend
    /// stages. Color conversion or another arithmetic primitive promotes that
    /// route to floating point even though procedural provenance remains.
    procedural_unorm8_blend: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorRestore {
    Default,
    Floating,
}

struct BuiltFilter {
    image_filter: Option<ImageFilter>,
    /// Generated-only graphs finish with the same exact unorm8 SrcOver used
    /// between their internal nodes. Source-derived coverage stays on Skia's
    /// floating restore path.
    restore_blender: Option<Blender>,
    source_preflatten: bool,
}

fn source_alpha_filter() -> Option<ImageFilter> {
    let mut matrix = ColorMatrix::default();
    matrix.set_scale(0.0, 0.0, 0.0, 1.0);
    let color_filter = skia_safe::color_filters::matrix(&matrix, None);
    skia_safe::image_filters::color_filter(
        color_filter,
        None,
        skia_safe::image_filters::CropRect::default(),
    )
}

fn convert_filter_space(
    result: BuiltFilterResult,
    target: ResolvedFilterColorSpace,
) -> Result<BuiltFilterResult, String> {
    if result.color_space == target {
        return Ok(result);
    }
    let color_filter = match (result.color_space, target) {
        (ResolvedFilterColorSpace::Srgb, ResolvedFilterColorSpace::LinearRgb) => {
            skia_safe::color_filters::srgb_to_linear_gamma()
        }
        (ResolvedFilterColorSpace::LinearRgb, ResolvedFilterColorSpace::Srgb) => {
            skia_safe::color_filters::linear_to_srgb_gamma()
        }
        _ => unreachable!("equal spaces returned above"),
    };
    let image_filter = skia_safe::image_filters::color_filter(
        color_filter,
        result.image_filter,
        skia_safe::image_filters::CropRect::default(),
    )
    .ok_or_else(|| "the backend could not construct a color-space conversion".to_string())?;
    Ok(BuiltFilterResult {
        image_filter: Some(image_filter),
        color_space: target,
        source_dependent: result.source_dependent,
        // A color-space conversion resumes the floating filter path; its
        // eventual restore must not be forced through the N32 shadow rule.
        requires_exact_restore: false,
        // Gamma conversion materializes a new floating result and ends the
        // matrix-specific sRGB restore boundary.
        color_restore: None,
        source_preflatten: result.source_preflatten,
        procedural_provenance: result.procedural_provenance,
        // Gamma conversion promotes the next blend to Skia's floating path.
        procedural_unorm8_blend: false,
    })
}

fn transparent_filter(region: n0_model::math::RectF) -> Option<ImageFilter> {
    skia_safe::image_filters::shader(
        shaders::color(Color::TRANSPARENT),
        Rect::from_xywh(region.x, region.y, region.w, region.h),
    )
}

thread_local! {
    // Runtime-effect handles are cheap to clone after compilation. Keep the
    // deterministic filter blenders local to the painting thread instead of
    // rebuilding SkSL for every graph replay.
    static FILTER_BLENDERS: RefCell<Vec<Option<Blender>>> =
        const { RefCell::new(Vec::new()) };
}

fn compile_filter_blender(source: String) -> Result<Blender, String> {
    let options = skia_safe::runtime_effect::Options {
        force_unoptimized: true,
        name: "n0_svg_filter_blender",
    };
    let effect = skia_safe::RuntimeEffect::make_for_blender(source, Some(&options))
        .map_err(|error| format!("the backend could not compile a filter blender: {error}"))?;
    effect
        .make_blender(Data::new_empty(), None)
        .ok_or_else(|| "the backend could not construct a filter blender".to_string())
}

fn cached_filter_blender(slot: usize, source: impl FnOnce() -> String) -> Result<Blender, String> {
    FILTER_BLENDERS.with(|cache| {
        if let Some(blender) = cache.borrow().get(slot).and_then(Option::as_ref).cloned() {
            return Ok(blender);
        }
        let blender = compile_filter_blender(source())?;
        let mut cache = cache.borrow_mut();
        if cache.len() <= slot {
            cache.resize_with(slot + 1, || None);
        }
        cache[slot] = Some(blender.clone());
        Ok(blender)
    })
}

fn porter_duff_slot(operator: ResolvedFilterComposite) -> Result<usize, String> {
    match operator {
        ResolvedFilterComposite::Over => Ok(0),
        ResolvedFilterComposite::In => Ok(1),
        ResolvedFilterComposite::Out => Ok(2),
        ResolvedFilterComposite::Atop => Ok(3),
        ResolvedFilterComposite::Xor => Ok(4),
        ResolvedFilterComposite::Lighter => Ok(5),
        ResolvedFilterComposite::Arithmetic { .. } => {
            Err("arithmetic composition does not use a Porter-Duff blender".to_string())
        }
    }
}

fn porter_duff_expression(operator: ResolvedFilterComposite) -> Result<&'static str, String> {
    let expression = match operator {
        ResolvedFilterComposite::Over => "src + dst * (1.0 - src.a)",
        ResolvedFilterComposite::In => "src * dst.a",
        ResolvedFilterComposite::Out => "src * (1.0 - dst.a)",
        ResolvedFilterComposite::Atop => "src * dst.a + dst * (1.0 - src.a)",
        ResolvedFilterComposite::Xor => "src * (1.0 - dst.a) + dst * (1.0 - src.a)",
        ResolvedFilterComposite::Lighter => "min(src + dst, half4(1.0))",
        ResolvedFilterComposite::Arithmetic { .. } => {
            return Err("arithmetic composition does not use a Porter-Duff blender".to_string());
        }
    };
    Ok(expression)
}

fn floating_porter_duff_blender(operator: ResolvedFilterComposite) -> Result<Blender, String> {
    let slot = porter_duff_slot(operator)?;
    let expression = porter_duff_expression(operator)?;
    cached_filter_blender(slot, || {
        format!(r#"half4 main(half4 src, half4 dst) {{ return {expression}; }}"#)
    })
}

fn quantized_floating_porter_duff_blender(
    operator: ResolvedFilterComposite,
) -> Result<Blender, String> {
    // Procedural shaders must stay floating until the final composition, but
    // leaving the last N32 quantization implicit selects different Skia CPU
    // packing paths on NEON and x86. Quantize only the composed result: doing
    // so earlier changes Chromium's blend and morphology arithmetic.
    let slot = 21 + porter_duff_slot(operator)?;
    let expression = porter_duff_expression(operator)?
        .replace("src", "s")
        .replace("dst", "d");
    cached_filter_blender(slot, || {
        format!(
            r#"
half4 main(half4 src, half4 dst) {{
    float4 s = float4(src);
    float4 d = float4(dst);
    float4 result = {expression};
    return half4(floor(clamp(result, 0.0, 1.0) * 255.0 + 0.5) / 255.0);
}}
"#
        )
    })
}

fn exact_unorm8_blender(operator: ResolvedFilterComposite) -> Result<Blender, String> {
    // Skia's low-precision blend stages deliberately use an approximate
    // divide-by-255 on x86 while NEON uses the accurate operation. Spell the
    // rounding for generated-source graphs so both CPU families agree.
    let slot = 6 + porter_duff_slot(operator)?;
    let expression = match operator {
        ResolvedFilterComposite::Over => "s + div255(d * (255.0 - s.a))",
        ResolvedFilterComposite::In => "div255(s * d.a)",
        ResolvedFilterComposite::Out => "div255(s * (255.0 - d.a))",
        ResolvedFilterComposite::Atop => "div255(s * d.a + d * (255.0 - s.a))",
        ResolvedFilterComposite::Xor => "div255(s * (255.0 - d.a) + d * (255.0 - s.a))",
        ResolvedFilterComposite::Lighter => "min(s + d, float4(255.0))",
        ResolvedFilterComposite::Arithmetic { .. } => {
            return Err("arithmetic composition does not use a Porter-Duff blender".to_string());
        }
    };
    cached_filter_blender(slot, || {
        format!(
            r#"
float4 div255(float4 value) {{
    return floor((value + 127.0) / 255.0);
}}

half4 main(half4 src, half4 dst) {{
    float4 s = floor(float4(src) * 255.0 + 0.5);
    float4 d = floor(float4(dst) * 255.0 + 0.5);
    float4 result = {expression};
    return half4(clamp(result, 0.0, 255.0) / 255.0);
}}
"#
        )
    })
}

fn deterministic_porter_duff_blender(
    operator: ResolvedFilterComposite,
    source_dependent: bool,
) -> Result<Blender, String> {
    if source_dependent {
        floating_porter_duff_blender(operator)
    } else {
        exact_unorm8_blender(operator)
    }
}

// Skia's N32 low-precision pipeline uses exact divide-by-255 rounding on
// NEON, but an intentionally approximate `(value + 255) / 256` on x86. The
// approximation is observable in SVG blend pixels. Re-state the nine modes
// that use that pipeline over explicit unorm8 values so both CPU families
// reproduce the committed Chromium bytes. The remaining seven modes use Skia's
// high-precision path and stay native unless measurement proves otherwise.
fn exact_unorm8_filter_blend_source(expression: &str) -> String {
    format!(
        r#"
float div255(float value) {{
    return floor((value + 127.0) / 255.0);
}}

float4 div255(float4 value) {{
    return floor((value + 127.0) / 255.0);
}}

float3 div255_3(float3 value) {{
    return floor((value + 127.0) / 255.0);
}}

float overlay_channel(float s, float d, float sa, float da) {{
    float blend = 2.0 * d <= da
        ? 2.0 * s * d
        : sa * da - 2.0 * (sa - s) * (da - d);
    return div255(s * (255.0 - da) + d * (255.0 - sa) + blend);
}}

float hard_light_channel(float s, float d, float sa, float da) {{
    float blend = 2.0 * s <= sa
        ? 2.0 * s * d
        : sa * da - 2.0 * (sa - s) * (da - d);
    return div255(s * (255.0 - da) + d * (255.0 - sa) + blend);
}}

half4 main(half4 src, half4 dst) {{
    float4 s = floor(float4(src) * 255.0 + 0.5);
    float4 d = floor(float4(dst) * 255.0 + 0.5);
    float4 result = {expression};
    return half4(clamp(result, 0.0, 255.0) / 255.0);
}}
"#
    )
}

fn floating_filter_blend_source(expression: &str) -> String {
    format!(
        r#"
float overlay_channel(float s, float d, float sa, float da) {{
    float blend = 2.0 * d <= da
        ? 2.0 * s * d
        : sa * da - 2.0 * (sa - s) * (da - d);
    return s * (1.0 - da) + d * (1.0 - sa) + blend;
}}

float hard_light_channel(float s, float d, float sa, float da) {{
    float blend = 2.0 * s <= sa
        ? 2.0 * s * d
        : sa * da - 2.0 * (sa - s) * (da - d);
    return s * (1.0 - da) + d * (1.0 - sa) + blend;
}}

float3 unorm8_product(float3 value) {{
    return floor((value * 65025.0 + 127.0) / 255.0) / 255.0;
}}

half4 main(half4 src, half4 dst) {{
    float4 s = float4(src);
    float4 d = float4(dst);
    float4 result = {expression};
    return half4(clamp(result, 0.0, 1.0));
}}
"#
    )
}

fn deterministic_filter_blender(mode: ResolvedFilterBlend) -> Result<Blender, String> {
    let (slot, expression) = match mode {
        ResolvedFilterBlend::Normal => (12, "s + div255(d * (255.0 - s.a))"),
        ResolvedFilterBlend::Multiply => (
            13,
            "div255(s * (255.0 - d.a) + d * (255.0 - s.a) + s * d)",
        ),
        ResolvedFilterBlend::Screen => (14, "s + d - div255(s * d)"),
        ResolvedFilterBlend::Overlay => (
            15,
            "float4(overlay_channel(s.r, d.r, s.a, d.a), overlay_channel(s.g, d.g, s.a, d.a), overlay_channel(s.b, d.b, s.a, d.a), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        ResolvedFilterBlend::Darken => (
            16,
            "float4(s.rgb + d.rgb - div255_3(max(s.rgb * d.a, d.rgb * s.a)), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        ResolvedFilterBlend::Lighten => (
            17,
            "float4(s.rgb + d.rgb - div255_3(min(s.rgb * d.a, d.rgb * s.a)), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        ResolvedFilterBlend::HardLight => (
            18,
            "float4(hard_light_channel(s.r, d.r, s.a, d.a), hard_light_channel(s.g, d.g, s.a, d.a), hard_light_channel(s.b, d.b, s.a, d.a), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        ResolvedFilterBlend::Difference => (
            19,
            "float4(s.rgb + d.rgb - 2.0 * div255_3(min(s.rgb * d.a, d.rgb * s.a)), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        ResolvedFilterBlend::Exclusion => (
            20,
            "float4(s.rgb + d.rgb - 2.0 * div255_3(s.rgb * d.rgb), s.a + div255(d.a * (255.0 - s.a)))",
        ),
        _ => return Ok(sk_filter_blend_mode(mode).into()),
    };
    cached_filter_blender(slot, || exact_unorm8_filter_blend_source(expression))
}

fn procedural_filter_blender(
    mode: ResolvedFilterBlend,
    unorm8_input: bool,
) -> Result<Blender, String> {
    // The same nine modes that enter Skia's architecture-dependent lowp path
    // need explicit formulas for procedural inputs. Most stay floating; direct
    // or materialized sRGB difference/exclusion retain one measured unorm8
    // product-rounding step. The remaining modes use Skia's high-precision path.
    let (slot, expression) = match mode {
        ResolvedFilterBlend::Normal => (27, "s + d * (1.0 - s.a)"),
        ResolvedFilterBlend::Multiply => (
            28,
            "s * (1.0 - d.a) + d * (1.0 - s.a) + s * d",
        ),
        ResolvedFilterBlend::Screen => (29, "s + d - s * d"),
        ResolvedFilterBlend::Overlay => (
            30,
            "float4(overlay_channel(s.r, d.r, s.a, d.a), overlay_channel(s.g, d.g, s.a, d.a), overlay_channel(s.b, d.b, s.a, d.a), s.a + d.a - s.a * d.a)",
        ),
        ResolvedFilterBlend::Darken => (31, "s + d - max(s * d.a, d * s.a)"),
        ResolvedFilterBlend::Lighten => (32, "s + d - min(s * d.a, d * s.a)"),
        ResolvedFilterBlend::HardLight => (
            33,
            "float4(hard_light_channel(s.r, d.r, s.a, d.a), hard_light_channel(s.g, d.g, s.a, d.a), hard_light_channel(s.b, d.b, s.a, d.a), s.a + d.a - s.a * d.a)",
        ),
        ResolvedFilterBlend::Difference => (
            if unorm8_input { 36 } else { 34 },
            if unorm8_input {
                "float4(s.rgb + d.rgb - 2.0 * unorm8_product(min(s.rgb * d.a, d.rgb * s.a)), s.a + d.a - s.a * d.a)"
            } else {
                "float4(s.rgb + d.rgb - 2.0 * min(s.rgb * d.a, d.rgb * s.a), s.a + d.a - s.a * d.a)"
            },
        ),
        ResolvedFilterBlend::Exclusion => (
            if unorm8_input { 37 } else { 35 },
            if unorm8_input {
                "float4(s.rgb + d.rgb - 2.0 * unorm8_product(s.rgb * d.rgb), s.a + d.a - s.a * d.a)"
            } else {
                "float4(s.rgb + d.rgb - 2.0 * s.rgb * d.rgb, s.a + d.a - s.a * d.a)"
            },
        ),
        _ => return Ok(sk_filter_blend_mode(mode).into()),
    };
    cached_filter_blender(slot, || floating_filter_blend_source(expression))
}

fn sk_displacement_channel(channel: ResolvedFilterDisplacementChannel) -> ColorChannel {
    match channel {
        ResolvedFilterDisplacementChannel::Red => ColorChannel::R,
        ResolvedFilterDisplacementChannel::Green => ColorChannel::G,
        ResolvedFilterDisplacementChannel::Blue => ColorChannel::B,
        ResolvedFilterDisplacementChannel::Alpha => ColorChannel::A,
    }
}

fn sk_convolve_tile_mode(mode: ResolvedFilterConvolveEdgeMode) -> skia_safe::TileMode {
    match mode {
        ResolvedFilterConvolveEdgeMode::Duplicate => skia_safe::TileMode::Clamp,
        ResolvedFilterConvolveEdgeMode::Wrap => skia_safe::TileMode::Repeat,
        ResolvedFilterConvolveEdgeMode::None => skia_safe::TileMode::Decal,
    }
}

fn sk_lighting_color(color: n0_model::model::Color, space: ResolvedFilterColorSpace) -> Color {
    let argb = color.argb();
    let channel = |shift| ((argb >> shift) & 0xff_u32) as u8;
    let to_linear_byte = |component: u8| {
        let component = f32::from(component) / 255.0;
        let linear = if component <= 0.04045 {
            component / 12.92
        } else {
            ((component + 0.055) / 1.055).powf(2.4)
        };
        (linear * 255.0).round() as u8
    };
    let (r, g, b) = (channel(16), channel(8), channel(0));
    match space {
        ResolvedFilterColorSpace::Srgb => Color::from_argb(u8::MAX, r, g, b),
        ResolvedFilterColorSpace::LinearRgb => Color::from_argb(
            u8::MAX,
            to_linear_byte(r),
            to_linear_byte(g),
            to_linear_byte(b),
        ),
    }
}

fn sk_point3(point: [f32; 3]) -> Point3 {
    Point3::new(point[0], point[1], point[2])
}

/// Build one checked private filter graph and its final-composition policy.
fn build_filter(filter: &ResolvedFilter) -> Result<BuiltFilter, String> {
    let explicit_transparent_source = if filter.source_is_transparent {
        Some(transparent_filter(filter.region).ok_or_else(|| {
            "the backend could not construct an explicit transparent filter source".to_string()
        })?)
    } else {
        None
    };
    let source = BuiltFilterResult {
        image_filter: explicit_transparent_source.clone(),
        color_space: ResolvedFilterColorSpace::Srgb,
        source_dependent: true,
        requires_exact_restore: false,
        color_restore: None,
        source_preflatten: false,
        procedural_provenance: false,
        procedural_unorm8_blend: false,
    };
    let source_alpha = BuiltFilterResult {
        image_filter: if let Some(source) = explicit_transparent_source {
            Some(source)
        } else {
            Some(source_alpha_filter().ok_or_else(|| {
                "the backend could not construct the SourceAlpha input".to_string()
            })?)
        },
        color_space: ResolvedFilterColorSpace::Srgb,
        source_dependent: true,
        requires_exact_restore: false,
        color_restore: None,
        source_preflatten: false,
        procedural_provenance: false,
        procedural_unorm8_blend: false,
    };
    let mut results: Vec<BuiltFilterResult> = Vec::with_capacity(filter.nodes.len());
    for node in filter.nodes.iter() {
        let mut inputs = Vec::with_capacity(node.inputs.len());
        for input in node.inputs.iter() {
            let input = match *input {
                ResolvedFilterInput::Source => source.clone(),
                ResolvedFilterInput::SourceAlpha => source_alpha.clone(),
                ResolvedFilterInput::Node(index) => results
                    .get(index)
                    .cloned()
                    .expect("the resolved filter contract checked every node index"),
            };
            inputs.push(convert_filter_space(input, node.color_space)?);
        }
        let crop = Rect::from_xywh(node.region.x, node.region.y, node.region.w, node.region.h);
        let source_dependent = inputs.iter().any(|input| input.source_dependent);
        let requires_exact_restore = inputs.iter().any(|input| input.requires_exact_restore);
        let inherited_color_restore = if inputs
            .iter()
            .any(|input| input.color_restore == Some(ColorRestore::Floating))
        {
            Some(ColorRestore::Floating)
        } else if inputs
            .iter()
            .any(|input| input.color_restore == Some(ColorRestore::Default))
        {
            Some(ColorRestore::Default)
        } else {
            None
        };
        let color_restore = match node.primitive.clone() {
            ResolvedFilterPrimitive::ColorMatrix { matrix } => {
                Some(if matrix[19] > 0.0 || !source_dependent {
                    ColorRestore::Default
                } else {
                    ColorRestore::Floating
                })
            }
            ResolvedFilterPrimitive::ComponentTransfer { tables } => {
                Some(if tables[3][0] > 0 || !source_dependent {
                    ColorRestore::Default
                } else {
                    ColorRestore::Floating
                })
            }
            ResolvedFilterPrimitive::DiffuseLighting { .. } => Some(ColorRestore::Default),
            ResolvedFilterPrimitive::SolidColor { .. } => None,
            _ => inherited_color_restore,
        };
        let source_preflatten = inputs.iter().any(|input| input.source_preflatten)
            || matches!(
                &node.primitive,
                ResolvedFilterPrimitive::ColorMatrix { matrix }
                    if source_dependent && matrix[19] <= 0.0
            )
            || matches!(
                &node.primitive,
                ResolvedFilterPrimitive::ComponentTransfer { .. } if source_dependent
            )
            || matches!(
                &node.primitive,
                ResolvedFilterPrimitive::Morphology { radius_x, radius_y, .. }
                    if source_dependent && (*radius_x > 0.0 || *radius_y > 0.0)
            )
            || matches!(
                &node.primitive,
                ResolvedFilterPrimitive::ConvolveMatrix { .. } if source_dependent
            );
        let has_procedural_input = inputs.iter().any(|input| input.procedural_provenance);
        let mut procedural_provenance = has_procedural_input
            || matches!(&node.primitive, ResolvedFilterPrimitive::Turbulence { .. });
        let mut procedural_unorm8_blend =
            if matches!(&node.primitive, ResolvedFilterPrimitive::Turbulence { .. }) {
                node.color_space == ResolvedFilterColorSpace::Srgb
            } else {
                has_procedural_input
                    && inputs
                        .iter()
                        .filter(|input| input.procedural_provenance)
                        .all(|input| input.procedural_unorm8_blend)
            };
        let (image_filter, output_space, source_dependent, requires_exact_restore) = match node
            .primitive
            .clone()
        {
            ResolvedFilterPrimitive::GaussianBlur { sigma_x, sigma_y } => {
                let input = inputs.pop().expect("Gaussian blur has one checked input");
                if sigma_x == 0.0 && sigma_y == 0.0 {
                    let filter = skia_safe::image_filters::crop(
                        crop,
                        Some(skia_safe::TileMode::Decal),
                        input.image_filter,
                    )
                    .ok_or_else(|| {
                        "the backend could not construct a zero-sigma blur crop".to_string()
                    })?;
                    (
                        Some(filter),
                        node.color_space,
                        input.source_dependent,
                        input.requires_exact_restore,
                    )
                } else {
                    let filter = skia_safe::image_filters::blur(
                        (sigma_x, sigma_y),
                        Some(skia_safe::TileMode::Decal),
                        input.image_filter,
                        crop,
                    )
                    .ok_or_else(|| {
                        "the backend could not construct a Gaussian blur operation".to_string()
                    })?;
                    (
                        Some(filter),
                        node.color_space,
                        input.source_dependent,
                        input.requires_exact_restore,
                    )
                }
            }
            ResolvedFilterPrimitive::Offset { dx, dy } => {
                let input = inputs.pop().expect("offset has one checked input");
                let filter = skia_safe::image_filters::offset((dx, dy), input.image_filter, crop)
                    .ok_or_else(|| {
                    "the backend could not construct an offset operation".to_string()
                })?;
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    input.requires_exact_restore,
                )
            }
            ResolvedFilterPrimitive::SolidColor { color } => {
                let color = Color4f::new(color.r(), color.g(), color.b(), color.a());
                // Blink's FEFlood is a constant Src color filter over a null
                // input, not a shader image filter. Those graph shapes differ
                // under arithmetic and gamma conversion even when a lone
                // flood stores the same pixels.
                let color_filter = skia_safe::color_filters::blend_with_color_space(
                    color,
                    Option::<ColorSpace>::None,
                    skia_safe::BlendMode::Src,
                )
                .ok_or_else(|| {
                    "the backend could not construct a solid-source color filter".to_string()
                })?;
                let filter = skia_safe::image_filters::color_filter(color_filter, None, crop)
                    .ok_or_else(|| {
                        "the backend could not construct a solid filter source".to_string()
                    })?;
                (Some(filter), ResolvedFilterColorSpace::Srgb, false, false)
            }
            ResolvedFilterPrimitive::Composite { operator } => {
                let background = inputs.pop().expect("composite has two checked inputs");
                let foreground = inputs.pop().expect("composite has two checked inputs");
                let filter = match operator {
                    ResolvedFilterComposite::Arithmetic { k1, k2, k3, k4 } => {
                        skia_safe::image_filters::arithmetic(
                            k1,
                            k2,
                            k3,
                            k4,
                            true,
                            background.image_filter,
                            foreground.image_filter,
                            crop,
                        )
                    }
                    operator => skia_safe::image_filters::blend(
                        deterministic_porter_duff_blender(operator, source_dependent)?,
                        background.image_filter,
                        foreground.image_filter,
                        crop,
                    ),
                }
                .ok_or_else(|| {
                    "the backend could not construct a composite operation".to_string()
                })?;
                (
                    Some(filter),
                    node.color_space,
                    source_dependent,
                    requires_exact_restore,
                )
            }
            ResolvedFilterPrimitive::Blend { mode } => {
                let background = inputs.pop().expect("blend has two checked inputs");
                let foreground = inputs.pop().expect("blend has two checked inputs");
                let blender = if procedural_provenance {
                    procedural_filter_blender(mode, procedural_unorm8_blend)?
                } else {
                    deterministic_filter_blender(mode)?
                };
                let filter = skia_safe::image_filters::blend(
                    blender,
                    background.image_filter,
                    foreground.image_filter,
                    crop,
                )
                .ok_or_else(|| "the backend could not construct a blend operation".to_string())?;
                // Exact mode arithmetic can still differ by one code value
                // across NEON and x86 during the final N32 sRGB restore. A
                // later color-space conversion clears this policy before its
                // own floating-point arithmetic.
                let requires_exact_restore =
                    requires_exact_restore || node.color_space == ResolvedFilterColorSpace::Srgb;
                // The runtime blender computes this operation in the measured
                // domain, then an sRGB result materializes before a later blend.
                // Linear output remains floating. Final procedural provenance
                // is independent and survives either transition.
                procedural_unorm8_blend = node.color_space == ResolvedFilterColorSpace::Srgb;
                (
                    Some(filter),
                    node.color_space,
                    source_dependent,
                    requires_exact_restore,
                )
            }
            ResolvedFilterPrimitive::DropShadow {
                dx,
                dy,
                sigma_x,
                sigma_y,
                color,
            } => {
                let input = inputs.pop().expect("drop shadow has one checked input");
                let to_linear = |component: f32| {
                    if component <= 0.04045 {
                        component / 12.92
                    } else {
                        ((component + 0.055) / 1.055).powf(2.4)
                    }
                };
                // Blink resolves flood-color in device sRGB, multiplies its
                // alpha by flood-opacity, then adapts the color channels to
                // this primitive's operating interpolation space before
                // constructing one native shadow-and-foreground filter.
                let (r, g, b) = match node.color_space {
                    ResolvedFilterColorSpace::Srgb => (color.r(), color.g(), color.b()),
                    ResolvedFilterColorSpace::LinearRgb => (
                        to_linear(color.r()),
                        to_linear(color.g()),
                        to_linear(color.b()),
                    ),
                };
                let shadow = Color4f::new(r, g, b, color.a());
                // Skia's DropShadow helper is itself Blur -> solid SrcIn ->
                // linear MatrixTransform -> Merge. Spell the blend stages
                // explicitly so their byte-domain rounding is architecture
                // independent while retaining the helper's native blur and
                // linear-offset raster.
                let blurred = skia_safe::image_filters::blur(
                    (sigma_x, sigma_y),
                    Some(skia_safe::TileMode::Decal),
                    input.image_filter.clone(),
                    skia_safe::image_filters::CropRect::default(),
                )
                .ok_or_else(|| {
                    "the backend could not construct a native drop-shadow blur".to_string()
                })?;
                let colorize = skia_safe::color_filters::blend_with_color_space(
                    shadow,
                    Option::<ColorSpace>::None,
                    skia_safe::BlendMode::Src,
                )
                .ok_or_else(|| {
                    "the backend could not construct a native drop-shadow colorizer".to_string()
                })?;
                let pre_offset_region = Rect::from_xywh(
                    node.region.x - dx,
                    node.region.y - dy,
                    node.region.w,
                    node.region.h,
                );
                let solid =
                    skia_safe::image_filters::color_filter(colorize, None, pre_offset_region)
                        .ok_or_else(|| {
                            "the backend could not construct a native drop-shadow color source"
                                .to_string()
                        })?;
                let colored = skia_safe::image_filters::blend(
                    exact_unorm8_blender(ResolvedFilterComposite::In)?,
                    Some(blurred),
                    Some(solid),
                    pre_offset_region,
                )
                .ok_or_else(|| {
                    "the backend could not colorize a native drop-shadow raster".to_string()
                })?;
                let shadow_filter = skia_safe::image_filters::matrix_transform(
                    &Matrix::translate((dx, dy)),
                    skia_safe::FilterMode::Linear,
                    Some(colored),
                )
                .ok_or_else(|| {
                    "the backend could not offset a native drop-shadow raster".to_string()
                })?;
                let foreground_blender: Blender =
                    exact_unorm8_blender(ResolvedFilterComposite::Over)?;
                let filter = skia_safe::image_filters::blend(
                    foreground_blender,
                    Some(shadow_filter),
                    input.image_filter,
                    crop,
                )
                .ok_or_else(|| {
                    "the backend could not compose a native drop-shadow foreground".to_string()
                })?;
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    node.color_space == ResolvedFilterColorSpace::Srgb,
                )
            }
            ResolvedFilterPrimitive::ColorMatrix { matrix } => {
                let input = inputs.pop().expect("color matrix has one checked input");
                procedural_unorm8_blend = false;
                let color_filter = skia_safe::color_filters::matrix_row_major(&matrix, None);
                let filter =
                    skia_safe::image_filters::color_filter(color_filter, input.image_filter, crop)
                        .ok_or_else(|| {
                            "the backend could not construct a color-matrix operation".to_string()
                        })?;
                // Matrix arithmetic supersedes any final-restore policy on
                // its input. Chromium's source-derived and generated-only
                // matrix results take two distinct measured sRGB restores.
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    false,
                )
            }
            ResolvedFilterPrimitive::ComponentTransfer { tables } => {
                let input = inputs
                    .pop()
                    .expect("component transfer has one checked input");
                procedural_unorm8_blend = false;
                let color_filter = skia_safe::color_filters::table_argb(
                    Some(&tables[3]),
                    Some(&tables[0]),
                    Some(&tables[1]),
                    Some(&tables[2]),
                )
                .ok_or_else(|| {
                    "the backend could not construct a component-transfer table".to_string()
                })?;
                let filter =
                    skia_safe::image_filters::color_filter(color_filter, input.image_filter, crop)
                        .ok_or_else(|| {
                            "the backend could not construct a component-transfer operation"
                                .to_string()
                        })?;
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    false,
                )
            }
            ResolvedFilterPrimitive::Morphology {
                operator,
                radius_x,
                radius_y,
            } => {
                let input = inputs.pop().expect("morphology has one checked input");
                let active = radius_x > 0.0 || radius_y > 0.0;
                // Active sRGB morphology over a direct procedural source
                // establishes the exact byte restore Chromium uses for that
                // kernel. A preceding exact blend leaves its stronger
                // procedural-composition provenance intact.
                if active
                    && node.color_space == ResolvedFilterColorSpace::Srgb
                    && !input.requires_exact_restore
                {
                    procedural_provenance = false;
                }
                if active {
                    procedural_unorm8_blend = false;
                }
                let filter = match operator {
                    ResolvedFilterMorphology::Erode => skia_safe::image_filters::erode(
                        (radius_x, radius_y),
                        input.image_filter,
                        crop,
                    ),
                    ResolvedFilterMorphology::Dilate => skia_safe::image_filters::dilate(
                        (radius_x, radius_y),
                        input.image_filter,
                        crop,
                    ),
                }
                .ok_or_else(|| {
                    "the backend could not construct a morphology operation".to_string()
                })?;
                // Native sRGB morphology reaches Skia's low-precision N32
                // layer restore. NEON performs exact divide-by-255 rounding
                // there, while x86 uses the backend's approximate division;
                // carry the same explicit byte-domain restore used by the
                // other measured low-precision filter operations. A zero
                // radius stays on the pre-existing pass-through policy, and
                // a later color-space conversion clears this flag.
                let requires_exact_restore = input.requires_exact_restore
                    || (active && node.color_space == ResolvedFilterColorSpace::Srgb);
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    requires_exact_restore,
                )
            }
            ResolvedFilterPrimitive::Turbulence {
                kind,
                base_frequency_x,
                base_frequency_y,
                num_octaves,
                seed,
                stitch_tiles,
            } => {
                // Blink truncates the finite primitive subregion dimensions
                // to signed integer tile lengths before constructing a
                // stitched Perlin source. Rust's float-to-int cast has the
                // same saturating, truncating contract.
                let tile_size =
                    stitch_tiles.then(|| ISize::new(node.region.w as i32, node.region.h as i32));
                let shader = match kind {
                    ResolvedFilterTurbulenceKind::Turbulence => shaders::turbulence(
                        (base_frequency_x, base_frequency_y),
                        usize::from(num_octaves),
                        seed,
                        tile_size,
                    ),
                    ResolvedFilterTurbulenceKind::FractalNoise => shaders::fractal_noise(
                        (base_frequency_x, base_frequency_y),
                        usize::from(num_octaves),
                        seed,
                        tile_size,
                    ),
                }
                .ok_or_else(|| "the backend could not construct a turbulence shader".to_string())?;
                let filter = skia_safe::image_filters::shader(shader, crop).ok_or_else(|| {
                    "the backend could not construct a turbulence operation".to_string()
                })?;
                (Some(filter), node.color_space, false, false)
            }
            ResolvedFilterPrimitive::DisplacementMap {
                scale,
                x_channel,
                y_channel,
            } => {
                let displacement = inputs
                    .pop()
                    .expect("displacement map has two checked inputs");
                let color = inputs
                    .pop()
                    .expect("displacement map has two checked inputs");
                let filter = skia_safe::image_filters::displacement_map(
                    (
                        sk_displacement_channel(x_channel),
                        sk_displacement_channel(y_channel),
                    ),
                    scale,
                    displacement.image_filter,
                    color.image_filter,
                    crop,
                )
                .ok_or_else(|| {
                    "the backend could not construct a displacement-map operation".to_string()
                })?;
                // The first hosted-x86 corpus run isolated the same final
                // N32 restore split in every admitted sRGB displacement
                // shape: 18 of the 22 failing rung cells (294 pixels total,
                // all one code value). Keep the floating sampling operation
                // native, then restore its sRGB result through the
                // architecture-neutral byte path.
                (
                    Some(filter),
                    node.color_space,
                    source_dependent,
                    requires_exact_restore || node.color_space == ResolvedFilterColorSpace::Srgb,
                )
            }
            ResolvedFilterPrimitive::ConvolveMatrix {
                order_x,
                order_y,
                kernel,
                gain,
                bias,
                target_x,
                target_y,
                edge_mode,
                preserve_alpha,
            } => {
                let input = inputs
                    .pop()
                    .expect("convolution matrix has one checked input");
                procedural_unorm8_blend = false;
                if node.color_space == ResolvedFilterColorSpace::Srgb
                    && !input.requires_exact_restore
                {
                    procedural_provenance = false;
                }
                let filter = skia_safe::image_filters::matrix_convolution(
                    (i32::from(order_x), i32::from(order_y)),
                    &kernel,
                    gain,
                    bias * 255.0,
                    (i32::from(target_x), i32::from(target_y)),
                    sk_convolve_tile_mode(edge_mode),
                    !preserve_alpha,
                    input.image_filter,
                    crop,
                )
                .ok_or_else(|| {
                    "the backend could not construct a convolution-matrix operation".to_string()
                })?;
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    input.requires_exact_restore
                        || node.color_space == ResolvedFilterColorSpace::Srgb,
                )
            }
            ResolvedFilterPrimitive::DiffuseLighting {
                surface_scale,
                diffuse_constant,
                color,
                light,
            } => {
                let input = inputs
                    .pop()
                    .expect("diffuse lighting has one checked input");
                procedural_provenance = false;
                procedural_unorm8_blend = false;
                let color = sk_lighting_color(color, node.color_space);
                let filter = match light {
                    ResolvedFilterLightSource::Distant { direction } => {
                        skia_safe::image_filters::distant_lit_diffuse(
                            sk_point3(direction),
                            color,
                            surface_scale,
                            diffuse_constant,
                            input.image_filter,
                            crop,
                        )
                    }
                    ResolvedFilterLightSource::Point { location } => {
                        skia_safe::image_filters::point_lit_diffuse(
                            sk_point3(location),
                            color,
                            surface_scale,
                            diffuse_constant,
                            input.image_filter,
                            crop,
                        )
                    }
                    ResolvedFilterLightSource::Spot {
                        location,
                        target,
                        falloff_exponent,
                        cutoff_angle,
                    } => skia_safe::image_filters::spot_lit_diffuse(
                        sk_point3(location),
                        sk_point3(target),
                        falloff_exponent,
                        cutoff_angle,
                        color,
                        surface_scale,
                        diffuse_constant,
                        input.image_filter,
                        crop,
                    ),
                }
                .ok_or_else(|| {
                    "the backend could not construct a diffuse-lighting operation".to_string()
                })?;
                (
                    Some(filter),
                    node.color_space,
                    input.source_dependent,
                    input.requires_exact_restore
                        || node.color_space == ResolvedFilterColorSpace::Srgb,
                )
            }
            ResolvedFilterPrimitive::Merge => {
                let mut inputs = inputs.into_iter();
                let image_filter = if let Some(first) = inputs.next() {
                    let mut merged = first.image_filter;
                    let blender = if inputs.len() > 0 {
                        Some(deterministic_porter_duff_blender(
                            ResolvedFilterComposite::Over,
                            source_dependent,
                        )?)
                    } else {
                        None
                    };
                    for foreground in inputs {
                        merged = skia_safe::image_filters::blend(
                            blender
                                .clone()
                                .expect("a non-empty merge tail compiled its blender"),
                            merged,
                            foreground.image_filter,
                            crop,
                        );
                        if merged.is_none() {
                            return Err(
                                "the backend could not construct a merge operation".to_string()
                            );
                        }
                    }
                    skia_safe::image_filters::crop(crop, Some(skia_safe::TileMode::Decal), merged)
                        .ok_or_else(|| "the backend could not crop a merge operation".to_string())?
                } else {
                    transparent_filter(node.region).ok_or_else(|| {
                        "the backend could not construct an empty merge result".to_string()
                    })?
                };
                (
                    Some(image_filter),
                    node.color_space,
                    source_dependent,
                    requires_exact_restore,
                )
            }
        };
        results.push(BuiltFilterResult {
            image_filter,
            color_space: output_space,
            source_dependent,
            requires_exact_restore,
            color_restore,
            source_preflatten,
            procedural_provenance,
            procedural_unorm8_blend,
        });
    }
    let output = results
        .pop()
        .expect("the resolved filter contract requires a non-empty program");
    let mut output = convert_filter_space(output, ResolvedFilterColorSpace::Srgb)?;
    if output.image_filter.is_none() {
        // The backend's `None` sentinel is a valid pass-through graph, but it
        // has no operation on which to carry the outer hard region.
        output.image_filter = Some(
            skia_safe::image_filters::crop(
                Rect::from_xywh(
                    filter.region.x,
                    filter.region.y,
                    filter.region.w,
                    filter.region.h,
                ),
                Some(skia_safe::TileMode::Decal),
                None,
            )
            .ok_or_else(|| {
                "the backend could not construct the hard filter-region crop".to_string()
            })?,
        );
    }
    let restore_blender = if output.procedural_provenance {
        Some(quantized_floating_porter_duff_blender(
            ResolvedFilterComposite::Over,
        )?)
    } else if output.requires_exact_restore {
        Some(exact_unorm8_blender(ResolvedFilterComposite::Over)?)
    } else if output.color_restore == Some(ColorRestore::Floating) {
        Some(floating_porter_duff_blender(ResolvedFilterComposite::Over)?)
    } else if output.color_restore == Some(ColorRestore::Default) || output.source_dependent {
        None
    } else {
        Some(exact_unorm8_blender(ResolvedFilterComposite::Over)?)
    };
    Ok(BuiltFilter {
        image_filter: output.image_filter,
        restore_blender,
        source_preflatten: output.source_preflatten,
    })
}

#[cfg(test)]
mod filter_policy_tests {
    use std::sync::Arc;

    use n0_model::math::RectF;
    use n0_model::model::{Color as ModelColor, Color32F};

    use crate::drawlist::{
        ResolvedFilterConvolveEdgeMode, ResolvedFilterLightSource, ResolvedFilterMorphology,
        ResolvedFilterNode,
    };

    use super::{
        build_filter, ResolvedFilter, ResolvedFilterColorSpace, ResolvedFilterInput,
        ResolvedFilterPrimitive,
    };

    const REGION: RectF = RectF {
        x: 0.0,
        y: 0.0,
        w: 16.0,
        h: 16.0,
    };

    fn identity(alpha_offset: f32) -> [f32; 20] {
        let mut matrix = [0.0; 20];
        matrix[0] = 1.0;
        matrix[6] = 1.0;
        matrix[12] = 1.0;
        matrix[18] = 1.0;
        matrix[19] = alpha_offset;
        matrix
    }

    fn identity_tables(alpha_zero: u8) -> Arc<[[u8; 256]; 4]> {
        let identity = std::array::from_fn(|index| index as u8);
        let mut alpha = identity;
        alpha[0] = alpha_zero;
        Arc::new([identity, identity, identity, alpha])
    }

    fn source_matrix(alpha_offset: f32, color_space: ResolvedFilterColorSpace) -> ResolvedFilter {
        ResolvedFilter {
            region: REGION,
            nodes: Arc::from([ResolvedFilterNode {
                inputs: Arc::from([ResolvedFilterInput::Source]),
                region: REGION,
                color_space,
                primitive: ResolvedFilterPrimitive::ColorMatrix {
                    matrix: identity(alpha_offset),
                },
            }]),
            may_paint_transparent_input: alpha_offset > 0.0,
            source_is_transparent: false,
        }
    }

    #[test]
    fn source_generated_and_alpha_creating_matrices_keep_distinct_layer_policies() {
        let source = build_filter(&source_matrix(0.0, ResolvedFilterColorSpace::Srgb))
            .expect("source matrix builds");
        assert!(source.source_preflatten);
        assert!(
            source.restore_blender.is_some(),
            "source-derived sRGB output uses the measured floating restore"
        );

        let alpha_creating = build_filter(&source_matrix(0.25, ResolvedFilterColorSpace::Srgb))
            .expect("alpha-creating matrix builds");
        assert!(!alpha_creating.source_preflatten);
        assert!(alpha_creating.restore_blender.is_none());

        let generated = ResolvedFilter {
            region: REGION,
            nodes: Arc::from([
                ResolvedFilterNode {
                    inputs: Arc::from([]),
                    region: REGION,
                    color_space: ResolvedFilterColorSpace::Srgb,
                    primitive: ResolvedFilterPrimitive::SolidColor {
                        color: Color32F::new(0.2, 0.4, 0.8, 0.5).expect("unit color"),
                    },
                },
                ResolvedFilterNode {
                    inputs: Arc::from([ResolvedFilterInput::Node(0)]),
                    region: REGION,
                    color_space: ResolvedFilterColorSpace::Srgb,
                    primitive: ResolvedFilterPrimitive::ColorMatrix {
                        matrix: identity(0.0),
                    },
                },
            ]),
            may_paint_transparent_input: true,
            source_is_transparent: false,
        };
        let generated = build_filter(&generated).expect("generated matrix builds");
        assert!(!generated.source_preflatten);
        assert!(
            generated.restore_blender.is_none(),
            "generated matrix output keeps the backend-default measured restore"
        );

        let converted = build_filter(&source_matrix(0.0, ResolvedFilterColorSpace::LinearRgb))
            .expect("linear matrix builds");
        assert!(converted.source_preflatten);
        assert!(
            converted.restore_blender.is_none(),
            "the output gamma conversion ends the matrix-specific sRGB restore"
        );
    }

    #[test]
    fn source_generated_and_alpha_creating_tables_keep_measured_layer_policies() {
        let transfer = |input, tables| ResolvedFilterNode {
            inputs: Arc::from([input]),
            region: REGION,
            color_space: ResolvedFilterColorSpace::Srgb,
            primitive: ResolvedFilterPrimitive::ComponentTransfer { tables },
        };
        let source = ResolvedFilter {
            region: REGION,
            nodes: Arc::from([transfer(ResolvedFilterInput::Source, identity_tables(0))]),
            may_paint_transparent_input: false,
            source_is_transparent: false,
        };
        let source = build_filter(&source).expect("source table builds");
        assert!(source.source_preflatten);
        assert!(source.restore_blender.is_some());

        let alpha_creating = ResolvedFilter {
            region: REGION,
            nodes: Arc::from([transfer(ResolvedFilterInput::Source, identity_tables(127))]),
            may_paint_transparent_input: true,
            source_is_transparent: false,
        };
        let alpha_creating = build_filter(&alpha_creating).expect("alpha table builds");
        assert!(alpha_creating.source_preflatten);
        assert!(alpha_creating.restore_blender.is_none());

        let generated = ResolvedFilter {
            region: REGION,
            nodes: Arc::from([
                ResolvedFilterNode {
                    inputs: Arc::from([]),
                    region: REGION,
                    color_space: ResolvedFilterColorSpace::Srgb,
                    primitive: ResolvedFilterPrimitive::SolidColor {
                        color: Color32F::new(0.2, 0.4, 0.8, 0.5).expect("unit color"),
                    },
                },
                transfer(ResolvedFilterInput::Node(0), identity_tables(0)),
            ]),
            may_paint_transparent_input: true,
            source_is_transparent: false,
        };
        let generated = build_filter(&generated).expect("generated table builds");
        assert!(!generated.source_preflatten);
        assert!(generated.restore_blender.is_none());
    }

    #[test]
    fn only_active_source_dependent_morphology_preflattens_its_source() {
        let morphology = |input, radius, color_space| ResolvedFilterNode {
            inputs: Arc::from([input]),
            region: REGION,
            color_space,
            primitive: ResolvedFilterPrimitive::Morphology {
                operator: ResolvedFilterMorphology::Dilate,
                radius_x: radius,
                radius_y: radius,
            },
        };
        let one = |node| ResolvedFilter {
            region: REGION,
            nodes: Arc::from([node]),
            may_paint_transparent_input: false,
            source_is_transparent: false,
        };

        let active = build_filter(&one(morphology(
            ResolvedFilterInput::Source,
            2.0,
            ResolvedFilterColorSpace::Srgb,
        )))
        .expect("active source morphology builds");
        assert!(active.source_preflatten);
        assert!(
            active.restore_blender.is_some(),
            "active sRGB morphology uses an architecture-neutral layer restore"
        );

        let zero = build_filter(&one(morphology(
            ResolvedFilterInput::Source,
            0.0,
            ResolvedFilterColorSpace::Srgb,
        )))
        .expect("zero source morphology builds");
        assert!(!zero.source_preflatten);
        assert!(zero.restore_blender.is_none());

        let linear = build_filter(&one(morphology(
            ResolvedFilterInput::Source,
            2.0,
            ResolvedFilterColorSpace::LinearRgb,
        )))
        .expect("linear source morphology builds");
        assert!(linear.source_preflatten);
        assert!(
            linear.restore_blender.is_none(),
            "the output gamma conversion ends the sRGB restore boundary"
        );

        let generated = ResolvedFilter {
            region: REGION,
            nodes: Arc::from([
                ResolvedFilterNode {
                    inputs: Arc::from([]),
                    region: REGION,
                    color_space: ResolvedFilterColorSpace::Srgb,
                    primitive: ResolvedFilterPrimitive::SolidColor {
                        color: Color32F::new(0.2, 0.4, 0.8, 0.5).expect("unit color"),
                    },
                },
                morphology(
                    ResolvedFilterInput::Node(0),
                    2.0,
                    ResolvedFilterColorSpace::Srgb,
                ),
            ]),
            may_paint_transparent_input: true,
            source_is_transparent: false,
        };
        let generated = build_filter(&generated).expect("generated morphology builds");
        assert!(!generated.source_preflatten);
    }

    #[test]
    fn checked_convolution_builds_at_the_kernel_bound_and_keeps_source_policy() {
        let convolve = |input, count, bias, preserve_alpha| ResolvedFilterNode {
            inputs: Arc::from([input]),
            region: REGION,
            color_space: ResolvedFilterColorSpace::Srgb,
            primitive: ResolvedFilterPrimitive::ConvolveMatrix {
                order_x: count,
                order_y: 1,
                kernel: vec![0.0; usize::from(count)].into(),
                gain: 1.0,
                bias,
                target_x: count / 2,
                target_y: 0,
                edge_mode: ResolvedFilterConvolveEdgeMode::Wrap,
                preserve_alpha,
            },
        };
        let one = |node, may_paint_transparent_input| ResolvedFilter {
            region: REGION,
            nodes: Arc::from([node]),
            may_paint_transparent_input,
            source_is_transparent: false,
        };

        let bounded = build_filter(&one(
            convolve(ResolvedFilterInput::Source, 256, 0.0, false),
            false,
        ))
        .expect("the checked maximum kernel builds transactionally");
        assert!(bounded.source_preflatten);
        assert!(bounded.restore_blender.is_some());

        let alpha_creating = build_filter(&one(
            convolve(ResolvedFilterInput::Source, 1, 0.25, false),
            true,
        ))
        .expect("a biased convolution builds over an explicit source");
        assert!(alpha_creating.source_preflatten);

        let alpha_preserving = build_filter(&one(
            convolve(ResolvedFilterInput::SourceAlpha, 1, 0.25, true),
            false,
        ))
        .expect("preserved alpha remains a checked native operation");
        assert!(alpha_preserving.source_preflatten);
    }

    #[test]
    fn every_checked_diffuse_light_kind_builds_with_its_color_space_policy() {
        let lights = [
            ResolvedFilterLightSource::Distant {
                direction: [0.5, -0.5, std::f32::consts::FRAC_1_SQRT_2],
            },
            ResolvedFilterLightSource::Point {
                location: [4.0, 8.0, 12.0],
            },
            ResolvedFilterLightSource::Spot {
                location: [2.0, 3.0, 8.0],
                target: [7.0, 6.0, 0.0],
                falloff_exponent: 8.0,
                cutoff_angle: 35.0,
            },
        ];
        for color_space in [
            ResolvedFilterColorSpace::Srgb,
            ResolvedFilterColorSpace::LinearRgb,
        ] {
            for light in lights {
                let filter = ResolvedFilter {
                    region: REGION,
                    nodes: Arc::from([ResolvedFilterNode {
                        inputs: Arc::from([ResolvedFilterInput::SourceAlpha]),
                        region: REGION,
                        color_space,
                        primitive: ResolvedFilterPrimitive::DiffuseLighting {
                            surface_scale: -2.0,
                            diffuse_constant: 0.75,
                            color: ModelColor(0xffff_b347),
                            light,
                        },
                    }]),
                    may_paint_transparent_input: true,
                    source_is_transparent: false,
                };
                let built = build_filter(&filter).expect("checked native diffuse light builds");
                assert!(!built.source_preflatten);
                assert_eq!(
                    built.restore_blender.is_some(),
                    color_space == ResolvedFilterColorSpace::Srgb,
                    "sRGB lighting uses the architecture-neutral outer restore; gamma conversion ends it"
                );
            }
        }
    }
}

/// Product-build preflight for a resolved image-filter graph. Replay repeats
/// the same pure builders and may therefore treat success as proven.
pub(crate) fn preflight_filter(filter: &ResolvedFilter) -> Result<(), String> {
    build_filter(filter).map(|_| ())
}

/// Push a command run into a Skia builder verbatim — the one place the
/// backend-independent vocabulary becomes Skia calls.
fn emit_commands(builder: &mut PathBuilder, commands: &[PathCommand]) {
    for command in commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                builder.move_to((x, y));
            }
            PathCommand::LineTo { x, y } => {
                builder.line_to((x, y));
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                builder.quad_to((x1, y1), (x, y));
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                builder.cubic_to((x1, y1), (x2, y2), (x, y));
            }
            PathCommand::ConicTo {
                x1,
                y1,
                x,
                y,
                weight,
            } => {
                builder.conic_to((x1, y1), (x, y), weight);
            }
            PathCommand::Close => {
                builder.close();
            }
        }
    }
}

#[cfg(test)]
mod backend_path_tests {
    use super::{any_contour_may_be_degenerate, backend_path};
    use n0_model::path::{analyze, materialize, FillRule};

    /// The predicate that decides whether a closed contour's cap may be
    /// normalised away. It must say *yes, possibly degenerate* for anything
    /// that closes on the point it opened at — that is the case SVG2 §13.2
    /// renders as a dot from the cap alone — and *no* for a contour with real
    /// extent, or the normalisation never happens and the defect stays.
    ///
    /// The move-only spelling of the dot (`M44 32 Z`) is not here: this door
    /// (`analyze`) is the Grida-format one and requires a drawing segment,
    /// while SVG's door admits it. That spelling is gated end-to-end by
    /// `fixtures/web-first/svg-stroke-zero-length-dot.svg`, which is what
    /// caught the normalisation erasing it.
    #[test]
    fn a_contour_that_closes_where_it_opened_reads_as_degenerate() {
        let degenerate = [
            "M.2 .5 L.2 .5",                   // open, zero length
            "M.1 .1 L.1 .1 Z",                 // closed through a zero-length line
            "M.1 .1 C.1 .1 .1 .1 .1 .1 Z",     // closed through a collapsed cubic
            "M.1 .1 L.9 .9 Z M.2 .2 L.2 .2 Z", // one contour with extent, one dot
        ];
        for d in degenerate {
            assert!(possibly_degenerate(d), "{d} must read as degenerate");
        }

        let extended = [
            // the shape of the corpus's closed-contour cap cells
            "M.5 .1 C.9 .1 1 .4 1 .6 C1 .85 .75 1 .5 1 C.25 1 0 .85 0 .6 C0 .4 .1 .1 .5 .1 Z",
            "M.1 .1 L.9 .1 L.9 .9 Z",
            "M0 0 L.5 0 Z M.6 .6 L.9 .6 Z", // two closed contours, both with extent
            "M0 0 L.9 .9",                  // open, with extent
        ];
        for d in extended {
            assert!(!possibly_degenerate(d), "{d} must read as extended");
        }
    }

    fn possibly_degenerate(d: &str) -> bool {
        let artifact = analyze(d, FillRule::NonZero).expect("probe path is valid");
        let resolved = materialize(artifact.geometry(), artifact.fill_rule(), 64.0, 64.0)
            .expect("probe path resolves");
        any_contour_may_be_degenerate(&resolved)
    }

    #[test]
    fn analytical_arc_bounds_match_the_materialized_conics() {
        let cases = [
            "M .5 0 A .5 .5 0 0 1 1 .5 A .5 .5 0 0 1 .5 1 A .5 .5 0 0 1 0 .5 A .5 .5 0 0 1 .5 0 Z",
            "M .2 .5 A .35 .2 37 0 1 .8 .5",
            "M .2 .2 A .4 .3 25 0 1 .8 .7",
            "M .5 .5 A .000001 .000001 0 0 1 .500001 .500001",
        ];
        for d in cases {
            let artifact = analyze(d, FillRule::NonZero)
                .unwrap_or_else(|error| panic!("arc corpus must be valid: {d}: {error}"));
            let (width, height) = (137.0, 83.0);
            let resolved = materialize(artifact.geometry(), artifact.fill_rule(), width, height)
                .expect("arc corpus must fit its finite resolved box");
            let actual = backend_path(&resolved).compute_tight_bounds();
            let expected = resolved.local_bounds;
            let epsilon = 2.0e-4;
            assert!(actual.left >= expected.x - epsilon, "{d}: left escaped");
            assert!(actual.top >= expected.y - epsilon, "{d}: top escaped");
            assert!(
                actual.right <= expected.x + expected.w + epsilon,
                "{d}: right escaped"
            );
            assert!(
                actual.bottom <= expected.y + expected.h + epsilon,
                "{d}: bottom escaped"
            );
        }
    }
}

fn draw_stroke(
    canvas: &Canvas,
    source: &Path,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
) {
    let geometry = stroke_geometry(source, stroke, dash_phase);
    draw_painted_geometry(
        canvas,
        &geometry,
        &stroke.paints,
        post_paint_opacity,
        paint_box,
        ctx,
    );
}

fn draw_painted_geometry(
    canvas: &Canvas,
    geometry: &Path,
    paints: &Paints,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
) {
    if geometry.is_empty() {
        return;
    }
    for model in paints.iter() {
        if let Some(paint) = sk_paint(model, paint_box, ctx, post_paint_opacity) {
            canvas.draw_path(geometry, &paint);
        }
    }
}

fn draw_rectangular_stroke(
    canvas: &Canvas,
    w: f32,
    h: f32,
    radius: &RectangularCornerRadius,
    widths: RectangularStrokeWidth,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
) {
    let geometry = rectangular_stroke_geometry(w, h, radius, widths, stroke, dash_phase);
    draw_painted_geometry(
        canvas,
        &geometry,
        &stroke.paints,
        post_paint_opacity,
        paint_box,
        ctx,
    );
}

/// Use Skia's native stroke rasterization for centered strokes. Converting a
/// centered stroke into filled outline geometry is semantically unnecessary
/// and changes edge coverage relative to the native primitive operations.
fn native_stroke_paint(
    model: &ModelPaint,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
) -> Option<Paint> {
    native_stroke_paint_mapped(
        model,
        stroke,
        dash_phase,
        post_paint_opacity,
        paint_box,
        ctx,
        &Affine::IDENTITY,
    )
}

fn native_stroke_paint_mapped(
    model: &ModelPaint,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
) -> Option<Paint> {
    let width = uniform_stroke_width(stroke)?;
    let mut paint = sk_paint_mapped(model, paint_box, ctx, post_paint_opacity, paint_to_canvas)?;
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width);
    paint.set_stroke_cap(sk_stroke_cap(stroke.cap));
    paint.set_stroke_join(sk_stroke_join(stroke.join));
    paint.set_stroke_miter(stroke.miter_limit);
    if let Some(values) = stroke.dash_array.as_deref() {
        if !values.is_empty() {
            let intervals = normalized_dash_array(values)?;
            paint.set_path_effect(PathEffect::dash(&intervals, dash_phase.value())?);
        }
    }
    Some(paint)
}

fn draw_native_centered_stroke(
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    mut draw: impl FnMut(&Paint),
) {
    for model in stroke.paints.iter() {
        if let Some(paint) = native_stroke_paint(
            model,
            stroke,
            dash_phase,
            post_paint_opacity,
            paint_box,
            ctx,
        ) {
            draw(&paint);
        }
    }
}

fn draw_native_centered_stroke_mapped(
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
    paint_to_canvas: &Affine,
    mut draw: impl FnMut(&Paint),
) {
    for model in stroke.paints.iter() {
        if let Some(paint) = native_stroke_paint_mapped(
            model,
            stroke,
            dash_phase,
            post_paint_opacity,
            paint_box,
            ctx,
            paint_to_canvas,
        ) {
            draw(&paint);
        }
    }
}

/// Draw one centerline already expressed in item-local coordinates. A
/// frame-space stroke maps that centerline before dashing/widening while its
/// paint mapping follows the same local-to-frame transform. The host view is
/// deliberately left outside this operation.
#[allow(clippy::too_many_arguments)]
fn draw_frame_space_stroke(
    canvas: &Canvas,
    view: &Affine,
    world: &Affine,
    source: &Path,
    stroke: &Stroke,
    dash_phase: StrokeDashPhase,
    post_paint_opacity: PostPaintOpacity,
    paint_box: PaintBox,
    ctx: &PaintCtx,
) {
    let transformed = source.make_transform(&skia_matrix(world));
    with_local_transform(canvas, view, &Affine::IDENTITY, || {
        draw_native_centered_stroke_mapped(
            stroke,
            dash_phase,
            post_paint_opacity,
            paint_box,
            ctx,
            world,
            |paint| {
                canvas.draw_path(&transformed, paint);
            },
        );
    });
}

#[derive(Default)]
struct GlyphScratch {
    ids: Vec<u16>,
    positions: Vec<Point>,
}

impl GlyphScratch {
    fn with_run<K>(
        &mut self,
        run: &n0_model::text_layout::TextGlyphRun,
        list: &DrawList<K>,
        mut use_run: impl FnMut(&Font, &[u16], &[Point]),
    ) {
        self.ids.clear();
        self.positions.clear();
        self.ids.extend(run.glyphs.iter().map(|glyph| glyph.id));
        self.positions
            .extend(run.glyphs.iter().map(|glyph| Point::new(glyph.x, glyph.y)));
        let font = list.text_fonts().font(run.font);
        use_run(&font, &self.ids, &self.positions);
    }
}

fn text_path<K>(
    layout: &n0_model::text_layout::TextLayout,
    list: &DrawList<K>,
    scratch: &mut GlyphScratch,
) -> Path {
    let mut builder = PathBuilder::new();
    for run in &layout.glyph_runs {
        scratch.with_run(run, list, |font, glyphs, positions| {
            for (glyph, position) in glyphs.iter().zip(positions) {
                if let Some(path) = font.get_path(*glyph) {
                    let path = path.make_transform(&Matrix::translate((position.x, position.y)));
                    builder.add_path(&path, None);
                }
            }
        });
    }
    builder.snapshot()
}

/// Replay a raw [`DrawList`] without a frame-environment check.
///
/// This low-level entry exists for engine-owned resource-free glyphless
/// products, structural probes, and internal retained-list replay. A host
/// rendering an ordinary semantic frame must call
/// [`crate::frame::FrameProduct::execute`], which refuses a context whose
/// incarnation or resource revision differs from the one captured at build.
pub fn execute_unchecked<K>(canvas: &Canvas, list: &DrawList<K>, view: &Affine, ctx: &PaintCtx) {
    // Install Skia's runtime-selected raster pipeline before any drawlist
    // replay. Without this initialization, x86 stays on the baseline SSE
    // implementation while ARM enters the default NEON implementation. That
    // changes the fused arithmetic used by procedural shaders such as Perlin
    // noise and can move a boundary value across N32 quantization.
    skia_safe::graphics::init();

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Scope {
        Opacity,
        Clip,
        MaskContent,
        MaskSource,
        Filter { source_preflatten: bool },
    }

    let initial_save_count = canvas.save_count();
    let mut scopes = Vec::new();
    let mut glyph_scratch = GlyphScratch::default();
    for item in &list.items {
        match &item.kind {
            ItemKind::BeginOpacity { opacity } => {
                // Copy the current backdrop into the group layer so descendant
                // paint blend modes see the same accumulated result they would
                // see without node opacity. On restore, arithmetic blending
                // computes `opacity * group + (1-opacity) * backdrop` directly
                // in premultiplied space. Plain SrcOver alpha would double a
                // translucent backdrop copied into the source layer.
                let opacity = opacity.clamp(0.0, 1.0);
                let mut restore_paint = Paint::default();
                restore_paint.set_blender(
                    Blender::arithmetic(0.0, opacity, 1.0 - opacity, 0.0, true)
                        .expect("finite opacity produces an arithmetic blender"),
                );
                let layer = SaveLayerRec::default()
                    .paint(&restore_paint)
                    .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                canvas.save_layer(&layer);
                scopes.push(Scope::Opacity);
            }
            ItemKind::BeginIsolatedOpacity { opacity } => {
                // The Web's isolated group: the layer starts empty (no
                // backdrop copy), contents blend among themselves, and the
                // restore composites the layer source-over modulated by the
                // opacity — the plain Skia layer Chromium itself restores
                // through, which is what makes the quantization match the
                // oracle byte-for-byte.
                let mut restore_paint = Paint::default();
                restore_paint.set_alpha_f(opacity.clamp(0.0, 1.0));
                let layer = SaveLayerRec::default().paint(&restore_paint);
                canvas.save_layer(&layer);
                scopes.push(Scope::Opacity);
            }
            ItemKind::EndOpacity => {
                let scope = scopes.pop();
                debug_assert_eq!(scope, Some(Scope::Opacity));
                if scope.is_some() {
                    canvas.restore();
                }
            }
            ItemKind::BeginClipRect {
                w,
                h,
                corner_radius,
                corner_smoothing,
            } => {
                let total = view.then(&item.world);
                canvas.save();
                canvas.set_matrix(&skia_matrix(&total).into());
                if corner_radius.is_zero() {
                    canvas.clip_rect(Rect::from_wh(*w, *h), None, false);
                } else {
                    let path = rounded_rect_path(*w, *h, corner_radius, corner_smoothing.value());
                    canvas.clip_path(&path, ClipOp::Intersect, true);
                }
                scopes.push(Scope::Clip);
            }
            ItemKind::BeginClipPath { clip } => {
                let total = view.then(&item.world);
                let path = backend_clip_path(clip)
                    .expect("geometric clip path operations were preflighted at product build");
                canvas.save();
                canvas.set_matrix(&skia_matrix(&total).into());
                canvas.clip_path(&path, ClipOp::Intersect, clip.anti_alias);
                scopes.push(Scope::Clip);
            }
            ItemKind::EndClip => {
                let scope = scopes.pop();
                debug_assert_eq!(scope, Some(Scope::Clip));
                if scope.is_some() {
                    canvas.restore();
                }
            }
            ItemKind::BeginMaskContent => {
                canvas.save_layer(&SaveLayerRec::default());
                scopes.push(Scope::MaskContent);
            }
            ItemKind::BeginMaskSource { mode, region } => {
                let mut restore_paint = Paint::default();
                restore_paint.set_blend_mode(skia_safe::BlendMode::DstIn);
                if *mode == ResolvedMaskMode::Luminance {
                    restore_paint.set_color_filter(skia_safe::ColorFilter::luma());
                }
                let layer = SaveLayerRec::default().paint(&restore_paint);
                canvas.save_layer(&layer);
                let total = view.then(&item.world);
                canvas.set_matrix(&skia_matrix(&total).into());
                let path = backend_clip_path(region)
                    .expect("geometric mask region was preflighted at product build");
                // Chromium realizes the mask region as the backing image's
                // hard bounds. It does not contribute another coverage ramp:
                // a fractional default region cuts a stroked target at pixel
                // centers, while the mask source's own geometry remains
                // antialiased inside those bounds.
                canvas.clip_path(&path, ClipOp::Intersect, false);
                scopes.push(Scope::MaskSource);
            }
            ItemKind::EndMaskSource => {
                let scope = scopes.pop();
                debug_assert_eq!(scope, Some(Scope::MaskSource));
                if scope.is_some() {
                    canvas.restore();
                }
            }
            ItemKind::EndMaskContent => {
                let scope = scopes.pop();
                debug_assert_eq!(scope, Some(Scope::MaskContent));
                if scope.is_some() {
                    canvas.restore();
                }
            }
            ItemKind::BeginFilter { filter } => {
                let total = view.then(&item.world);
                canvas.save();
                canvas.set_matrix(&skia_matrix(&total).into());
                let built_filter = build_filter(filter)
                    .expect("resolved image-filter builders were preflighted at product build");
                let mut restore_paint = Paint::default();
                restore_paint.set_image_filter(built_filter.image_filter);
                if let Some(blender) = built_filter.restore_blender {
                    restore_paint.set_blender(blender);
                }
                // The filter region is a hard output crop, not an input-image
                // crop. Every checked graph output already carries its
                // intersected primitive/filter crop. Bounding this source
                // layer to `region` would discard pixels a spatial kernel
                // must sample just outside that output region before it crops
                // the result (most visibly, morphology dilation).
                let layer = SaveLayerRec::default().paint(&restore_paint);
                canvas.save_layer(&layer);
                if built_filter.source_preflatten {
                    canvas.save_layer(&SaveLayerRec::default());
                }
                if filter.source_is_transparent && filter.may_paint_transparent_input {
                    // A source-independent primitive still needs a material
                    // source layer for Skia to evaluate its restore filter.
                    // Seed that lazy layer with an opaque Src draw; the graph
                    // cannot observe it because every Source input was replaced
                    // above by an explicit transparent filter.
                    let mut seed = Paint::default();
                    seed.set_blend_mode(skia_safe::BlendMode::Src);
                    let region = Rect::from_xywh(
                        filter.region.x,
                        filter.region.y,
                        filter.region.w,
                        filter.region.h,
                    );
                    // This raster only forces the lazy-layer restore to run and
                    // cannot leak into graph output.
                    seed.set_color(Color::BLACK);
                    canvas.draw_rect(region, &seed);
                }
                scopes.push(Scope::Filter {
                    source_preflatten: built_filter.source_preflatten,
                });
            }
            ItemKind::EndFilter => {
                let scope = scopes.pop();
                debug_assert!(matches!(scope, Some(Scope::Filter { .. })));
                if let Some(Scope::Filter { source_preflatten }) = scope {
                    if source_preflatten {
                        canvas.restore();
                    }
                    // Restore the filtered layer, then the local-space hard
                    // region and transform saved immediately outside it.
                    canvas.restore();
                    canvas.restore();
                }
            }
            ItemKind::PatternFill {
                geometry,
                pattern,
                post_paint_opacity,
            } => {
                with_local_transform(canvas, view, &item.world, || {
                    let paint = pattern_paint(pattern, *post_paint_opacity, ctx)
                        .expect("preflighted pattern shader construction failed");
                    match geometry {
                        ResolvedPatternGeometry::Rect { x, y, w, h } => {
                            canvas.draw_rect(Rect::from_xywh(*x, *y, *w, *h), &paint);
                        }
                        ResolvedPatternGeometry::Oval { x, y, w, h } => {
                            canvas.draw_oval(Rect::from_xywh(*x, *y, *w, *h), &paint);
                        }
                        ResolvedPatternGeometry::Path(path) => {
                            canvas.draw_path(&backend_path(path), &paint);
                        }
                    }
                });
            }
            ItemKind::PatternStroke {
                geometry,
                pattern,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                debug_assert_eq!(stroke.align, StrokeAlign::Center);
                let adjusted = match geometry {
                    ResolvedPatternGeometry::Oval { w, h, .. } if *w > 0.0 && *h > 0.0 => {
                        stroke_cap_for_closed_contours(stroke)
                    }
                    ResolvedPatternGeometry::Path(path)
                        if path.all_contours_closed && !any_contour_may_be_degenerate(path) =>
                    {
                        stroke_cap_for_closed_contours(stroke)
                    }
                    _ => stroke.clone(),
                };
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            let paint = pattern_stroke_paint(
                                pattern,
                                &adjusted,
                                *dash_phase,
                                *post_paint_opacity,
                                ctx,
                            )
                            .expect("preflighted pattern stroke shader construction failed");
                            match geometry {
                                ResolvedPatternGeometry::Rect { x, y, w, h } => {
                                    canvas.draw_rect(Rect::from_xywh(*x, *y, *w, *h), &paint);
                                }
                                ResolvedPatternGeometry::Oval { x, y, w, h } => {
                                    canvas.draw_oval(Rect::from_xywh(*x, *y, *w, *h), &paint);
                                }
                                ResolvedPatternGeometry::Path(path) => {
                                    canvas.draw_path(&backend_path(path), &paint);
                                }
                            }
                        });
                    }
                    StrokeSpace::Frame => {
                        let source = match geometry {
                            ResolvedPatternGeometry::Rect { x, y, w, h } => {
                                let mut builder = PathBuilder::new();
                                builder.add_rect(
                                    Rect::from_xywh(*x, *y, *w, *h),
                                    Some(PathDirection::CW),
                                    Some(0),
                                );
                                builder.snapshot()
                            }
                            ResolvedPatternGeometry::Oval { x, y, w, h } => {
                                let mut builder = PathBuilder::new();
                                builder.add_oval(
                                    Rect::from_xywh(*x, *y, *w, *h),
                                    Some(PathDirection::CW),
                                    Some(1),
                                );
                                builder.snapshot()
                            }
                            ResolvedPatternGeometry::Path(path) => backend_path(path),
                        };
                        let transformed = source.make_transform(&skia_matrix(&item.world));
                        with_local_transform(canvas, view, &Affine::IDENTITY, || {
                            let paint = pattern_stroke_paint_mapped(
                                pattern,
                                &adjusted,
                                *dash_phase,
                                *post_paint_opacity,
                                ctx,
                                &item.world,
                            )
                            .expect("preflighted pattern stroke shader construction failed");
                            canvas.draw_path(&transformed, &paint);
                        });
                    }
                }
            }
            ItemKind::RectFill {
                w,
                h,
                corner_radius,
                corner_smoothing,
                paints,
                post_paint_opacity,
            } => {
                with_local_transform(canvas, view, &item.world, || {
                    let paint_box = PaintBox::from_size(*w, *h);
                    if corner_radius.is_zero() {
                        for model in paints.iter() {
                            if let Some(paint) =
                                sk_paint(model, paint_box, ctx, *post_paint_opacity)
                            {
                                canvas.draw_rect(Rect::from_wh(*w, *h), &paint);
                            }
                        }
                    } else {
                        let path =
                            rounded_rect_path(*w, *h, corner_radius, corner_smoothing.value());
                        for model in paints.iter() {
                            if let Some(paint) =
                                sk_paint(model, paint_box, ctx, *post_paint_opacity)
                            {
                                canvas.draw_path(&path, &paint);
                            }
                        }
                    }
                });
            }
            ItemKind::OvalFill {
                w,
                h,
                paints,
                post_paint_opacity,
            } => {
                with_local_transform(canvas, view, &item.world, || {
                    let paint_box = PaintBox::from_size(*w, *h);
                    for model in paints.iter() {
                        if let Some(paint) = sk_paint(model, paint_box, ctx, *post_paint_opacity) {
                            canvas.draw_oval(Rect::from_wh(*w, *h), &paint);
                        }
                    }
                });
            }
            ItemKind::PathFill {
                w,
                h,
                path,
                paints,
                post_paint_opacity,
            } => {
                with_local_transform(canvas, view, &item.world, || {
                    let paint_box = PaintBox::from_size(*w, *h);
                    let geometry = backend_path(path);
                    draw_painted_geometry(
                        canvas,
                        &geometry,
                        paints,
                        *post_paint_opacity,
                        paint_box,
                        ctx,
                    );
                });
            }
            ItemKind::TextFill {
                layout,
                paints,
                paint_w,
                paint_h,
                post_paint_opacity,
            } => {
                with_local_transform(canvas, view, &item.world, || {
                    let paint_box = PaintBox::from_size(*paint_w, *paint_h);
                    for run in &layout.glyph_runs {
                        glyph_scratch.with_run(run, list, |font, glyphs, positions| {
                            if let Some(run_paints) = paints.for_source_run(run.source_run) {
                                for model in run_paints.iter() {
                                    if let Some(paint) =
                                        sk_paint(model, paint_box, ctx, *post_paint_opacity)
                                    {
                                        canvas.draw_glyphs_at(
                                            glyphs,
                                            positions,
                                            Point::new(0.0, 0.0),
                                            font,
                                            &paint,
                                        );
                                    }
                                }
                            }
                        });
                    }
                });
            }
            ItemKind::RectStroke {
                w,
                h,
                corner_radius,
                corner_smoothing,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                let paint_box = PaintBox::from_size(*w, *h);
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            match stroke.width.normalized() {
                                StrokeWidth::None => {}
                                StrokeWidth::Rectangular(widths) => draw_rectangular_stroke(
                                    canvas,
                                    *w,
                                    *h,
                                    corner_radius,
                                    widths,
                                    stroke,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                ),
                                StrokeWidth::Uniform(_) => {
                                    if corner_radius.is_zero()
                                        && stroke.align == StrokeAlign::Center
                                    {
                                        draw_native_centered_stroke(
                                            stroke,
                                            *dash_phase,
                                            *post_paint_opacity,
                                            paint_box,
                                            ctx,
                                            |paint| {
                                                canvas.draw_rect(Rect::from_wh(*w, *h), paint);
                                            },
                                        );
                                    } else {
                                        let path = rounded_rect_path(
                                            *w,
                                            *h,
                                            corner_radius,
                                            corner_smoothing.value(),
                                        );
                                        if stroke.align == StrokeAlign::Center {
                                            draw_native_centered_stroke(
                                                stroke,
                                                *dash_phase,
                                                *post_paint_opacity,
                                                paint_box,
                                                ctx,
                                                |paint| {
                                                    canvas.draw_path(&path, paint);
                                                },
                                            );
                                        } else {
                                            draw_stroke(
                                                canvas,
                                                &path,
                                                stroke,
                                                *dash_phase,
                                                *post_paint_opacity,
                                                paint_box,
                                                ctx,
                                            );
                                        }
                                    }
                                }
                            }
                        });
                    }
                    StrokeSpace::Frame => {
                        debug_assert_eq!(stroke.align, StrokeAlign::Center);
                        debug_assert!(matches!(stroke.width.normalized(), StrokeWidth::Uniform(_)));
                        let source = if corner_radius.is_zero() {
                            let mut builder = PathBuilder::new();
                            builder.add_rect(
                                Rect::from_wh(*w, *h),
                                Some(PathDirection::CW),
                                Some(0),
                            );
                            builder.snapshot()
                        } else {
                            rounded_rect_path(*w, *h, corner_radius, corner_smoothing.value())
                        };
                        draw_frame_space_stroke(
                            canvas,
                            view,
                            &item.world,
                            &source,
                            stroke,
                            *dash_phase,
                            *post_paint_opacity,
                            paint_box,
                            ctx,
                        );
                    }
                }
            }
            ItemKind::OvalStroke {
                w,
                h,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                let paint_box = PaintBox::from_size(*w, *h);
                // A solid oval is one closed contour, so its cap is inert;
                // a dashed oval keeps the authored cap because every dash has
                // ends. A zero-axis oval instead degenerates to a segment.
                let stroke = if *w > 0.0 && *h > 0.0 {
                    &stroke_cap_for_closed_contours(stroke)
                } else {
                    stroke
                };
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            if stroke.align == StrokeAlign::Center {
                                draw_native_centered_stroke(
                                    stroke,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                    |paint| {
                                        canvas.draw_oval(Rect::from_wh(*w, *h), paint);
                                    },
                                );
                            } else {
                                draw_stroke(
                                    canvas,
                                    &oval_path(*w, *h),
                                    stroke,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                );
                            }
                        });
                    }
                    StrokeSpace::Frame => draw_frame_space_stroke(
                        canvas,
                        view,
                        &item.world,
                        &oval_path(*w, *h),
                        stroke,
                        *dash_phase,
                        *post_paint_opacity,
                        paint_box,
                        ctx,
                    ),
                }
            }
            ItemKind::AbsoluteDashedOvalStroke {
                x,
                y,
                w,
                h,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                let paint_box = PaintBox::from_xywh(*x, *y, *w, *h);
                let stroke = if *w > 0.0 && *h > 0.0 {
                    &stroke_cap_for_closed_contours(stroke)
                } else {
                    stroke
                };
                debug_assert_eq!(stroke.align, StrokeAlign::Center);
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            draw_native_centered_stroke(
                                stroke,
                                *dash_phase,
                                *post_paint_opacity,
                                paint_box,
                                ctx,
                                |paint| {
                                    canvas.draw_oval(Rect::from_xywh(*x, *y, *w, *h), paint);
                                },
                            );
                        });
                    }
                    StrokeSpace::Frame => {
                        let mut builder = PathBuilder::new();
                        builder.add_oval(
                            Rect::from_xywh(*x, *y, *w, *h),
                            Some(PathDirection::CW),
                            Some(1),
                        );
                        draw_frame_space_stroke(
                            canvas,
                            view,
                            &item.world,
                            &builder.snapshot(),
                            stroke,
                            *dash_phase,
                            *post_paint_opacity,
                            paint_box,
                            ctx,
                        );
                    }
                }
            }
            ItemKind::LineStroke {
                x1,
                y1,
                x2,
                y2,
                paint_w,
                paint_h,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                let paint_box = PaintBox::from_size(*paint_w, *paint_h);
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            if stroke.align == StrokeAlign::Center {
                                draw_native_centered_stroke(
                                    stroke,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                    |paint| {
                                        canvas.draw_line((*x1, *y1), (*x2, *y2), paint);
                                    },
                                );
                            } else {
                                draw_stroke(
                                    canvas,
                                    &line_path(*x1, *y1, *x2, *y2),
                                    stroke,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                );
                            }
                        });
                    }
                    StrokeSpace::Frame => draw_frame_space_stroke(
                        canvas,
                        view,
                        &item.world,
                        &line_path(*x1, *y1, *x2, *y2),
                        stroke,
                        *dash_phase,
                        *post_paint_opacity,
                        paint_box,
                        ctx,
                    ),
                }
            }
            ItemKind::PathStroke {
                w,
                h,
                path,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                let paint_box = PaintBox::from_size(*w, *h);
                let geometry = backend_path(path);
                // One draw, so one composite pass. A solid closed contour's
                // cap is inert; a dashed path keeps its authored cap.
                let adjusted = if path.all_contours_closed && !any_contour_may_be_degenerate(path) {
                    stroke_cap_for_closed_contours(stroke)
                } else {
                    stroke.clone()
                };
                match effective_stroke_space(*space, &item.world) {
                    StrokeSpace::Local => {
                        with_local_transform(canvas, view, &item.world, || {
                            if adjusted.align != StrokeAlign::Center {
                                draw_stroke(
                                    canvas,
                                    &geometry,
                                    &adjusted,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                );
                            } else {
                                draw_native_centered_stroke(
                                    &adjusted,
                                    *dash_phase,
                                    *post_paint_opacity,
                                    paint_box,
                                    ctx,
                                    |paint| {
                                        canvas.draw_path(&geometry, paint);
                                    },
                                );
                            }
                        });
                    }
                    StrokeSpace::Frame => draw_frame_space_stroke(
                        canvas,
                        view,
                        &item.world,
                        &geometry,
                        &adjusted,
                        *dash_phase,
                        *post_paint_opacity,
                        paint_box,
                        ctx,
                    ),
                }
            }
            ItemKind::TextStroke {
                layout,
                paint_w,
                paint_h,
                stroke,
                space,
                dash_phase,
                post_paint_opacity,
            } => {
                if layout.glyph_runs.is_empty() {
                    continue;
                }
                debug_assert_eq!(*space, StrokeSpace::Local);
                with_local_transform(canvas, view, &item.world, || {
                    let paint_box = PaintBox::from_size(*paint_w, *paint_h);
                    if stroke.align == StrokeAlign::Center {
                        draw_native_centered_stroke(
                            stroke,
                            *dash_phase,
                            *post_paint_opacity,
                            paint_box,
                            ctx,
                            |paint| {
                                for run in &layout.glyph_runs {
                                    glyph_scratch.with_run(run, list, |font, glyphs, positions| {
                                        canvas.draw_glyphs_at(
                                            glyphs,
                                            positions,
                                            Point::new(0.0, 0.0),
                                            font,
                                            paint,
                                        );
                                    });
                                }
                            },
                        );
                    } else {
                        let source = text_path(layout, list, &mut glyph_scratch);
                        draw_stroke(
                            canvas,
                            &source,
                            stroke,
                            *dash_phase,
                            *post_paint_opacity,
                            paint_box,
                            ctx,
                        );
                    }
                });
            }
        }
    }
    debug_assert!(scopes.is_empty(), "unclosed drawlist scopes: {scopes:?}");
    debug_assert_eq!(canvas.save_count(), initial_save_count);
    // Protect host state even if a hand-authored DrawList violates the internal
    // balancing invariant in a release build.
    canvas.restore_to_count(initial_save_count);
}

/// Render a raw drawlist to a fresh raster surface without a frame-environment
/// check and return its premultiplied pixel bytes. This is the low-level
/// reference for glyphless differential probes; complete products use
/// [`crate::frame::FrameProduct::raster_to_bytes`].
///
/// Bytes, NOT PNG: the encoder is not the system under test, and byte
/// equality is exact (ENG-0.3), not a tolerance. Resource-bearing complete
/// products enter through [`crate::frame::FrameProduct::raster_to_bytes`].
pub fn raster_to_bytes_unchecked<K>(
    list: &DrawList<K>,
    view: &Affine,
    w: i32,
    h: i32,
    ctx: &PaintCtx,
) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).expect("raster surface");
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    execute_unchecked(canvas, list, view, ctx);
    read_pixels(&mut surface, w, h)
}

/// Read a raster surface's premultiplied pixels into a byte buffer.
///
/// Requests explicit RGBA8888 so the byte order does not depend on the
/// platform's native N32 (BGRA on x86 Linux/Windows, RGBA on Apple
/// Silicon) — same rule as the forced-RGBA readback in
/// `tests/n0_xml.rs`.
pub fn read_pixels(surface: &mut skia_safe::Surface, w: i32, h: i32) -> Vec<u8> {
    let info = ImageInfo::new(
        (w, h),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let row_bytes = (w * 4) as usize;
    let mut buf = vec![0u8; row_bytes * h as usize];
    let ok = surface.read_pixels(&info, &mut buf, row_bytes, (0, 0));
    assert!(ok, "read_pixels failed");
    buf
}
