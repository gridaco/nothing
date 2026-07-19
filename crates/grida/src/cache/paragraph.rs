//! # Paragraph Cache
//!
//! High-performance cache for text layout and rendering operations.
//!
//! ## Design Philosophy
//!
//! Separates **geometry measurement** from **rendering** to optimize for distinct use cases:
//! - **Measurement**: Fast, cacheable, paint-independent operations for geometry calculations
//! - **Rendering**: Paint-aware operations optimized for actual text drawing
//!
//! ### Why This Separation?
//!
//! 1. **Skia Limitations**: Skia paragraphs cannot be cloned or have their paint modified after creation.
//!    For shader paints (gradients, images), the measured size is required, so measurement must happen first.
//!
//! 2. **Performance Optimization**: While we could use glyph iteration (`visit()`) to apply custom paints,
//!    this is actually more expensive than re-creating the paragraph with paint applied, allowing us to
//!    leverage Skia's optimized single-command rendering.
//!
//! 3. **Future-Proof Design**: This separation provides a clean API foundation. While we currently use
//!    separate caching for measurement and painting, this design allows for future optimizations and
//!    more performant solutions as the codebase evolves.
//!
//! ## Key Features
//!
//! - **Content-based caching**: Uses text content hash as key, not NodeId (prevents memory leaks)
//! - **Layout result caching**: Caches layout measurements by width to avoid redundant layout calls
//! - **Paint strategy optimization**: Chooses between re-creation vs visit() based on paint complexity
//! - **Font generation tracking**: Invalidates cache when fonts change
//! - **Resolved-text artifact**: This cache is the single producer of the
//!   immutable [`crate::text::resolved::ResolvedTextLayout`] artifact — the
//!   engine's realization of the Universal Shaped Text Layout RFD
//!   (`docs/wg/feat-paragraph/text-layout.md`). [`ParagraphCache::measure`]
//!   is a projection of that artifact; painting keeps the cached Skia
//!   `Paragraph` (geometry is the artifact's surface, paint is not).
//!
//! ## Usage Patterns
//!
//! ### For Geometry/Measurement
//! ```rust
//! # use grida::cache::paragraph::ParagraphCache;
//! # use grida::cg::prelude::*;
//! # use std::sync::{Arc, Mutex};
//! # use grida::resources::ByteStore;
//! # use grida::runtime::font_repository::FontRepository;
//! # let mut cache = ParagraphCache::new();
//! # let text = "Hello World";
//! # let style = TextStyleRec::from_font("Arial", 16.0);
//! # let align = TextAlign::Left;
//! # let max_lines = None;
//! # let ellipsis = None;
//! # let width = Some(100.0);
//! # let fonts = FontRepository::new(Arc::new(Mutex::new(ByteStore::new())));
//! let measurements = cache.measure(text, &style, &align, &max_lines, &ellipsis, width, &fonts, None);
//! // Use measurements.max_width, measurements.height, etc.
//! ```
//!
//! ### For Rendering
//! ```rust
//! # use grida::cache::paragraph::ParagraphCache;
//! # use grida::cg::prelude::*;
//! # use std::sync::{Arc, Mutex};
//! # use grida::resources::ByteStore;
//! # use grida::runtime::{font_repository::FontRepository, image_repository::ImageRepository};
//! # let mut cache = ParagraphCache::new();
//! # let text = "Hello World";
//! # let fills = &[Paint::Solid(CGColor::BLACK.into())];
//! # let align = TextAlign::Left;
//! # let style = TextStyleRec::from_font("Arial", 16.0);
//! # let max_lines = None;
//! # let ellipsis = None;
//! # let width = Some(100.0);
//! # let fonts = FontRepository::new(Arc::new(Mutex::new(ByteStore::new())));
//! # let images = ImageRepository::new(Arc::new(Mutex::new(ByteStore::new())));
//! let paragraph = cache.paragraph(text, fills, &align, &style, &max_lines, &ellipsis, width, &fonts, &images, None);
//! // paragraph.paint(canvas, point); // Use with actual canvas and point
//! ```

