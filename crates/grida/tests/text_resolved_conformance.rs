//! Conformance suite for the resolved-text-layout artifact
//! (`crate::text::resolved`) against the Universal Shaped Text Layout RFD
//! (`docs/wg/feat-paragraph/text-layout.md`).
//!
//! Test names carry the RFD's scenario vocabulary verbatim so conformance
//! drops stay grep-able across engines. Where the Skia oracle cannot supply
//! what the RFD demands, the artifact declares the absence and a test here
//! asserts the *declared* absence (see the module docs' "Declared gaps").
//!
//! Fonts: the deterministic embedded Geist face, plus the Noto Sans Hebrew
//! fixture (`fixtures/fonts/Noto_Sans_Hebrew`) for bidirectional scenarios —
//! no system fonts, so results are machine-independent.

use std::sync::{Arc, Mutex};

use grida::cache::paragraph::{LayoutMeasurements, ParagraphCache};
use grida::cg::prelude::*;
use grida::node::factory::NodeFactory;
use grida::resources::ByteStore;
use grida::runtime::font_repository::FontRepository;
use grida::text::resolved::{
    LineBreakKind, ResolvedDirection, ResolvedTextLayout, ShapingTransform,
    PINNED_SKIA_SAFE_VERSION, SKPARAGRAPH_ORACLE_VERSION,
};

const NOTO_HEBREW: &[u8] = include_bytes!(
    "../../../fixtures/fonts/Noto_Sans_Hebrew/NotoSansHebrew-VariableFont_wdth,wght.ttf"
);
const BUNGEE_REGULAR: &[u8] = include_bytes!("../../../fixtures/fonts/Bungee/Bungee-Regular.ttf");

struct Ctx {
    store: Arc<Mutex<ByteStore>>,
    fonts: FontRepository,
    cache: ParagraphCache,
    style: TextStyleRec,
    align: TextAlign,
}

impl Ctx {
    fn new() -> Self {
        let store = Arc::new(Mutex::new(ByteStore::new()));
        let mut fonts = FontRepository::new(store.clone());
        fonts.register_embedded_fonts();
        let node = NodeFactory::new().create_text_span_node();
        Self {
            store,
            fonts,
            cache: ParagraphCache::new(),
            style: node.text_style,
            align: node.text_align,
        }
    }

    fn with_hebrew_fallback() -> Self {
        let mut ctx = Self::new();
        let hash = 0x4845_4252_u64;
        ctx.store.lock().unwrap().insert(hash, NOTO_HEBREW.to_vec());
        ctx.fonts.add(hash, "Noto Sans Hebrew");
        ctx.fonts
            .set_user_fallback_families(vec!["Noto Sans Hebrew".to_string()]);
        ctx
    }

    fn resolve(&mut self, text: &str, width: Option<f32>) -> Arc<ResolvedTextLayout> {
        self.cache
            .resolve(
                text,
                &self.style,
                &self.align,
                &None,
                &None,
                width,
                &self.fonts,
                None,
            )
            .expect("resolution must publish an artifact")
    }

    fn resolve_truncated(&mut self, text: &str, width: Option<f32>) -> Arc<ResolvedTextLayout> {
        self.cache
            .resolve(
                text,
                &self.style,
                &self.align,
                &Some(1),
                &Some("…".to_string()),
                width,
                &self.fonts,
                None,
            )
            .expect("resolution must publish an artifact")
    }
}

// ---------------------------------------------------------------------------
// Resolution identity
// ---------------------------------------------------------------------------

#[test]
fn resolution_identity_records_the_oracle_version_and_environment_identity() {
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("hello", None);
    assert_eq!(artifact.oracle_version, SKPARAGRAPH_ORACLE_VERSION);
    assert_eq!(artifact.oracle_version, "skparagraph@skia-0.93.1");
    assert_eq!(
        artifact.environment.font_generation,
        ctx.fonts.generation(),
        "the environment recorded by the artifact must be exactly the one used to produce it"
    );
    assert_eq!(
        artifact.environment.label(),
        format!("fontrepository@gen-{}", ctx.fonts.generation())
    );
}

