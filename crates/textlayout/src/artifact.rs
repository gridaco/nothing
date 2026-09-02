//! The immutable resolved text layout — the one answer every
//! geometry-sensitive consumer projects from.
//!
//! Geometry is stated in the text node's **local logical space**: x grows
//! right, y grows down, the pen origin is `(0, 0)` on the alphabetic
//! baseline, and fractional values are preserved (device-pixel rounding is
//! not part of text resolution). Font units are y-up; the flip happens
//! exactly once, inside this crate, so no consumer ever re-derives it.

use std::ops::Range;
use std::sync::Arc;

use crate::ORACLE_VERSION;
use crate::environment::FontKey;
use crate::source::{SourceRun, SourceRunTag};

/// An axis-aligned box in the artifact's local space. Deliberately this
/// crate's own four floats: the artifact depends on no geometry vocabulary,
/// so no render contract leaks in through a shared rectangle type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundsBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The face one resolution actually used, by verified identity — glyph ids
/// in this artifact are meaningful only together with this face.
#[derive(Clone, Debug)]
pub struct ResolvedFace {
    pub key: FontKey,
    pub face_index: u32,
    pub units_per_em: u16,
}

/// Line metrics in local px: distances from the baseline, both positive
/// (ascent reaches up, descent reaches down).
///
/// Oracle v4 retains the face's `hhea` ascent/descent metric policy as the
/// parser reports them; line gap (leading) is a declared deferral, arriving
/// as a field when a consumer first needs line stacking. For the pinned gate
/// font every metric table agrees, which is why the gate can hold before the
/// policy question is forced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMetrics {
    pub ascent: f32,
    pub descent: f32,
}

/// One shaping cluster's complete source/glyph cardinality at oracle v4.
///
/// The source has three coordinate spaces on purpose. HarfBuzz clusters are
/// seeded from UTF-8 byte offsets, Web text APIs address UTF-16 code units,
/// and source grammar is stated in Unicode scalars. Precomposed and
/// decomposed Latin make those ranges diverge; the artifact states all three
/// so no consumer can confuse one with another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapingCluster {
    source_utf8: Range<usize>,
    source_utf16: Range<usize>,
    source_scalars: Range<usize>,
    glyphs: Range<usize>,
    source_run_tag: SourceRunTag,
}

impl ShapingCluster {
    pub(crate) fn new(
        source_utf8: Range<usize>,
        source_utf16: Range<usize>,
        source_scalars: Range<usize>,
        glyphs: Range<usize>,
        source_run_tag: SourceRunTag,
    ) -> Self {
        Self {
            source_utf8,
            source_utf16,
            source_scalars,
            glyphs,
            source_run_tag,
        }
    }

    /// Covered source bytes, on UTF-8 scalar boundaries.
    pub fn source_utf8(&self) -> Range<usize> {
        self.source_utf8.clone()
    }

    /// Covered source UTF-16 code units.
    pub fn source_utf16(&self) -> Range<usize> {
        self.source_utf16.clone()
    }

    /// Covered source Unicode-scalar indices. Kept distinct from both byte
    /// and UTF-16 coordinates: combining text makes all three differ.
    pub fn source_scalars(&self) -> Range<usize> {
        self.source_scalars.clone()
    }

    /// Contiguous placed-glyph indices belonging to this cluster.
    pub fn glyphs(&self) -> Range<usize> {
        self.glyphs.clone()
    }

    /// The opaque source-run tag covering this cluster's first authored
    /// scalar. A source-run boundary inside the cluster never splits it.
    pub const fn source_run_tag(&self) -> SourceRunTag {
        self.source_run_tag
    }
}

