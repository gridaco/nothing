//! # Resolved Text Layout
//!
//! The legacy engine's realization of the **Universal Shaped Text Layout**
//! RFD (`docs/wg/feat-paragraph/text-layout.md`): one immutable,
//! backend-neutral, inspectable artifact that carries the geometry a Skia
//! `Paragraph` resolved — lines, glyph runs, cluster mappings, logical and
//! ink bounds — stamped with the oracle version and the resolution-environment
//! identity that produced it.
//!
//! ## Anti-goal: this is not a private retained IR
//!
//! The artifact is **derived, immutable, and rebuilt from inputs**. It is
//! - never serialized into `.grida` (authored source stays authoritative;
//!   resolved geometry is derived output),
//! - never a mutable cache value (a cache may hold an [`ResolvedTextLayout`]
//!   behind `Arc`, but the value is replaced wholesale — never patched in
//!   place, e.g. by a late font swap), and
//! - never a second scene representation: it describes exactly one text
//!   resolution under one identity.
//!
//! ## The geometry surface, not the paint surface
//!
//! Skia paragraphs cannot be cloned or repainted after creation
//! (see `crate::cache::paragraph`), so painting keeps consuming the cached
//! `Paragraph` object. This artifact carries **no paints**: per the RFD,
//! geometry is fixed before rasterization and rasterization stays
//! backend-specific.
//!
//! ## Index spaces
//!
//! A grapheme cluster, a shaping cluster, a glyph, and a caret stop are
//! distinct concepts; this artifact does not collapse them into one index
//! space. All byte coordinates in this module address the **shaping text**
//! (the sequence handed to the shaper) in UTF-8 at Unicode-scalar boundaries.
//! When [`ResolvedTextLayout::transform`] is [`ShapingTransform::Identity`],
//! shaping-text coordinates are source-text coordinates.
//!
//! ## Declared gaps (oracle limitations — declared, never fabricated)
//!
//! Where the RFD demands something `skia_safe::textlayout::Paragraph` does
//! not expose, the artifact declares the absence instead of inventing a
//! value. Each gap is asserted by a conformance test
//! (`tests/text_resolved_conformance.rs`):
//!
//! 1. **Caret stops** ([`ResolvedTextLayout::caret_stops`] is `None`): Skia
//!    exposes only interactive caret queries
//!    (`get_glyph_position_at_coordinate`, `get_rects_for_range`) on the live
//!    `Paragraph`; it has no API that enumerates legal caret stops with
//!    affinity. Editing consumers keep querying the live paragraph until a
//!    later oracle version can enumerate stops.
//! 2. **Source mapping under text transforms**
//!    ([`ShapingTransform::Uniform`]/[`ShapingTransform::PerRun`]): when a
//!    `text-transform` policy rewrites the source before shaping (possibly
//!    changing byte lengths, e.g. `ß` → `SS`), Skia never sees the authored
//!    source, so no shaping-to-source byte mapping exists. The artifact
//!    records the policy and the shaping text; it does not fabricate source
//!    ranges.
//! 3. **Exact font-content identity** ([`ResolvedFontId::typeface_id`] is
//!    process-local): Skia's `Typeface` exposes family metadata and a
//!    process-local unique id, not a content digest of the font bytes. The
//!    identity is inspectable but not portable or durable across processes.
//! 4. **Environment identity is process-local** ([`EnvironmentId`]): derived
//!    from the `FontRepository` generation counter the paragraph cache
//!    already keys on — every environment mutation (font registration,
//!    fallback change) bumps it — not a portable manifest of exact font
//!    resources.
//! 5. **Per-glyph advances** ([`ResolvedGlyphRun::glyph_advances`] is
//!    `None`): Skia's paragraph visitor exposes glyph positions and a
//!    run-level advance, not per-glyph shaped advances. Positions are the
//!    authoritative per-glyph geometry.
//! 6. **Truncation-marker labeling** ([`ResolvedGlyphRun::synthetic`]): the
//!    visitor does not label the ellipsis run; it even aliases source byte
//!    offsets for it. The marker is identified as the final visual run of the
//!    final line when the oracle reports truncation, and its source mapping
//!    is withheld ([`ResolvedGlyphRun::cluster_starts`] is `None`) so a
//!    synthetic unit never masquerades as authored content.
//! 7. **Omitted-by-truncation ranges**
//!    ([`ResolvedTextLayout::omitted_by_truncation`]): Skia reports only the
//!    consumed prefix; the omission is published as the single suffix after
//!    it. This engine always resolves with an LTR base paragraph direction,
//!    which is the regime where Skia's end-of-line ellipsis makes the
//!    omission a logical suffix.
//! 8. **The final assigned text box** is not retained here: this artifact is
//!    produced below box assignment (the layout engine assigns the node box
//!    *from* these measurements). The RFD's resolution identity spans both
//!    layers; the constraint input and the width handed to the oracle are
//!    retained ([`ResolvedTextLayout::width_constraint`],
//!    [`ResolvedTextLayout::layout_width`]).
//!
//! ## Production
//!
//! The only producer is the paragraph cache
//! ([`crate::cache::paragraph::ParagraphCache`]) — the existing choke point
//! where every text node's Skia paragraph is built and measured.

