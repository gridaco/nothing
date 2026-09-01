//! Resolution: the one place attributed text becomes geometry.
//!
//! A pure function of its declared inputs. The same text, style, and
//! environment always produce the same artifact; nothing ambient — no
//! locale, no clock, no system font — can reach in.

use std::sync::Arc;

use crate::artifact::{
    BoundsBox, LineMetrics, PlacedGlyph, ResolvedFace, ResolvedTextLayout, ShapingCluster,
};
use crate::environment::Environment;

/// The complete layout-affecting style of the one run oracle v1 admits.
#[derive(Clone, Debug)]
pub struct Style {
    /// Family name resolved against the environment's declared manifest —
    /// exact match, no fallback.
    pub family: String,
    /// Font size in local px. Finite and positive, validated.
    pub size: f32,
}

/// Attributed source at the v1 profile: one string under one complete style.
/// The authoring layer owns document-level transformations — whitespace
/// collapsing, entity expansion — and hands resolution the post-transform
/// text; resolution never rewrites what it is given.
#[derive(Clone, Debug)]
pub struct AttributedText {
    pub text: String,
    pub style: Style,
}

/// Why resolution refused. Typed, named, and carrying the byte position
/// where one exists — a consumer surfaces these at a stable node path.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolveError {
    /// `size` is not a finite positive number.
    InvalidFontSize { size: f32 },
    /// The declared family is not in the environment's manifest. There is
    /// no system fallback to fall into; the diagnostic names the family so
    /// the host can declare it.
    UnknownFamily { family: String },
    /// The environment's bytes for this family do not parse as a face.
    UnparseableFace { family: String },
    /// The face defines glyphs the v1 outline projection cannot honestly
    /// realize — color or bitmap glyph tables whose ink is not the outline.
    /// Streaming a monochrome placeholder for a color emoji is a silently
    /// wrong pixel, so the face refuses whole; color faces arrive as a new
    /// oracle version.
    UnsupportedFaceFormat { family: String },
    /// The character is outside oracle v1's admitted repertoire. The profile
    /// is an explicit admit-list — printable ASCII — because everything
    /// beyond it (a bidi control, a strong right-to-left letter, a line or
    /// paragraph separator, a default-ignorable the shaper would silently
    /// substitute) has semantics v1 does not implement, and shaping it
    /// anyway would be approximation. The profile widens by oracle version;
    /// until then the refusal names the byte.
    UnsupportedCharacter { byte_index: usize, character: char },
    /// The resolved face has no glyph for this cluster. v1 permits no
    /// missing-glyph policy: no tofu, no substitution — a refusal naming
    /// the source position.
    MissingGlyph { byte_index: usize, character: char },
    /// Shaping merged multiple source scalars into one cluster or split one
    /// scalar into multiple glyphs. Oracle v1 carries explicit source and
    /// glyph spans, but deliberately admits only one-to-one clusters until a
    /// later version also owns caret positions inside an inseparable glyph
    /// set. Painting while pretending the source mapping stayed one-to-one
    /// would poison every geometry-sensitive consumer.
    UnsupportedClusterMapping {
        source_utf8_start: usize,
        source_utf8_end: usize,
        glyph_start: usize,
        glyph_end: usize,
    },
    /// Shaping produced placement the v1 profile has no semantics for — a
    /// glyph offset (mark attachment, positioning feature), a vertical pen
    /// advance, or a negative advance. Dropping any of them would be
    /// silently mispositioned ink, so the run refuses; richer placement
    /// arrives as a new oracle version.
    UnsupportedShaping { byte_index: usize },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
                "character {character:?} at byte {byte_index} is outside the printable-ASCII v1 profile"
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
                "shaping cluster mapping source bytes {source_utf8_start}..{source_utf8_end} to glyphs {glyph_start}..{glyph_end} is outside textlayout-v1's one-source-scalar/one-glyph profile"
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
/// them refuses whole: v1's projection is outlines, and realizing a color
/// glyph as its monochrome fallback outline is a wrong pixel, not a policy.
const NON_OUTLINE_GLYPH_TABLES: [&[u8; 4]; 5] = [b"COLR", b"CBDT", b"CBLC", b"sbix", b"SVG "];