#[test]
fn the_oracle_version_matches_the_pinned_skia_safe_dependency() {
    // "Latest" is not a valid durable oracle version: the stamped version is
    // derived from one pinned const, and this test keeps that const in
    // lockstep with the actual Cargo dependency pin.
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest must be readable");
    let line = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("skia-safe"))
        .expect("Cargo.toml must pin skia-safe");
    let needle = "version = \"";
    let start = line.find(needle).expect("skia-safe pin must be explicit") + needle.len();
    let end = start + line[start..].find('"').expect("unterminated version");
    let pinned_in_manifest = &line[start..end];
    assert_eq!(
        PINNED_SKIA_SAFE_VERSION, pinned_in_manifest,
        "resolved::PINNED_SKIA_SAFE_VERSION must match the skia-safe pin in Cargo.toml \
         (bump both together: a skia change is an oracle-version change)"
    );
    assert!(SKPARAGRAPH_ORACLE_VERSION.ends_with(pinned_in_manifest));
}

#[test]
fn the_arrival_of_a_font_resource_creates_a_new_environment_identity_and_a_new_resolved_artifact() {
    let mut ctx = Ctx::new();
    let before = ctx.resolve("hello", None);

    // Register a new font resource: the environment identity must change and
    // the next resolution must be a new artifact, never the old one patched.
    let hash = 0x42_554e_u64;
    ctx.store
        .lock()
        .unwrap()
        .insert(hash, BUNGEE_REGULAR.to_vec());
    ctx.fonts.add(hash, "Bungee");

    let after = ctx.resolve("hello", None);
    assert_ne!(before.environment, after.environment);
    assert!(
        !Arc::ptr_eq(&before, &after),
        "a supposedly immutable result is never patched in place by a late font swap"
    );
    assert_eq!(after.environment.font_generation, ctx.fonts.generation());
}

#[test]
fn cache_reuse_publishes_the_same_immutable_artifact_for_the_same_identity() {
    let mut ctx = Ctx::new();
    let first = ctx.resolve("hello", Some(120.0));
    let second = ctx.resolve("hello", Some(120.0));
    assert!(
        Arc::ptr_eq(&first, &second),
        "internal cache reuse must republish the equivalent artifact for the same identity"
    );

    // A different constraint produces a different resolution — it is not
    // another view of the old result.
    let other = ctx.resolve("hello", Some(60.0));
    assert!(!Arc::ptr_eq(&first, &other));
    assert_eq!(other.width_constraint, Some(60.0));
}

#[test]
fn paint_only_changes_do_not_invalidate_the_geometry_artifact() {
    use grida::runtime::image_repository::ImageRepository;
    let mut ctx = Ctx::new();
    let images = ImageRepository::new(ctx.store.clone());
    let before = ctx.resolve("hello", Some(120.0));

    // Build the painted paragraph for the same text (the paint surface).
    let fills = [Paint::Solid(CGColor::BLACK.into())];
    let _painted = ctx.cache.paragraph(
        "hello",
        &fills,
        &ctx.align,
        &ctx.style,
        &None,
        &None,
        Some(120.0),
        &ctx.fonts,
        &images,
        None,
    );

    let after = ctx.resolve("hello", Some(120.0));
    assert!(
        Arc::ptr_eq(&before, &after),
        "paint state is not a geometry input; painting must not invalidate the artifact"
    );
}

#[test]
fn the_constraint_input_and_the_final_oracle_layout_width_are_retained() {
    let mut ctx = Ctx::new();
    let constrained = ctx.resolve("hello world", Some(100.0));
    assert_eq!(constrained.width_constraint, Some(100.0));
    assert_eq!(constrained.layout_width, 100.0);

    let unconstrained = ctx.resolve("hello", None);
    assert_eq!(unconstrained.width_constraint, None);
    assert_eq!(
        unconstrained.layout_width, unconstrained.metrics.max_intrinsic_width,
        "an unconstrained inline axis lays out at the measured intrinsic width"
    );
}

// ---------------------------------------------------------------------------
// Measurement as a projection
// ---------------------------------------------------------------------------

