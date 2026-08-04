//! Resolution: the one place attributed text becomes geometry.
//!
//! A pure function of its declared inputs. The same text, style, and
//! environment always produce the same artifact; nothing ambient — no
//! locale, no clock, no system font — can reach in.

use std::sync::Arc;

use crate::artifact::{BoundsBox, LineMetrics, PlacedGlyph, ResolvedFace, ResolvedTextLayout};
use crate::environment::Environment;

/// The complete layout-affecting style of the one run oracle v0 admits.
#[derive(Clone, Debug)]
pub struct Style {
    /// Family name resolved against the environment's declared manifest —
    /// exact match, no fallback.
    pub family: String,
    /// Font size in local px. Finite and positive, validated.
    pub size: f32,
}

/// Attributed source at the v0 profile: one string under one complete style.
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
    /// The face defines glyphs the v0 outline projection cannot honestly
    /// realize — color or bitmap glyph tables whose ink is not the outline.
    /// Streaming a monochrome placeholder for a color emoji is a silently
    /// wrong pixel, so the face refuses whole; color faces arrive as a new
    /// oracle version.
    UnsupportedFaceFormat { family: String },
    /// The character is outside oracle v0's admitted repertoire. The profile
    /// is an explicit admit-list — printable ASCII — because everything
    /// beyond it (a bidi control, a strong right-to-left letter, a line or
    /// paragraph separator, a default-ignorable the shaper would silently
    /// substitute) has semantics v0 does not implement, and shaping it
    /// anyway would be approximation. The profile widens by oracle version;
    /// until then the refusal names the byte.
    UnsupportedCharacter { byte_index: usize, character: char },
    /// The resolved face has no glyph for this cluster. v0 permits no
    /// missing-glyph policy: no tofu, no substitution — a refusal naming
    /// the source position.
    MissingGlyph { byte_index: usize, character: char },
    /// Shaping produced placement the v0 profile has no semantics for — a
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
                "character {character:?} at byte {byte_index} is outside the printable-ASCII v0 profile"
            ),
            ResolveError::MissingGlyph {
                byte_index,
                character,
            } => write!(
                f,
                "no glyph for {character:?} at byte {byte_index} in the resolved face"
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
/// them refuses whole: v0's projection is outlines, and realizing a color
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
    // refusal names the source byte. Printable ASCII is the whole v0
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
    let shaped = rustybuzz::shape(&face, &[], buffer);

    let mut glyphs = Vec::with_capacity(shaped.len());
    let mut pen_x = 0.0f32;
    for (info, pos) in shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions().iter())
    {
        let byte_index = info.cluster as usize;
        // Glyph 0 is .notdef: the face cannot render this cluster, and v0
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
        // v0's profile has offsets, vertical advances, and backward pens in
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
            cluster: info.cluster,
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
        glyphs,
        pen_x,
        ink_bounds,
        Arc::clone(&resource.bytes),
    ))
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
