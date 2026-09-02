//! SVG `<text>`: source semantics only.
//!
//! This module owns what the *document* says — the character data a `<text>`
//! element contributes after XML whitespace collapsing, its anchor, and the
//! numeric domain the gate admits. It owns no shaping: geometry comes from
//! [`textlayout`], the Web family's resolution oracle, which is the only
//! place a glyph, a font, or an advance exists. The lowering here turns that
//! resolved artifact into the resolved contract's existing path facts, so no
//! font identity crosses into `rframe`.

use rframe::{FillRule, PathCommand, PathData};
use textlayout::{
    AttributedText, Environment, OutlineSink, ResolveError, ResolvedTextLayout, Style,
};

/// SVG2 §11.1 `text-anchor`: where the resolved run sits relative to the
/// element's authored position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Anchor {
    Start,
    Middle,
    End,
}

impl Anchor {
    /// The keyword set exactly; an unadmitted spelling refuses by name
    /// rather than falling back to `start`. Chromium drops an invalid value
    /// to the inherited/initial anchor, which would place ink somewhere the
    /// author did not write — an over-refusal is the standing trade.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Anchor::Start),
            "middle" => Some(Anchor::Middle),
            "end" => Some(Anchor::End),
            _ => None,
        }
    }

    /// The run's start x, given its authored anchor point and total advance.
    fn start_x(self, x: f32, advance: f32) -> f32 {
        match self {
            Anchor::Start => x,
            Anchor::Middle => x - advance / 2.0,
            Anchor::End => x - advance,
        }
    }
}

/// XML/SVG whitespace collapsing under the default `xml:space="default"`.
///
/// Measured in Chromium: a tab, a newline, and a run of spaces each collapse
/// to exactly one advance, and leading and trailing whitespace is stripped —
/// so an indented, newline-wrapped `<text>` element renders the same run as
/// the one-line spelling. Carriage returns join the set per the SVG2 rule.
pub(crate) fn collapse_whitespace(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut pending_space = false;
    for character in raw.chars() {
        if matches!(character, ' ' | '\t' | '\n' | '\r') {
            // Only a space that precedes further content survives, which
            // strips the leading and trailing runs in the same pass.
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(character);
    }
    out
}

/// Why a `<text>` element is outside the admitted text slice.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TextError {
    /// Resolution refused: the environment, the repertoire, the face, or the
    /// shaping. Carries the oracle's own typed reason.
    Resolve(ResolveError),
    /// The resolved geometry leaves the admitted numeric domain, where every
    /// rasterizer's coverage is 0 or 1 per pixel and the byte-exact gate
    /// holds. Chromium snaps such geometry by a rasterizer-internal rule;
    /// codifying that rule is what this refusal declines to do.
    OutsideNumericDomain(String),
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::Resolve(error) => write!(f, "text resolution refused: {error}"),
            TextError::OutsideNumericDomain(reason) => {
                write!(
                    f,
                    "text geometry is outside the admitted numeric domain: {reason}"
                )
            }
        }
    }
}

/// Collects one resolved run's glyph outlines into one absolute command
/// stream, translated to the run's baseline origin.
struct PathSink {
    origin_x: f32,
    origin_y: f32,
    commands: Vec<PathCommand>,
}

impl OutlineSink for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::MoveTo {
            x: self.origin_x + x,
            y: self.origin_y + y,
        });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::LineTo {
            x: self.origin_x + x,
            y: self.origin_y + y,
        });
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.commands.push(PathCommand::QuadTo {
            x1: self.origin_x + x1,
            y1: self.origin_y + y1,
            x: self.origin_x + x,
            y: self.origin_y + y,
        });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.commands.push(PathCommand::CubicTo {
            x1: self.origin_x + x1,
            y1: self.origin_y + y1,
            x2: self.origin_x + x2,
            y2: self.origin_y + y2,
            x: self.origin_x + x,
            y: self.origin_y + y,
        });
    }

    fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }
}

