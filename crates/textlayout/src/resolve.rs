//! Resolution: the one place attributed text becomes geometry.
//!
//! A pure function of its declared inputs. The same text, style, and
//! environment always produce the same artifact; nothing ambient — no
//! locale, no clock, no system font — can reach in.

use std::sync::Arc;

use crate::artifact::{
    BoundsBox, LineMetrics, OutlineSink, PlacedGlyph, ResolvedFace, ResolvedShapingChunk,
    ResolvedTextLayout, ShapingCluster, stream_glyph_outline,
};
use crate::environment::{Environment, FamilyMatch, FontResource};
use crate::face_descriptor::StaticFaceDescriptor;
use crate::source::{
    ShapingChunk, ShapingChunkCoverageError, SourceRun, SourceRunCoverageError, SourceRunTag,
    source_run_tag_at, validate_shaping_chunks, validate_source_runs,
};

/// One family candidate in an ordered font request.
///
/// The caller has already parsed and classified the source syntax. A named
/// family is eligible for matching against the explicit environment; a
/// generic family is a policy boundary that this oracle deliberately cannot
/// map. Keeping the variants distinct prevents a quoted name such as `serif`
/// from becoming an implicit generic, or vice versa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontFamily {
    /// A family name eligible for lookup in the explicit environment.
    Named(String),
    /// An already-classified generic whose mapping would require policy not
    /// present in this oracle.
    Generic(String),
}

impl FontFamily {
    /// Construct one named-family candidate.
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    /// Construct one generic-family policy boundary.
    pub fn generic(name: impl Into<String>) -> Self {
        Self::Generic(name.into())
    }

    /// The exact candidate spelling supplied by the caller.
    pub fn name(&self) -> &str {
        match self {
            Self::Named(name) | Self::Generic(name) => name,
        }
    }
}

/// The complete layout-affecting style of the one run oracle v6 admits.
#[derive(Clone, Debug)]
pub struct Style {
    /// Ordered family candidates resolved against the declared environment.
    /// Traversal stops at the first named family with resources or the first
    /// generic boundary. An exact descriptor miss cannot fall through, and
    /// missing glyphs never restart this list.
    pub families: Vec<FontFamily>,
    /// Complete exact static descriptor requested within the reached family.
    pub face_descriptor: StaticFaceDescriptor,
    /// Font size in local px. Finite and positive, validated.
    pub size: f32,
}

/// Attributed source at the v6 profile: one string under one complete
/// layout-affecting style, complete source-run coverage carrying opaque caller
/// tags, and a complete shaping-chunk partition.
///
/// The authoring layer owns document-level transformations — whitespace
/// collapsing, entity expansion — and hands resolution the post-transform
/// text; resolution never rewrites what it is given. Source-run boundaries
/// are metadata-only. Shaping-chunk boundaries are explicit geometry input:
/// each chunk resolves independently, with no interaction across a boundary.
#[derive(Clone, Debug)]
pub struct AttributedText {
    text: String,
    style: Style,
    source_runs: Vec<SourceRun>,
    shaping_chunks: Vec<ShapingChunk>,
}

impl AttributedText {
    /// Construct attributed text with explicit complete source-run coverage
    /// and the default shaping partition: one whole-source chunk, or no chunk
    /// for empty source.
    ///
    /// Validity is decided by [`resolve`], which returns typed coverage errors
    /// before doing font work or shaping. Use [`Self::with_shaping_chunks`] to
    /// declare geometry-producing boundaries.
    pub fn new(text: String, style: Style, source_runs: Vec<SourceRun>) -> Self {
        let shaping_chunks = if text.is_empty() {
            Vec::new()
        } else {
            vec![ShapingChunk::new(0..text.len())]
        };
        Self {
            text,
            style,
            source_runs,
            shaping_chunks,
        }
    }