use crate::cache::fast_hash::DenseNodeMap;
use crate::cg::prelude::*;
use crate::node::schema::NodeId;
use crate::painter::paint;
use crate::runtime::font_repository::FontRepository;
use crate::runtime::render_policy::RenderIntent;
use crate::text::resolved::{self, EnvironmentId, ResolvedTextLayout, ShapingTransform};
use crate::text::text_style::textstyle;
use skia_safe::textlayout;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// Identifies a paragraph cache entry by either NodeId or shape-based hash key.
///
/// This enum allows the paragraph cache to support two different caching strategies:
/// - `ById`: Direct lookup by node ID (primary usage for text nodes)
/// - `ByShapeKey`: Content-based lookup by hashed text properties (flexible usage)
#[derive(Clone, Debug)]
pub enum ParagraphIdentifier {
    /// Cache entry identified by node ID
    ById(NodeId),
    /// Cache entry identified by shape-based hash key
    ByShapeKey(u64),
}

/// Baseline information for a single line of text, used for overlay rendering.
///
/// This struct contains the geometric information needed to draw baseline paths
/// for text overlay features like hit testing and stroke visualization.
#[derive(Clone, Copy, Debug)]
pub struct BaselineInfo {
    /// Left edge of the line in text coordinates
    pub left: f32,
    /// Width of the line in text coordinates
    pub width: f32,
    /// Y position of the baseline in text coordinates
    pub baseline_y: f32,
}

/// Comprehensive layout measurements for a text paragraph.
///
/// This struct contains all available measurement results from the Skia Paragraph API,
/// providing complete geometric information for layout calculations and rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutMeasurements {
    // Basic dimensions
    /// Total height of the paragraph
    pub height: f32,
    /// Maximum width used during layout
    pub max_width: f32,
    /// Minimum intrinsic width (tightest possible width)
    pub min_intrinsic_width: f32,
    /// Maximum intrinsic width (widest possible width)
    pub max_intrinsic_width: f32,

    // Baseline information
    /// Y position of the alphabetic baseline
    pub alphabetic_baseline: f32,
    /// Y position of the ideographic baseline
    pub ideographic_baseline: f32,

    // Line information
    /// Width of the longest line in the paragraph
    pub longest_line: f32,
    /// Total number of lines in the paragraph
    pub line_number: usize,
    /// Whether the paragraph exceeded the maximum line limit
    pub did_exceed_max_lines: bool,
}

/// A cached paragraph entry containing the paragraph object and metadata.
///
/// This struct stores a Skia paragraph along with its cache metadata, including
/// the content hash and font generation for cache invalidation.
#[derive(Clone, Debug)]
pub struct ParagraphCacheEntry {
    /// Content-based hash key for the paragraph
    pub hash: u64,
    /// Font generation at the time of caching (for invalidation)
    pub font_generation: usize,
    /// The cached Skia paragraph object
    pub paragraph: Rc<RefCell<textlayout::Paragraph>>,
    /// The resolved-text artifact extracted at the last layout width —
    /// avoids re-calling Skia `paragraph.layout()` on every access when the
    /// width hasn't changed. Immutable: a width or environment change
    /// replaces it wholesale (see [`crate::text::resolved`]).
    pub artifact: Arc<ResolvedTextLayout>,
    /// The width constraint `artifact` was resolved under.
    pub artifact_width: Option<f32>,
}

impl From<&ResolvedTextLayout> for LayoutMeasurements {
    /// Measurement is a query over the resolved artifact, not a parallel
    /// text operation: the numbers are the oracle's paragraph-level readout
    /// carried by the artifact, verbatim.
    fn from(artifact: &ResolvedTextLayout) -> Self {
        let m = &artifact.metrics;
        LayoutMeasurements {
            height: m.height,
            max_width: m.max_width,
            min_intrinsic_width: m.min_intrinsic_width,
            max_intrinsic_width: m.max_intrinsic_width,
            alphabetic_baseline: m.alphabetic_baseline,
            ideographic_baseline: m.ideographic_baseline,
            longest_line: m.longest_line,
            line_number: m.line_count,
            did_exceed_max_lines: m.did_exceed_max_lines,
        }
    }
}

/// Accumulated statistics from `measure()` calls — for benchmarking only.
/// All fields are simple integer counters (zero-cost increments).
#[derive(Default, Debug, Clone, Copy)]
pub struct ParagraphMeasureStats {
    pub calls: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

#[derive(Default, Debug, Clone)]
pub struct ParagraphCache {
    // ID-based cache for text nodes (primary usage) — Vec-backed for O(1) access
    entries_measurement_by_id: DenseNodeMap<ParagraphCacheEntry>,
    // Shape-key-based cache for flexible usage (not currently used)
    entries_measurement_by_shapekey_unstable: HashMap<u64, ParagraphCacheEntry>,
    /// Benchmark statistics — zero-cost when not read.
    pub stats: ParagraphMeasureStats,
    /// When true, `measure()` returns a zero-size stub without calling Skia.
    /// For benchmarking only — isolates text shaping cost from layout cost.
    pub skip_text_measure: bool,
}

impl ParagraphCache {
    pub fn new() -> Self {
        Self {
            entries_measurement_by_id: DenseNodeMap::new(),
            entries_measurement_by_shapekey_unstable: HashMap::new(),
            stats: ParagraphMeasureStats::default(),
            skip_text_measure: false,
        }
    }