use crate::cg::types::TextTransform;
use math2::rect::Rectangle;
use skia_safe::textlayout;
use std::ops::Range;

/// The `skia-safe` version this crate pins (single source for the oracle
/// version string). Must match the `skia-safe` dependency in `Cargo.toml`;
/// a conformance test asserts the two stay in lockstep.
macro_rules! pinned_skia_safe_version {
    () => {
        "0.93.1"
    };
}

/// The pinned `skia-safe` version realizing the text oracle.
pub const PINNED_SKIA_SAFE_VERSION: &str = pinned_skia_safe_version!();

/// The text-oracle version stamped into every resolved artifact.
///
/// Identifies the complete geometry-producing policy: Skia's `skparagraph`
/// module as bound by the pinned `skia-safe` version. Any dependency bump
/// that can alter a glyph choice, position, line, baseline, mapping, or
/// reported bound changes this string. "Latest" is not a valid durable
/// oracle version.
pub const SKPARAGRAPH_ORACLE_VERSION: &str =
    concat!("skparagraph@skia-", pinned_skia_safe_version!());

/// The resolution-environment identity a resolved artifact was produced
/// under.
///
/// Process-local (declared gap 4 in the module docs): the identity is the
/// `FontRepository` generation counter, which the repository bumps on every
/// environment mutation — font registration, embedded-font loading, fallback
/// configuration. A changed generation is a different environment and yields
/// a different resolved artifact; equal generations within one process imply
/// an identical font environment. It is not a portable manifest of exact
/// font-content identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvironmentId {
    /// `FontRepository::generation()` at resolution time.
    pub font_generation: usize,
}

impl EnvironmentId {
    /// Human-readable identity label, e.g. `fontrepository@gen-42`.
    pub fn label(&self) -> String {
        format!("fontrepository@gen-{}", self.font_generation)
    }
}

/// The explicit source-to-shaping-text transformation policy (RFD: a source
/// transformation is an explicit input, never ambient behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapingTransform {
    /// The shaping text is byte-for-byte the authored source text; all byte
    /// coordinates in the artifact are source coordinates.
    Identity,
    /// One `text-transform` policy was applied to the whole source before
    /// shaping. Byte coordinates address the transformed text; the mapping
    /// back to authored source is a declared absence (gap 2).
    Uniform(TextTransform),
    /// Per-run `text-transform` policies were applied (attributed text).
    /// Byte coordinates address the concatenated transformed text; the
    /// mapping back to authored source is a declared absence (gap 2).
    PerRun,
}

impl ShapingTransform {
    /// True when shaping-text byte coordinates are source-text coordinates.
    pub fn is_identity(&self) -> bool {
        matches!(self, ShapingTransform::Identity)
    }
}

/// Why a resolved line ended (RFD line construction: explicit, soft,
/// terminal, and truncated ends are distinct; a `hard_break` bit cannot
/// carry the distinction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBreakKind {
    /// The oracle chose a wrap opportunity before more source content.
    Soft,
    /// An authored line terminator ended the line; the terminator is part of
    /// the line's consumed [`ResolvedLine::source_range`].
    Explicit,
    /// The line reaches the end of the complete, untruncated source. Every
    /// non-truncated paragraph has exactly one terminal line.
    Terminal,
    /// The paragraph's truncation policy omitted the remaining source after
    /// this line.
    Truncated,
}

/// Resolved direction of a shaping cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedDirection {
    Ltr,
    Rtl,
}

/// Paragraph-level logical metrics, verbatim from the oracle's paragraph
/// readout. These are the numbers surrounding layout consumes; the
/// measurement path projects them without re-querying the live paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParagraphMetrics {
    /// Total height of the paragraph.
    pub height: f32,
    /// Maximum width used during layout.
    pub max_width: f32,
    /// Minimum intrinsic width (tightest possible width).
    pub min_intrinsic_width: f32,
    /// Maximum intrinsic width (widest possible width).
    pub max_intrinsic_width: f32,
    /// Y position of the alphabetic baseline.
    pub alphabetic_baseline: f32,
    /// Y position of the ideographic baseline.
    pub ideographic_baseline: f32,
    /// Width of the longest line.
    pub longest_line: f32,
    /// The oracle's own line count. May differ from
    /// [`ResolvedTextLayout::lines`]`.len()`: the oracle reports zero lines
    /// for empty source, while the RFD requires one empty terminal line.
    pub line_count: usize,
    /// Whether the paragraph exceeded its maximum line limit (truncation
    /// occurred).
    pub did_exceed_max_lines: bool,
}