#[test]
fn measurement_is_a_query_over_the_resolved_artifact_not_a_parallel_text_operation() {
    let mut ctx = Ctx::new();
    let text = "h\u{e9}llo\nw\u{f6}rld";
    let measured = ctx.cache.measure(
        text, &ctx.style, &ctx.align, &None, &None, None, &ctx.fonts, None,
    );
    let artifact = ctx.resolve(text, None);
    let projected = LayoutMeasurements::from(artifact.as_ref());

    assert_eq!(measured.height, projected.height);
    assert_eq!(measured.max_width, projected.max_width);
    assert_eq!(measured.min_intrinsic_width, projected.min_intrinsic_width);
    assert_eq!(measured.max_intrinsic_width, projected.max_intrinsic_width);
    assert_eq!(measured.alphabetic_baseline, projected.alphabetic_baseline);
    assert_eq!(
        measured.ideographic_baseline,
        projected.ideographic_baseline
    );
    assert_eq!(measured.longest_line, projected.longest_line);
    assert_eq!(measured.line_number, projected.line_number);
    assert_eq!(
        measured.did_exceed_max_lines,
        projected.did_exceed_max_lines
    );
}

// ---------------------------------------------------------------------------
// Paragraphs and lines
// ---------------------------------------------------------------------------

#[test]
fn lines_carry_utf8_byte_ranges_at_scalar_boundaries() {
    let mut ctx = Ctx::new();
    // é and ö are two UTF-8 bytes each: byte ranges must be UTF-8, not
    // UTF-16 code-unit ranges leaked from the oracle.
    let text = "h\u{e9}llo\nw\u{f6}rld";
    let artifact = ctx.resolve(text, None);

    assert_eq!(artifact.lines.len(), 2);
    assert_eq!(
        artifact.lines[0].source_range,
        0..7,
        "line 0 consumes its explicit terminator"
    );
    assert_eq!(artifact.lines[1].source_range, 7..13);
    for line in &artifact.lines {
        assert!(artifact
            .shaping_text
            .is_char_boundary(line.source_range.start));
        assert!(artifact
            .shaping_text
            .is_char_boundary(line.source_range.end));
        assert_eq!(line.index, artifact.lines[line.index].index);
    }
    assert_eq!(artifact.line_at_byte(8), Some(1));
    assert_eq!(artifact.line_at_byte(13), Some(1));
}

#[test]
fn explicit_soft_terminal_and_truncated_line_ends_are_distinguished() {
    let mut ctx = Ctx::new();

    // Explicit break, then terminal line.
    let explicit = ctx.resolve("a\nb", None);
    assert_eq!(explicit.lines.len(), 2);
    assert_eq!(explicit.lines[0].break_kind, LineBreakKind::Explicit);
    assert_eq!(explicit.lines[1].break_kind, LineBreakKind::Terminal);

    // Explicit source line breaks are source content, whatever the
    // terminator: CRLF and LINE SEPARATOR are consumed by the line they end.
    let crlf = ctx.resolve("a\r\nb", None);
    assert_eq!(crlf.lines[0].break_kind, LineBreakKind::Explicit);
    assert_eq!(crlf.lines[0].source_range, 0..3);
    let ls = ctx.resolve("a\u{2028}b", None);
    assert_eq!(ls.lines[0].break_kind, LineBreakKind::Explicit);
    assert_eq!(ls.lines[0].source_range, 0..4);

    // Soft wrap chosen by the oracle, then terminal line.
    let soft = ctx.resolve("aa bb", Some(30.0));
    assert_eq!(soft.lines.len(), 2);
    assert_eq!(soft.lines[0].break_kind, LineBreakKind::Soft);
    assert_eq!(
        soft.lines[0].source_range,
        0..3,
        "the separator consumed at a soft wrap stays represented in line coverage"
    );
    assert_eq!(soft.lines[1].break_kind, LineBreakKind::Terminal);

    // Truncation policy replaced remaining source.
    let truncated = ctx.resolve_truncated("aaaa bbbb cccc dddd eeee ffff", Some(80.0));
    assert!(truncated.metrics.did_exceed_max_lines);
    assert_eq!(
        truncated.lines.last().unwrap().break_kind,
        LineBreakKind::Truncated
    );
}

#[test]
fn an_empty_source_resolves_to_one_empty_terminal_line() {
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("", None);

    assert_eq!(artifact.lines.len(), 1);
    let line = &artifact.lines[0];
    assert_eq!(line.break_kind, LineBreakKind::Terminal);
    assert_eq!(line.source_range, 0..0, "no source character is invented");
    assert!(
        line.synthesized,
        "the oracle reports zero lines for empty source; the terminal line is declared synthesized"
    );
    assert!(
        line.baseline > 0.0,
        "the empty terminal line carries the default style's line metrics"
    );
    assert!(artifact.metrics.height > 0.0);
    assert_eq!(
        artifact.metrics.line_count, 0,
        "the oracle's own line count is preserved verbatim"
    );
    assert!(artifact.glyph_runs.is_empty());
    assert!(artifact.clusters.is_empty());
    assert!(artifact.ink_bounds.is_none());
}

