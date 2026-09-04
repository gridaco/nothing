//! Resolution: the one place attributed text becomes geometry.
//!
//! A pure function of its declared inputs. The same text, style, and
//! environment always produce the same artifact; nothing ambient — no
//! locale, no clock, no system font — can reach in.

use std::collections::HashMap;
use std::sync::Arc;

use crate::artifact::{
    BoundsBox, LineMetrics, OutlineSink, PlacedGlyph, ResolvedFace, ResolvedFaceRun,
    ResolvedShapingChunk, ResolvedTextLayout, ShapingCluster, stream_glyph_outline,
};
use crate::environment::{Environment, FamilyMatch, FontKey, FontResource};
use crate::face_descriptor::{FontStyle, StaticFaceDescriptor};
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

/// The complete layout-affecting style of the one run oracle v8 admits.
#[derive(Clone, Debug)]
pub struct Style {
    /// Ordered family candidates resolved against the declared environment.
    /// Each source cluster traverses this list independently. An unavailable
    /// name falls through; a matched face without the complete cluster moves
    /// to the next family. A reached generic, winning-tuple tie, or synthesis
    /// requirement on a face that would actually be used is terminal.
    pub families: Vec<FontFamily>,
    /// Complete exact static descriptor requested within the reached family.
    pub face_descriptor: StaticFaceDescriptor,
    /// Font size in local px. Finite and positive, validated.
    pub size: f32,
}