/// The admitted numeric domain (the ratified text-oracle method): a glyph box
/// edge must land on an integer coordinate, so every rasterizer's per-pixel
/// coverage is 0 or 1 and bilevel and antialiased raster agree.
///
/// `font_size` divisible by 5 keeps a 0.8/0.2 em split integral; integer
/// authored and anchor-resolved positions keep the run's origin integral.
fn admit_numeric_domain(x: f32, y: f32, font_size: f32, start_x: f32) -> Result<(), TextError> {
    let integral = |value: f32| value.fract() == 0.0;
    if !integral(x) || !integral(y) {
        return Err(TextError::OutsideNumericDomain(format!(
            "position ({x}, {y}) is not integral"
        )));
    }
    if !integral(font_size) || font_size % 5.0 != 0.0 {
        return Err(TextError::OutsideNumericDomain(format!(
            "font-size {font_size} is not an integer multiple of 5"
        )));
    }
    if !integral(start_x) {
        return Err(TextError::OutsideNumericDomain(format!(
            "the anchor-resolved start x {start_x} is not integral"
        )));
    }
    Ok(())
}

/// Admit only resolved geometry that Chromium's SVG text-query projection
/// leaves unchanged.
///
/// Blink carries horizontal character boundaries through a 1/64 fixed-point
/// `LayoutUnit` and builds each queried cell by flooring its start and
/// ceiling its end. It exposes vertical character cells through fixed
/// integer ascent/descent metrics. If this artifact falls between either
/// grid, the DOM geometry Chromium reports is not the artifact we would
/// lower. That route refuses until a later oracle version deliberately owns
/// the normalization; the patrol never changes a glyph position.
fn admit_chromium_query_geometry(layout: &ResolvedTextLayout) -> Result<(), TextError> {
    const QUERY_GRID: f32 = 64.0;
    let on_query_grid = |value: f32| {
        let scaled = value * QUERY_GRID;
        value.is_finite() && scaled.is_finite() && scaled.fract() == 0.0
    };
    let integral = |value: f32| value.is_finite() && value.fract() == 0.0;

    let metrics = layout.metrics();
    if !integral(metrics.ascent) || !integral(metrics.descent) {
        return Err(TextError::OutsideNumericDomain(format!(
            "Chromium SVG text query metrics would round ascent/descent ({}, {}) to its integer fixed-metric grid",
            metrics.ascent, metrics.descent
        )));
    }
    for (cluster_index, cluster) in layout.clusters().iter().enumerate() {
        let source_utf8 = cluster.source_utf8();
        let source_utf16 = cluster.source_utf16();
        let source_scalars = cluster.source_scalars();
        let glyph_range = cluster.glyphs();
        let source_scalar_count = layout
            .source()
            .get(source_utf8.clone())
            .map(|source| source.chars().count());
        if source_scalar_count != Some(source_scalars.len())
            || source_utf16.len() != source_scalars.len()
            || !matches!(glyph_range.len(), 1 | 2)
        {
            return Err(TextError::OutsideNumericDomain(format!(
                "Chromium SVG text query cluster mapping is outside the scalar-addressable profile for source bytes {source_utf8:?}, UTF-16 units {source_utf16:?}, scalars {source_scalars:?}, and glyphs {glyph_range:?}"
            )));
        }
        let cluster_glyphs = layout.glyphs().get(glyph_range.clone()).ok_or_else(|| {
            TextError::OutsideNumericDomain(format!(
                "Chromium SVG text query cluster {cluster_index} names missing glyph range {glyph_range:?}"
            ))
        })?;
        if cluster_glyphs
            .iter()
            .any(|glyph| glyph.cluster_index != cluster_index)
        {
            return Err(TextError::OutsideNumericDomain(format!(
                "Chromium SVG text query glyph range {glyph_range:?} does not map wholly to cluster {cluster_index}"
            )));
        }
        let start = cluster_glyphs[0].x;
        let advance: f32 = cluster_glyphs.iter().map(|glyph| glyph.advance).sum();
        let end = start + advance;
        if !on_query_grid(start) || !on_query_grid(end) {
            return Err(TextError::OutsideNumericDomain(format!(
                "cluster {cluster_index} boundaries ({start}, {end}) do not lie on Chromium's 1/64 SVG text query grid"
            )));
        }
    }
    Ok(())
}