/// Resolve one attributed run against a declared environment into the
/// immutable artifact, or refuse with a name.
pub fn resolve(
    text: &AttributedText,
    env: &Environment,
) -> Result<ResolvedTextLayout, ResolveError> {
    let size = text.style.size;
    if !size.is_finite() || size <= 0.0 {
        return Err(ResolveError::InvalidFontSize { size });
    }

    // The profile guard is the resolver's property, not an accident of any
    // font's coverage: an explicit admit-list, checked before shaping so the
    // refusal names the source byte. Printable ASCII is the whole v1
    // repertoire — everything else (bidi controls and strong RTL, Zl/Zp
    // separators, default-ignorables the shaper would substitute unasked)
    // refuses here rather than shaping approximately.
    for (byte_index, character) in text.text.char_indices() {
        if !matches!(character, ' '..='~') {
            return Err(ResolveError::UnsupportedCharacter {
                byte_index,
                character,
            });
        }
    }

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
    // shipped and the T3 probes measured; v1 makes that fact explicit.
    buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
    let shaped = rustybuzz::shape(&face, &[], buffer);
    let clusters = one_to_one_clusters(&text.text, shaped.glyph_infos())?;

    let mut glyphs = Vec::with_capacity(shaped.len());
    let mut pen_x = 0.0f32;
    for (glyph_index, (info, pos)) in shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions().iter())
        .enumerate()
    {
        let byte_index = info.cluster as usize;
        // Glyph 0 is .notdef: the face cannot render this cluster, and v1
        // has no permitted replacement policy.
        if info.glyph_id == 0 {
            let character = text.text[byte_index..]
                .chars()
                .next()
                .unwrap_or(char::REPLACEMENT_CHARACTER);
            return Err(ResolveError::MissingGlyph {
                byte_index,
                character,
            });
        }
        // v1's profile has offsets, vertical advances, and backward pens in
        // no place. Dropping any of them would be silently mispositioned
        // ink; each refuses instead.
        if pos.x_offset != 0 || pos.y_offset != 0 || pos.y_advance != 0 || pos.x_advance < 0 {
            return Err(ResolveError::UnsupportedShaping { byte_index });
        }
        // Glyph ids originate from the face's 16-bit space; a wider value
        // is shaping output the profile cannot state.
        let glyph_id = u16::try_from(info.glyph_id)
            .map_err(|_| ResolveError::UnsupportedShaping { byte_index })?;
        glyphs.push(PlacedGlyph {
            glyph_id,
            x: pen_x,
            advance: pos.x_advance as f32 * scale,
            // `one_to_one_clusters` proved one cluster per glyph in this
            // same order before any placement entered the artifact.
            cluster_index: glyph_index,
        });
        pen_x += pos.x_advance as f32 * scale;
    }

    let resolved_face = ResolvedFace {
        key: resource.key,
        face_index: resource.face_index,
        units_per_em,
    };
    let ink_bounds = ink_union(&face, &glyphs, scale);

    Ok(ResolvedTextLayout::new(
        text.text.clone(),
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

/// Build the complete UTF-8/UTF-16/glyph association and enforce oracle v1's
/// direct-cluster profile. The shaper's monotone LTR guarantee lets the next
/// distinct cluster start close the current source span; every boundary is
/// nevertheless validated before it enters the artifact.
fn one_to_one_clusters(
    source: &str,
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
        if source_slice.chars().count() != 1 || glyph_end - glyph_start != 1 {
            return Err(ResolveError::UnsupportedClusterMapping {
                source_utf8_start,
                source_utf8_end,
                glyph_start,
                glyph_end,
            });
        }
        let source_utf16_end = source_utf16_start + source_slice.encode_utf16().count();
        clusters.push(ShapingCluster::new(
            source_utf8_start..source_utf8_end,
            source_utf16_start..source_utf16_end,
            glyph_start..glyph_end,
        ));
        glyph_start = glyph_end;
        source_utf16_start = source_utf16_end;
    }
    Ok(clusters)
}

/// The tight union of glyph bounding boxes in local y-down px. Uses the
/// face's own extents (curve extrema are the font's, already tight for the
/// outlines shaping selected).
fn ink_union(face: &rustybuzz::Face<'_>, glyphs: &[PlacedGlyph], scale: f32) -> Option<BoundsBox> {
    let mut union: Option<(f32, f32, f32, f32)> = None;
    for glyph in glyphs {
        let Some(rect) = face.glyph_bounding_box(rustybuzz::ttf_parser::GlyphId(glyph.glyph_id))
        else {
            continue; // no ink: advance only
        };
        // Font units are y-up: y_max maps to the box top (negative y).
        let x0 = glyph.x + f32::from(rect.x_min) * scale;
        let x1 = glyph.x + f32::from(rect.x_max) * scale;
        let y0 = -f32::from(rect.y_max) * scale;
        let y1 = -f32::from(rect.y_min) * scale;
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