/// Attributed source at the v8 profile: one string under one complete
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
    /// A generic candidate was reached before a static face was selected.
    /// Generic mapping is host policy and is absent from this environment.
    UnmappedGenericFamily {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
    },
    /// More than one resource in the reached named family has the complete
    /// winning descriptor. Environment vector order cannot stand in for CSS
    /// source order and break the tie.
    AmbiguousFace {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
        requested: StaticFaceDescriptor,
        selected: StaticFaceDescriptor,
        /// Number of resources carrying the winning tuple.
        matching_resources: usize,
    },
    /// Static matching selected a face, but browser parity requires synthetic
    /// weight, style, or both. The outline contract has no platform-invariant
    /// realization for that operation, so resolution refuses before shaping
    /// instead of emitting backend-dependent pixels.
    SyntheticFaceRequired {
        /// Zero-based position in [`Style::families`].
        candidate_index: usize,
        family: String,
        requested: StaticFaceDescriptor,
        selected: StaticFaceDescriptor,
        synthetic_weight: bool,
        synthetic_style: bool,
    },
    /// The environment's bytes for this family do not parse as a face.
    UnparseableFace { family: String },
    /// The face defines glyphs the v8 outline projection cannot honestly
    /// realize — color or bitmap glyph tables whose ink is not the outline.
    /// Streaming a monochrome placeholder for a color emoji is a silently
    /// wrong pixel, so the face refuses whole; color faces arrive as a new
    /// oracle version.
    UnsupportedFaceFormat { family: String },
    /// The character is outside oracle v8's admitted repertoire. The profile
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
    /// No statically matched declared face can shape the complete cluster.
    /// Oracle v8 performs no installed/system fallback and never splits a
    /// base from its mark; the refusal names the first missing source scalar.
    MissingGlyph { byte_index: usize, character: char },
    /// Shaping produced cardinality outside oracle v8's direct or bounded
    /// combining clusters. Direct clusters remain one scalar/one glyph; an
    /// admitted base-plus-mark cluster may compose to one glyph or attach one
    /// mark glyph. Every other merge or split still refuses.
    UnsupportedClusterMapping {
        source_utf8_start: usize,
        source_utf8_end: usize,
        glyph_start: usize,
        glyph_end: usize,
    },
    /// Shaping produced placement the v8 profile has no semantics for — an
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
            ResolveError::AmbiguousFace {
                candidate_index,
                family,
                requested,
                selected,
                matching_resources,
            } => write!(
                f,
                "font family \"{family}\" at candidate {candidate_index} is ambiguous: request {requested:?} selects winning static face {selected:?}, which matches {matching_resources} declared resources"
            ),
            ResolveError::SyntheticFaceRequired {
                candidate_index,
                family,
                requested,
                selected,
                synthetic_weight,
                synthetic_style,
            } => {
                let kind = match (*synthetic_weight, *synthetic_style) {
                    (true, true) => "weight and style",
                    (true, false) => "weight",
                    (false, true) => "style",
                    (false, false) => unreachable!("a synthesis refusal names no synthesis"),
                };
                write!(
                    f,
                    "font family \"{family}\" at candidate {candidate_index} selects {selected:?} for request {requested:?}, which requires unsupported synthetic {kind}"
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
                "character {character:?} at byte {byte_index} is outside textlayout-v8's admitted printable-ASCII, canonical precomposed Latin-1, and bounded combining-mark repertoire"
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
                "no declared face can shape the complete cluster containing {character:?} at byte {byte_index}"
            ),
            ResolveError::UnsupportedClusterMapping {
                source_utf8_start,
                source_utf8_end,
                glyph_start,
                glyph_end,
            } => write!(
                f,
                "shaping cluster mapping source bytes {source_utf8_start}..{source_utf8_end} to glyphs {glyph_start}..{glyph_end} is outside textlayout-v8's direct-or-one-mark profile"
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
/// them refuses whole: v8's projection is outlines, and realizing a color
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
    // font's coverage. Source spelling matters: v8 admits two marks only in
    // one exact base-plus-mark grammar and never normalizes authored text.
    validate_repertoire(&text.text)?;

    // CSS keeps one "first available font" for metrics independently from
    // per-cluster glyph fallback. Chromium's SVG query cells retain those
    // primary metrics even when every outline comes from a later family.
    let primary = select_primary_face(&text.style.families, text.style.face_descriptor, env)?;
    let mut parsed_faces = ParsedFaceCache::default();
    let primary_parsed_face = parsed_faces.get_or_parse(primary.resource, &primary.family)?;
    let metrics = {
        let primary_face = parsed_faces.get(primary_parsed_face);
        let primary_scale = size / f32::from(primary_face.units_per_em);
        LineMetrics {
            ascent: f32::from(primary_face.face.ascender()) * primary_scale,
            // ttf-parser reports descent as a negative distance; the artifact
            // states it as a positive reach below the baseline.
            descent: f32::from(-primary_face.face.descender()) * primary_scale,
        }
    };

    // The first pass uses the primary face only as a grapheme-boundary
    // instrument. MonotoneGraphemes gives the exact cluster cuts v8 retains
    // admitted, including canonical composition and base-plus-mark grouping;
    // .notdef is intentionally allowed here because fallback owns it below.
    // This pass also preserves the old typed refusal for an authored shaping
    // chunk that begins inside a combining cluster.
    let mut source_clusters_by_chunk = Vec::with_capacity(text.shaping_chunks.len());
    let mut probe_glyph_start = 0usize;
    let mut probe_utf16_start = 0usize;
    let mut probe_scalar_start = 0usize;
    for chunk in &text.shaping_chunks {
        let source_utf8 = chunk.source_utf8();
        let chunk_source = &text.text[source_utf8.clone()];
        let shaped = shape_ltr(&parsed_faces.get(primary_parsed_face).face, chunk_source);
        let provisional = admitted_clusters(
            chunk_source,
            &text.source_runs,
            shaped.glyph_infos(),
            ChunkOffsets {
                source_utf8: source_utf8.start,
                source_utf16: probe_utf16_start,
                source_scalars: probe_scalar_start,
                glyphs: probe_glyph_start,
            },
        )?;
        source_clusters_by_chunk.push(
            provisional
                .iter()
                .map(ShapingCluster::source_utf8)
                .collect::<Vec<_>>(),
        );
        probe_glyph_start += shaped.glyph_infos().len();
        probe_utf16_start += chunk_source.encode_utf16().count();
        probe_scalar_start += chunk_source.chars().count();
    }

    let primary_resolved_face = ResolvedFace {
        key: primary.resource.key,
        face_index: primary.resource.face_index,
        units_per_em: parsed_faces.get(primary_parsed_face).units_per_em,
    };
    let mut faces = vec![primary_resolved_face];
    let mut font_bytes = vec![Arc::clone(&primary.resource.bytes)];
    let mut resolved_chunks = Vec::with_capacity(text.shaping_chunks.len());
    let mut face_runs = Vec::new();
    let mut clusters = Vec::new();
    let mut glyphs = Vec::new();
    let mut layout_pen_x = 0.0f32;
    for (chunk, source_cluster_ranges) in text
        .shaping_chunks
        .iter()
        .zip(source_clusters_by_chunk.iter())
    {
        let source_utf8 = chunk.source_utf8();
        let chunk_source = &text.text[source_utf8.clone()];
        let source_utf16_start = text.text[..source_utf8.start].encode_utf16().count();
        let source_utf16_end = source_utf16_start + chunk_source.encode_utf16().count();
        let source_scalar_start = text.text[..source_utf8.start].chars().count();
        let source_scalar_end = source_scalar_start + chunk_source.chars().count();
        let cluster_start = clusters.len();
        let glyph_start = glyphs.len();
        let chunk_origin_x = layout_pen_x;
        let chunk_face_run_start = face_runs.len();
        let mut planned_runs: Vec<PlannedFaceRun<'_>> = Vec::new();
        for cluster_source_utf8 in source_cluster_ranges {
            let candidate = select_face_for_cluster(
                &text.style.families,
                text.style.face_descriptor,
                env,
                &text.text,
                cluster_source_utf8.clone(),
                &mut parsed_faces,
                primary_parsed_face,
            )?;
            if let Some(last) = planned_runs.last_mut()
                && same_face_resource(last.candidate.resource, candidate.resource)
                && last.source_utf8.end == cluster_source_utf8.start
            {
                last.source_utf8.end = cluster_source_utf8.end;
            } else {
                planned_runs.push(PlannedFaceRun {
                    source_utf8: cluster_source_utf8.clone(),
                    candidate,
                });
            }
        }

        for planned in planned_runs {
            let parsed_face =
                parsed_faces.get_or_parse(planned.candidate.resource, &planned.candidate.family)?;
            let parsed = parsed_faces.get(parsed_face);
            let resolved_face_index = register_face(
                planned.candidate.resource,
                parsed.units_per_em,
                &mut faces,
                &mut font_bytes,
            );
            let run_source = &text.text[planned.source_utf8.clone()];
            let shaped = shape_ltr(&parsed.face, run_source);
            let run_cluster_start = clusters.len();
            let run_glyph_start = glyphs.len();
            let run_source_utf16_start = text.text[..planned.source_utf8.start]
                .encode_utf16()
                .count();
            let run_source_scalar_start = text.text[..planned.source_utf8.start].chars().count();
            clusters.extend(admitted_clusters(
                run_source,
                &text.source_runs,
                shaped.glyph_infos(),
                ChunkOffsets {
                    source_utf8: planned.source_utf8.start,
                    source_utf16: run_source_utf16_start,
                    source_scalars: run_source_scalar_start,
                    glyphs: run_glyph_start,
                },
            )?);
            let run_cluster_end = clusters.len();
            let run_origin_x = layout_pen_x;
            let mut run_pen_x = 0.0f32;
            let scale = size / f32::from(parsed.units_per_em);
            let mut cluster_index = run_cluster_start;
            for (local_glyph_index, (info, position)) in shaped
                .glyph_infos()
                .iter()
                .zip(shaped.glyph_positions().iter())
                .enumerate()
            {
                let glyph_index = run_glyph_start + local_glyph_index;
                let byte_index = planned.source_utf8.start + info.cluster as usize;
                while glyph_index >= clusters[cluster_index].glyphs().end {
                    cluster_index += 1;
                }
                let cluster = &clusters[cluster_index];
                if info.glyph_id == 0 {
                    return Err(missing_glyph_error(
                        &text.text,
                        cluster.source_utf8(),
                        &parsed_faces.get(primary_parsed_face).face,
                    ));
                }
                validate_glyph_placement(cluster, glyph_index, position, byte_index)?;
                let glyph_id = u16::try_from(info.glyph_id)
                    .map_err(|_| ResolveError::UnsupportedShaping { byte_index })?;
                let advance = position.x_advance as f32 * scale;
                let offset_x = position.x_offset as f32 * scale;
                // HarfBuzz/font coordinates are y-up; artifact y is down.
                let offset_y = -(position.y_offset as f32) * scale;
                let glyph_x = run_origin_x + run_pen_x;
                if !glyph_x.is_finite()
                    || !advance.is_finite()
                    || !offset_x.is_finite()
                    || !offset_y.is_finite()
                {
                    return Err(ResolveError::UnsupportedShaping { byte_index });
                }
                let next_run_pen_x = run_pen_x + advance;
                if !next_run_pen_x.is_finite() {
                    return Err(ResolveError::UnsupportedShaping { byte_index });
                }
                glyphs.push(PlacedGlyph {
                    glyph_id,
                    resolved_face_index,
                    x: glyph_x,
                    offset_x,
                    offset_y,
                    advance,
                    cluster_index,
                    source_run_tag: cluster.source_run_tag(),
                });
                run_pen_x = next_run_pen_x;
            }
            let next_layout_pen_x = run_origin_x + run_pen_x;
            if !next_layout_pen_x.is_finite() {
                return Err(ResolveError::UnsupportedShaping {
                    byte_index: planned.source_utf8.start,
                });
            }
            let run_source_utf16_end = run_source_utf16_start + run_source.encode_utf16().count();
            let run_source_scalar_end = run_source_scalar_start + run_source.chars().count();
            face_runs.push(ResolvedFaceRun::new(
                planned.source_utf8,
                run_source_utf16_start..run_source_utf16_end,
                run_source_scalar_start..run_source_scalar_end,
                run_cluster_start..run_cluster_end,
                run_glyph_start..glyphs.len(),
                resolved_face_index,
                run_origin_x,
                run_pen_x,
            ));
            layout_pen_x = next_layout_pen_x;
        }

        let cluster_end = clusters.len();
        resolved_chunks.push(ResolvedShapingChunk::new(
            source_utf8,
            source_utf16_start..source_utf16_end,
            source_scalar_start..source_scalar_end,
            cluster_start..cluster_end,
            glyph_start..glyphs.len(),
            chunk_face_run_start..face_runs.len(),
            chunk_origin_x,
            layout_pen_x - chunk_origin_x,
        ));
    }

    // Preserve v8's selection refusal for an empty source: no cluster exists
    // to trigger it, but the requested primary face still cannot truthfully be
    // described as realizable under this oracle.
    if text.text.is_empty() {
        reject_required_synthesis(&primary)?;
    }
    let ink_bounds = ink_union(&parsed_faces, &faces, &glyphs, size);

    Ok(ResolvedTextLayout::new(
        text.text.clone(),
        text.source_runs.clone(),
        faces,
        size,
        metrics,
        resolved_chunks,
        face_runs,
        clusters,
        glyphs,
        layout_pen_x,
        ink_bounds,
        font_bytes,
    ))
}

#[derive(Clone)]
struct SelectedCandidate<'a> {
    candidate_index: usize,
    family: String,
    resource: &'a FontResource,
    requested: StaticFaceDescriptor,
    selected: StaticFaceDescriptor,
    synthetic_weight: bool,
    synthetic_style: bool,
}