    /// Generate cache key for geometry-only properties
    /// Excludes paint-related properties that don't affect layout
    fn shape_key(
        text: &str,
        style: &TextStyleRec,
        align: &TextAlign,
        max_lines: &Option<usize>,
    ) -> u64 {
        let mut h = DefaultHasher::new();
        text.hash(&mut h);
        style.font_family.hash(&mut h);
        style.font_size.to_bits().hash(&mut h);
        style.font_weight.0.hash(&mut h);
        style.font_style_italic.hash(&mut h);
        // TODO: Add letter_spacing and line_height to hash
        // style.letter_spacing.0.to_bits().hash(&mut h);
        // style.line_height.map(|v| v.to_bits()).hash(&mut h);
        style.text_transform.hash(&mut h);
        (*align as u8).hash(&mut h);
        max_lines.hash(&mut h);
        h.finish()
    }

    /// Get or create paragraph for measurement only
    /// Returns final measured metrics for the given width
    /// If id is provided, uses ID-based caching; otherwise uses shape-key-based caching
    ///
    /// The measurements are a projection of the resolved-text artifact this
    /// call produces (see [`Self::resolve`]): the measurement path consumes
    /// the artifact's logical metrics rather than re-querying the live Skia
    /// paragraph.
    pub fn measure(
        &mut self,
        text: &str,
        style: &TextStyleRec,
        align: &TextAlign,
        max_lines: &Option<usize>,
        ellipsis: &Option<String>,
        width: Option<f32>,
        fonts: &FontRepository,
        id: Option<&NodeId>,
    ) -> LayoutMeasurements {
        self.resolve(text, style, align, max_lines, ellipsis, width, fonts, id)
            .map(|artifact| LayoutMeasurements::from(artifact.as_ref()))
            .unwrap_or_default()
    }

    /// Resolve text into the immutable resolved-text-layout artifact
    /// ([`crate::text::resolved`]), producing (or reusing) the cached Skia
    /// paragraph at this choke point.
    ///
    /// Returns `None` only when [`Self::skip_text_measure`] is enabled (a
    /// benchmark-only stub): no artifact is fabricated for the stub path.
    pub fn resolve(
        &mut self,
        text: &str,
        style: &TextStyleRec,
        align: &TextAlign,
        max_lines: &Option<usize>,
        ellipsis: &Option<String>,
        width: Option<f32>,
        fonts: &FontRepository,
        id: Option<&NodeId>,
    ) -> Option<Arc<ResolvedTextLayout>> {
        let shape_key = Some(Self::shape_key(text, style, align, max_lines));
        self.resolve_inner(width, fonts, id, shape_key, |fonts| {
            let paragraph_style =
                crate::text::make_paragraph_style(*align, *max_lines, ellipsis.as_deref());

            let ctx = TextStyleRecBuildContext {
                color: CGColor::TRANSPARENT, // No color for measurement
            };
            let mut para_builder =
                textlayout::ParagraphBuilder::new(&paragraph_style, fonts.font_collection());
            let ts = textstyle(style, &Some(ctx), Some(fonts));
            para_builder.push_style(&ts);
            let transformed_text =
                crate::text::text_transform::transform_text(text, style.text_transform);
            para_builder.add_text(&transformed_text);
            let paragraph = para_builder.build();
            para_builder.pop();
            let transform = match style.text_transform {
                TextTransform::None => ShapingTransform::Identity,
                other => ShapingTransform::Uniform(other),
            };
            (paragraph, transformed_text, transform)
        })
    }