/// One independently resolved shaping chunk in the artifact's canonical
/// local coordinate space.
///
/// Source, cluster, and glyph ranges use the artifact's global indices. The
/// chunk's glyphs were shaped together and no shaping interaction crosses
/// either boundary. Chunks are concatenated by advance only to keep every
/// glyph, outline, and bound in one coherent local coordinate system;
/// authored placement remains an external projection.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedShapingChunk {
    source_utf8: Range<usize>,
    source_utf16: Range<usize>,
    source_scalars: Range<usize>,
    clusters: Range<usize>,
    glyphs: Range<usize>,
    origin_x: f32,
    advance: f32,
}

impl ResolvedShapingChunk {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_utf8: Range<usize>,
        source_utf16: Range<usize>,
        source_scalars: Range<usize>,
        clusters: Range<usize>,
        glyphs: Range<usize>,
        origin_x: f32,
        advance: f32,
    ) -> Self {
        Self {
            source_utf8,
            source_utf16,
            source_scalars,
            clusters,
            glyphs,
            origin_x,
            advance,
        }
    }

    /// Covered source bytes, on UTF-8 scalar boundaries.
    pub fn source_utf8(&self) -> Range<usize> {
        self.source_utf8.clone()
    }

    /// Covered source UTF-16 code units, globally indexed in the artifact.
    pub fn source_utf16(&self) -> Range<usize> {
        self.source_utf16.clone()
    }

    /// Covered source Unicode-scalar indices, globally indexed.
    pub fn source_scalars(&self) -> Range<usize> {
        self.source_scalars.clone()
    }

    /// Contiguous global cluster indices produced by this shaping operation.
    pub fn clusters(&self) -> Range<usize> {
        self.clusters.clone()
    }

    /// Contiguous global placed-glyph indices produced by this operation.
    pub fn glyphs(&self) -> Range<usize> {
        self.glyphs.clone()
    }

    /// This chunk's canonical baseline origin in the artifact's local x axis.
    /// It is the sum of preceding chunk advances, not authored placement.
    pub fn origin_x(&self) -> f32 {
        self.origin_x
    }

    /// This independently shaped chunk's local advance.
    pub fn advance(&self) -> f32 {
        self.advance
    }
}

/// One positioned glyph: identity, placement, and its mapping back to the
/// source cluster that produced it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Glyph identifier in the resolved face's space.
    pub glyph_id: u16,
    /// Pen x of this glyph's origin in the artifact's canonical local space.
    /// For a later shaping chunk this includes that chunk's canonical origin.
    pub x: f32,
    /// Shaper-provided displacement from the pen in local x-right px.
    /// This does not alter the pen advance or cluster's logical cell.
    pub offset_x: f32,
    /// Shaper-provided displacement from the baseline in local y-down px.
    /// Font/shaper y-up placement is flipped exactly once on admission.
    pub offset_y: f32,
    /// This glyph's advance, in local px. The next pen position is
    /// `x + advance`; fractional advances are preserved.
    pub advance: f32,
    /// Index into [`ResolvedTextLayout::clusters`]. This is an explicit
    /// association, not a source-offset guess reconstructed by a consumer.
    pub cluster_index: usize,
    /// The same immutable source-run association as the glyph's cluster.
    /// Private so callers cannot construct a glyph whose two associations
    /// disagree; use [`Self::source_run_tag`] to inspect it.
    pub(crate) source_run_tag: SourceRunTag,
}

impl PlacedGlyph {
    pub const fn source_run_tag(&self) -> SourceRunTag {
        self.source_run_tag
    }
}

/// Receives one glyph's outline in the artifact's local y-down px space,
/// positioned at the glyph's pen origin plus shaping offset. The vocabulary
/// mirrors what shaping can produce (TrueType quadratics, CFF cubics);
/// consumers translate into their own path types.
pub trait OutlineSink {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32);
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);
    fn close(&mut self);
}

