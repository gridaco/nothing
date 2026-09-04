//! Oracle v8 geometry and artifact identity against measured ground truth.
//!
//! The numbers here are not derived from this crate: they were measured from
//! the pinned Ahem bytes (fixtures/web-first/fonts/ahem.ttf) and verified
//! byte-exact against Chromium 149 during the text-0 probe rounds and the
//! engine-side crux spike (docs/wg/consolidation/text-oracle.md). If this
//! crate disagrees with them, the crate is wrong.

use std::sync::Arc;

use textlayout::{
    AttributedText, Environment, FontFamily, FontKey, FontResource, OutlineSink, ResolveError,
    ShapingChunk, ShapingChunkCoverageError, SourceRun, SourceRunCoverageError, SourceRunTag,
    StaticFaceDescriptor, Style, resolve,
};

const AHEM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/fonts/ahem.ttf"
);
const ALLERTA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/Allerta/Allerta-Regular.ttf"
);
const BUNGEE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/Bungee/Bungee-Regular.ttf"
);
const PT_SERIF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/PT_Serif/PTSerif-Regular.ttf"
);

/// Tests exercise resolution, not host-side digest verification, so the key
/// is a stand-in identity the artifact must nonetheless carry through.
const TEST_KEY: FontKey = FontKey::new([0xAB; 32]);
const DEFAULT_SOURCE_RUN: SourceRunTag = SourceRunTag::new(0);

fn unchecked_range(start: usize, end: usize) -> std::ops::Range<usize> {
    std::ops::Range { start, end }
}

/// Every Latin-1 letter whose canonical decomposition is exactly one ASCII
/// Latin base plus one combining mark. This is retained from v2;
/// block neighbors are tested as refusals below.
const PRECOMPOSED_LATIN_1: &str = "ÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåçèéêëìíîïñòóôõöùúûüýÿ";

fn ahem_environment() -> Environment {
    let bytes: Arc<[u8]> = std::fs::read(AHEM).expect("pinned Ahem bytes").into();
    Environment::new(vec![FontResource {
        key: TEST_KEY,
        family: "Ahem".to_string(),
        face_descriptor: StaticFaceDescriptor::NORMAL,
        face_index: 0,
        bytes,
    }])
}

fn fixture_environment(path: &str, family: &str) -> Environment {
    let bytes: Arc<[u8]> = std::fs::read(path).expect("fixture font bytes").into();
    Environment::new(vec![FontResource {
        key: TEST_KEY,
        family: family.to_string(),
        face_descriptor: StaticFaceDescriptor::NORMAL,
        face_index: 0,
        bytes,
    }])
}

fn ahem(text: &str, size: f32) -> AttributedText {
    attributed(text, "Ahem", size)
}

fn attributed(text: &str, family: &str, size: f32) -> AttributedText {
    AttributedText::single_source_run(
        text.to_string(),
        Style {
            families: vec![FontFamily::named(family)],
            face_descriptor: StaticFaceDescriptor::NORMAL,
            size,
        },
        DEFAULT_SOURCE_RUN,
    )
}

#[test]
fn x_at_50_resolves_the_measured_em_box() {
    let layout = resolve(&ahem("X", 50.0), &ahem_environment()).unwrap();

    assert_eq!(textlayout::ORACLE_VERSION, "textlayout-v8");
    assert_eq!(layout.oracle_version(), textlayout::ORACLE_VERSION);
    assert_eq!(layout.primary_face().key, TEST_KEY);
    assert_eq!(layout.primary_face().units_per_em, 1000);
    assert_eq!(layout.source_runs().len(), 1);
    assert_eq!(layout.source_runs()[0].source_utf8(), 0..1);
    assert_eq!(layout.source_runs()[0].tag(), DEFAULT_SOURCE_RUN);
    assert_eq!(layout.shaping_chunks().len(), 1);
    assert_eq!(layout.shaping_chunks()[0].source_utf8(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].source_utf16(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].source_scalars(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].clusters(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].glyphs(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].origin_x(), 0.0);
    assert_eq!(layout.shaping_chunks()[0].advance(), 50.0);

    // Measured: 'X' is glyph 58, advance 1000 font units.
    let glyphs = layout.glyphs();
    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].glyph_id, 58);
    assert_eq!(glyphs[0].x, 0.0);
    assert_eq!((glyphs[0].offset_x, glyphs[0].offset_y), (0.0, 0.0));
    assert_eq!(glyphs[0].advance, 50.0);
    assert_eq!(glyphs[0].cluster_index, 0);
    assert_eq!(layout.clusters().len(), 1);
    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[0].source_utf16(), 0..1);
    assert_eq!(layout.clusters()[0].source_scalars(), 0..1);
    assert_eq!(layout.clusters()[0].glyphs(), 0..1);
    assert_eq!(layout.clusters()[0].source_run_tag(), DEFAULT_SOURCE_RUN);
    assert_eq!(glyphs[0].source_run_tag(), DEFAULT_SOURCE_RUN);
    assert_eq!(layout.advance(), 50.0);

    // Ahem: ascent 800, descent -200, every metric policy agreeing.
    assert_eq!(layout.metrics().ascent, 40.0);
    assert_eq!(layout.metrics().descent, 10.0);

    // The em box in y-down local px: 0.8em above the baseline, 0.2em below.
    let ink = layout.ink_bounds().unwrap();
    assert_eq!(
        (ink.x, ink.y, ink.width, ink.height),
        (0.0, -40.0, 50.0, 50.0)
    );
    let logical = layout.logical_bounds();
    assert_eq!(
        (logical.x, logical.y, logical.width, logical.height),
        (0.0, -40.0, 50.0, 50.0)
    );
}