/// Resolve one `<text>` run and lower its glyphs to the resolved contract's
/// path vocabulary, in the element's local space.
///
/// `Ok(None)` is an admitted nothing: a run that resolves to no ink at all
/// (empty content, or whitespace that collapses away) is not a node, exactly
/// as a zero-extent rect is not.
pub(crate) fn resolve_text_path(
    text: &str,
    family: &str,
    font_size: f32,
    x: f32,
    y: f32,
    anchor: Anchor,
    fonts: &Environment,
) -> Result<Option<PathData>, TextError> {
    let attributed = AttributedText {
        text: text.to_string(),
        style: Style {
            family: family.to_string(),
            size: font_size,
        },
    };
    let layout = textlayout::resolve(&attributed, fonts).map_err(TextError::Resolve)?;

    let start_x = anchor.start_x(x, layout.advance());
    admit_numeric_domain(x, y, font_size, start_x)?;
    admit_chromium_query_geometry(&layout)?;

    let mut sink = PathSink {
        origin_x: start_x,
        origin_y: y,
        commands: Vec::new(),
    };
    for index in 0..layout.glyphs().len() {
        // A glyph with no outline contributes advance, not geometry.
        layout.outline(index, &mut sink);
    }
    if sink.commands.is_empty() {
        return Ok(None);
    }
    // SVG2 §11.4: text fills under the nonzero rule.
    PathData::new(sink.commands, FillRule::NonZero)
        .map(Some)
        .map_err(|error| TextError::OutsideNumericDomain(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_collapses_as_chromium_measured() {
        // Each case was measured in Chromium: the ink columns are identical
        // to the one-space spelling's.
        assert_eq!(collapse_whitespace("  X X  "), "X X");
        assert_eq!(collapse_whitespace("\n    X X\n  "), "X X");
        assert_eq!(collapse_whitespace("X\tX"), "X X");
        assert_eq!(collapse_whitespace("X   X"), "X X");
        assert_eq!(collapse_whitespace("X\r\nX"), "X X");
        assert_eq!(collapse_whitespace("   "), "");
        assert_eq!(collapse_whitespace(""), "");
    }

    #[test]
    fn anchor_keywords_are_exact() {
        assert_eq!(Anchor::parse("start"), Some(Anchor::Start));
        assert_eq!(Anchor::parse("middle"), Some(Anchor::Middle));
        assert_eq!(Anchor::parse("end"), Some(Anchor::End));
        // No case folding, no whitespace tolerance, no fallback.
        assert_eq!(Anchor::parse("Middle"), None);
        assert_eq!(Anchor::parse(" middle"), None);
        assert_eq!(Anchor::parse("centre"), None);
    }

    #[test]
    fn anchor_places_the_run_against_its_advance() {
        assert_eq!(Anchor::Start.start_x(50.0, 60.0), 50.0);
        assert_eq!(Anchor::Middle.start_x(50.0, 60.0), 20.0);
        assert_eq!(Anchor::End.start_x(90.0, 60.0), 30.0);
    }

    #[test]
    fn the_numeric_domain_refuses_what_chromium_would_snap() {
        assert!(admit_numeric_domain(25.0, 60.0, 50.0, 25.0).is_ok());
        assert!(admit_numeric_domain(25.5, 60.0, 50.0, 25.5).is_err());
        assert!(admit_numeric_domain(25.0, 60.5, 50.0, 25.0).is_err());
        // 48 is an integer but splits the em at 38.4/9.6.
        assert!(admit_numeric_domain(25.0, 60.0, 48.0, 25.0).is_err());
        // A middle anchor whose half-advance is fractional.
        assert!(admit_numeric_domain(50.0, 60.0, 15.0, 27.5).is_err());
    }
}