struct ParsedFace<'a> {
    face: rustybuzz::Face<'a>,
    units_per_em: u16,
}

#[derive(Default)]
struct ParsedFaceCache<'a> {
    by_identity: HashMap<(FontKey, u32), usize>,
    faces: Vec<ParsedFace<'a>>,
}

impl<'a> ParsedFaceCache<'a> {
    fn get_or_parse(
        &mut self,
        resource: &'a FontResource,
        family: &str,
    ) -> Result<usize, ResolveError> {
        let identity = (resource.key, resource.face_index);
        if let Some(index) = self.by_identity.get(&identity).copied() {
            return Ok(index);
        }
        let parsed = parse_face(resource, family)?;
        let index = self.faces.len();
        self.faces.push(parsed);
        self.by_identity.insert(identity, index);
        Ok(index)
    }

    fn get(&self, index: usize) -> &ParsedFace<'a> {
        &self.faces[index]
    }

    fn get_by_identity(&self, key: FontKey, face_index: u32) -> Option<&ParsedFace<'a>> {
        self.by_identity
            .get(&(key, face_index))
            .map(|index| &self.faces[*index])
    }
}

struct PlannedFaceRun<'a> {
    source_utf8: std::ops::Range<usize>,
    candidate: SelectedCandidate<'a>,
}