/// One resolved line, in block order.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLine {
    /// Zero-based index in block order.
    pub index: usize,
    /// The complete shaping-text UTF-8 range this line consumed, including
    /// any consumed line terminator. Ranges of consecutive lines are
    /// contiguous; under truncation the final line's range ends where
    /// consumption stopped (see
    /// [`ResolvedTextLayout::omitted_by_truncation`]).
    pub source_range: Range<usize>,
    /// Why the line ended.
    pub break_kind: LineBreakKind,
    /// Baseline y position from the top of the paragraph.
    pub baseline: f32,
    /// Ascent above the baseline (positive).
    pub ascent: f32,
    /// Descent below the baseline (positive).
    pub descent: f32,
    /// Left edge of the line's advance extent.
    pub left: f32,
    /// Advance width of the line.
    pub width: f32,
    /// Logical line box: `[left, baseline - ascent] × [width, ascent +
    /// descent]`, in paragraph-local coordinates. Includes advance space
    /// without glyph ink.
    pub logical_bounds: Rectangle,
    /// Union of glyph ink on this line; `None` when the line draws nothing
    /// (empty line, whitespace-only coverage) or when the oracle supplied no
    /// per-glyph ink for its runs.
    pub ink_bounds: Option<Rectangle>,
    /// True when this line was synthesized by the realization to satisfy the
    /// RFD's terminal-line requirement where the oracle under-reports (empty
    /// source resolves to one empty terminal line). Its metrics derive from
    /// the oracle's paragraph-level readout, not invented constants.
    pub synthesized: bool,
}

/// The resolved identity of the exact face that shaped a run.
///
/// Post-fallback: this names the face the oracle actually selected, which
/// may differ from the requested family. Glyph identifiers in
/// [`ResolvedGlyphRun::glyph_ids`] are meaningful only together with this
/// identity. `typeface_id` is process-local (declared gap 3): Skia exposes
/// no font-content digest, so the identity is inspectable but not portable.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFontId {
    /// Resolved family name of the selected typeface.
    pub family: String,
    /// Effective font size in logical units.
    pub size: f32,
    /// Resolved weight (CSS-scale, e.g. 400).
    pub weight: i32,
    /// Whether the resolved face is italic/oblique.
    pub italic: bool,
    /// Skia's process-local typeface unique id. Not portable, not durable.
    pub typeface_id: u32,
}

/// A run of positioned glyphs shaped with one exact face, in paint (visual)
/// order within its line.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGlyphRun {
    /// Index into [`ResolvedTextLayout::lines`].
    pub line: usize,
    /// The exact resolved face for every glyph in this run.
    pub font: ResolvedFontId,
    /// True for a layout-owned synthetic unit (the truncation marker). A
    /// synthetic run has no authored source mapping
    /// ([`Self::cluster_starts`] is `None`) and must never be treated as
    /// authored content by copy, editing, or accessibility traversal.
    pub synthetic: bool,
    /// Glyph identifiers in paint order, meaningful only with [`Self::font`].
    pub glyph_ids: Vec<u16>,
    /// Per-glyph pen positions in paragraph-local coordinates (fractional;
    /// no device rounding), paint order. `positions[i]` pairs with
    /// `glyph_ids[i]`.
    pub positions: Vec<[f32; 2]>,
    /// Per-glyph tight ink bounds in paragraph-local coordinates, paint
    /// order, when the oracle supplied them.
    pub ink_bounds: Option<Vec<Rectangle>>,
    /// Per-glyph shaping-text UTF-8 cluster start offsets, paint order.
    /// Glyphs sharing a start belong to one shaping cluster. `None` for a
    /// synthetic run (gap 6): the oracle aliases source offsets for the
    /// truncation marker, and a synthetic unit must not claim source
    /// coverage.
    pub cluster_starts: Option<Vec<usize>>,
    /// Advance width of the whole run.
    pub advance_width: f32,
    /// Declared absence (gap 5): Skia's visitor does not expose per-glyph
    /// shaped advances. Always `None` under this oracle; positions are the
    /// authoritative per-glyph geometry.
    pub glyph_advances: Option<Vec<f32>>,
}

