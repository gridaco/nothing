//! Resolution: the one place attributed text becomes geometry.
//!
//! A pure function of its declared inputs. The same text, style, and
//! environment always produce the same artifact; nothing ambient — no
//! locale, no clock, no system font — can reach in.

use std::sync::Arc;

use crate::artifact::{
    BoundsBox, LineMetrics, OutlineSink, PlacedGlyph, ResolvedFace, ResolvedTextLayout,
    ShapingCluster, stream_glyph_outline,
};
use crate::environment::Environment;
use crate::source::{
    SourceRun, SourceRunCoverageError, SourceRunTag, source_run_tag_at, validate_source_runs,
};

/// The complete layout-affecting style of the one run oracle v3 admits.
#[derive(Clone, Debug)]
pub struct Style {
    /// Family name resolved against the environment's declared manifest —
    /// exact match, no fallback.
    pub family: String,
    /// Font size in local px. Finite and positive, validated.
    pub size: f32,
}

/// Attributed source at the v3 profile: one string under one complete
/// layout-affecting style, plus complete source-run coverage carrying opaque
/// caller tags.
///
/// The authoring layer owns document-level transformations — whitespace
/// collapsing, entity expansion — and hands resolution the post-transform
/// text; resolution never rewrites what it is given. Source-run boundaries
/// are metadata-only and do not split the one shaping operation.
#[derive(Clone, Debug)]
pub struct AttributedText {
    text: String,
    style: Style,
    source_runs: Vec<SourceRun>,
}

impl AttributedText {
    /// Construct attributed text with explicit complete source-run coverage.
    /// Validity is decided by [`resolve`], which returns a typed coverage
    /// error before doing font work or shaping.
    pub fn new(text: String, style: Style, source_runs: Vec<SourceRun>) -> Self {
        Self {
            text,
            style,
            source_runs,
        }
    }