    /// Replace the default partition with explicit independently shaped
    /// chunks. Resolution validates complete ordered scalar-boundary coverage;
    /// this constructor never repairs or normalizes malformed ranges.
    pub fn with_shaping_chunks(mut self, shaping_chunks: Vec<ShapingChunk>) -> Self {
        self.shaping_chunks = shaping_chunks;
        self
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

    pub fn shaping_chunks(&self) -> &[ShapingChunk] {
        &self.shaping_chunks
    }
}

/// Why resolution refused. Typed, named, and carrying the byte position
/// where one exists — a consumer surfaces these at a stable node path.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolveError {
    /// Source-run coverage is not an exact ordered partition of the source
    /// on UTF-8 scalar boundaries.
    InvalidSourceRunCoverage(SourceRunCoverageError),
    /// Shaping chunks are not an exact ordered partition of the source on
    /// UTF-8 scalar boundaries.
    InvalidShapingChunkCoverage(ShapingChunkCoverageError),
    /// `size` is not a finite positive number.
    InvalidFontSize { size: f32 },
    /// An ordered family request must contain at least one candidate.
    EmptyFamilyList,
    /// No named candidate in the complete request matches a declared
    /// resource. There is no ambient fallback; the exact requested names are
    /// retained for diagnostics.
    NoMatchingFamily { families: Vec<String> },
    /// A generic candidate was reached before an exact face was selected.
    /// Generic mapping is host policy and is absent from this environment.
    UnmappedGenericFamily {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
    },
    /// The reached named family has resources, but none has the complete exact
    /// requested static descriptor. A reached family never masquerades as
    /// unavailable and falls through to a later candidate.
    NoExactFace {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
        requested: StaticFaceDescriptor,
        /// Number of resources inside the reached family.
        family_resources: usize,
    },
    /// More than one resource in the reached named family has the complete
    /// exact requested descriptor. Environment vector order cannot break the
    /// tie.
    AmbiguousFace {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
        requested: StaticFaceDescriptor,
        /// Number of resources carrying the exact requested tuple.
        matching_resources: usize,
    },
    /// The environment's bytes for this family do not parse as a face.
    UnparseableFace { family: String },
    /// The face defines glyphs the v6 outline projection cannot honestly
    /// realize — color or bitmap glyph tables whose ink is not the outline.
    /// Streaming a monochrome placeholder for a color emoji is a silently
    /// wrong pixel, so the face refuses whole; color faces arrive as a new
    /// oracle version.
    UnsupportedFaceFormat { family: String },
    /// The character is outside oracle v6's admitted repertoire. The profile
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
    /// The resolved face has no glyph for this cluster. v6 permits no
    /// missing-glyph policy: no tofu, no substitution — a refusal naming
    /// the source position.
    MissingGlyph { byte_index: usize, character: char },
    /// Shaping produced cardinality outside oracle v6's direct or bounded
    /// combining clusters. Direct clusters remain one scalar/one glyph; an
    /// admitted base-plus-mark cluster may compose to one glyph or attach one
    /// mark glyph. Every other merge or split still refuses.
    UnsupportedClusterMapping {
        source_utf8_start: usize,
        source_utf8_end: usize,
        glyph_start: usize,
        glyph_end: usize,
    },
    /// Shaping produced placement the v6 profile has no semantics for — an
    /// offset outside its one admitted mark glyph, a vertical pen advance,
    /// a spacing mark, or a negative advance. Dropping any of them would be
    /// silently mispositioned ink, so the run refuses.
    UnsupportedShaping { byte_index: usize },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::InvalidSourceRunCoverage(error) => error.fmt(f),
            ResolveError::InvalidShapingChunkCoverage(error) => error.fmt(f),
            ResolveError::InvalidFontSize { size } => {
                write!(f, "font-size {size} is not a finite positive length")
            }
            ResolveError::EmptyFamilyList => {
                write!(f, "font family list is empty")
            }
            ResolveError::NoMatchingFamily { families } => {
                write!(
                    f,
                    "no requested font family is in the declared environment: {}",
                    families
                        .iter()
                        .map(|family| format!("\"{family}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            ResolveError::UnmappedGenericFamily {
                candidate_index,
                family,
            } => write!(
                f,
                "generic font family \"{family}\" at candidate {candidate_index} has no declared mapping"
            ),
            ResolveError::NoExactFace {
                candidate_index,
                family,
                requested,
                family_resources,
            } => write!(
                f,
                "font family \"{family}\" at candidate {candidate_index} has {family_resources} declared resources but no exact static face for {requested:?}"
            ),
            ResolveError::AmbiguousFace {
                candidate_index,
                family,
                requested,
                matching_resources,
            } => write!(
                f,
                "font family \"{family}\" at candidate {candidate_index} is ambiguous: requested exact static face {requested:?} matches {matching_resources} declared resources"
            ),
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
                "character {character:?} at byte {byte_index} is outside textlayout-v6's admitted printable-ASCII, canonical precomposed Latin-1, and bounded combining-mark repertoire"
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
                "shaping cluster mapping source bytes {source_utf8_start}..{source_utf8_end} to glyphs {glyph_start}..{glyph_end} is outside textlayout-v6's direct-or-one-mark profile"
            ),
            ResolveError::UnsupportedShaping { byte_index } => write!(
                f,
                "shaping placed a glyph outside the horizontal chunk profile at byte {byte_index}"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Glyph tables whose ink is not the glyph outline. A face carrying any of
/// them refuses whole: v6's projection is outlines, and realizing a color
/// glyph as its monochrome fallback outline is a wrong pixel, not a policy.
const NON_OUTLINE_GLYPH_TABLES: [&[u8; 4]; 5] = [b"COLR", b"CBDT", b"CBLC", b"sbix", b"SVG "];

/// Resolve one attributed run against a declared environment into the
/// immutable artifact, or refuse with a name.
pub fn resolve(
    text: &AttributedText,
    env: &Environment,
) -> Result<ResolvedTextLayout, ResolveError> {
    // Both source partitions are contract input, validated before font lookup
    // or shaping. Nothing repairs a gap, guesses a missing tag, or silently
    // removes a geometry-producing boundary.
    validate_source_runs(&text.text, &text.source_runs)
        .map_err(ResolveError::InvalidSourceRunCoverage)?;
    validate_shaping_chunks(&text.text, &text.shaping_chunks)
        .map_err(ResolveError::InvalidShapingChunkCoverage)?;

    let size = text.style.size;
    if !size.is_finite() || size <= 0.0 {
        return Err(ResolveError::InvalidFontSize { size });
    }

    // The profile guard is the resolver's property, not an accident of any
    // font's coverage. Source spelling matters: v6 admits two marks only in
    // one exact base-plus-mark grammar and never normalizes authored text.
    validate_repertoire(&text.text)?;

    let resource = select_face(&text.style.families, text.style.face_descriptor, env)?;
    let face =
        rustybuzz::Face::from_slice(&resource.bytes, resource.face_index).ok_or_else(|| {
            ResolveError::UnparseableFace {
                family: resource.family.clone(),
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
                family: resource.family.clone(),
            });
        }
    }

    // rustybuzz reports upem as an i32 (the HarfBuzz convention); the value
    // is a 16-bit font quantity, so a face outside that range is malformed.
    let units_per_em =
        u16::try_from(face.units_per_em()).map_err(|_| ResolveError::UnparseableFace {
            family: resource.family.clone(),
        })?;
    let scale = size / f32::from(units_per_em);
    let metrics = LineMetrics {
        ascent: f32::from(face.ascender()) * scale,
        // ttf-parser reports descent as a negative distance; the artifact
        // states it as a positive reach below the baseline.
        descent: f32::from(-face.descender()) * scale,
    };

    let mut resolved_chunks = Vec::with_capacity(text.shaping_chunks.len());
    let mut clusters = Vec::new();
    let mut glyphs = Vec::new();
    let mut layout_pen_x = 0.0f32;
    let mut source_utf16_start = 0usize;
    let mut source_scalar_start = 0usize;

    for chunk in &text.shaping_chunks {
        let source_utf8 = chunk.source_utf8();
        let chunk_source = &text.text[source_utf8.clone()];
        let source_utf16_end = source_utf16_start + chunk_source.encode_utf16().count();
        let source_scalar_end = source_scalar_start + chunk_source.chars().count();
        let cluster_start = clusters.len();
        let glyph_start = glyphs.len();

        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(chunk_source);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        // Pin the cluster policy as part of the oracle identity instead of
        // inheriting rustybuzz's current default. Level 0 is the behavior v0
        // shipped and the T3 probes measured; v6 keeps that fact explicit.
        buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
        let shaped = rustybuzz::shape(&face, &[], buffer);
        clusters.extend(admitted_clusters(
            chunk_source,
            &text.source_runs,
            shaped.glyph_infos(),
            ChunkOffsets {
                source_utf8: source_utf8.start,
                source_utf16: source_utf16_start,
                source_scalars: source_scalar_start,
                glyphs: glyph_start,
            },
        )?);
        let cluster_end = clusters.len();

        let chunk_origin_x = layout_pen_x;
        let mut chunk_pen_x = 0.0f32;
        let mut cluster_index = cluster_start;
        for (local_glyph_index, (info, pos)) in shaped
            .glyph_infos()
            .iter()
            .zip(shaped.glyph_positions().iter())
            .enumerate()
        {
            let glyph_index = glyph_start + local_glyph_index;
            let byte_index = source_utf8.start + info.cluster as usize;
            while glyph_index >= clusters[cluster_index].glyphs().end {
                cluster_index += 1;
            }
            let cluster = &clusters[cluster_index];
            let cluster_glyphs = cluster.glyphs();
            let glyph_in_cluster = glyph_index - cluster_glyphs.start;
            let cluster_glyph_count = cluster_glyphs.len();
            let cluster_scalar_count = cluster.source_scalars().len();

            // Glyph 0 is .notdef: the face cannot render this cluster, and v6
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

            // A direct or composed cluster has one glyph at the pen. The
            // only richer placement v6 admits is the second glyph of one
            // two-scalar, two-glyph base-plus-mark cluster: it may carry x/y
            // offsets but must consume no advance of its own.
            let valid_placement = if cluster_glyph_count == 1 {
                pos.x_offset == 0 && pos.y_offset == 0 && pos.y_advance == 0 && pos.x_advance >= 0
            } else if cluster_scalar_count == 2 && cluster_glyph_count == 2 {
                if glyph_in_cluster == 0 {
                    pos.x_offset == 0
                        && pos.y_offset == 0
                        && pos.y_advance == 0
                        && pos.x_advance >= 0
                } else {
                    pos.x_advance == 0 && pos.y_advance == 0
                }
            } else {
                false
            };
            if !valid_placement {
                return Err(ResolveError::UnsupportedShaping { byte_index });
            }
            // Glyph ids originate from the face's 16-bit space; a wider
            // value is shaping output the profile cannot state.
            let glyph_id = u16::try_from(info.glyph_id)
                .map_err(|_| ResolveError::UnsupportedShaping { byte_index })?;
            let advance = pos.x_advance as f32 * scale;
            let offset_x = pos.x_offset as f32 * scale;
            // HarfBuzz/font coordinates are y-up; the artifact is y-down.
            let offset_y = -(pos.y_offset as f32) * scale;
            let glyph_x = chunk_origin_x + chunk_pen_x;
            if !glyph_x.is_finite()
                || !advance.is_finite()
                || !offset_x.is_finite()
                || !offset_y.is_finite()
            {
                return Err(ResolveError::UnsupportedShaping { byte_index });
            }
            let next_chunk_pen_x = chunk_pen_x + advance;
            if !next_chunk_pen_x.is_finite() {
                return Err(ResolveError::UnsupportedShaping { byte_index });
            }
            glyphs.push(PlacedGlyph {
                glyph_id,
                x: glyph_x,
                offset_x,
                offset_y,
                advance,
                cluster_index,
                source_run_tag: cluster.source_run_tag(),
            });
            chunk_pen_x = next_chunk_pen_x;
        }

        let next_layout_pen_x = chunk_origin_x + chunk_pen_x;
        if !next_layout_pen_x.is_finite() {
            return Err(ResolveError::UnsupportedShaping {
                byte_index: source_utf8.start,
            });
        }
        resolved_chunks.push(ResolvedShapingChunk::new(
            source_utf8,
            source_utf16_start..source_utf16_end,
            source_scalar_start..source_scalar_end,
            cluster_start..cluster_end,
            glyph_start..glyphs.len(),
            chunk_origin_x,
            chunk_pen_x,
        ));
        layout_pen_x = next_layout_pen_x;
        source_utf16_start = source_utf16_end;
        source_scalar_start = source_scalar_end;
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
        resolved_chunks,
        clusters,
        glyphs,
        layout_pen_x,
        ink_bounds,
        Arc::clone(&resource.bytes),
    ))
}

/// Resolve one complete ordered request before parsing or shaping a face.
///
/// A unique exact face in the first reached named family is final. An exact
/// miss, tuple tie, or reached generic is also final. Only an unavailable
/// named candidate falls through, and glyph coverage is deliberately not part
/// of this decision.
fn select_face<'a>(
    families: &[FontFamily],
    requested: StaticFaceDescriptor,
    env: &'a Environment,
) -> Result<&'a FontResource, ResolveError> {
    if families.is_empty() {
        return Err(ResolveError::EmptyFamilyList);
    }