/// One shaping cluster: a unit emitted by shaping that relates a
/// shaping-text range to zero or more positioned glyphs.
///
/// Not a grapheme cluster, not a glyph, not a caret stop — the index spaces
/// are distinct and deliberately not collapsed.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCluster {
    /// The cluster's shaping-text UTF-8 range.
    pub range: Range<usize>,
    /// Index into [`ResolvedTextLayout::lines`].
    pub line: usize,
    /// `(run index, glyph index range within the run)` of the glyphs this
    /// cluster produced, or `None` for a zero-glyph cluster (a line
    /// terminator, or trailing whitespace the oracle trimmed from visual
    /// runs at a soft wrap) — such source still maps to a line and retains
    /// its advance geometry.
    pub glyph_span: Option<(usize, Range<usize>)>,
    /// Resolved direction of this cluster.
    pub direction: ResolvedDirection,
    /// The cluster's logical advance box in paragraph-local coordinates
    /// (its share of the line's advance extent — not glyph ink).
    pub advance_bounds: Rectangle,
}

/// Caret affinity at a bidirectional or line boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretAffinity {
    Upstream,
    Downstream,
}

/// A legal editing boundary with a visual position (RFD vocabulary). Never
/// populated under this oracle — see [`ResolvedTextLayout::caret_stops`]
/// (declared gap 1).
#[derive(Debug, Clone, PartialEq)]
pub struct CaretStop {
    /// Shaping-text UTF-8 offset of the boundary.
    pub byte_offset: usize,
    /// Index into [`ResolvedTextLayout::lines`].
    pub line: usize,
    /// Visual x position in paragraph-local coordinates.
    pub x: f32,
    /// Affinity of the stop.
    pub affinity: CaretAffinity,
}

/// The immutable, self-identifying result of resolving one text input under
/// one identity. See the module docs for the contract and the declared gaps.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTextLayout {
    /// The versioned oracle that produced this result
    /// ([`SKPARAGRAPH_ORACLE_VERSION`]).
    pub oracle_version: &'static str,
    /// The resolution-environment identity this result was produced under.
    pub environment: EnvironmentId,
    /// The exact inline constraint input; `None` means the inline axis was
    /// unconstrained and the oracle laid out at its intrinsic width.
    pub width_constraint: Option<f32>,
    /// The width actually handed to the oracle's layout (the constraint, or
    /// the measured intrinsic width when unconstrained).
    pub layout_width: f32,
    /// The explicit source-to-shaping transformation policy.
    pub transform: ShapingTransform,
    /// The exact text handed to the shaper. All byte coordinates in this
    /// artifact address this string.
    pub shaping_text: String,
    /// Paragraph-level logical metrics, verbatim from the oracle.
    pub metrics: ParagraphMetrics,
    /// Lines in block order.
    pub lines: Vec<ResolvedLine>,
    /// Glyph runs in paint order (line by line, visual order within a line).
    pub glyph_runs: Vec<ResolvedGlyphRun>,
    /// Shaping clusters in logical (source) order.
    pub clusters: Vec<ResolvedCluster>,
    /// Union of the lines' logical boxes: layout extent including advance
    /// space without ink.
    pub logical_bounds: Rectangle,
    /// Union of glyph ink across all runs; `None` when nothing draws ink.
    /// Base drawing extents before node paints, strokes, or effects.
    pub ink_bounds: Option<Rectangle>,
    /// Glyphs the oracle could not resolve under the font environment
    /// (rendered as tofu by this permissive engine — reported, never
    /// silent). `None` when the oracle did not report a count.
    pub unresolved_glyphs: Option<usize>,
    /// The shaping-text suffix omitted by the truncation policy, when
    /// truncation occurred (gap 7: published as the single post-visible
    /// suffix, which is Skia's omission model under this engine's LTR base
    /// direction).
    pub omitted_by_truncation: Option<Range<usize>>,
    /// Consumed shaping-text ranges the oracle declined to map to any
    /// cluster. Expected empty; a non-empty value is an honest report of an
    /// oracle mapping hole, never silently dropped coverage.
    pub unmapped_ranges: Vec<Range<usize>>,
    /// Declared absence (gap 1): Skia does not enumerate legal caret stops
    /// with affinity. Always `None` under this oracle; editing consumers
    /// keep querying the live paragraph.
    pub caret_stops: Option<Vec<CaretStop>>,
}

impl ResolvedTextLayout {
    /// The line containing the given shaping-text byte offset, by consumed
    /// range. Offsets at `shaping_text.len()` map to the last line.
    pub fn line_at_byte(&self, offset: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|l| l.source_range.contains(&offset))
            .or_else(|| {
                (offset == self.shaping_text.len() && !self.lines.is_empty())
                    .then_some(self.lines.len() - 1)
            })
    }
}

// ---------------------------------------------------------------------------
// Extraction from a laid-out Skia paragraph
// ---------------------------------------------------------------------------

/// Raw per-run capture from `Paragraph::visit` / `Paragraph::extended_visit`.
struct VisitRun {
    line: usize,
    font: ResolvedFontId,
    advance_width: f32,
    glyph_ids: Vec<u16>,
    /// Paragraph-local absolute pen positions (`origin + position`).
    positions: Vec<[f32; 2]>,
    /// Paragraph-absolute UTF-8 cluster start per glyph, plus the visitor's
    /// end sentinel (valid only for LTR runs).
    utf8_starts: Vec<u32>,
    /// Paragraph-local absolute per-glyph ink bounds, when captured.
    ink_bounds: Option<Vec<Rectangle>>,
}