/// Resolve the first available font independently from glyph coverage. CSS
/// uses this identity for metrics; it is not permission to draw a missing
/// cluster with that face.
fn select_primary_face<'a>(
    families: &[FontFamily],
    requested: StaticFaceDescriptor,
    env: &'a Environment,
) -> Result<SelectedCandidate<'a>, ResolveError> {
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
            FamilyMatch::Unique { resource, selected } => {
                return Ok(selected_candidate(
                    candidate_index,
                    family,
                    resource,
                    requested,
                    selected,
                ));
            }
            FamilyMatch::AmbiguousWinner {
                selected,
                matching_resources,
            } => {
                return Err(ResolveError::AmbiguousFace {
                    candidate_index,
                    family: family.clone(),
                    requested,
                    selected,
                    matching_resources,
                });
            }
        }
    }

    Err(ResolveError::NoMatchingFamily {
        families: requested_names,
    })
}

/// Select the first statically matched face that shapes one complete admitted
/// cluster. Coverage is established by the same pinned shaper, not cmap
/// membership: canonical `A` + U+0301 may shape as A-acute even when U+0301
/// itself has no cmap entry. A failed face advances the family list, never the
/// descriptor choices inside that family.
fn select_face_for_cluster<'a>(
    families: &[FontFamily],
    requested: StaticFaceDescriptor,
    env: &'a Environment,
    source: &str,
    source_utf8: std::ops::Range<usize>,
    parsed_faces: &mut ParsedFaceCache<'a>,
    primary_parsed_face: usize,
) -> Result<SelectedCandidate<'a>, ResolveError> {
    for (candidate_index, family_candidate) in families.iter().enumerate() {
        let FontFamily::Named(family) = family_candidate else {
            return Err(ResolveError::UnmappedGenericFamily {
                candidate_index,
                family: family_candidate.name().to_string(),
            });
        };
        match env.match_face(family, requested) {
            FamilyMatch::None => continue,
            FamilyMatch::AmbiguousWinner {
                selected,
                matching_resources,
            } => {
                return Err(ResolveError::AmbiguousFace {
                    candidate_index,
                    family: family.clone(),
                    requested,
                    selected,
                    matching_resources,
                });
            }
            FamilyMatch::Unique { resource, selected } => {
                let candidate =
                    selected_candidate(candidate_index, family, resource, requested, selected);
                let parsed_face = parsed_faces.get_or_parse(resource, family)?;
                let parsed = parsed_faces.get(parsed_face);
                let shaped = shape_ltr(&parsed.face, &source[source_utf8.clone()]);
                let complete = !shaped.glyph_infos().is_empty()
                    && shaped.glyph_infos().iter().all(|info| info.glyph_id != 0);
                if !complete {
                    continue;
                }
                reject_required_synthesis(&candidate)?;
                return Ok(candidate);
            }
        }
    }
    Err(missing_glyph_error(
        source,
        source_utf8,
        &parsed_faces.get(primary_parsed_face).face,
    ))
}