    /// Shared cache-hit/miss/store logic for both `resolve()` and
    /// `resolve_attributed()`.
    ///
    /// Both public resolve methods delegate to this, passing different
    /// paragraph-building closures. The closure returns the built paragraph
    /// together with the exact shaping text it fed the shaper and the
    /// declared source-transformation policy.
    ///
    /// - `shape_key`: when `Some`, enables shape-key-based caching (used when `id` is `None`).
    ///   When `None`, only ID-based caching is used.
    fn resolve_inner<F>(
        &mut self,
        width: Option<f32>,
        fonts: &FontRepository,
        id: Option<&NodeId>,
        shape_key: Option<u64>,
        build_paragraph: F,
    ) -> Option<Arc<ResolvedTextLayout>>
    where
        F: FnOnce(&FontRepository) -> (textlayout::Paragraph, String, ShapingTransform),
    {
        if self.skip_text_measure {
            return None;
        }

        let fonts_gen = fonts.generation();
        let environment = EnvironmentId {
            font_generation: fonts_gen,
        };
        self.stats.calls += 1;

        // Check if we have a cached paragraph
        if let Some(node_id) = id {
            // Use ID-based cache
            if let Some(entry) = self.entries_measurement_by_id.get_mut(node_id) {
                if entry.font_generation == fonts_gen {
                    self.stats.cache_hits += 1;
                    // Fast path: reuse the cached artifact if width matches
                    if entry.artifact_width == width {
                        return Some(entry.artifact.clone());
                    }
                    // Width changed: re-layout the same paragraph and
                    // resolve a fresh artifact (never patched in place).
                    let prior = entry.artifact.clone();
                    let artifact = Arc::new(Self::resolve_with_width(
                        entry.paragraph.clone(),
                        width,
                        &prior.shaping_text,
                        prior.transform,
                        environment,
                    ));
                    entry.artifact = artifact.clone();
                    entry.artifact_width = width;
                    return Some(artifact);
                }
            }
        } else if let Some(hash) = shape_key {
            // Use shape-key-based cache
            if let Some(entry) = self.entries_measurement_by_shapekey_unstable.get_mut(&hash) {
                if entry.font_generation == fonts_gen {
                    self.stats.cache_hits += 1;
                    if entry.artifact_width == width {
                        return Some(entry.artifact.clone());
                    }
                    let prior = entry.artifact.clone();
                    let artifact = Arc::new(Self::resolve_with_width(
                        entry.paragraph.clone(),
                        width,
                        &prior.shaping_text,
                        prior.transform,
                        environment,
                    ));
                    entry.artifact = artifact.clone();
                    entry.artifact_width = width;
                    return Some(artifact);
                }
            }
        }
        self.stats.cache_misses += 1;

        // Build the paragraph (expensive operation) — no paint for measurement.
        let (paragraph, shaping_text, transform) = build_paragraph(fonts);
        let paragraph_rc = Rc::new(RefCell::new(paragraph));

        // Resolve the artifact and cache it with the entry
        let artifact = Arc::new(Self::resolve_with_width(
            paragraph_rc.clone(),
            width,
            &shaping_text,
            transform,
            environment,
        ));

        let entry = ParagraphCacheEntry {
            hash: shape_key.unwrap_or(0),
            font_generation: fonts_gen,
            paragraph: paragraph_rc,
            artifact: artifact.clone(),
            artifact_width: width,
        };

        // Store in the appropriate cache
        if let Some(node_id) = id {
            self.entries_measurement_by_id.insert(*node_id, entry);
        } else if let Some(hash) = shape_key {
            self.entries_measurement_by_shapekey_unstable
                .insert(hash, entry);
        }

        Some(artifact)
    }

    /// Lay the paragraph out for the requested width (identical layout
    /// sequence to the historical measurement path) and extract the resolved
    /// artifact from the laid-out paragraph.
    fn resolve_with_width(
        paragraph_rc: Rc<RefCell<textlayout::Paragraph>>,
        width: Option<f32>,
        shaping_text: &str,
        transform: ShapingTransform,
        environment: EnvironmentId,
    ) -> ResolvedTextLayout {
        // Calculate the final layout width
        let layout_width = if let Some(width) = width {
            width
        } else {
            // For intrinsic sizing, layout with infinity first to measure
            let mut para_ref = paragraph_rc.borrow_mut();
            para_ref.layout(f32::INFINITY);
            let intrinsic_width = para_ref.max_intrinsic_width();

            // Re-layout with the intrinsic width
            para_ref.layout(intrinsic_width);
            intrinsic_width
        };

        // Apply final layout with the determined width
        {
            let mut para_ref = paragraph_rc.borrow_mut();
            para_ref.layout(layout_width);
        }

        // Extract the artifact from the laid-out paragraph (read-only; the
        // paragraph keeps its layout state for painting and overlays).
        let mut para_ref = paragraph_rc.borrow_mut();
        resolved::resolve_from_paragraph(
            &mut para_ref,
            shaping_text,
            transform,
            width,
            layout_width,
            environment,
        )
    }