/// Extract a [`ResolvedTextLayout`] from a laid-out Skia paragraph.
///
/// `paragraph` must already be laid out at `layout_width`. The extraction
/// only reads; it never re-layouts, so the caller's paragraph keeps its
/// state for painting.
pub(crate) fn resolve_from_paragraph(
    paragraph: &mut textlayout::Paragraph,
    shaping_text: &str,
    transform: ShapingTransform,
    width_constraint: Option<f32>,
    layout_width: f32,
    environment: EnvironmentId,
) -> ResolvedTextLayout {
    let metrics = ParagraphMetrics {
        height: paragraph.height(),
        max_width: paragraph.max_width(),
        min_intrinsic_width: paragraph.min_intrinsic_width(),
        max_intrinsic_width: paragraph.max_intrinsic_width(),
        alphabetic_baseline: paragraph.alphabetic_baseline(),
        ideographic_baseline: paragraph.ideographic_baseline(),
        longest_line: paragraph.longest_line(),
        line_count: paragraph.line_number(),
        did_exceed_max_lines: paragraph.did_exceed_max_lines(),
    };

    let (mut lines, omitted_by_truncation) = extract_lines(paragraph, shaping_text, &metrics);
    let raw_runs = capture_visit_runs(paragraph);
    // Gap 6: when the oracle truncated, the truncation marker is the final
    // visual run of the final line. The visitor aliases source offsets for
    // it, so its source mapping is withheld below.
    let synthetic_run = if metrics.did_exceed_max_lines && !raw_runs.is_empty() {
        Some(raw_runs.len() - 1)
    } else {
        None
    };
    let mut unmapped_ranges = Vec::new();
    let clusters = extract_clusters(
        paragraph,
        &lines,
        &raw_runs,
        synthetic_run,
        &mut unmapped_ranges,
    );
    let glyph_runs = finalize_runs(raw_runs, synthetic_run);

    // Per-line and paragraph ink: union of per-glyph ink of the line's runs.
    for line in lines.iter_mut() {
        let mut acc: Option<Rectangle> = None;
        for run in glyph_runs.iter().filter(|r| r.line == line.index) {
            if let Some(bounds) = &run.ink_bounds {
                for b in bounds {
                    acc = Some(match acc {
                        Some(a) => union(a, *b),
                        None => *b,
                    });
                }
            }
        }
        line.ink_bounds = acc;
    }
    let ink_bounds = lines.iter().filter_map(|l| l.ink_bounds).reduce(union);
    let logical_bounds = lines
        .iter()
        .map(|l| l.logical_bounds)
        .reduce(union)
        .unwrap_or(Rectangle {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });

    ResolvedTextLayout {
        oracle_version: SKPARAGRAPH_ORACLE_VERSION,
        environment,
        width_constraint,
        layout_width,
        transform,
        shaping_text: shaping_text.to_owned(),
        metrics,
        lines,
        glyph_runs,
        clusters,
        logical_bounds,
        ink_bounds,
        unresolved_glyphs: paragraph.unresolved_glyphs(),
        omitted_by_truncation,
        unmapped_ranges,
        caret_stops: None, // Gap 1: declared absence, never estimated.
    }
}

fn union(a: Rectangle, b: Rectangle) -> Rectangle {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.width).max(b.x + b.width);
    let y1 = (a.y + a.height).max(b.y + b.height);
    Rectangle {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

/// Whether `s` ends in a hard line terminator the oracle breaks on
/// (`\n`/`\r\n`, VT, FF, NEL, LS, PS — a bare `\r` is not a break for
/// this oracle).
fn ends_with_hard_terminator(s: &str) -> bool {
    s.ends_with([
        '\n', '\u{000B}', '\u{000C}', '\u{0085}', '\u{2028}', '\u{2029}',
    ])
}

/// Incremental UTF-16 → UTF-8 offset converter over `text`. Targets must be
/// non-decreasing across calls.
struct Utf16ToUtf8<'a> {
    text: &'a str,
    iter: std::iter::Peekable<std::str::CharIndices<'a>>,
    run_u16: usize,
    run_byte: usize,
}

impl<'a> Utf16ToUtf8<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            iter: text.char_indices().peekable(),
            run_u16: 0,
            run_byte: 0,
        }
    }

    fn convert(&mut self, target_u16: usize) -> usize {
        while self.run_u16 < target_u16 {
            if let Some(&(byte_idx, ch)) = self.iter.peek() {
                self.run_u16 += ch.len_utf16();
                self.run_byte = byte_idx + ch.len_utf8();
                self.iter.next();
            } else {
                break;
            }
        }
        self.run_byte.min(self.text.len())
    }
}

