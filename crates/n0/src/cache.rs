//! ENG-2 · the scene raster cache (the compositor tier).
//!
//! Rasters the scene ONCE into a backend-matched offscreen image (GPU stays
//! GPU via `Canvas::new_surface`) covering the viewport plus a margin, then
//! re-composites it under camera PANS with a single blit — turning the
//! O(nodes) `execute` wall into an O(1) image draw. It re-rasters only on a
//! runtime-document incarnation, effective-value, resolve-option, or
//! paint-environment change; a ZOOM change (a bitmap can't be crisply rescaled
//! — that is the re-raster boundary, ENG-2 growth); or a pan beyond the cached
//! margin. The cached drawlist is reused across clean camera re-rasters, so a
//! camera-only frame never re-resolves or re-builds either (the
//! retained-drawlist win folded in).
//!
//! This is realtime-preview policy, not the accurate raster path. Its exact
//! pixel gates are deliberately fixture-scoped: the cache-cold and integer-pan
//! probes in `tests/cache.rs` remain byte-identical to immediate rendering.
//! That is not a universal promise for every drawlist. Every cached raster is
//! produced at a `+MARGIN` device translation and cropped back; Skia may round
//! antialiased coverage differently at that translated device origin for
//! rounded, dashed, translucent, or shaped geometry even when the translation
//! is an integer. Fractional pan additionally resamples. Accurate static and
//! exact-time export must execute the immutable frame product directly.

use n0_model::animation::SampleError;
use n0_model::math::Affine;
use n0_model::model::{Document, NodeKey};
use n0_model::properties::{PropertyError, PropertyValues, ValueView};
use n0_model::resolve::{ResolveOptions, RotationInFlow};
use skia_safe::{Canvas, Color, FilterMode, Image, ImageInfo, MipmapMode, SamplingOptions};

use crate::drawlist::DrawList;
use crate::frame::{
    resolve_and_build_view, EvaluatedFrameRequest, FrameBuildError, FrameExecutionError,
    FrameRequest, PaintEnvironmentMismatch,
};
use crate::paint::{execute_unchecked, PaintCtx, PaintEnvironmentKey};

/// Extra content rastered around the viewport, so small pans blit without a
/// re-raster. Larger margin = fewer re-raster hitches, more offscreen memory.
const MARGIN: f32 = 256.0;

/// Failure before a value-aware cached frame can reach the destination
/// canvas. Property validation precedes resolution; frame construction then
/// preflights the exact drawlist and resolved paint boxes.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneCacheError {
    Property(PropertyError),
    FrameBuild(FrameBuildError),
    FrameExecution(FrameExecutionError),
}

impl std::fmt::Display for SceneCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneCacheError::Property(error) => error.fmt(f),
            SceneCacheError::FrameBuild(error) => error.fmt(f),
            SceneCacheError::FrameExecution(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SceneCacheError {}

impl From<PropertyError> for SceneCacheError {
    fn from(error: PropertyError) -> Self {
        SceneCacheError::Property(error)
    }
}

impl From<FrameBuildError> for SceneCacheError {
    fn from(error: FrameBuildError) -> Self {
        SceneCacheError::FrameBuild(error)
    }
}

impl From<FrameExecutionError> for SceneCacheError {
    fn from(error: FrameExecutionError) -> Self {
        SceneCacheError::FrameExecution(error)
    }
}

/// Failure at the explicit Base/Sample cache seam. Sampling completes before
/// cache comparison or destination drawing.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneCacheRequestError {
    Sample(SampleError),
    Cache(SceneCacheError),
}

impl std::fmt::Display for SceneCacheRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneCacheRequestError::Sample(error) => error.fmt(f),
            SceneCacheRequestError::Cache(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SceneCacheRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SceneCacheRequestError::Sample(error) => Some(error),
            SceneCacheRequestError::Cache(error) => Some(error),
        }
    }
}

impl From<SampleError> for SceneCacheRequestError {
    fn from(error: SampleError) -> Self {
        SceneCacheRequestError::Sample(error)
    }
}