/// The immutable resolved text layout at oracle v4: one style run of
/// horizontal left-to-right text, already shaped in explicit chunks,
/// measured, and mapped.
///
/// Consumers project, they do not re-resolve: painting realizes the recorded
/// glyphs, measurement reads the recorded bounds, and any layout-affecting
/// input change means a *new* resolution — this value never mutates.
#[derive(Clone)]
pub struct ResolvedTextLayout {
    oracle_version: &'static str,
    source: String,
    source_runs: Vec<SourceRun>,
    face: ResolvedFace,
    font_size: f32,
    metrics: LineMetrics,
    shaping_chunks: Vec<ResolvedShapingChunk>,
    clusters: Vec<ShapingCluster>,
    glyphs: Vec<PlacedGlyph>,
    advance: f32,
    ink_bounds: Option<BoundsBox>,
    /// The exact bytes of the resolved face, retained so outline queries
    /// answer from the same identity that shaped — never from a second load.
    font_bytes: Arc<[u8]>,
}

impl ResolvedTextLayout {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source: String,
        source_runs: Vec<SourceRun>,
        face: ResolvedFace,
        font_size: f32,
        metrics: LineMetrics,
        shaping_chunks: Vec<ResolvedShapingChunk>,
        clusters: Vec<ShapingCluster>,
        glyphs: Vec<PlacedGlyph>,
        advance: f32,
        ink_bounds: Option<BoundsBox>,
        font_bytes: Arc<[u8]>,
    ) -> Self {
        Self {
            oracle_version: ORACLE_VERSION,
            source,
            source_runs,
            face,
            font_size,
            metrics,
            shaping_chunks,
            clusters,
            glyphs,
            advance,
            ink_bounds,
            font_bytes,
        }
    }

    /// The oracle version that produced this artifact.
    pub fn oracle_version(&self) -> &'static str {
        self.oracle_version
    }

    /// The exact source text this resolution shaped.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The exact validated source-run coverage supplied to resolution.
    pub fn source_runs(&self) -> &[SourceRun] {
        &self.source_runs
    }

    pub fn face(&self) -> &ResolvedFace {
        &self.face
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn metrics(&self) -> LineMetrics {
        self.metrics
    }

    /// Independently shaped chunks in source order. Their source, cluster,
    /// and glyph ranges are complete global partitions of this artifact.
    pub fn shaping_chunks(&self) -> &[ResolvedShapingChunk] {
        &self.shaping_chunks
    }

    /// Shaping clusters in logical order. Oracle v4's admitted LTR profile
    /// also makes this visual order; consumers must not assume that of a
    /// later bidi-capable version.
    pub fn clusters(&self) -> &[ShapingCluster] {
        &self.clusters
    }

    /// The placed glyphs in visual order — which at oracle v4 is also
    /// logical order, a fact of the LTR single-run profile rather than an
    /// assumption a consumer may carry to later versions.
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }

    /// Placed-glyph indices associated with `tag`, in this artifact's glyph
    /// order. A source run wholly inside a cluster may intentionally own no
    /// glyphs: the cluster belongs to the tag covering its first scalar.
    pub fn glyph_indices_for_source_run(
        &self,
        tag: SourceRunTag,
    ) -> impl Iterator<Item = usize> + '_ {
        self.glyphs
            .iter()
            .enumerate()
            .filter_map(move |(index, glyph)| (glyph.source_run_tag == tag).then_some(index))
    }

    /// The sum of all independently shaped chunk advances in local px.
    /// Per-chunk anchoring or placement is a projection of the corresponding
    /// [`ResolvedShapingChunk`] measurement, not a re-measurement.
    pub fn advance(&self) -> f32 {
        self.advance
    }

    /// Typographic extents of the canonical advance-concatenated chunks,
    /// crossed with ascent/descent. Present even where no ink is (an
    /// all-spaces chunk has logical extent).
    pub fn logical_bounds(&self) -> BoundsBox {
        BoundsBox {
            x: 0.0,
            y: -self.metrics.ascent,
            width: self.advance,
            height: self.metrics.ascent + self.metrics.descent,
        }
    }

    /// The tight union of glyph outline extents in the canonical
    /// advance-concatenated coordinate space, before any paint effect. `None`
    /// when the chunks draw nothing (spaces advance without ink).
    pub fn ink_bounds(&self) -> Option<BoundsBox> {
        self.ink_bounds
    }

    /// Stream the outline of the glyph at `index` in [`Self::glyphs`] into
    /// `sink`, positioned at its recorded pen origin plus shaping offset, in
    /// local y-down px.
    /// Returns `false` for a glyph with no outline (the space): it
    /// contributes advance, not geometry, and a consumer emits nothing.
    ///
    /// Index-based on purpose: glyph identifiers are meaningful only with
    /// this artifact's face, so only placements this resolution recorded can
    /// be realized — a glyph from another layout has no route in.
    ///
    /// # Panics
    ///
    /// If `index` is out of range, like any slice index.
    ///
    /// Answers from the retained resolved bytes — re-parsing the face per
    /// query is deliberate v1 policy: the reuse structure exists (the bytes
    /// and identity are pinned here), and a memo arrives only measured,
    /// with its `*_matches_fresh` law.
    pub fn outline(&self, index: usize, sink: &mut dyn OutlineSink) -> bool {
        let glyph = &self.glyphs[index];
        let Ok(face) = rustybuzz::ttf_parser::Face::parse(&self.font_bytes, self.face.face_index)
        else {
            // The face parsed at resolution; bytes are immutable since.
            unreachable!("resolved font bytes stopped parsing");
        };
        let scale = self.font_size / f32::from(self.face.units_per_em);
        stream_glyph_outline(&face, glyph, scale, sink)
    }
}