/// Build the RFD line list from Skia's UTF-16 line metrics.
///
/// Normalizations (documented, oracle-sourced):
/// - Consumed ranges are derived from consecutive line starts, so each line
///   includes its consumed terminator.
/// - When the source ends in a line terminator, Skia emits the trailing
///   empty line but repeats the previous line's indices for it; the terminal
///   empty line is normalized to `[len, len)`.
/// - Empty source: Skia reports zero lines; one empty terminal line is
///   synthesized from the oracle's paragraph-level metrics.
fn extract_lines(
    paragraph: &textlayout::Paragraph,
    shaping_text: &str,
    metrics: &ParagraphMetrics,
) -> (Vec<ResolvedLine>, Option<Range<usize>>) {
    let len = shaping_text.len();
    let skia_lines = paragraph.get_line_metrics();

    if skia_lines.is_empty() {
        // RFD: an empty source resolves to one empty terminal line carrying
        // line metrics, without inventing a source character. Metrics derive
        // from the oracle's paragraph readout for the empty paragraph.
        let baseline = metrics.alphabetic_baseline;
        let ascent = baseline;
        let descent = (metrics.height - baseline).max(0.0);
        let line = ResolvedLine {
            index: 0,
            source_range: 0..0,
            break_kind: LineBreakKind::Terminal,
            baseline,
            ascent,
            descent,
            left: 0.0,
            width: 0.0,
            logical_bounds: Rectangle {
                x: 0.0,
                y: baseline - ascent,
                width: 0.0,
                height: ascent + descent,
            },
            ink_bounds: None,
            synthesized: true,
        };
        return (vec![line], None);
    }

    let mut conv = Utf16ToUtf8::new(shaping_text);
    let mut starts_u8: Vec<usize> = Vec::with_capacity(skia_lines.len());
    for lm in &skia_lines {
        starts_u8.push(conv.convert(lm.start_index));
    }
    // The truncated final line's consumed end is its own end index; every
    // other boundary is the next line's start.
    let last_end_u8 = if metrics.did_exceed_max_lines {
        conv.convert(skia_lines[skia_lines.len() - 1].end_index)
    } else {
        len
    };

    // Skia emits a trailing empty line when the source ends in a hard
    // terminator, but repeats the previous line's indices for it; normalize
    // that degenerate line to the empty range at the end of the source. The
    // guard (>= 2 lines, zero-width last line) keeps the normalization from
    // ever touching a real final line.
    let ends_in_terminator = !metrics.did_exceed_max_lines
        && skia_lines.len() >= 2
        && skia_lines[skia_lines.len() - 1].width == 0.0
        && ends_with_hard_terminator(shaping_text);
    if ends_in_terminator {
        if let Some(last) = starts_u8.last_mut() {
            *last = len;
        }
    }

    let line_count = skia_lines.len();
    let mut lines: Vec<ResolvedLine> = Vec::with_capacity(line_count);
    for (i, lm) in skia_lines.iter().enumerate() {
        let start = starts_u8[i];
        let end = if i + 1 < line_count {
            starts_u8[i + 1]
        } else {
            last_end_u8
        };
        let start = start.min(end);
        let is_last = i + 1 == line_count;
        let consumed = &shaping_text[start..end];
        let break_kind = if is_last {
            if metrics.did_exceed_max_lines {
                LineBreakKind::Truncated
            } else {
                LineBreakKind::Terminal
            }
        } else if ends_with_hard_terminator(consumed) {
            LineBreakKind::Explicit
        } else {
            LineBreakKind::Soft
        };

        let baseline = lm.baseline as f32;
        let ascent = lm.ascent as f32;
        let descent = lm.descent as f32;
        let left = lm.left as f32;
        let width = lm.width as f32;
        lines.push(ResolvedLine {
            index: i,
            source_range: start..end,
            break_kind,
            baseline,
            ascent,
            descent,
            left,
            width,
            logical_bounds: Rectangle {
                x: left,
                y: baseline - ascent,
                width,
                height: ascent + descent,
            },
            ink_bounds: None,
            synthesized: false,
        });
    }

    let omitted = if metrics.did_exceed_max_lines && last_end_u8 < len {
        Some(last_end_u8..len)
    } else {
        None
    };
    (lines, omitted)
}