#[test]
fn a_source_ending_in_an_explicit_terminator_keeps_an_empty_terminal_line_after_it() {
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("a\n", None);

    assert_eq!(artifact.lines.len(), 2);
    assert_eq!(artifact.lines[0].break_kind, LineBreakKind::Explicit);
    assert_eq!(
        artifact.lines[0].source_range,
        0..2,
        "the preceding line ends explicitly and consumes its terminator"
    );
    assert_eq!(artifact.lines[1].break_kind, LineBreakKind::Terminal);
    assert_eq!(artifact.lines[1].source_range, 2..2);
}

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

#[test]
fn logical_bounds_and_ink_bounds_are_kept_distinct() {
    let mut ctx = Ctx::new();

    // A line box includes layout space where no ink is drawn: glyph ink for
    // lowercase latin is strictly shorter than the typographic line box.
    let artifact = ctx.resolve("aaa", None);
    let logical = artifact.logical_bounds;
    let ink = artifact
        .ink_bounds
        .expect("visible glyphs must produce base ink");
    assert!(
        ink.height < logical.height,
        "ink height ({}) must stay below the typographic line box height ({})",
        ink.height,
        logical.height
    );
    assert_ne!(logical, ink);
    for line in &artifact.lines {
        assert!(line.ink_bounds.is_some());
        assert_ne!(Some(line.logical_bounds), line.ink_bounds);
    }

    // A newline-only source has logical extent but draws nothing.
    let empty_lines = ctx.resolve("\n", None);
    assert!(empty_lines.ink_bounds.is_none());
    assert!(empty_lines.logical_bounds.height > 0.0);
}

// ---------------------------------------------------------------------------
// Clusters and UTF-8 mapping
// ---------------------------------------------------------------------------

#[test]
fn every_visible_glyph_belongs_to_exactly_one_shaped_run_line_and_cluster() {
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("h\u{e9}llo\nw\u{f6}rld", None);

    assert!(!artifact.glyph_runs.is_empty());
    for (run_index, run) in artifact.glyph_runs.iter().enumerate() {
        assert!(run.line < artifact.lines.len());
        assert_eq!(run.glyph_ids.len(), run.positions.len());
        let starts = run
            .cluster_starts
            .as_ref()
            .expect("non-synthetic runs carry cluster mappings");
        assert_eq!(starts.len(), run.glyph_ids.len());
        for glyph_index in 0..run.glyph_ids.len() {
            let owners: Vec<_> = artifact
                .clusters
                .iter()
                .filter(|c| {
                    c.glyph_span
                        .as_ref()
                        .is_some_and(|(r, g)| *r == run_index && g.contains(&glyph_index))
                })
                .collect();
            assert_eq!(
                owners.len(),
                1,
                "glyph {glyph_index} of run {run_index} must belong to exactly one cluster"
            );
            assert_eq!(owners[0].line, run.line);
        }
    }
}

#[test]
fn non_synthetic_clusters_cover_the_shaping_text_without_gaps_or_overlap() {
    let mut ctx = Ctx::new();
    for (text, width) in [
        ("h\u{e9}llo\nw\u{f6}rld", None),
        ("aa bb", Some(30.0)),
        ("a\n", None),
        ("xe\u{301}y z", None),
    ] {
        let artifact = ctx.resolve(text, width);
        assert!(
            artifact.unmapped_ranges.is_empty(),
            "{text:?}: the oracle mapped all consumed source; nothing may remain implicit"
        );
        let mut cursor = 0usize;
        for cluster in &artifact.clusters {
            assert_eq!(
                cluster.range.start, cursor,
                "{text:?}: clusters must be contiguous without gaps or overlap at byte {cursor}"
            );
            assert!(cluster.range.end > cluster.range.start);
            cursor = cluster.range.end;
        }
        assert_eq!(
            cursor,
            text.len(),
            "{text:?}: clusters must cover the complete shaping text"
        );

        // Advance boxes of a run's clusters partition the run's advance
        // extent (cluster advance is real geometry, not an estimate).
        for (run_index, run) in artifact.glyph_runs.iter().enumerate() {
            let total: f32 = artifact
                .clusters
                .iter()
                .filter(|c| c.glyph_span.as_ref().is_some_and(|(r, _)| *r == run_index))
                .map(|c| c.advance_bounds.width)
                .sum();
            assert!(
                (total - run.advance_width).abs() < 0.05,
                "{text:?}: cluster advance boxes ({total}) must partition run advance ({})",
                run.advance_width
            );
        }
    }
}