/// Stream one placed glyph through the single font-unit → artifact-space
/// mapping. Resolution uses this same route to derive tight ink bounds, so
/// bounds and realized paths cannot disagree about offsets or the y flip.
pub(crate) fn stream_glyph_outline(
    face: &rustybuzz::ttf_parser::Face<'_>,
    glyph: &PlacedGlyph,
    scale: f32,
    sink: &mut dyn OutlineSink,
) -> bool {
    let mut flip = FlipSink {
        sink,
        origin_x: glyph.x + glyph.offset_x,
        origin_y: glyph.offset_y,
        scale,
    };
    face.outline_glyph(rustybuzz::ttf_parser::GlyphId(glyph.glyph_id), &mut flip)
        .is_some()
}

impl std::fmt::Debug for ResolvedTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The retained font bytes are identity, not information — the key
        // already names them, so diagnostics never dump a font.
        f.debug_struct("ResolvedTextLayout")
            .field("oracle_version", &self.oracle_version)
            .field("source", &self.source)
            .field("source_runs", &self.source_runs)
            .field("face", &self.face)
            .field("font_size", &self.font_size)
            .field("metrics", &self.metrics)
            .field("shaping_chunks", &self.shaping_chunks)
            .field("clusters", &self.clusters)
            .field("glyphs", &self.glyphs)
            .field("advance", &self.advance)
            .field("ink_bounds", &self.ink_bounds)
            .finish_non_exhaustive()
    }
}

/// Maps y-up font units onto the artifact's y-down local px space:
/// `x' = pen + dx + x·s`, `y' = −dy − y·s`. The one home of the flip.
struct FlipSink<'a> {
    sink: &'a mut dyn OutlineSink,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
}

impl FlipSink<'_> {
    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin_x + x * self.scale,
            self.origin_y - y * self.scale,
        )
    }
}

impl rustybuzz::ttf_parser::OutlineBuilder for FlipSink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.sink.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.sink.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.map(x1, y1);
        let (x, y) = self.map(x, y);
        self.sink.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.map(x1, y1);
        let (x2, y2) = self.map(x2, y2);
        let (x, y) = self.map(x, y);
        self.sink.curve_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.sink.close();
    }
}