/// Capture glyph runs via `visit` (absolute UTF-8 cluster starts, fonts,
/// positions) zipped with `extended_visit` (per-glyph ink bounds).
fn capture_visit_runs(paragraph: &mut textlayout::Paragraph) -> Vec<VisitRun> {
    let mut runs: Vec<VisitRun> = Vec::new();
    paragraph.visit(|line, info| {
        if let Some(info) = info {
            let origin = info.origin();
            let font = info.font();
            let typeface = font.typeface();
            let style = typeface.font_style();
            let positions = info
                .positions()
                .iter()
                .map(|p| [p.x + origin.x, p.y + origin.y])
                .collect();
            runs.push(VisitRun {
                line,
                font: ResolvedFontId {
                    family: typeface.family_name(),
                    size: font.size(),
                    weight: *style.weight(),
                    italic: typeface.is_italic(),
                    typeface_id: typeface.unique_id(),
                },
                advance_width: info.advance_x(),
                glyph_ids: info.glyphs().to_vec(),
                positions,
                utf8_starts: info.utf8_starts().to_vec(),
                ink_bounds: None,
            });
        }
    });

    // Second pass: per-glyph ink bounds. The extended visitor iterates the
    // same visual runs; zip by order and cross-check glyph identity. On any
    // mismatch the ink stays a declared `None` rather than misattributed.
    let mut index = 0usize;
    paragraph.extended_visit(|line, info| {
        if let Some(info) = info {
            if let Some(run) = runs.get_mut(index) {
                if run.line == line && run.glyph_ids.as_slice() == info.glyphs() {
                    let origin = info.origin();
                    let positions = info.positions();
                    let ink = info
                        .bounds()
                        .iter()
                        .zip(positions.iter())
                        .map(|(b, p)| Rectangle {
                            x: b.left + p.x + origin.x,
                            y: b.top + p.y + origin.y,
                            width: b.right - b.left,
                            height: b.bottom - b.top,
                        })
                        .collect();
                    run.ink_bounds = Some(ink);
                } else {
                    debug_assert!(false, "visit/extended_visit run order diverged");
                }
            }
            index += 1;
        }
    });

    runs
}

/// Convert raw visitor captures into the published run vocabulary.
fn finalize_runs(raw: Vec<VisitRun>, synthetic_run: Option<usize>) -> Vec<ResolvedGlyphRun> {
    raw.into_iter()
        .enumerate()
        .map(|(i, run)| {
            let synthetic = synthetic_run == Some(i);
            let glyph_count = run.glyph_ids.len();
            let cluster_starts = if synthetic {
                // A synthetic unit must never masquerade as authored content.
                None
            } else {
                Some(
                    run.utf8_starts
                        .iter()
                        .take(glyph_count)
                        .map(|&s| s as usize)
                        .collect(),
                )
            };
            ResolvedGlyphRun {
                line: run.line,
                font: run.font,
                synthetic,
                glyph_ids: run.glyph_ids,
                positions: run.positions,
                ink_bounds: run.ink_bounds,
                cluster_starts,
                advance_width: run.advance_width,
                // Gap 5: the visitor exposes no per-glyph advances.
                glyph_advances: None,
            }
        })
        .collect()
}