    let mut requested_names = Vec::with_capacity(families.len());
    for (candidate_index, candidate) in families.iter().enumerate() {
        let FontFamily::Named(family) = candidate else {
            return Err(ResolveError::UnmappedGenericFamily {
                candidate_index,
                family: candidate.name().to_string(),
            });
        };
        requested_names.push(family.clone());

        match env.match_face(family, requested) {
            FamilyMatch::None => continue,
            FamilyMatch::NoExactFace { family_resources } => {
                return Err(ResolveError::NoExactFace {
                    candidate_index,
                    family: family.clone(),
                    requested,
                    family_resources,
                });
            }
            FamilyMatch::Unique(resource) => return Ok(resource),
            FamilyMatch::AmbiguousExact { matching_resources } => {
                return Err(ResolveError::AmbiguousFace {
                    candidate_index,
                    family: family.clone(),
                    requested,
                    matching_resources,
                });
            }
        }
    }

    Err(ResolveError::NoMatchingFamily {
        families: requested_names,
    })
}

/// The global coordinate bases for one chunk-local shaping result.
#[derive(Clone, Copy)]
struct ChunkOffsets {
    source_utf8: usize,
    source_utf16: usize,
    source_scalars: usize,
    glyphs: usize,
}

/// Build the complete UTF-8/UTF-16/scalar/glyph association and enforce
/// oracle v6's direct-or-one-mark cluster profile for one independently
/// shaped chunk. The shaper reports chunk-local byte and glyph positions;
/// this function promotes every coordinate into the artifact's global index
/// spaces before publishing it.
fn admitted_clusters(
    source: &str,
    source_runs: &[SourceRun],
    infos: &[rustybuzz::GlyphInfo],
    offsets: ChunkOffsets,
) -> Result<Vec<ShapingCluster>, ResolveError> {
    if source.is_empty() {
        if infos.is_empty() {
            return Ok(Vec::new());
        }
        return Err(ResolveError::UnsupportedShaping {
            byte_index: offsets.source_utf8,
        });
    }
    if infos.is_empty() {
        return Err(ResolveError::UnsupportedClusterMapping {
            source_utf8_start: offsets.source_utf8,
            source_utf8_end: offsets.source_utf8 + source.len(),
            glyph_start: offsets.glyphs,
            glyph_end: offsets.glyphs,
        });
    }

    let mut clusters = Vec::new();
    let mut local_glyph_start = 0;
    let mut local_source_utf16_start = 0;
    let mut local_source_scalar_start = 0;
    let mut expected_source_utf8_start = 0;
    while local_glyph_start < infos.len() {
        let local_source_utf8_start = infos[local_glyph_start].cluster as usize;
        let mut local_glyph_end = local_glyph_start + 1;
        while local_glyph_end < infos.len()
            && infos[local_glyph_end].cluster == infos[local_glyph_start].cluster
        {
            local_glyph_end += 1;
        }
        let local_source_utf8_end = if local_glyph_end < infos.len() {
            infos[local_glyph_end].cluster as usize
        } else {
            source.len()
        };

        let valid_source_range = local_source_utf8_start < local_source_utf8_end
            && local_source_utf8_end <= source.len()
            && source.is_char_boundary(local_source_utf8_start)
            && source.is_char_boundary(local_source_utf8_end)
            && local_source_utf8_start == expected_source_utf8_start;
        if !valid_source_range {
            return Err(ResolveError::UnsupportedShaping {
                byte_index: offsets.source_utf8 + local_source_utf8_start.min(source.len()),
            });
        }

        let source_slice = &source[local_source_utf8_start..local_source_utf8_end];
        let source_characters: Vec<char> = source_slice.chars().collect();
        let glyph_count = local_glyph_end - local_glyph_start;
        let direct = source_characters.len() == 1
            && is_direct_character(source_characters[0])
            && glyph_count == 1;
        let one_mark = source_characters.len() == 2
            && source_characters[0].is_ascii_alphabetic()
            && is_admitted_mark(source_characters[1])
            && matches!(glyph_count, 1 | 2);
        let source_utf8_start = offsets.source_utf8 + local_source_utf8_start;
        let source_utf8_end = offsets.source_utf8 + local_source_utf8_end;
        let glyph_start = offsets.glyphs + local_glyph_start;
        let glyph_end = offsets.glyphs + local_glyph_end;
        if !direct && !one_mark {
            return Err(ResolveError::UnsupportedClusterMapping {
                source_utf8_start,
                source_utf8_end,
                glyph_start,
                glyph_end,
            });
        }
        let local_source_utf16_end = local_source_utf16_start + source_slice.encode_utf16().count();
        let local_source_scalar_end = local_source_scalar_start + source_characters.len();
        clusters.push(ShapingCluster::new(
            source_utf8_start..source_utf8_end,
            offsets.source_utf16 + local_source_utf16_start
                ..offsets.source_utf16 + local_source_utf16_end,
            offsets.source_scalars + local_source_scalar_start
                ..offsets.source_scalars + local_source_scalar_end,
            glyph_start..glyph_end,
            source_run_tag_at(source_runs, source_utf8_start),
        ));
        local_glyph_start = local_glyph_end;
        local_source_utf16_start = local_source_utf16_end;
        local_source_scalar_start = local_source_scalar_end;
        expected_source_utf8_start = local_source_utf8_end;
    }
    Ok(clusters)
}

/// Validate oracle v6's complete source grammar before font selection.
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

/// Oracle v6's directly admitted source scalars.
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

/// The complete decomposed-mark vocabulary at v6. U+0301 proves the common
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