fn selected_candidate<'a>(
    candidate_index: usize,
    family: &str,
    resource: &'a FontResource,
    requested: StaticFaceDescriptor,
    selected: StaticFaceDescriptor,
) -> SelectedCandidate<'a> {
    SelectedCandidate {
        candidate_index,
        family: family.to_string(),
        resource,
        requested,
        selected,
        synthetic_weight: requested.weight().value() >= 600 && selected.weight().value() < 600,
        synthetic_style: requested.style() == FontStyle::Italic
            && selected.style() == FontStyle::Normal,
    }
}

fn reject_required_synthesis(candidate: &SelectedCandidate<'_>) -> Result<(), ResolveError> {
    if !candidate.synthetic_weight && !candidate.synthetic_style {
        return Ok(());
    }
    Err(ResolveError::SyntheticFaceRequired {
        candidate_index: candidate.candidate_index,
        family: candidate.family.clone(),
        requested: candidate.requested,
        selected: candidate.selected,
        synthetic_weight: candidate.synthetic_weight,
        synthetic_style: candidate.synthetic_style,
    })
}

fn same_face_resource(left: &FontResource, right: &FontResource) -> bool {
    left.key == right.key && left.face_index == right.face_index
}

fn parse_face<'a>(
    resource: &'a FontResource,
    family: &str,
) -> Result<ParsedFace<'a>, ResolveError> {
    let face = rustybuzz::Face::from_slice(resource.bytes.as_ref(), resource.face_index)
        .ok_or_else(|| ResolveError::UnparseableFace {
            family: family.to_string(),
        })?;
    for tag in NON_OUTLINE_GLYPH_TABLES {
        if face
            .raw_face()
            .table(rustybuzz::ttf_parser::Tag::from_bytes(tag))
            .is_some()
        {
            return Err(ResolveError::UnsupportedFaceFormat {
                family: family.to_string(),
            });
        }
    }
    let units_per_em =
        u16::try_from(face.units_per_em()).map_err(|_| ResolveError::UnparseableFace {
            family: family.to_string(),
        })?;
    Ok(ParsedFace { face, units_per_em })
}