    /// Measure an attributed string (per-run styled text) for geometry.
    ///
    /// Unlike [`measure`](Self::measure), this builds a paragraph with per-run styles so that
    /// varying font sizes, weights, and families are reflected in the measured
    /// dimensions. No paint is applied — this is geometry-only.
    ///
    /// Results are cached by node ID with the same invalidation strategy as
    /// [`measure`](Self::measure). Like [`measure`](Self::measure), the returned numbers are a
    /// projection of the resolved-text artifact
    /// (see [`Self::resolve_attributed`]).
    pub fn measure_attributed(
        &mut self,
        attr: &crate::cg::types::AttributedString,
        align: &TextAlign,
        max_lines: &Option<usize>,
        ellipsis: &Option<String>,
        width: Option<f32>,
        fonts: &FontRepository,
        id: Option<&NodeId>,
    ) -> LayoutMeasurements {
        self.resolve_attributed(attr, align, max_lines, ellipsis, width, fonts, id)
            .map(|artifact| LayoutMeasurements::from(artifact.as_ref()))
            .unwrap_or_default()
    }

    /// Resolve an attributed string into the immutable
    /// resolved-text-layout artifact ([`crate::text::resolved`]).
    ///
    /// Returns `None` only when [`Self::skip_text_measure`] is enabled (a
    /// benchmark-only stub): no artifact is fabricated for the stub path.
    pub fn resolve_attributed(
        &mut self,
        attr: &crate::cg::types::AttributedString,
        align: &TextAlign,
        max_lines: &Option<usize>,
        ellipsis: &Option<String>,
        width: Option<f32>,
        fonts: &FontRepository,
        id: Option<&NodeId>,
    ) -> Option<Arc<ResolvedTextLayout>> {
        self.resolve_inner(width, fonts, id, None, |fonts| {
            let paragraph_style =
                crate::text::make_paragraph_style(*align, *max_lines, ellipsis.as_deref());

            let mut para_builder =
                textlayout::ParagraphBuilder::new(&paragraph_style, fonts.font_collection());

            let mut shaping_text = String::with_capacity(attr.text.len());
            let mut any_transform = false;
            for run in &attr.runs {
                let ctx = TextStyleRecBuildContext {
                    color: CGColor::TRANSPARENT,
                };
                let ts = textstyle(&run.style, &Some(ctx), Some(fonts));
                para_builder.push_style(&ts);
                let run_text = &attr.text[run.start as usize..run.end as usize];
                let transformed =
                    crate::text::text_transform::transform_text(run_text, run.style.text_transform);
                para_builder.add_text(&transformed);
                any_transform |= run.style.text_transform != TextTransform::None;
                shaping_text.push_str(&transformed);
            }

            let transform = if any_transform {
                ShapingTransform::PerRun
            } else {
                ShapingTransform::Identity
            };
            (para_builder.build(), shaping_text, transform)
        })
    }