#[test]
fn space_advances_without_ink() {
    let layout = resolve(&ahem("X X", 20.0), &ahem_environment()).unwrap();

    // Measured: gids [58, 3, 58], each advancing exactly one em.
    let glyphs = layout.glyphs();
    assert_eq!(glyphs.len(), 3);
    assert_eq!(
        glyphs.iter().map(|g| g.glyph_id).collect::<Vec<_>>(),
        vec![58, 3, 58]
    );
    assert_eq!(
        glyphs.iter().map(|g| g.x).collect::<Vec<_>>(),
        vec![0.0, 20.0, 40.0]
    );
    assert_eq!(layout.advance(), 60.0);
    assert_eq!(
        glyphs.iter().map(|g| g.cluster_index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        layout
            .clusters()
            .iter()
            .map(|cluster| (
                cluster.source_utf8(),
                cluster.source_utf16(),
                cluster.source_scalars(),
                cluster.glyphs()
            ))
            .collect::<Vec<_>>(),
        vec![
            (0..1, 0..1, 0..1, 0..1),
            (1..2, 1..2, 1..2, 1..2),
            (2..3, 2..3, 2..3, 2..3)
        ]
    );

    // Ink spans both boxes; the space contributes advance only.
    let ink = layout.ink_bounds().unwrap();
    assert_eq!(
        (ink.x, ink.y, ink.width, ink.height),
        (0.0, -16.0, 60.0, 20.0)
    );

    // The space glyph refuses to stream an outline.
    let mut sink = CollectSink::default();
    assert!(!layout.outline(1, &mut sink));
    assert!(sink.points.is_empty() && sink.moves == 0 && sink.closes == 0);
}

#[test]
fn outline_streams_the_flipped_box() {
    let layout = resolve(&ahem("X", 50.0), &ahem_environment()).unwrap();
    let mut sink = CollectSink::default();
    assert!(layout.outline(0, &mut sink));

    // One rectangular contour: a move, straight lines, one close.
    assert_eq!(sink.moves, 1);
    assert!(sink.lines >= 3, "a box outline draws at least three sides");
    assert_eq!(sink.closes, 1);
    assert_eq!(sink.curves, 0, "Ahem's box has no curves");

    // Every vertex lies on the em box's edges in y-down px — the y-flip is
    // the classic silent-wrong-pixel trap, so it is pinned here.
    for &(x, y) in &sink.points {
        assert!(x == 0.0 || x == 50.0, "vertex x {x} off the box");
        assert!(
            y == -40.0 || y == 10.0,
            "vertex y {y} off the box (flip broken?)"
        );
    }
    let xs: Vec<f32> = sink.points.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = sink.points.iter().map(|p| p.1).collect();
    assert!(xs.contains(&0.0) && xs.contains(&50.0));
    assert!(ys.contains(&-40.0) && ys.contains(&10.0));
}

/// In-process purity only: cross-run and cross-upgrade stability is pinned
/// by the measured-ground-truth assertions above and the exact shaper pin.
#[test]
fn resolution_is_deterministic() {
    let env = ahem_environment();
    let text = ahem("X X", 20.0);
    let first = resolve(&text, &env).unwrap();
    let second = resolve(&text, &env).unwrap();
    let explicit_whole_source = resolve(
        &text
            .clone()
            .with_shaping_chunks(vec![ShapingChunk::new(0..text.text().len())]),
        &env,
    )
    .unwrap();
    assert_eq!(first.glyphs(), second.glyphs());
    assert_eq!(first.clusters(), second.clusters());
    assert_eq!(first.shaping_chunks(), second.shaping_chunks());
    assert_eq!(first.advance(), second.advance());
    assert_eq!(first.ink_bounds(), second.ink_bounds());
    assert_eq!(first.metrics(), second.metrics());
    assert_eq!(first.glyphs(), explicit_whole_source.glyphs());
    assert_eq!(first.clusters(), explicit_whole_source.clusters());
    assert_eq!(
        first.shaping_chunks(),
        explicit_whole_source.shaping_chunks()
    );
    assert_eq!(first.advance(), explicit_whole_source.advance());
    assert_eq!(first.ink_bounds(), explicit_whole_source.ink_bounds());
}

#[test]
fn empty_text_resolves_to_metrics_without_glyphs() {
    let layout = resolve(&ahem("", 20.0), &ahem_environment()).unwrap();
    assert!(layout.source_runs().is_empty());
    assert!(layout.shaping_chunks().is_empty());
    assert!(layout.glyphs().is_empty());
    assert!(layout.clusters().is_empty());
    assert_eq!(layout.advance(), 0.0);
    assert!(layout.ink_bounds().is_none());
    // The empty run still carries the line's typographic extent.
    assert_eq!(layout.logical_bounds().height, 20.0);
}

#[test]
fn default_pair_kerning_preserves_direct_cluster_mapping() {
    let layout = resolve(
        &attributed("ff", "Allerta", 5120.0),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("one-to-one kerning remains inside oracle v8");

    assert_eq!(layout.advance(), 4685.0);
    assert_eq!(layout.glyphs().len(), 2);
    assert_eq!(layout.clusters().len(), 2);
    assert_eq!(layout.glyphs()[0].glyph_id, 70);
    assert_eq!(layout.glyphs()[0].advance, 2330.0);
    assert_eq!(layout.glyphs()[1].glyph_id, 70);
    assert_eq!(layout.glyphs()[1].advance, 2355.0);
    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[0].source_utf16(), 0..1);
    assert_eq!(layout.clusters()[0].source_scalars(), 0..1);
    assert_eq!(layout.clusters()[0].glyphs(), 0..1);
    assert_eq!(layout.clusters()[1].source_utf8(), 1..2);
    assert_eq!(layout.clusters()[1].source_utf16(), 1..2);
    assert_eq!(layout.clusters()[1].source_scalars(), 1..2);
    assert_eq!(layout.clusters()[1].glyphs(), 1..2);
    assert_eq!(layout.shaping_chunks().len(), 1);
    let chunk = &layout.shaping_chunks()[0];
    assert_eq!(chunk.source_utf8(), 0..2);
    assert_eq!(chunk.source_utf16(), 0..2);
    assert_eq!(chunk.source_scalars(), 0..2);
    assert_eq!(chunk.clusters(), 0..2);
    assert_eq!(chunk.glyphs(), 0..2);
    assert_eq!(chunk.origin_x(), 0.0);
    assert_eq!(chunk.advance(), 4685.0);
}

#[test]
fn explicit_chunks_suppress_cross_boundary_kerning_and_keep_global_mappings() {
    let first = SourceRunTag::new(40);
    let second = SourceRunTag::new(41);
    let layout = resolve(
        &AttributedText::new(
            "ff".to_string(),
            Style {
                families: vec![FontFamily::named("Allerta")],
                face_descriptor: StaticFaceDescriptor::NORMAL,
                size: 5120.0,
            },
            vec![SourceRun::new(0..1, first), SourceRun::new(1..2, second)],
        )
        .with_shaping_chunks(vec![ShapingChunk::new(0..1), ShapingChunk::new(1..2)]),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("two complete chunks shape independently");

    // The one-chunk control above is 2330 + 2355 = 4685. Independent
    // one-scalar operations suppress pair positioning at the boundary.
    assert_eq!(layout.advance(), 4710.0);
    assert_eq!(layout.glyphs().len(), 2);
    assert_eq!(layout.glyphs()[0].glyph_id, 70);
    assert_eq!(layout.glyphs()[0].x, 0.0);
    assert_eq!(layout.glyphs()[0].advance, 2355.0);
    assert_eq!(layout.glyphs()[0].source_run_tag(), first);
    assert_eq!(layout.glyphs()[1].glyph_id, 70);
    assert_eq!(layout.glyphs()[1].x, 2355.0);
    assert_eq!(layout.glyphs()[1].advance, 2355.0);
    assert_eq!(layout.glyphs()[1].source_run_tag(), second);
    assert_eq!(layout.clusters()[0].source_run_tag(), first);
    assert_eq!(layout.clusters()[1].source_run_tag(), second);
    assert_eq!(
        layout
            .glyph_indices_for_source_run(first)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert_eq!(
        layout
            .glyph_indices_for_source_run(second)
            .collect::<Vec<_>>(),
        vec![1]
    );

    let chunks = layout.shaping_chunks();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].source_utf8(), 0..1);
    assert_eq!(chunks[0].source_utf16(), 0..1);
    assert_eq!(chunks[0].source_scalars(), 0..1);
    assert_eq!(chunks[0].clusters(), 0..1);
    assert_eq!(chunks[0].glyphs(), 0..1);
    assert_eq!(chunks[0].origin_x(), 0.0);
    assert_eq!(chunks[0].advance(), 2355.0);
    assert_eq!(chunks[1].source_utf8(), 1..2);
    assert_eq!(chunks[1].source_utf16(), 1..2);
    assert_eq!(chunks[1].source_scalars(), 1..2);
    assert_eq!(chunks[1].clusters(), 1..2);
    assert_eq!(chunks[1].glyphs(), 1..2);
    assert_eq!(chunks[1].origin_x(), 2355.0);
    assert_eq!(chunks[1].advance(), 2355.0);
    assert_eq!(
        chunks.iter().map(|chunk| chunk.advance()).sum::<f32>(),
        4710.0
    );
}

#[test]
fn allerta_source_run_boundary_preserves_one_shaping_result_and_maps_glyphs() {
    let first = SourceRunTag::new(10);
    let second = SourceRunTag::new(11);
    let layout = resolve(
        &AttributedText::new(
            "ff".to_string(),
            Style {
                families: vec![FontFamily::named("Allerta")],
                face_descriptor: StaticFaceDescriptor::NORMAL,
                size: 5120.0,
            },
            vec![SourceRun::new(0..1, first), SourceRun::new(1..2, second)],
        ),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("a metadata-only source boundary must not split shaping");

    // Chromium 149 and the pinned shaper agree on this one-call result. If
    // each source run were shaped independently, the first advance would be
    // 2355 instead of the measured kerned 2330.
    assert_eq!(layout.oracle_version(), "textlayout-v8");
    assert_eq!(layout.advance(), 4685.0);
    assert_eq!(layout.glyphs().len(), 2);
    assert_eq!(layout.glyphs()[0].glyph_id, 70);
    assert_eq!(layout.glyphs()[0].advance, 2330.0);
    assert_eq!(layout.glyphs()[1].glyph_id, 70);
    assert_eq!(layout.glyphs()[1].advance, 2355.0);

    assert_eq!(layout.source_runs()[0].source_utf8(), 0..1);
    assert_eq!(layout.source_runs()[0].tag(), first);
    assert_eq!(layout.source_runs()[1].source_utf8(), 1..2);
    assert_eq!(layout.source_runs()[1].tag(), second);
    assert_eq!(layout.clusters()[0].source_run_tag(), first);
    assert_eq!(layout.clusters()[1].source_run_tag(), second);
    assert_eq!(layout.glyphs()[0].source_run_tag(), first);
    assert_eq!(layout.glyphs()[1].source_run_tag(), second);
    assert_eq!(
        layout
            .glyph_indices_for_source_run(first)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert_eq!(
        layout
            .glyph_indices_for_source_run(second)
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn merged_ligature_cluster_refuses_before_it_can_poison_source_geometry() {
    let error = resolve(
        &attributed("fi", "PT Serif", 5000.0),
        &fixture_environment(PT_SERIF, "PT Serif"),
    )
    .expect_err("one ligature glyph cannot masquerade as two source characters");

    assert_eq!(
        error,
        ResolveError::UnsupportedClusterMapping {
            source_utf8_start: 0,
            source_utf8_end: 2,
            glyph_start: 0,
            glyph_end: 1,
        }
    );
}

#[test]
fn precomposed_latin_carries_distinct_utf8_and_utf16_source_ranges() {
    let source = format!("A{PRECOMPOSED_LATIN_1}Z");
    assert_eq!(PRECOMPOSED_LATIN_1.chars().count(), 53);
    assert_eq!(PRECOMPOSED_LATIN_1.len(), 106);

    let layout = resolve(
        &AttributedText::single_source_run(
            source.clone(),
            Style {
                families: vec![FontFamily::named("Allerta")],
                face_descriptor: StaticFaceDescriptor::NORMAL,
                size: 5120.0,
            },
            DEFAULT_SOURCE_RUN,
        ),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("every measured precomposed Latin-1 member is direct in Allerta");

    // Chromium 149 measured these exact advances at 5120px. `hb-shape`
    // against the same pinned bytes independently reported the same glyph
    // identities, placements, and UTF-8 clusters.
    let expected_glyph_ids = [
        35, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 119, 120,
        121, 122, 123, 124, 126, 127, 128, 129, 130, 133, 134, 135, 136, 137, 138, 139, 140, 141,
        142, 143, 144, 145, 146, 147, 149, 150, 151, 152, 153, 154, 156, 157, 158, 159, 160, 162,
        60,
    ];
    let expected_advances = [
        3795.0, 3795.0, 3795.0, 3795.0, 3795.0, 3795.0, 3795.0, 3355.0, 3140.0, 3140.0, 3140.0,
        3140.0, 1540.0, 1540.0, 1540.0, 1540.0, 3580.0, 3840.0, 3840.0, 3840.0, 3840.0, 3840.0,
        3685.0, 3685.0, 3685.0, 3685.0, 3275.0, 3000.0, 3000.0, 3000.0, 3000.0, 3000.0, 3000.0,
        2925.0, 3320.0, 3320.0, 3320.0, 3320.0, 1455.0, 1455.0, 1455.0, 1455.0, 3170.0, 3310.0,
        3310.0, 3310.0, 3310.0, 3310.0, 3165.0, 3165.0, 3165.0, 3165.0, 3360.0, 3360.0, 3225.0,
    ];
    assert_eq!(layout.glyphs().len(), source.chars().count());
    assert_eq!(layout.clusters().len(), source.chars().count());
    assert_eq!(layout.advance(), 171785.0);

    let mut utf16_start = 0;
    let mut pen_x = 0.0;
    for (index, (source_utf8_start, scalar)) in source.char_indices().enumerate() {
        let source_utf8_end = source_utf8_start + scalar.len_utf8();
        let source_utf16_end = utf16_start + scalar.len_utf16();
        let cluster = &layout.clusters()[index];
        let glyph = &layout.glyphs()[index];

        assert_eq!(cluster.source_utf8(), source_utf8_start..source_utf8_end);
        assert_eq!(cluster.source_utf16(), utf16_start..source_utf16_end);
        assert_eq!(cluster.source_scalars(), index..index + 1);
        assert_eq!(cluster.glyphs(), index..index + 1);
        assert_eq!(glyph.cluster_index, index);
        assert_eq!(glyph.glyph_id, expected_glyph_ids[index]);
        assert_eq!(glyph.x, pen_x);
        assert_eq!(glyph.advance, expected_advances[index]);

        utf16_start = source_utf16_end;
        pen_x += glyph.advance;
    }
    assert_eq!(utf16_start, 55);
    assert_eq!(pen_x, layout.advance());
    assert_eq!(layout.clusters()[1].source_utf8(), 1..3);
    assert_eq!(layout.clusters()[1].source_utf16(), 1..2);
    assert_eq!(layout.clusters()[54].source_utf8(), 107..108);
    assert_eq!(layout.clusters()[54].source_utf16(), 54..55);
}

#[test]
fn chunk_records_promote_multibyte_source_coordinates_to_global_indices() {
    let layout = resolve(
        &attributed("AéZ", "Allerta", 5120.0).with_shaping_chunks(vec![
            ShapingChunk::new(0..1),
            ShapingChunk::new(1..3),
            ShapingChunk::new(3..4),
        ]),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("scalar-aligned multibyte chunks resolve");

    let chunks = layout.shaping_chunks();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].source_utf8(), 0..1);
    assert_eq!(chunks[0].source_utf16(), 0..1);
    assert_eq!(chunks[0].source_scalars(), 0..1);
    assert_eq!(chunks[0].clusters(), 0..1);
    assert_eq!(chunks[0].glyphs(), 0..1);
    assert_eq!(chunks[1].source_utf8(), 1..3);
    assert_eq!(chunks[1].source_utf16(), 1..2);
    assert_eq!(chunks[1].source_scalars(), 1..2);
    assert_eq!(chunks[1].clusters(), 1..2);
    assert_eq!(chunks[1].glyphs(), 1..2);
    assert_eq!(chunks[2].source_utf8(), 3..4);
    assert_eq!(chunks[2].source_utf16(), 2..3);
    assert_eq!(chunks[2].source_scalars(), 2..3);
    assert_eq!(chunks[2].clusters(), 2..3);
    assert_eq!(chunks[2].glyphs(), 2..3);

    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[1].source_utf8(), 1..3);
    assert_eq!(layout.clusters()[2].source_utf8(), 3..4);
    assert_eq!(chunks[1].origin_x(), chunks[0].advance());
    assert_eq!(
        chunks[2].origin_x(),
        chunks[0].advance() + chunks[1].advance()
    );
    assert_eq!(
        layout.advance(),
        chunks.iter().map(|chunk| chunk.advance()).sum::<f32>()
    );
}

#[test]
fn decomposed_acute_composes_without_rewriting_source_coordinates() {
    let layout = resolve(
        &attributed("Ae\u{0301}Z", "Allerta", 5120.0),
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("the measured decomposed acute composes inside oracle v8");

    assert_eq!(layout.source(), "Ae\u{0301}Z");
    assert_eq!(layout.advance(), 10340.0);
    assert_eq!(layout.glyphs().len(), 3);
    assert_eq!(layout.clusters().len(), 3);
    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[0].source_utf16(), 0..1);
    assert_eq!(layout.clusters()[0].source_scalars(), 0..1);
    assert_eq!(layout.clusters()[0].glyphs(), 0..1);
    assert_eq!(layout.clusters()[1].source_utf8(), 1..4);
    assert_eq!(layout.clusters()[1].source_utf16(), 1..3);
    assert_eq!(layout.clusters()[1].source_scalars(), 1..3);
    assert_eq!(layout.clusters()[1].glyphs(), 1..2);
    assert_eq!(layout.clusters()[2].source_utf8(), 4..5);
    assert_eq!(layout.clusters()[2].source_utf16(), 3..4);
    assert_eq!(layout.clusters()[2].source_scalars(), 3..4);
    assert_eq!(layout.clusters()[2].glyphs(), 2..3);
    assert_eq!(
        layout
            .glyphs()
            .iter()
            .map(|glyph| (
                glyph.glyph_id,
                glyph.x,
                glyph.offset_x,
                glyph.offset_y,
                glyph.advance,
                glyph.cluster_index,
            ))
            .collect::<Vec<_>>(),
        vec![
            (35, 0.0, 0.0, 0.0, 3795.0, 0),
            (141, 3795.0, 0.0, 0.0, 3320.0, 1),
            (60, 7115.0, 0.0, 0.0, 3225.0, 2),
        ]
    );
    let ink = layout.ink_bounds().unwrap();
    assert_eq!(
        (ink.x, ink.y, ink.width, ink.height),
        (315.0, -3885.0, 9640.0, 3905.0)
    );
}

#[test]
fn attached_marks_carry_pen_independent_x_and_y_offsets() {
    let environment = fixture_environment(BUNGEE, "Bungee");
    let resolve_mark = |mark| {
        resolve(
            &AttributedText::single_source_run(
                format!("Ax{mark}Z"),
                Style {
                    families: vec![FontFamily::named("Bungee")],
                    face_descriptor: StaticFaceDescriptor::NORMAL,
                    size: 1000.0,
                },
                DEFAULT_SOURCE_RUN,
            ),
            &environment,
        )
        .expect("the measured Bungee attachment resolves")
    };

    let acute = resolve_mark('\u{0301}');
    assert_eq!(acute.source(), "Ax\u{0301}Z");
    assert_eq!(acute.advance(), 2127.0);
    assert_eq!(acute.glyphs().len(), 4);
    assert_eq!(acute.clusters().len(), 3);
    assert_eq!(acute.clusters()[1].source_utf8(), 1..4);
    assert_eq!(acute.clusters()[1].source_utf16(), 1..3);
    assert_eq!(acute.clusters()[1].source_scalars(), 1..3);
    assert_eq!(acute.clusters()[1].glyphs(), 1..3);
    assert_eq!(
        acute
            .glyphs()
            .iter()
            .map(|glyph| (
                glyph.glyph_id,
                glyph.x,
                glyph.offset_x,
                glyph.offset_y,
                glyph.advance,
                glyph.cluster_index,
            ))
            .collect::<Vec<_>>(),
        vec![
            (2, 0.0, 0.0, 0.0, 730.0, 0),
            (773, 730.0, 0.0, 0.0, 737.0, 1),
            (975, 1467.0, -369.0, 0.0, 0.0, 1),
            (110, 1467.0, 0.0, 0.0, 660.0, 2),
        ]
    );
    let ink = acute.ink_bounds().unwrap();
    assert_eq!(
        (ink.x, ink.y, ink.width, ink.height),
        (54.0, -961.3902, 2027.0, 961.3902)
    );

    let double_acute = resolve_mark('\u{030B}');
    let mark = double_acute.glyphs()[2];
    assert_eq!(
        (
            mark.glyph_id,
            mark.x,
            mark.offset_x,
            mark.offset_y,
            mark.advance,
            mark.cluster_index,
        ),
        (984, 1467.0, -369.0, -7.0, 0.0, 1)
    );
    let ink = double_acute.ink_bounds().unwrap();
    assert_eq!(
        (ink.x, ink.y, ink.width, ink.height),
        (54.0, -1044.0, 2027.0, 1044.0)
    );
}

#[test]
fn bungee_cluster_crossing_a_source_run_uses_the_first_scalar_tag() {
    let prefix = SourceRunTag::new(20);
    let base = SourceRunTag::new(21);
    let mark = SourceRunTag::new(22);
    let suffix = SourceRunTag::new(23);
    let layout = resolve(
        &AttributedText::new(
            "Ax\u{0301}Z".to_string(),
            Style {
                families: vec![FontFamily::named("Bungee")],
                face_descriptor: StaticFaceDescriptor::NORMAL,
                size: 1000.0,
            },
            vec![
                SourceRun::new(0..1, prefix),
                SourceRun::new(1..2, base),
                SourceRun::new(2..4, mark),
                SourceRun::new(4..5, suffix),
            ],
        ),
        &fixture_environment(BUNGEE, "Bungee"),
    )
    .expect("a run boundary inside the measured base-plus-mark cluster is transparent");

    // The measured Bungee answer is one two-glyph cluster over x + U+0301.
    // Both placed glyphs belong to the tag covering the cluster's first
    // scalar; the mark-only source run intentionally owns no glyph.
    assert_eq!(layout.advance(), 2127.0);
    assert_eq!(layout.clusters().len(), 3);
    assert_eq!(layout.glyphs().len(), 4);
    assert_eq!(layout.clusters()[1].source_utf8(), 1..4);
    assert_eq!(layout.clusters()[1].glyphs(), 1..3);
    assert_eq!(layout.clusters()[1].source_run_tag(), base);
    assert_eq!(layout.glyphs()[1].source_run_tag(), base);
    assert_eq!(layout.glyphs()[2].source_run_tag(), base);
    assert_eq!(
        layout
            .glyph_indices_for_source_run(base)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(layout.glyph_indices_for_source_run(mark).next().is_none());

    assert_eq!(
        layout
            .glyphs()
            .iter()
            .map(|glyph| (
                glyph.glyph_id,
                glyph.x,
                glyph.offset_x,
                glyph.offset_y,
                glyph.advance,
                glyph.cluster_index,
            ))
            .collect::<Vec<_>>(),
        vec![
            (2, 0.0, 0.0, 0.0, 730.0, 0),
            (773, 730.0, 0.0, 0.0, 737.0, 1),
            (975, 1467.0, -369.0, 0.0, 0.0, 1),
            (110, 1467.0, 0.0, 0.0, 660.0, 2),
        ]
    );
}

#[test]
fn chunks_may_surround_but_not_split_an_admitted_combining_cluster() {
    let prefix = SourceRunTag::new(50);
    let base = SourceRunTag::new(51);
    let mark = SourceRunTag::new(52);
    let suffix = SourceRunTag::new(53);
    let attributed = AttributedText::new(
        "Ax\u{0301}Z".to_string(),
        Style {
            families: vec![FontFamily::named("Bungee")],
            face_descriptor: StaticFaceDescriptor::NORMAL,
            size: 1000.0,
        },
        vec![
            SourceRun::new(0..1, prefix),
            SourceRun::new(1..2, base),
            SourceRun::new(2..4, mark),
            SourceRun::new(4..5, suffix),
        ],
    );
    let environment = fixture_environment(BUNGEE, "Bungee");
    let layout = resolve(
        &attributed.clone().with_shaping_chunks(vec![
            ShapingChunk::new(0..1),
            ShapingChunk::new(1..4),
            ShapingChunk::new(4..5),
        ]),
        &environment,
    )
    .expect("boundaries around a complete base-plus-mark cluster are valid");

    assert_eq!(layout.advance(), 2127.0);
    assert_eq!(layout.shaping_chunks().len(), 3);
    assert_eq!(layout.shaping_chunks()[0].clusters(), 0..1);
    assert_eq!(layout.shaping_chunks()[0].glyphs(), 0..1);
    assert_eq!(layout.shaping_chunks()[1].source_utf8(), 1..4);
    assert_eq!(layout.shaping_chunks()[1].source_utf16(), 1..3);
    assert_eq!(layout.shaping_chunks()[1].source_scalars(), 1..3);
    assert_eq!(layout.shaping_chunks()[1].clusters(), 1..2);
    assert_eq!(layout.shaping_chunks()[1].glyphs(), 1..3);
    assert_eq!(layout.shaping_chunks()[1].origin_x(), 730.0);
    assert_eq!(layout.shaping_chunks()[1].advance(), 737.0);
    assert_eq!(layout.shaping_chunks()[2].clusters(), 2..3);
    assert_eq!(layout.shaping_chunks()[2].glyphs(), 3..4);
    assert_eq!(layout.clusters()[1].source_run_tag(), base);
    assert_eq!(layout.glyphs()[1].source_run_tag(), base);
    assert_eq!(layout.glyphs()[2].source_run_tag(), base);
    assert!(layout.glyph_indices_for_source_run(mark).next().is_none());

    let error = resolve(
        &attributed.with_shaping_chunks(vec![
            ShapingChunk::new(0..2),
            ShapingChunk::new(2..4),
            ShapingChunk::new(4..5),
        ]),
        &environment,
    )
    .expect_err("a boundary inside the admitted base-plus-mark cluster must refuse");
    assert_eq!(
        error,
        ResolveError::UnsupportedClusterMapping {
            source_utf8_start: 2,
            source_utf8_end: 4,
            glyph_start: 2,
            glyph_end: 3,
        }
    );
}

#[test]
fn missing_combining_glyph_names_the_mark_not_its_base() {
    let error = resolve(&ahem("Ax\u{0301}Z", 1000.0), &ahem_environment())
        .expect_err("Ahem has no attachable acute for x");
    assert_eq!(
        error,
        ResolveError::MissingGlyph {
            byte_index: 2,
            character: '\u{0301}',
        }
    );
}

#[test]
fn malformed_source_run_coverage_refuses_by_typed_reason_before_font_work() {
    let tag = SourceRunTag::new(30);
    let coverage_error = |source: &str, runs: Vec<SourceRun>| {
        let error = resolve(
            &AttributedText::new(
                source.to_string(),
                Style {
                    families: vec![FontFamily::named("deliberately undeclared")],
                    face_descriptor: StaticFaceDescriptor::NORMAL,
                    size: 20.0,
                },
                runs,
            ),
            &Environment::default(),
        )
        .expect_err("invalid coverage must refuse before font lookup or shaping");
        match error {
            ResolveError::InvalidSourceRunCoverage(reason) => reason,
            other => panic!("expected source-run coverage error, got {other:?}"),
        }
    };

    assert_eq!(
        coverage_error("AB", vec![]),
        SourceRunCoverageError::Missing { source_len: 2 }
    );
    assert_eq!(
        coverage_error("AB", vec![SourceRun::new(unchecked_range(1, 0), tag)]),
        SourceRunCoverageError::Reversed {
            run_index: 0,
            start: 1,
            end: 0,
        }
    );
    assert_eq!(
        coverage_error("AB", vec![SourceRun::new(0..0, tag)]),
        SourceRunCoverageError::Empty {
            run_index: 0,
            byte_index: 0,
        }
    );
    assert_eq!(
        coverage_error("AB", vec![SourceRun::new(0..3, tag)]),
        SourceRunCoverageError::OutOfBounds {
            run_index: 0,
            start: 0,
            end: 3,
            source_len: 2,
        }
    );
    assert_eq!(
        coverage_error("AéZ", vec![SourceRun::new(0..2, tag)]),
        SourceRunCoverageError::NotScalarBoundary {
            run_index: 0,
            byte_index: 2,
        }
    );
    assert_eq!(
        coverage_error(
            "ABC",
            vec![SourceRun::new(0..1, tag), SourceRun::new(2..3, tag)],
        ),
        SourceRunCoverageError::Gap {
            run_index: 1,
            expected_start: 1,
            actual_start: 2,
        }
    );
    assert_eq!(
        coverage_error(
            "ABC",
            vec![SourceRun::new(0..2, tag), SourceRun::new(1..3, tag)],
        ),
        SourceRunCoverageError::Overlap {
            run_index: 1,
            previous_end: 2,
            actual_start: 1,
        }
    );
    assert_eq!(
        coverage_error("ABC", vec![SourceRun::new(0..1, tag)]),
        SourceRunCoverageError::Incomplete {
            covered_end: 1,
            source_len: 3,
        }
    );
}

#[test]
fn malformed_shaping_chunk_coverage_refuses_by_typed_reason_before_font_work() {
    let coverage_error = |source: &str, chunks: Vec<ShapingChunk>| {
        let error = resolve(
            &AttributedText::single_source_run(
                source.to_string(),
                Style {
                    families: vec![FontFamily::named("deliberately undeclared")],
                    face_descriptor: StaticFaceDescriptor::NORMAL,
                    size: 20.0,
                },
                DEFAULT_SOURCE_RUN,
            )
            .with_shaping_chunks(chunks),
            &Environment::default(),
        )
        .expect_err("invalid shaping chunks must refuse before font lookup or shaping");
        match error {
            ResolveError::InvalidShapingChunkCoverage(reason) => reason,
            other => panic!("expected shaping-chunk coverage error, got {other:?}"),
        }
    };

    assert_eq!(
        coverage_error("AB", vec![]),
        ShapingChunkCoverageError::Missing { source_len: 2 }
    );
    assert_eq!(
        coverage_error("AB", vec![ShapingChunk::new(unchecked_range(1, 0))]),
        ShapingChunkCoverageError::Reversed {
            chunk_index: 0,
            start: 1,
            end: 0,
        }
    );
    assert_eq!(
        coverage_error("AB", vec![ShapingChunk::new(0..0)]),
        ShapingChunkCoverageError::Empty {
            chunk_index: 0,
            byte_index: 0,
        }
    );
    assert_eq!(
        coverage_error("AB", vec![ShapingChunk::new(0..3)]),
        ShapingChunkCoverageError::OutOfBounds {
            chunk_index: 0,
            start: 0,
            end: 3,
            source_len: 2,
        }
    );
    assert_eq!(
        coverage_error(
            "AéZ",
            vec![ShapingChunk::new(0..2), ShapingChunk::new(2..4)],
        ),
        ShapingChunkCoverageError::NotScalarBoundary {
            chunk_index: 0,
            byte_index: 2,
        }
    );
    assert_eq!(
        coverage_error(
            "ABC",
            vec![ShapingChunk::new(0..1), ShapingChunk::new(2..3)],
        ),
        ShapingChunkCoverageError::Gap {
            chunk_index: 1,
            expected_start: 1,
            actual_start: 2,
        }
    );
    assert_eq!(
        coverage_error(
            "ABC",
            vec![ShapingChunk::new(0..2), ShapingChunk::new(1..3)],
        ),
        ShapingChunkCoverageError::Overlap {
            chunk_index: 1,
            previous_end: 2,
            actual_start: 1,
        }
    );
    assert_eq!(
        coverage_error("ABC", vec![ShapingChunk::new(0..1)]),
        ShapingChunkCoverageError::Incomplete {
            covered_end: 1,
            source_len: 3,
        }
    );
}

#[test]
fn malformed_and_unadmitted_mark_sequences_refuse_before_shaping() {
    let environment = fixture_environment(BUNGEE, "Bungee");
    for (source, byte_index, character) in [
        ("\u{0301}AX", 0, '\u{0301}'),
        ("Ax\u{0301}\u{0301}Z", 4, '\u{0301}'),
        ("A1\u{0301}Z", 2, '\u{0301}'),
        ("Aé\u{0301}Z", 3, '\u{0301}'),
    ] {
        let error = resolve(&attributed(source, "Bungee", 1000.0), &environment)
            .expect_err("malformed combining sequence must refuse");
        assert_eq!(
            error,
            ResolveError::UnsupportedCombiningSequence {
                byte_index,
                character,
            },
            "{source:?}"
        );
    }

    let error = resolve(&attributed("Ax\u{0300}Z", "Bungee", 1000.0), &environment)
        .expect_err("an unlisted combining mark stays outside the repertoire");
    assert_eq!(
        error,
        ResolveError::UnsupportedCharacter {
            byte_index: 2,
            character: '\u{0300}',
        }
    );
}

#[test]
fn undeclared_family_list_refuses_by_names() {
    let err = resolve(&ahem("X", 20.0), &Environment::default()).unwrap_err();
    assert_eq!(
        err,
        ResolveError::NoMatchingFamily {
            families: vec!["Ahem".to_string()]
        }
    );
}

#[test]
fn the_profile_is_the_resolvers_property_not_the_fonts() {
    // Every class of out-of-profile character refuses by position at the
    // admit-list, before any font's coverage can decide differently:
    // a control, a strong-RTL letter, a bidi override, a Zl separator, a
    // default-ignorable the shaper would silently substitute, and an emoji.
    for (text, byte_index, character) in [
        ("X\nX", 1, '\n'),
        ("X\u{05D0}", 1, '\u{05D0}'),
        ("ab\u{202E}cd", 2, '\u{202E}'),
        ("a\u{2028}b", 1, '\u{2028}'),
        ("X\u{200B}Y", 1, '\u{200B}'),
        ("X\u{00AD}Y", 1, '\u{00AD}'),
        ("X\u{1F600}", 1, '\u{1F600}'),
        ("X\u{00A0}Y", 1, '\u{00A0}'),
        ("X\u{00AA}Y", 1, '\u{00AA}'),
        ("X\u{00C6}Y", 1, '\u{00C6}'),
        ("X\u{00D0}Y", 1, '\u{00D0}'),
        ("X\u{00D7}Y", 1, '\u{00D7}'),
        ("X\u{00D8}Y", 1, '\u{00D8}'),
        ("X\u{00DE}Y", 1, '\u{00DE}'),
        ("X\u{00DF}Y", 1, '\u{00DF}'),
        ("X\u{00E6}Y", 1, '\u{00E6}'),
        ("X\u{00F0}Y", 1, '\u{00F0}'),
        ("X\u{00F7}Y", 1, '\u{00F7}'),
        ("X\u{00F8}Y", 1, '\u{00F8}'),
        ("X\u{00FE}Y", 1, '\u{00FE}'),
        ("Ae\u{0300}Z", 2, '\u{0300}'),
        ("X\u{0100}Y", 1, '\u{0100}'),
    ] {
        let err = resolve(&ahem(text, 20.0), &ahem_environment()).unwrap_err();
        assert_eq!(
            err,
            ResolveError::UnsupportedCharacter {
                byte_index,
                character
            },
            "{text:?} must refuse at the profile guard"
        );
    }
}

// The missing combining-mark path is pinned above. A direct-scalar missing
// glyph remains unreachable through the current fixture identities: Ahem
// covers printable ASCII and Allerta covers the admitted Latin extension.

#[test]
fn invalid_sizes_refuse() {
    let env = ahem_environment();
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let err = resolve(&ahem("X", size), &env).unwrap_err();
        assert!(matches!(err, ResolveError::InvalidFontSize { .. }));
    }
}

#[derive(Default)]
struct CollectSink {
    moves: usize,
    lines: usize,
    curves: usize,
    closes: usize,
    points: Vec<(f32, f32)>,
}

impl OutlineSink for CollectSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.moves += 1;
        self.points.push((x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.lines += 1;
        self.points.push((x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.curves += 1;
        self.points.extend([(x1, y1), (x, y)]);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.curves += 1;
        self.points.extend([(x1, y1), (x2, y2), (x, y)]);
    }
    fn close(&mut self) {
        self.closes += 1;
    }
}