#[test]
fn grapheme_cluster_shaping_cluster_glyph_and_caret_stop_index_spaces_are_not_collapsed() {
    let mut ctx = Ctx::new();

    // Ligature: two graphemes ("f", "f") resolve into one shaping cluster
    // with one glyph. The cluster spans bytes 0..2; byte 1 is a grapheme
    // boundary but not a cluster boundary.
    let ligature = ctx.resolve("ffi", None);
    let first = &ligature.clusters[0];
    assert_eq!(first.range, 0..2, "Geist ligates the ff pair");
    let (run, glyphs) = first.glyph_span.clone().expect("visible cluster");
    assert_eq!(glyphs.len(), 1, "one ligature glyph for two graphemes");
    assert_eq!(run, 0);
    assert!(
        !ligature.clusters.iter().any(|c| c.range.start == 1),
        "a grapheme boundary inside a ligature is not a shaping-cluster boundary"
    );

    // Combining sequence: one grapheme ("e" + U+0301) spanning three bytes
    // forms one cluster; interior scalar boundaries are not cluster starts.
    let combining = ctx.resolve("xe\u{301}y", None);
    assert!(
        combining.clusters.iter().any(|c| c.range == (1..4)),
        "the combining sequence must resolve to one cluster covering bytes 1..4"
    );
}

#[test]
fn within_a_line_visual_glyph_order_may_differ_from_logical_source_order() {
    let mut ctx = Ctx::with_hebrew_fallback();
    // "ab שלום cd" — Hebrew word bytes 3..11, two Hebrew-letter clusters of
    // two bytes each.
    let text = "ab \u{5e9}\u{5dc}\u{5d5}\u{5dd} cd";
    let artifact = ctx.resolve(text, None);
    assert_eq!(
        artifact.unresolved_glyphs,
        Some(0),
        "the fixture font must cover the Hebrew"
    );

    let rtl: Vec<_> = artifact
        .clusters
        .iter()
        .filter(|c| c.direction == ResolvedDirection::Rtl)
        .collect();
    let ranges: Vec<_> = rtl.iter().map(|c| c.range.clone()).collect();
    assert_eq!(ranges, vec![3..5, 5..7, 7..9, 9..11]);

    // Logical order ascends; visual x positions descend: the first Hebrew
    // letter (bytes 3..5) sits visually right of the last (bytes 9..11).
    let first_logical = rtl.iter().find(|c| c.range == (3..5)).unwrap();
    let last_logical = rtl.iter().find(|c| c.range == (9..11)).unwrap();
    assert!(
        first_logical.advance_bounds.x > last_logical.advance_bounds.x,
        "visual order differs from logical order in a bidirectional line"
    );

    // The run that shaped the Hebrew records the exact resolved fallback
    // face, not the requested family.
    let hebrew_run = artifact
        .glyph_runs
        .iter()
        .find(|r| r.font.family == "Noto Sans Hebrew")
        .expect("fallback resolution must be recorded per run");
    assert!(!hebrew_run.glyph_ids.is_empty());
}

// ---------------------------------------------------------------------------
// Truncation
// ---------------------------------------------------------------------------