fn shape_ltr(face: &rustybuzz::Face<'_>, source: &str) -> rustybuzz::GlyphBuffer {
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(source);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    // Pin the cluster policy instead of inheriting rustybuzz's default.
    buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
    rustybuzz::shape(face, &[], buffer)
}

fn register_face(
    resource: &FontResource,
    units_per_em: u16,
    faces: &mut Vec<ResolvedFace>,
    font_bytes: &mut Vec<Arc<[u8]>>,
) -> usize {
    if let Some(index) = faces
        .iter()
        .position(|face| face.key == resource.key && face.face_index == resource.face_index)
    {
        return index;
    }
    let index = faces.len();
    faces.push(ResolvedFace {
        key: resource.key,
        face_index: resource.face_index,
        units_per_em,
    });
    font_bytes.push(Arc::clone(&resource.bytes));
    index
}

fn validate_glyph_placement(
    cluster: &ShapingCluster,
    glyph_index: usize,
    position: &rustybuzz::GlyphPosition,
    byte_index: usize,
) -> Result<(), ResolveError> {
    let cluster_glyphs = cluster.glyphs();
    let glyph_in_cluster = glyph_index - cluster_glyphs.start;
    let cluster_glyph_count = cluster_glyphs.len();
    let cluster_scalar_count = cluster.source_scalars().len();
    let valid = if cluster_glyph_count == 1 {
        position.x_offset == 0
            && position.y_offset == 0
            && position.y_advance == 0
            && position.x_advance >= 0
    } else if cluster_scalar_count == 2 && cluster_glyph_count == 2 {
        if glyph_in_cluster == 0 {
            position.x_offset == 0
                && position.y_offset == 0
                && position.y_advance == 0
                && position.x_advance >= 0
        } else {
            position.x_advance == 0 && position.y_advance == 0
        }
    } else {
        false
    };
    if valid {
        Ok(())
    } else {
        Err(ResolveError::UnsupportedShaping { byte_index })
    }
}

fn missing_glyph_error(
    source: &str,
    source_utf8: std::ops::Range<usize>,
    primary_face: &rustybuzz::Face<'_>,
) -> ResolveError {
    let (byte_index, character) = source[source_utf8.clone()]
        .char_indices()
        .find_map(|(relative, character)| {
            primary_face
                .glyph_index(character)
                .is_none()
                .then_some((source_utf8.start + relative, character))
        })
        .unwrap_or_else(|| {
            (
                source_utf8.start,
                source[source_utf8]
                    .chars()
                    .next()
                    .unwrap_or(char::REPLACEMENT_CHARACTER),
            )
        });
    ResolveError::MissingGlyph {
        byte_index,
        character,
    }
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
/// oracle v8's direct-or-one-mark cluster profile for one independently
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

/// Validate oracle v8's complete source grammar before font selection.
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

/// Oracle v8's directly admitted source scalars.
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

/// The complete decomposed-mark vocabulary at v8. U+0301 proves the common
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
fn ink_union(
    parsed_faces: &ParsedFaceCache<'_>,
    faces: &[ResolvedFace],
    glyphs: &[PlacedGlyph],
    font_size: f32,
) -> Option<BoundsBox> {
    let mut union: Option<(f32, f32, f32, f32)> = None;
    for glyph in glyphs {
        let resolved_face = &faces[glyph.resolved_face_index];
        let parsed = parsed_faces
            .get_by_identity(resolved_face.key, resolved_face.face_index)
            .expect("every retained resolved face is cached before artifact construction");
        let scale = font_size / f32::from(resolved_face.units_per_em);
        let mut sink = TightBoundsSink::default();
        if !stream_glyph_outline(parsed.face.as_ref(), glyph, scale, &mut sink) {
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