/// Derive the shaping-cluster ledger.
///
/// Glyph-backed clusters come from the visitor's per-glyph cluster starts
/// (consecutive glyphs sharing a start form one cluster); their advance
/// boxes partition each run's advance extent at the cluster's first visual
/// pen position. Direction derives from the monotonicity of cluster starts
/// within the run; runs with a single cluster are disambiguated with one
/// direct oracle query. The logically-last cluster's end comes from the
/// visitor's end sentinel for LTR runs and from one oracle query for RTL
/// runs (whose sentinel is not meaningful). Consumed source not covered by
/// any visual glyph (line terminators, whitespace trimmed at a soft wrap) is
/// recovered with targeted oracle queries so the artifact keeps a complete
/// source-disposition ledger; anything the oracle declines to map is
/// reported in `unmapped_ranges`.
fn extract_clusters(
    paragraph: &textlayout::Paragraph,
    lines: &[ResolvedLine],
    raw_runs: &[VisitRun],
    synthetic_run: Option<usize>,
    unmapped_ranges: &mut Vec<Range<usize>>,
) -> Vec<ResolvedCluster> {
    let mut clusters: Vec<ResolvedCluster> = Vec::new();

    for (run_index, run) in raw_runs.iter().enumerate() {
        if synthetic_run == Some(run_index) {
            continue; // synthetic marker: no authored source coverage
        }
        let glyph_count = run.glyph_ids.len();
        if glyph_count == 0 {
            continue;
        }
        let line = &lines[run.line];

        // Group consecutive glyphs sharing a cluster start (visual order).
        struct Group {
            start: usize,
            glyphs: Range<usize>,
            visual_x: f32,
        }
        let mut groups: Vec<Group> = Vec::new();
        for gi in 0..glyph_count {
            let s = run.utf8_starts[gi] as usize;
            match groups.last_mut() {
                Some(g) if g.start == s => {
                    g.glyphs.end = gi + 1;
                    g.visual_x = g.visual_x.min(run.positions[gi][0]);
                }
                _ => groups.push(Group {
                    start: s,
                    glyphs: gi..gi + 1,
                    visual_x: run.positions[gi][0],
                }),
            }
        }

        // Direction from monotonicity of starts across groups; a
        // single-cluster run is resolved with one oracle query.
        let direction = if groups.len() > 1 {
            if groups.windows(2).all(|w| w[0].start < w[1].start) {
                ResolvedDirection::Ltr
            } else {
                ResolvedDirection::Rtl
            }
        } else {
            match paragraph.get_glyph_cluster_at(groups[0].start) {
                Some(info) if info.position == textlayout::TextDirection::RTL => {
                    ResolvedDirection::Rtl
                }
                _ => ResolvedDirection::Ltr,
            }
        };

        // Logical range end per group: the next logical start within the
        // run; the logically-last group's end comes from the visitor's end
        // sentinel (LTR) or one oracle query (RTL).
        let mut logical_order: Vec<usize> = (0..groups.len()).collect();
        logical_order.sort_by_key(|&i| groups[i].start);
        let last_logical = *logical_order.last().unwrap();
        let run_logical_end = match direction {
            ResolvedDirection::Ltr => run.utf8_starts[glyph_count] as usize,
            ResolvedDirection::Rtl => paragraph
                .get_glyph_cluster_at(groups[last_logical].start)
                .map(|info| info.text_range.end)
                .unwrap_or(groups[last_logical].start),
        };

        // Visual boundaries: sorted first-glyph x positions partition the
        // run's advance extent [min_x, min_x + advance].
        let run_left = groups
            .iter()
            .map(|g| g.visual_x)
            .fold(f32::INFINITY, f32::min);
        let mut visual_order: Vec<usize> = (0..groups.len()).collect();
        visual_order.sort_by(|&a, &b| {
            groups[a]
                .visual_x
                .partial_cmp(&groups[b].visual_x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut right_edge = vec![0.0f32; groups.len()];
        for (vi, &gi) in visual_order.iter().enumerate() {
            right_edge[gi] = if vi + 1 < visual_order.len() {
                groups[visual_order[vi + 1]].visual_x
            } else {
                run_left + run.advance_width
            };
        }

        let top = line.baseline - line.ascent;
        let height = line.ascent + line.descent;
        for (li, &gi) in logical_order.iter().enumerate() {
            let g = &groups[gi];
            let end = if li + 1 < logical_order.len() {
                groups[logical_order[li + 1]].start
            } else {
                run_logical_end
            };
            clusters.push(ResolvedCluster {
                range: g.start..end.max(g.start),
                line: run.line,
                glyph_span: Some((run_index, g.glyphs.clone())),
                direction,
                advance_bounds: Rectangle {
                    x: g.visual_x,
                    y: top,
                    width: (right_edge[gi] - g.visual_x).max(0.0),
                    height,
                },
            });
        }
    }

    clusters.sort_by_key(|c| (c.range.start, c.range.end));

    // Recover consumed source with no visual glyph (terminators, trimmed
    // trailing whitespace) via targeted oracle queries, line by line.
    let mut recovered: Vec<ResolvedCluster> = Vec::new();
    for line in lines {
        let mut gaps: Vec<Range<usize>> = Vec::new();
        {
            let mut cursor = line.source_range.start;
            let mut covered: Vec<&ResolvedCluster> =
                clusters.iter().filter(|c| c.line == line.index).collect();
            covered.sort_by_key(|c| c.range.start);
            for c in covered {
                if c.range.start > cursor {
                    gaps.push(cursor..c.range.start);
                }
                cursor = cursor.max(c.range.end);
            }
            if cursor < line.source_range.end {
                gaps.push(cursor..line.source_range.end);
            }
        }
        for gap in gaps {
            let mut at = gap.start;
            while at < gap.end {
                match paragraph.get_glyph_cluster_at(at) {
                    Some(info) if info.text_range.end > at => {
                        let end = info.text_range.end.min(gap.end);
                        recovered.push(ResolvedCluster {
                            range: at..end,
                            line: line.index,
                            glyph_span: None,
                            direction: if info.position == textlayout::TextDirection::RTL {
                                ResolvedDirection::Rtl
                            } else {
                                ResolvedDirection::Ltr
                            },
                            advance_bounds: Rectangle {
                                x: info.bounds.left,
                                y: info.bounds.top,
                                width: info.bounds.right - info.bounds.left,
                                height: info.bounds.bottom - info.bounds.top,
                            },
                        });
                        at = end;
                    }
                    _ => {
                        // The oracle declined to map this consumed range:
                        // report it, never silently drop it.
                        unmapped_ranges.push(at..gap.end);
                        break;
                    }
                }
            }
        }
    }
    clusters.extend(recovered);
    clusters.sort_by_key(|c| (c.range.start, c.range.end));
    clusters
}