    /// Get or create paragraph for rendering with fill paint applied.
    ///
    /// This method handles all fill paint types (solid, gradient, image, multiple fills) using `cvt::sk_paint_stack`.
    /// The returned paragraph is ready for rendering with `paragraph.paint(canvas, point)`.
    ///
    /// # Stroke Paint Limitation
    ///
    /// **This method does NOT handle stroke paint.** Skia paragraphs cannot hold stroke paint with stroke alignment
    /// (inside, center, outside). Stroke rendering must be handled externally by:
    /// 1. Getting the text path from the paragraph
    /// 2. Applying stroke paint with the appropriate stroke alignment
    /// 3. Drawing the stroked path separately
    ///
    /// # Parameters
    ///
    /// - `text`: The text content to render
    /// - `fills`: Fill paints to apply (solid, gradient, image, etc.)
    /// - `align`: Text alignment
    /// - `style`: Text style properties
    /// - `max_lines`: Maximum number of lines (optional)
    /// - `ellipsis`: Ellipsis string for overflow (optional)
    /// - `width`: Layout width (optional, uses intrinsic width if None)
    /// - `fonts`: Font repository for text shaping
    /// - `images`: Image repository for image fills
    /// - `id`: Node ID for caching (optional, uses shape-key caching if None)
    ///
    /// # Returns
    ///
    /// A `Rc<RefCell<textlayout::Paragraph>>` ready for rendering with fill paint applied.
    pub fn paragraph(
        &mut self,
        text: &str,
        fills: &[Paint],
        align: &TextAlign,
        style: &TextStyleRec,
        max_lines: &Option<usize>,
        ellipsis: &Option<String>,
        width: Option<f32>,
        fonts: &FontRepository,
        images: &crate::runtime::image_repository::ImageRepository,
        id: Option<&NodeId>,
    ) -> Rc<RefCell<textlayout::Paragraph>> {
        let _fonts_gen = fonts.generation();
        let _hash = Self::shape_key(text, style, align, max_lines);

        // First, get the layout measurements to determine the size for paint
        let measurements = self.measure(text, style, align, max_lines, ellipsis, width, fonts, id);
        let layout_size = (measurements.max_width, measurements.height);

        // Build the paragraph with paint applied (for rendering)
        let fill_paint = if !fills.is_empty() {
            // Use sk_paint_stack for all paint types (solid, gradient, image, multiple fills).
            // The paragraph (with paint baked in) is cached and reused across frames and
            // clients, so it cannot carry a live render intent. Text with an *image* fill is
            // an edge case; always render it best-quality (`Render`) rather than key the
            // paragraph cache on intent.
            paint::sk_paint_stack(fills, layout_size, images, true, RenderIntent::Render)
        } else {
            None
        };

        let paragraph_style =
            crate::text::make_paragraph_style(*align, *max_lines, ellipsis.as_deref());

        let ctx = TextStyleRecBuildContext {
            color: fills
                .first()
                .and_then(|f| f.solid_color())
                .unwrap_or(CGColor::TRANSPARENT),
        };
        let mut para_builder =
            textlayout::ParagraphBuilder::new(&paragraph_style, fonts.font_collection());
        let mut ts = textstyle(style, &Some(ctx), Some(fonts));
        if let Some(ref paint) = fill_paint {
            ts.set_foreground_paint(paint);
        }
        para_builder.push_style(&ts);
        let transformed_text =
            crate::text::text_transform::transform_text(text, style.text_transform);
        para_builder.add_text(&transformed_text);
        let paragraph: skia_safe::textlayout::Paragraph = para_builder.build();
        para_builder.pop();

        let paragraph_rc = Rc::new(RefCell::new(paragraph));

        // Apply layout with the determined width
        let layout_width = width.unwrap_or(measurements.max_intrinsic_width);
        paragraph_rc.borrow_mut().layout(layout_width);

        paragraph_rc
    }

    /// Get baseline information for overlay purposes, only if paragraph is already cached by ID.
    ///
    /// Returns `None` if the paragraph is not cached or if the cached entry
    /// was built with a stale font generation (the caller must re-measure first).
    pub fn get_baseline_info_if_cached_by_id(
        &self,
        id: &NodeId,
        width: Option<f32>,
        font_generation: usize,
    ) -> Option<(Vec<BaselineInfo>, f32)> {
        // Check if we have a cached paragraph by ID
        if let Some(entry) = self.entries_measurement_by_id.get(id) {
            // Reject stale entries — prevents reading tofu-width baselines
            // when fonts have been loaded since this entry was cached.
            if entry.font_generation != font_generation {
                return None;
            }
            let paragraph_rc = &entry.paragraph;

            // Apply layout if width is specified
            {
                let mut paragraph_ref = paragraph_rc.borrow_mut();

                // Apply layout if width is specified
                if let Some(w) = width {
                    paragraph_ref.layout(w);
                }
            }

            // Collect baseline info and layout height in a separate scope to avoid borrowing issues
            let (layout_height, baseline_info) = {
                let paragraph_ref = paragraph_rc.borrow();
                let lines = paragraph_ref.line_number();
                let mut baseline_info = Vec::new();
                for i in 0..lines {
                    if let Some(metrics) = paragraph_ref.get_line_metrics_at(i) {
                        baseline_info.push(BaselineInfo {
                            left: metrics.left as f32,
                            width: metrics.width as f32,
                            baseline_y: metrics.baseline as f32,
                        });
                    }
                }
                (paragraph_ref.height(), baseline_info)
            };
            return Some((baseline_info, layout_height));
        }

        None
    }

    pub fn invalidate(&mut self) {
        self.entries_measurement_by_id.clear();
        self.entries_measurement_by_shapekey_unstable.clear();
    }

    /// Invalidate the cached paragraph for a single node.
    pub fn invalidate_by_id(&mut self, id: NodeId) {
        self.entries_measurement_by_id.remove(&id);
    }

    pub fn len(&self) -> usize {
        self.entries_measurement_by_id.len() + self.entries_measurement_by_shapekey_unstable.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