    /// The explicit one-selection spelling. Non-empty text receives one run
    /// over the complete source; empty text receives the only valid empty
    /// coverage. No implicit tag exists.
    pub fn single_source_run(text: String, style: Style, tag: SourceRunTag) -> Self {
        let source_runs = if text.is_empty() {
            Vec::new()
        } else {
            vec![SourceRun::new(0..text.len(), tag)]
        };
        Self::new(text, style, source_runs)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn style(&self) -> &Style {
        &self.style
    }

    pub fn source_runs(&self) -> &[SourceRun] {
        &self.source_runs
    }
}

/// Why resolution refused. Typed, named, and carrying the byte position
/// where one exists — a consumer surfaces these at a stable node path.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolveError {
    /// Source-run coverage is not an exact ordered partition of the source
    /// on UTF-8 scalar boundaries.
    InvalidSourceRunCoverage(SourceRunCoverageError),
    /// `size` is not a finite positive number.
    InvalidFontSize { size: f32 },
    /// The declared family is not in the environment's manifest. There is
    /// no system fallback to fall into; the diagnostic names the family so
    /// the host can declare it.
    UnknownFamily { family: String },
    /// The environment's bytes for this family do not parse as a face.
    UnparseableFace { family: String },
    /// The face defines glyphs the v3 outline projection cannot honestly
    /// realize — color or bitmap glyph tables whose ink is not the outline.
    /// Streaming a monochrome placeholder for a color emoji is a silently
    /// wrong pixel, so the face refuses whole; color faces arrive as a new
    /// oracle version.
    UnsupportedFaceFormat { family: String },
    /// The character is outside oracle v3's admitted repertoire. The profile
    /// is an explicit admit-list — printable ASCII plus the canonical
    /// precomposed Latin-1 letters whose decomposition is one ASCII Latin
    /// base and one combining mark, plus the two explicitly admitted
    /// decomposed marks. Non-decomposable Latin-1 letters, every other mark,
    /// bidi controls, strong right-to-left letters, separators, and
    /// default-ignorables remain outside it. The profile widens by oracle
    /// version; until then the refusal names the byte.
    UnsupportedCharacter { byte_index: usize, character: char },
    /// An admitted mark is not the sole mark immediately following one ASCII
    /// Latin base. Leading, repeated, and non-letter-attached marks are a
    /// different shaping grammar and refuse before font behavior can decide.
    UnsupportedCombiningSequence { byte_index: usize, character: char },
    /// The resolved face has no glyph for this cluster. v3 permits no
    /// missing-glyph policy: no tofu, no substitution — a refusal naming
    /// the source position.
    MissingGlyph { byte_index: usize, character: char },
    /// Shaping produced cardinality outside oracle v3's direct or bounded
    /// combining clusters. Direct clusters remain one scalar/one glyph; an
    /// admitted base-plus-mark cluster may compose to one glyph or attach one
    /// mark glyph. Every other merge or split still refuses.
    UnsupportedClusterMapping {
        source_utf8_start: usize,
        source_utf8_end: usize,
        glyph_start: usize,
        glyph_end: usize,
    },
    /// Shaping produced placement the v3 profile has no semantics for — an
    /// offset outside its one admitted mark glyph, a vertical pen advance,
    /// a spacing mark, or a negative advance. Dropping any of them would be
    /// silently mispositioned ink, so the run refuses.
    UnsupportedShaping { byte_index: usize },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::InvalidSourceRunCoverage(error) => error.fmt(f),
            ResolveError::InvalidFontSize { size } => {
                write!(f, "font-size {size} is not a finite positive length")
            }
            ResolveError::UnknownFamily { family } => {
                write!(
                    f,
                    "font family \"{family}\" is not in the declared environment"
                )
            }
            ResolveError::UnparseableFace { family } => {
                write!(
                    f,
                    "declared bytes for font family \"{family}\" do not parse as a face"
                )
            }
            ResolveError::UnsupportedFaceFormat { family } => {
                write!(
                    f,
                    "font family \"{family}\" carries color or bitmap glyphs outside the outline profile"
                )
            }
            ResolveError::UnsupportedCharacter {
                byte_index,
                character,
            } => write!(
                f,
                "character {character:?} at byte {byte_index} is outside textlayout-v3's admitted printable-ASCII, canonical precomposed Latin-1, and bounded combining-mark repertoire"
            ),
            ResolveError::UnsupportedCombiningSequence {
                byte_index,
                character,
            } => write!(
                f,
                "combining mark {character:?} at byte {byte_index} is not the sole admitted mark after one ASCII Latin base"
            ),
            ResolveError::MissingGlyph {
                byte_index,
                character,
            } => write!(
                f,
                "no glyph for {character:?} at byte {byte_index} in the resolved face"
            ),
            ResolveError::UnsupportedClusterMapping {
                source_utf8_start,
                source_utf8_end,
                glyph_start,
                glyph_end,
            } => write!(
                f,
                "shaping cluster mapping source bytes {source_utf8_start}..{source_utf8_end} to glyphs {glyph_start}..{glyph_end} is outside textlayout-v3's direct-or-one-mark profile"
            ),
            ResolveError::UnsupportedShaping { byte_index } => write!(
                f,
                "shaping placed a glyph outside the one-run profile at byte {byte_index}"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Glyph tables whose ink is not the glyph outline. A face carrying any of
/// them refuses whole: v3's projection is outlines, and realizing a color
/// glyph as its monochrome fallback outline is a wrong pixel, not a policy.
const NON_OUTLINE_GLYPH_TABLES: [&[u8; 4]; 5] = [b"COLR", b"CBDT", b"CBLC", b"sbix", b"SVG "];

/// Resolve one attributed run against a declared environment into the
/// immutable artifact, or refuse with a name.
pub fn resolve(
    text: &AttributedText,
    env: &Environment,
) -> Result<ResolvedTextLayout, ResolveError> {
    // Source coverage is contract input, validated before font lookup or the
    // one shaping call. Nothing repairs a gap or guesses a missing tag.
    validate_source_runs(&text.text, &text.source_runs)
        .map_err(ResolveError::InvalidSourceRunCoverage)?;

    let size = text.style.size;
    if !size.is_finite() || size <= 0.0 {
        return Err(ResolveError::InvalidFontSize { size });
    }

    // The profile guard is the resolver's property, not an accident of any
    // font's coverage. Source spelling matters: v3 admits two marks only in
    // one exact base-plus-mark grammar and never normalizes authored text.
    validate_repertoire(&text.text)?;

    let resource = env
        .find(&text.style.family)
        .ok_or_else(|| ResolveError::UnknownFamily {
            family: text.style.family.clone(),
        })?;
    let face =
        rustybuzz::Face::from_slice(&resource.bytes, resource.face_index).ok_or_else(|| {
            ResolveError::UnparseableFace {
                family: text.style.family.clone(),
            }
        })?;

    // Raw table presence, deliberately independent of which tables the
    // parser was compiled to interpret.
    for tag in NON_OUTLINE_GLYPH_TABLES {
        if face
            .raw_face()
            .table(rustybuzz::ttf_parser::Tag::from_bytes(tag))
            .is_some()
        {
            return Err(ResolveError::UnsupportedFaceFormat {
                family: text.style.family.clone(),
            });
        }
    }

    // rustybuzz reports upem as an i32 (the HarfBuzz convention); the value
    // is a 16-bit font quantity, so a face outside that range is malformed.
    let units_per_em =
        u16::try_from(face.units_per_em()).map_err(|_| ResolveError::UnparseableFace {
            family: text.style.family.clone(),
        })?;
    let scale = size / f32::from(units_per_em);
    let metrics = LineMetrics {
        ascent: f32::from(face.ascender()) * scale,
        // ttf-parser reports descent as a negative distance; the artifact
        // states it as a positive reach below the baseline.
        descent: f32::from(-face.descender()) * scale,
    };

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(&text.text);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    // Pin the cluster policy as part of the oracle identity instead of
    // inheriting rustybuzz's current default. Level 0 is the behavior v0
    // shipped and the T3 probes measured; v3 keeps that fact explicit.
    buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
    let shaped = rustybuzz::shape(&face, &[], buffer);
    let clusters = admitted_clusters(&text.text, &text.source_runs, shaped.glyph_infos())?;

    let mut glyphs = Vec::with_capacity(shaped.len());
    let mut pen_x = 0.0f32;
    let mut cluster_index = 0usize;
    for (glyph_index, (info, pos)) in shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions().iter())
        .enumerate()
    {
        let byte_index = info.cluster as usize;
        while glyph_index >= clusters[cluster_index].glyphs().end {
            cluster_index += 1;
        }
        let cluster = &clusters[cluster_index];
        let cluster_glyphs = cluster.glyphs();
        let glyph_in_cluster = glyph_index - cluster_glyphs.start;
        let cluster_glyph_count = cluster_glyphs.len();
        let cluster_scalar_count = cluster.source_scalars().len();

        // Glyph 0 is .notdef: the face cannot render this cluster, and v3
        // has no permitted replacement policy.
        if info.glyph_id == 0 {
            let source_range = cluster.source_utf8();
            let (byte_index, character) = text.text[source_range.clone()]
                .char_indices()
                .find_map(|(relative, character)| {
                    face.glyph_index(character)
                        .is_none()
                        .then_some((source_range.start + relative, character))
                })
                .unwrap_or_else(|| {
                    (
                        source_range.start,
                        text.text[source_range]
                            .chars()
                            .next()
                            .unwrap_or(char::REPLACEMENT_CHARACTER),
                    )
                });
            return Err(ResolveError::MissingGlyph {
                byte_index,
                character,
            });
        }

        // A direct or composed cluster has one glyph at the pen. The only
        // richer placement v3 admits is the second glyph of one two-scalar,
        // two-glyph base-plus-mark cluster: it may carry x/y offsets but must
        // consume no advance of its own.
        let valid_placement = if cluster_glyph_count == 1 {
            pos.x_offset == 0 && pos.y_offset == 0 && pos.y_advance == 0 && pos.x_advance >= 0
        } else if cluster_scalar_count == 2 && cluster_glyph_count == 2 {
            if glyph_in_cluster == 0 {
                pos.x_offset == 0 && pos.y_offset == 0 && pos.y_advance == 0 && pos.x_advance >= 0
            } else {
                pos.x_advance == 0 && pos.y_advance == 0
            }
        } else {
            false
        };
        if !valid_placement {
            return Err(ResolveError::UnsupportedShaping { byte_index });
        }
        // Glyph ids originate from the face's 16-bit space; a wider value
        // is shaping output the profile cannot state.
        let glyph_id = u16::try_from(info.glyph_id)
            .map_err(|_| ResolveError::UnsupportedShaping { byte_index })?;
        let advance = pos.x_advance as f32 * scale;
        let offset_x = pos.x_offset as f32 * scale;
        // HarfBuzz/font coordinates are y-up; the artifact is y-down.
        let offset_y = -(pos.y_offset as f32) * scale;
        if !pen_x.is_finite()
            || !advance.is_finite()
            || !offset_x.is_finite()
            || !offset_y.is_finite()
        {
            return Err(ResolveError::UnsupportedShaping { byte_index });
        }
        let next_pen_x = pen_x + advance;
        if !next_pen_x.is_finite() {
            return Err(ResolveError::UnsupportedShaping { byte_index });
        }
        glyphs.push(PlacedGlyph {
            glyph_id,
            x: pen_x,
            offset_x,
            offset_y,
            advance,
            cluster_index,
            source_run_tag: cluster.source_run_tag(),
        });
        pen_x = next_pen_x;
    }

    let resolved_face = ResolvedFace {
        key: resource.key,
        face_index: resource.face_index,
        units_per_em,
    };
    let ink_bounds = ink_union(&face, &glyphs, scale);

    Ok(ResolvedTextLayout::new(
        text.text.clone(),
        text.source_runs.clone(),
        resolved_face,
        size,
        metrics,
        clusters,
        glyphs,
        pen_x,
        ink_bounds,
        Arc::clone(&resource.bytes),
    ))
}

/// Build the complete UTF-8/UTF-16/scalar/glyph association and enforce
/// oracle v3's direct-or-one-mark cluster profile. The shaper's monotone LTR
/// guarantee lets the next distinct cluster start close the current source
/// span; every boundary is nevertheless validated before it enters the
/// artifact.
fn admitted_clusters(
    source: &str,
    source_runs: &[SourceRun],
    infos: &[rustybuzz::GlyphInfo],
) -> Result<Vec<ShapingCluster>, ResolveError> {
    if source.is_empty() {
        if infos.is_empty() {
            return Ok(Vec::new());
        }
        return Err(ResolveError::UnsupportedShaping { byte_index: 0 });
    }
    if infos.is_empty() {
        return Err(ResolveError::UnsupportedClusterMapping {
            source_utf8_start: 0,
            source_utf8_end: source.len(),
            glyph_start: 0,
            glyph_end: 0,
        });
    }

    let mut clusters = Vec::new();
    let mut glyph_start = 0;
    let mut source_utf16_start = 0;
    let mut source_scalar_start = 0;
    while glyph_start < infos.len() {
        let source_utf8_start = infos[glyph_start].cluster as usize;
        let mut glyph_end = glyph_start + 1;
        while glyph_end < infos.len() && infos[glyph_end].cluster == infos[glyph_start].cluster {
            glyph_end += 1;
        }
        let source_utf8_end = if glyph_end < infos.len() {
            infos[glyph_end].cluster as usize
        } else {
            source.len()
        };

        let follows_previous = if let Some(previous) = clusters.last() {
            let previous: &ShapingCluster = previous;
            previous.source_utf8().end == source_utf8_start
        } else {
            source_utf8_start == 0
        };
        let valid_source_range = source_utf8_start < source_utf8_end
            && source_utf8_end <= source.len()
            && source.is_char_boundary(source_utf8_start)
            && source.is_char_boundary(source_utf8_end)
            && follows_previous;
        if !valid_source_range {
            return Err(ResolveError::UnsupportedShaping {
                byte_index: source_utf8_start.min(source.len()),
            });
        }

        let source_slice = &source[source_utf8_start..source_utf8_end];
        let source_characters: Vec<char> = source_slice.chars().collect();
        let glyph_count = glyph_end - glyph_start;
        let direct = source_characters.len() == 1
            && is_direct_character(source_characters[0])
            && glyph_count == 1;
        let one_mark = source_characters.len() == 2
            && source_characters[0].is_ascii_alphabetic()
            && is_admitted_mark(source_characters[1])
            && matches!(glyph_count, 1 | 2);
        if !direct && !one_mark {
            return Err(ResolveError::UnsupportedClusterMapping {
                source_utf8_start,
                source_utf8_end,
                glyph_start,
                glyph_end,
            });
        }
        let source_utf16_end = source_utf16_start + source_slice.encode_utf16().count();
        let source_scalar_end = source_scalar_start + source_characters.len();
        clusters.push(ShapingCluster::new(
            source_utf8_start..source_utf8_end,
            source_utf16_start..source_utf16_end,
            source_scalar_start..source_scalar_end,
            glyph_start..glyph_end,
            source_run_tag_at(source_runs, source_utf8_start),
        ));
        glyph_start = glyph_end;
        source_utf16_start = source_utf16_end;
        source_scalar_start = source_scalar_end;
    }
    Ok(clusters)
}

/// Validate oracle v3's complete source grammar before font selection.
fn validate_repertoire(source: &str) -> Result<(), ResolveError> {
    let mut previous = None;
    for (byte_index, character) in source.char_indices() {
        if is_direct_character(character) {
            previous = Some(character);
            continue;
        }
        if is_admitted_mark(character) {
            if previous.is_some_and(|base: char| base.is_ascii_alphabetic()) {
                previous = Some(character);
                continue;
            }
            return Err(ResolveError::UnsupportedCombiningSequence {
                byte_index,
                character,
            });
        }
        return Err(ResolveError::UnsupportedCharacter {
            byte_index,
            character,
        });
    }
    Ok(())
}

/// Oracle v3's directly admitted source scalars.
///
/// The Latin-1 ranges are deliberately discontinuous: every admitted member
/// has a canonical two-scalar decomposition to an ASCII Latin base plus one
/// combining mark. Letters without that decomposition (`Æ`, `Ð`, `Ø`, `Þ`,
/// `ß` and lowercase peers) are not smuggled in by block membership.
fn is_direct_character(character: char) -> bool {
    matches!(
        character,
        ' '..='~'
            | '\u{00C0}'..='\u{00C5}'
            | '\u{00C7}'..='\u{00CF}'
            | '\u{00D1}'..='\u{00D6}'
            | '\u{00D9}'..='\u{00DD}'
            | '\u{00E0}'..='\u{00E5}'
            | '\u{00E7}'..='\u{00EF}'
            | '\u{00F1}'..='\u{00F6}'
            | '\u{00F9}'..='\u{00FD}'
            | '\u{00FF}'
    )
}

/// The complete decomposed-mark vocabulary at v3. U+0301 proves the common
/// composed and attached branches; U+030B is the second measured class whose
/// Bungee attachment has nonzero displacement on both axes.
fn is_admitted_mark(character: char) -> bool {
    matches!(character, '\u{0301}' | '\u{030B}')
}

/// The tight union of the realized glyph outlines in local y-down px.
///
/// A font's stored glyph header box may include Bézier control points that
/// the curve never reaches. The artifact promises painted-path bounds, so
/// measure the exact mapped outline stream that consumers receive instead of
/// trusting that enclosing metadata.
fn ink_union(face: &rustybuzz::Face<'_>, glyphs: &[PlacedGlyph], scale: f32) -> Option<BoundsBox> {
    let mut union: Option<(f32, f32, f32, f32)> = None;
    for glyph in glyphs {
        let mut sink = TightBoundsSink::default();
        if !stream_glyph_outline(face.as_ref(), glyph, scale, &mut sink) {
            continue; // no ink: advance only
        }
        let Some((x0, y0, x1, y1)) = sink.bounds else {
            continue;
        };
        union = Some(match union {
            None => (x0, y0, x1, y1),
            Some((ux0, uy0, ux1, uy1)) => (ux0.min(x0), uy0.min(y0), ux1.max(x1), uy1.max(y1)),
        });
    }
    union.map(|(x0, y0, x1, y1)| BoundsBox {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    })
}

/// Tight bounds over an already mapped outline stream. This deliberately
/// implements the backend-neutral path mathematics locally: textlayout's
/// dependency perimeter remains Rustybuzz alone.
#[derive(Default)]
struct TightBoundsSink {
    current: Option<[f32; 2]>,
    bounds: Option<(f32, f32, f32, f32)>,
}

impl TightBoundsSink {
    fn include(&mut self, [x, y]: [f32; 2]) {
        self.bounds = Some(match self.bounds {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }

    fn include_quadratic(&mut self, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2]) {
        self.include(p0);
        self.include(p2);
        for axis in 0..2 {
            let denominator = p0[axis] - 2.0 * p1[axis] + p2[axis];
            if denominator == 0.0 {
                continue;
            }
            let t = (p0[axis] - p1[axis]) / denominator;
            if t > 0.0 && t < 1.0 {
                let mt = 1.0 - t;
                self.include([
                    mt * mt * p0[0] + 2.0 * mt * t * p1[0] + t * t * p2[0],
                    mt * mt * p0[1] + 2.0 * mt * t * p1[1] + t * t * p2[1],
                ]);
            }
        }
    }

    fn include_cubic(&mut self, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) {
        self.include(p0);
        self.include(p3);
        for axis in 0..2 {
            let c0 = -p0[axis] + 3.0 * p1[axis] - 3.0 * p2[axis] + p3[axis];
            let c1 = 3.0 * p0[axis] - 6.0 * p1[axis] + 3.0 * p2[axis];
            let c2 = -3.0 * p0[axis] + 3.0 * p1[axis];
            for t in solve_quadratic(3.0 * c0, 2.0 * c1, c2) {
                if (0.0..=1.0).contains(&t) {
                    let mt = 1.0 - t;
                    self.include([
                        mt * mt * mt * p0[0]
                            + 3.0 * mt * mt * t * p1[0]
                            + 3.0 * mt * t * t * p2[0]
                            + t * t * t * p3[0],
                        mt * mt * mt * p0[1]
                            + 3.0 * mt * mt * t * p1[1]
                            + 3.0 * mt * t * t * p2[1]
                            + t * t * t * p3[1],
                    ]);
                }
            }
        }
    }
}

impl OutlineSink for TightBoundsSink {
    fn move_to(&mut self, x: f32, y: f32) {
        let point = [x, y];
        self.include(point);
        self.current = Some(point);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let point = [x, y];
        self.include(point);
        self.current = Some(point);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let end = [x, y];
        if let Some(start) = self.current {
            self.include_quadratic(start, [x1, y1], end);
        } else {
            self.include(end);
        }
        self.current = Some(end);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let end = [x, y];
        if let Some(start) = self.current {
            self.include_cubic(start, [x1, y1], [x2, y2], end);
        } else {
            self.include(end);
        }
        self.current = Some(end);
    }

    fn close(&mut self) {}
}

fn solve_quadratic(a: f32, b: f32, c: f32) -> Vec<f32> {
    if a == 0.0 {
        return if b == 0.0 { Vec::new() } else { vec![-c / b] };
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        Vec::new()
    } else if discriminant == 0.0 {
        vec![-b / (2.0 * a)]
    } else {
        let root = discriminant.sqrt();
        vec![(-b + root) / (2.0 * a), (-b - root) / (2.0 * a)]
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::{OutlineSink, TightBoundsSink};

    #[test]
    fn quadratic_bounds_use_curve_extrema_not_control_points() {
        let mut sink = TightBoundsSink::default();
        sink.move_to(0.0, 0.0);
        sink.quad_to(1.0, 2.0, 2.0, 0.0);
        assert_eq!(sink.bounds, Some((0.0, 0.0, 2.0, 1.0)));
    }

    #[test]
    fn cubic_bounds_use_curve_extrema_not_control_points() {
        let mut sink = TightBoundsSink::default();
        sink.move_to(0.0, 0.0);
        sink.curve_to(0.0, 3.0, 3.0, 3.0, 3.0, 0.0);
        assert_eq!(sink.bounds, Some((0.0, 0.0, 3.0, 2.25)));
    }
}
