//! The immutable resolved text layout — the one answer every
//! geometry-sensitive consumer projects from.
//!
//! Geometry is stated in the text node's **local logical space**: x grows
//! right, y grows down, the pen origin is `(0, 0)` on the alphabetic
//! baseline, and fractional values are preserved (device-pixel rounding is
//! not part of text resolution). Font units are y-up; the flip happens
//! exactly once, inside this crate, so no consumer ever re-derives it.

use std::sync::Arc;

use crate::ORACLE_VERSION;
use crate::environment::FontKey;

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
/// Oracle v0's metric policy is the face's `hhea` ascent/descent as the
/// parser reports them; line gap (leading) is a declared deferral, arriving
/// as a field when a consumer first needs line stacking. For the pinned gate
/// font every metric table agrees, which is why the gate can hold before the
/// policy question is forced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMetrics {
    pub ascent: f32,
    pub descent: f32,
}

/// One positioned glyph: identity, placement, and its mapping back to the
/// source bytes that produced it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Glyph identifier in the resolved face's space.
    pub glyph_id: u16,
    /// Pen x of this glyph's origin, in local px from the run origin.
    pub x: f32,
    /// This glyph's advance, in local px. The next pen position is
    /// `x + advance`; fractional advances are preserved.
    pub advance: f32,
    /// UTF-8 byte offset into the source text of the cluster this glyph
    /// renders — shaping's mapping, not a per-`char` guess.
    pub cluster: u32,
}

/// Receives one glyph's outline in the artifact's local y-down px space,
/// positioned at the glyph's pen origin on the baseline. The vocabulary
/// mirrors what shaping can produce (TrueType quadratics, CFF cubics);
/// consumers translate into their own path types.
pub trait OutlineSink {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32);
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);
    fn close(&mut self);
}

/// The immutable resolved text layout at oracle v0: one style run of
/// horizontal left-to-right text, already shaped, measured, and mapped.
///
/// Consumers project, they do not re-resolve: painting realizes the recorded
/// glyphs, measurement reads the recorded bounds, and any layout-affecting
/// input change means a *new* resolution — this value never mutates.
#[derive(Clone)]
pub struct ResolvedTextLayout {
    oracle_version: &'static str,
    source: String,
    face: ResolvedFace,
    font_size: f32,
    metrics: LineMetrics,
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
        face: ResolvedFace,
        font_size: f32,
        metrics: LineMetrics,
        glyphs: Vec<PlacedGlyph>,
        advance: f32,
        ink_bounds: Option<BoundsBox>,
        font_bytes: Arc<[u8]>,
    ) -> Self {
        Self {
            oracle_version: ORACLE_VERSION,
            source,
            face,
            font_size,
            metrics,
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

    pub fn face(&self) -> &ResolvedFace {
        &self.face
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn metrics(&self) -> LineMetrics {
        self.metrics
    }

    /// The placed glyphs in visual order — which at oracle v0 is also
    /// logical order, a fact of the LTR single-run profile rather than an
    /// assumption a consumer may carry to later versions.
    pub fn glyphs(&self) -> &[PlacedGlyph] {
        &self.glyphs
    }

    /// The run's total advance in local px. An anchor policy (SVG
    /// `text-anchor`) is a *projection* of this recorded measurement by the
    /// document layer that owns the anchor point — not a re-measurement.
    pub fn advance(&self) -> f32 {
        self.advance
    }

    /// Typographic extents: the advance run crossed with ascent/descent.
    /// Present even where no ink is (an all-spaces run has logical extent).
    pub fn logical_bounds(&self) -> BoundsBox {
        BoundsBox {
            x: 0.0,
            y: -self.metrics.ascent,
            width: self.advance,
            height: self.metrics.ascent + self.metrics.descent,
        }
    }

    /// The tight union of glyph outline extents, before any paint effect.
    /// `None` when the run draws nothing (spaces advance without ink).
    pub fn ink_bounds(&self) -> Option<BoundsBox> {
        self.ink_bounds
    }

    /// Stream the outline of the glyph at `index` in [`Self::glyphs`] into
    /// `sink`, positioned at its recorded pen origin, in local y-down px.
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
    /// query is deliberate v0 policy: the reuse structure exists (the bytes
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
        let mut flip = FlipSink {
            sink,
            pen_x: glyph.x,
            scale,
        };
        face.outline_glyph(rustybuzz::ttf_parser::GlyphId(glyph.glyph_id), &mut flip)
            .is_some()
    }
}

impl std::fmt::Debug for ResolvedTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The retained font bytes are identity, not information — the key
        // already names them, so diagnostics never dump a font.
        f.debug_struct("ResolvedTextLayout")
            .field("oracle_version", &self.oracle_version)
            .field("source", &self.source)
            .field("face", &self.face)
            .field("font_size", &self.font_size)
            .field("metrics", &self.metrics)
            .field("glyphs", &self.glyphs)
            .field("advance", &self.advance)
            .field("ink_bounds", &self.ink_bounds)
            .finish_non_exhaustive()
    }
}

/// Maps y-up font units onto the artifact's y-down local px space:
/// `x' = pen + x·s`, `y' = −y·s`. The one home of the flip.
struct FlipSink<'a> {
    sink: &'a mut dyn OutlineSink,
    pen_x: f32,
    scale: f32,
}

impl FlipSink<'_> {
    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        (self.pen_x + x * self.scale, -y * self.scale)
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