#[test]
fn truncation_marker_is_synthetic_and_never_masquerades_as_authored_content() {
    let mut ctx = Ctx::new();
    let text = "aaaa bbbb cccc dddd eeee ffff";
    let artifact = ctx.resolve_truncated(text, Some(80.0));

    assert!(artifact.metrics.did_exceed_max_lines);
    let marker = artifact
        .glyph_runs
        .iter()
        .find(|r| r.synthetic)
        .expect("truncation must record a synthetic marker run");
    assert!(
        marker.cluster_starts.is_none(),
        "a synthetic unit must never claim authored source coverage"
    );
    assert!(
        !marker.glyph_ids.is_empty(),
        "the marker's shaped glyphs are recorded"
    );
    assert_eq!(
        artifact.glyph_runs.iter().filter(|r| r.synthetic).count(),
        1
    );

    // Every omitted source byte is declared, anchored after the consumed
    // prefix; visible clusters never reach into the omission.
    let omitted = artifact
        .omitted_by_truncation
        .clone()
        .expect("truncation must declare the omitted source range");
    assert_eq!(omitted.end, text.len());
    assert_eq!(
        omitted.start,
        artifact.lines.last().unwrap().source_range.end
    );
    for cluster in &artifact.clusters {
        assert!(cluster.range.end <= omitted.start);
    }
}

// ---------------------------------------------------------------------------
// Declared gaps (asserted absences — never fabricated values)
// ---------------------------------------------------------------------------

#[test]
fn caret_stops_are_a_declared_absence_under_this_oracle() {
    // RFD: a caret stop is not assumed to exist at every UTF-8 or glyph
    // boundary. Skia does not enumerate legal caret stops with affinity, so
    // the artifact declares the absence instead of estimating positions
    // (declared gap 1 in `text::resolved`).
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("hello world", None);
    assert!(artifact.caret_stops.is_none());
}

#[test]
fn per_glyph_advances_are_a_declared_absence_under_this_oracle() {
    // Skia's visitor exposes positions and run advances only (declared gap
    // 5); per-glyph advances are never inferred from position deltas.
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("hello world", None);
    assert!(!artifact.glyph_runs.is_empty());
    for run in &artifact.glyph_runs {
        assert!(run.glyph_advances.is_none());
    }
}

#[test]
fn a_transformed_shaping_text_declares_its_transformation_policy_not_a_source_mapping() {
    // RFD: a source transformation is an explicit input and the result
    // retains a mapping back to source. Skia never sees the authored source
    // (declared gap 2), so the artifact records the policy and the derived
    // shaping text — and declares that its byte coordinates are not source
    // coordinates — rather than fabricating a mapping.
    let mut ctx = Ctx::new();
    ctx.style.text_transform = TextTransform::Uppercase;
    let artifact = ctx.resolve("\u{df} and co", None); // ß → SS changes byte length
    assert_eq!(
        artifact.transform,
        ShapingTransform::Uniform(TextTransform::Uppercase)
    );
    assert!(!artifact.transform.is_identity());
    assert_eq!(artifact.shaping_text, "SS AND CO");
    assert_ne!(
        artifact.shaping_text.chars().count(),
        "\u{df} and co".chars().count(),
        "\u{df} \u{2192} SS changes the scalar count: byte coordinates cannot be source coordinates"
    );

    // Identity is declared when no transformation runs.
    let mut plain = Ctx::new();
    let identity = plain.resolve("hello", None);
    assert!(identity.transform.is_identity());
}

#[test]
fn unresolved_glyph_coverage_is_reported_never_silent() {
    // Strict RFD resolution fails on missing glyph coverage; this engine's
    // environment explicitly permits tofu, so the artifact must report the
    // unresolved count instead of presenting tofu as resolved coverage.
    let mut ctx = Ctx::new(); // embedded Geist only: no Hebrew coverage
    let artifact = ctx.resolve("ab \u{5e9}\u{5dc}\u{5d5}\u{5dd}", None);
    let unresolved = artifact
        .unresolved_glyphs
        .expect("the oracle reports an unresolved-glyph count");
    assert!(
        unresolved > 0,
        "missing coverage must be reported, not silently dropped"
    );
}

// ---------------------------------------------------------------------------
// Coordinates
// ---------------------------------------------------------------------------

#[test]
fn fractional_logical_geometry_is_preserved_without_device_rounding() {
    let mut ctx = Ctx::new();
    let artifact = ctx.resolve("h\u{e9}llo", None);
    let has_fractional_position = artifact
        .glyph_runs
        .iter()
        .flat_map(|r| r.positions.iter())
        .any(|p| p[0].fract() != 0.0);
    let has_fractional_metric = artifact.metrics.max_intrinsic_width.fract() != 0.0
        || artifact.lines.iter().any(|l| l.width.fract() != 0.0);
    assert!(
        has_fractional_position || has_fractional_metric,
        "fractional advances and offsets must be preserved in logical units"
    );
}