impl From<SceneCacheError> for SceneCacheRequestError {
    fn from(error: SceneCacheError) -> Self {
        SceneCacheRequestError::Cache(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolveOptionsKey {
    viewport_width: u32,
    viewport_height: u32,
    rotation_in_flow: RotationInFlow,
}

impl From<&ResolveOptions> for ResolveOptionsKey {
    fn from(options: &ResolveOptions) -> Self {
        Self {
            viewport_width: options.viewport.0.to_bits(),
            viewport_height: options.viewport.1.to_bits(),
            rotation_in_flow: options.rotation_in_flow,
        }
    }
}

/// One paint-ready drawlist and the exact resource environment it names. This
/// is deliberately private: source front-ends may prove the seam inside this
/// crate before it earns promotion, but hosts cannot bypass resolution or
/// checked frame construction.
struct DrawListPaintInput {
    list: DrawList,
    environment: PaintEnvironmentKey,
}

/// How the source-facing adapter supplies the next drawlist.
///
/// Ordinary n0 invalidation forces replacement even when rebuilding happens
/// to yield equal data. Its `FrameProduct` has already completed gradient
/// preflight. The test-only comparison arm accepts an independently assembled
/// drawlist and performs that preflight transactionally only if replacement is
/// needed.
enum DrawListUpdate {
    Retained,
    ReplacePreflighted(DrawListPaintInput),
    #[cfg(test)]
    CompareUnchecked(DrawListPaintInput),
}

#[derive(Debug, Clone, PartialEq)]
struct SourceCacheKeys {
    options: ResolveOptionsKey,
    scene: NodeKey,
    values: PropertyValues,
}

struct SourceCacheRequest<'a> {
    options: ResolveOptionsKey,
    scene: NodeKey,
    values: &'a PropertyValues,
    environment: PaintEnvironmentKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceDrawListDecision {
    Retain,
    Rebuild,
}

/// Source keys are an adapter concern, not drawlist raster identity.
enum SourceKeyCommit {
    Preserve,
    Replace(SourceCacheKeys),
    #[cfg(test)]
    ClearIfDrawListChanged,
}

/// The realtime-preview scene compositor. Holds a cached image
/// (backend-matched) and the view it was rastered at.
pub struct SceneCache {
    image: Option<Image>,
    /// The drawlist the cached image was rendered from — reused across clean
    /// camera re-rasters (pan-out / zoom). Semantic input changes rebuild it.
    list: Option<DrawList>,
    ref_view: Affine,
    vw: i32,
    vh: i32,
    /// Resource environment under which the drawlist was resolved and rastered.
    /// The drawlist retains exact text fonts, but a different or revised host
    /// context requests a new semantic resolution rather than stale reuse.
    environment_key: Option<PaintEnvironmentKey>,
    /// One coherent source-validity domain. It is deliberately separate from
    /// drawlist raster identity and is cleared atomically whenever a
    /// source-neutral drawlist replacement succeeds.
    source_key: Option<SourceCacheKeys>,
}

impl SceneCache {
    pub fn new(vw: i32, vh: i32) -> Self {
        SceneCache {
            image: None,
            list: None,
            ref_view: Affine::IDENTITY,
            vw,
            vh,
            environment_key: None,
            source_key: None,
        }
    }

    /// Composite the scene for `view` onto `canvas`. `doc_dirty` = the host
    /// mutated the document since the last frame (it knows: it applied an op).
    /// Returns `true` if this frame re-rastered (a diagnostic for the probe;
    /// the amortized win is that most frames return `false`). This is the live
    /// preview entry; accuracy-critical rendering executes a
    /// [`crate::frame::FrameProduct`] directly.
    pub fn frame(
        &mut self,
        canvas: &Canvas,
        doc: &Document,
        opts: &ResolveOptions,
        view: &Affine,
        ctx: &PaintCtx,
        doc_dirty: bool,
    ) -> Result<bool, SceneCacheError> {
        let values = PropertyValues::default();
        self.frame_view(
            canvas,
            &ValueView::base(doc),
            &values,
            opts,
            view,
            ctx,
            doc_dirty,
        )
    }

    /// Composite one explicit Base or Sample request. Time itself never enters
    /// the cache key: the sampled `PropertyValues` is the complete visual key.
    pub fn frame_request(
        &mut self,
        canvas: &Canvas,
        doc: &Document,
        request: FrameRequest<'_>,
        opts: &ResolveOptions,
        view: &Affine,
        ctx: &PaintCtx,
        doc_dirty: bool,
    ) -> Result<bool, SceneCacheRequestError> {
        match request.evaluate(doc)? {
            EvaluatedFrameRequest::Base => self
                .frame(canvas, doc, opts, view, ctx, doc_dirty)
                .map_err(Into::into),
            EvaluatedFrameRequest::Sample { values } => self
                .frame_with_values(canvas, doc, &values, opts, view, ctx, doc_dirty)
                .map_err(Into::into),
        }
    }

    /// Composite one frame with immutable effective property values. Invalid
    /// or stale targets fail before cache comparison or raster work. A changed
    /// value set rebuilds the retained drawlist even when `doc_dirty` is
    /// false.
    pub fn frame_with_values(
        &mut self,
        canvas: &Canvas,
        doc: &Document,
        values: &PropertyValues,
        opts: &ResolveOptions,
        view: &Affine,
        ctx: &PaintCtx,
        doc_dirty: bool,
    ) -> Result<bool, SceneCacheError> {
        let value_view = ValueView::new(doc, values)?;
        self.frame_view(canvas, &value_view, values, opts, view, ctx, doc_dirty)
    }

    fn frame_view(
        &mut self,
        canvas: &Canvas,
        values: &ValueView<'_>,
        values_key: &PropertyValues,
        opts: &ResolveOptions,
        view: &Affine,
        ctx: &PaintCtx,
        doc_dirty: bool,
    ) -> Result<bool, SceneCacheError> {
        let document = values.document();
        let scene_key = document
            .key_of(document.root)
            .expect("a render document has one live implicit root");
        let source_request = SourceCacheRequest {
            options: opts.into(),
            scene: scene_key,
            values: values_key,
            environment: ctx.environment_key(),
        };
        let decision = self.source_drawlist_decision(&source_request, doc_dirty);
        let (drawlist, source_keys) = if decision == SourceDrawListDecision::Rebuild {
            let product = resolve_and_build_view(values, opts, ctx)?;
            let (_, list, environment) = product.into_parts();
            (
                DrawListUpdate::ReplacePreflighted(DrawListPaintInput { list, environment }),
                SourceKeyCommit::Replace(SourceCacheKeys {
                    options: source_request.options,
                    scene: source_request.scene,
                    values: source_request.values.clone(),
                }),
            )
        } else {
            (DrawListUpdate::Retained, SourceKeyCommit::Preserve)
        };

        self.composite(canvas, view, ctx, drawlist, source_keys)
    }

    fn source_drawlist_decision(
        &self,
        requested: &SourceCacheRequest<'_>,
        doc_dirty: bool,
    ) -> SourceDrawListDecision {
        let same_source = self.source_key.as_ref().is_some_and(|cached| {
            cached.options == requested.options
                && cached.scene == requested.scene
                && cached.values == *requested.values
        });
        if doc_dirty
            || self.list.is_none()
            || self.environment_key != Some(requested.environment)
            || !same_source
        {
            SourceDrawListDecision::Rebuild
        } else {
            SourceDrawListDecision::Retain
        }
    }

    /// Test-only entrance for a complete drawlist. Equality compares
    /// every paint-consumed drawlist field and its private shaped-text font
    /// registry while ignoring only diagnostic item node slots, plus the
    /// opaque paint-environment key. Raster-equal reuse retains the cached
    /// drawlist's diagnostic node slots; preflight errors therefore keep
    /// reporting those retained slots. Promotion requires an explicit
    /// provenance policy. No source-model key enters comparison.
    #[cfg(test)]
    pub(crate) fn frame_drawlist(
        &mut self,
        canvas: &Canvas,
        list: DrawList,
        environment: PaintEnvironmentKey,
        view: &Affine,
        ctx: &PaintCtx,
    ) -> Result<bool, SceneCacheError> {
        self.composite(
            canvas,
            view,
            ctx,
            DrawListUpdate::CompareUnchecked(DrawListPaintInput { list, environment }),
            SourceKeyCommit::ClearIfDrawListChanged,
        )
    }

    /// Composite one drawlist into the destination. Every source adapter
    /// enters here, so cache comparison, preflight, offscreen replay, commit,
    /// and final blit cannot drift into parallel algorithms.
    fn composite(
        &mut self,
        canvas: &Canvas,
        view: &Affine,
        ctx: &PaintCtx,
        update: DrawListUpdate,
        source_keys: SourceKeyCommit,
    ) -> Result<bool, SceneCacheError> {
        let (replacement, needs_gradient_preflight) = match update {
            DrawListUpdate::Retained => (None, false),
            DrawListUpdate::ReplacePreflighted(drawlist) => (Some(drawlist), false),
            #[cfg(test)]
            DrawListUpdate::CompareUnchecked(drawlist) => {
                let equal = self
                    .list
                    .as_ref()
                    .is_some_and(|cached| cached.raster_eq(&drawlist.list))
                    && self.environment_key == Some(drawlist.environment);
                if equal {
                    (None, false)
                } else {
                    (Some(drawlist), true)
                }
            }
        };
        let drawlist_changed = replacement.is_some();
        let environment_key = replacement
            .as_ref()
            .map(|drawlist| drawlist.environment)
            .or(self.environment_key)
            .expect("a retained raster has one paint environment");
        let actual_environment = ctx.environment_key();
        if environment_key != actual_environment {
            return Err(FrameExecutionError::from(PaintEnvironmentMismatch {
                expected: environment_key,
                actual: actual_environment,
            })
            .into());
        }
        let list = replacement
            .as_ref()
            .map(|drawlist| &drawlist.list)
            .or(self.list.as_ref())
            .expect("a retained raster has one drawlist");

        let dx = view.e - self.ref_view.e;
        let dy = view.f - self.ref_view.f;
        let same_zoom = view.a == self.ref_view.a
            && view.b == self.ref_view.b
            && view.c == self.ref_view.c
            && view.d == self.ref_view.d;
        let reraster = self.image.is_none()
            || drawlist_changed
            || !same_zoom
            || dx.abs() > MARGIN
            || dy.abs() > MARGIN;

        if reraster {
            // Both local-matrix and view-dependent capability checks finish
            // before an offscreen or destination canvas is changed.
            if needs_gradient_preflight {
                crate::paint::preflight_gradients(list).map_err(FrameBuildError::from)?;
            }
            let mut shifted = *view;
            shifted.e += MARGIN;
            shifted.f += MARGIN;
            crate::paint::preflight_images(list, &shifted, ctx)
                .map_err(FrameExecutionError::from)?;

            let m = MARGIN as i32;
            let info = ImageInfo::new_n32_premul((self.vw + 2 * m, self.vh + 2 * m), None);
            let mut off = canvas
                .new_surface(&info, None)
                .expect("backend-matched offscreen surface");
            let oc = off.canvas();
            oc.clear(Color::WHITE);
            execute_unchecked(oc, list, &shifted, ctx);

            let image = off.image_snapshot();

            // Commit every cache field only after build, preflight, and
            // offscreen replay have all succeeded.
            if let Some(drawlist) = replacement {
                self.list = Some(drawlist.list);
            }
            self.image = Some(image);
            self.ref_view = *view;
            self.environment_key = Some(environment_key);
            match source_keys {
                SourceKeyCommit::Preserve => {}
                SourceKeyCommit::Replace(keys) => {
                    self.source_key = Some(keys);
                }
                #[cfg(test)]
                SourceKeyCommit::ClearIfDrawListChanged if drawlist_changed => {
                    self.source_key = None;
                }
                #[cfg(test)]
                SourceKeyCommit::ClearIfDrawListChanged => {}
            }
        }

        // Blit the cached image at the (now possibly zero) integer pan offset.
        let (dx, dy) = (view.e - self.ref_view.e, view.f - self.ref_view.f);
        let img = self.image.as_ref().expect("image present after raster");
        // Nearest sampling: for an integer offset each dest pixel maps to exactly
        // one src pixel (byte-exact); it never silently blurs at non-integer.
        let sampling = SamplingOptions::new(FilterMode::Nearest, MipmapMode::None);
        canvas.draw_image_with_sampling_options(img, (-MARGIN + dx, -MARGIN + dy), sampling, None);
        Ok(reraster)
    }
}

/// Render one preview-composited frame to a fresh raster surface and return its
/// bytes. Pairs with [`crate::paint::raster_to_bytes_unchecked`] in
/// fixture-scoped cache equivalence probes. A fresh cache is passed so the
/// first frame is a cache-cold, margin-shifted re-raster; call twice with
/// panned views to exercise the blit path. This helper does not turn preview
/// composition into the accurate frame path described in the module doctrine.
pub fn composited_to_bytes(
    cache: &mut SceneCache,
    doc: &Document,
    opts: &ResolveOptions,
    view: &Affine,
    ctx: &PaintCtx,
    doc_dirty: bool,
    w: i32,
    h: i32,
) -> Result<Vec<u8>, SceneCacheError> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).expect("raster surface");
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    cache.frame(canvas, doc, opts, view, ctx, doc_dirty)?;
    Ok(crate::paint::read_pixels(&mut surface, w, h))
}

/// Value-aware counterpart to [`composited_to_bytes`].
pub fn composited_to_bytes_with_values(
    cache: &mut SceneCache,
    doc: &Document,
    values: &PropertyValues,
    opts: &ResolveOptions,
    view: &Affine,
    ctx: &PaintCtx,
    doc_dirty: bool,
    w: i32,
    h: i32,
) -> Result<Vec<u8>, SceneCacheError> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).expect("raster surface");
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    cache.frame_with_values(canvas, doc, values, opts, view, ctx, doc_dirty)?;
    Ok(crate::paint::read_pixels(&mut surface, w, h))
}

#[cfg(test)]
#[path = "cache_drawlist_spike.rs"]
mod drawlist_spike;
