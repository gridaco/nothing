//! Oracle v3 against measured ground truth.
//!
//! The numbers here are not derived from this crate: they were measured from
//! the pinned Ahem bytes (fixtures/web-first/fonts/ahem.ttf) and verified
//! byte-exact against Chromium 149 during the text-0 probe rounds and the
//! engine-side crux spike (docs/wg/consolidation/text-oracle.md). If this
//! crate disagrees with them, the crate is wrong.

use std::sync::Arc;

use textlayout::{
    AttributedText, Environment, FontKey, FontResource, OutlineSink, ResolveError, Style, resolve,
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

/// Every Latin-1 letter whose canonical decomposition is exactly one ASCII
/// Latin base plus one combining mark. This is retained from v2;
/// block neighbors are tested as refusals below.
const PRECOMPOSED_LATIN_1: &str = "ÀÁÂÃÄÅÇÈÉÊËÌÍÎÏÑÒÓÔÕÖÙÚÛÜÝàáâãäåçèéêëìíîïñòóôõöùúûüýÿ";

fn ahem_environment() -> Environment {
    let bytes: Arc<[u8]> = std::fs::read(AHEM).expect("pinned Ahem bytes").into();
    Environment::new(vec![FontResource {
        key: TEST_KEY,
        family: "Ahem".to_string(),
        face_index: 0,
        bytes,
    }])
}

fn fixture_environment(path: &str, family: &str) -> Environment {
    let bytes: Arc<[u8]> = std::fs::read(path).expect("fixture font bytes").into();
    Environment::new(vec![FontResource {
        key: TEST_KEY,
        family: family.to_string(),
        face_index: 0,
        bytes,
    }])
}

fn ahem(text: &str, size: f32) -> AttributedText {
    AttributedText {
        text: text.to_string(),
        style: Style {
            family: "Ahem".to_string(),
            size,
        },
    }
}

#[test]
fn x_at_50_resolves_the_measured_em_box() {
    let layout = resolve(&ahem("X", 50.0), &ahem_environment()).unwrap();

    assert_eq!(textlayout::ORACLE_VERSION, "textlayout-v3");
    assert_eq!(layout.oracle_version(), textlayout::ORACLE_VERSION);
    assert_eq!(layout.face().key, TEST_KEY);
    assert_eq!(layout.face().units_per_em, 1000);

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
    assert_eq!(first.glyphs(), second.glyphs());
    assert_eq!(first.clusters(), second.clusters());
    assert_eq!(first.advance(), second.advance());
    assert_eq!(first.ink_bounds(), second.ink_bounds());
    assert_eq!(first.metrics(), second.metrics());
}

#[test]
fn empty_text_resolves_to_metrics_without_glyphs() {
    let layout = resolve(&ahem("", 20.0), &ahem_environment()).unwrap();
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
        &AttributedText {
            text: "ff".to_string(),
            style: Style {
                family: "Allerta".to_string(),
                size: 5120.0,
            },
        },
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("one-to-one kerning remains inside oracle v3");

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
}

#[test]
fn merged_ligature_cluster_refuses_before_it_can_poison_source_geometry() {
    let error = resolve(
        &AttributedText {
            text: "fi".to_string(),
            style: Style {
                family: "PT Serif".to_string(),
                size: 5000.0,
            },
        },
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
        &AttributedText {
            text: source.clone(),
            style: Style {
                family: "Allerta".to_string(),
                size: 5120.0,
            },
        },
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
fn decomposed_acute_composes_without_rewriting_source_coordinates() {
    let layout = resolve(
        &AttributedText {
            text: "Ae\u{0301}Z".to_string(),
            style: Style {
                family: "Allerta".to_string(),
                size: 5120.0,
            },
        },
        &fixture_environment(ALLERTA, "Allerta"),
    )
    .expect("the measured decomposed acute composes inside oracle v3");

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
            &AttributedText {
                text: format!("Ax{mark}Z"),
                style: Style {
                    family: "Bungee".to_string(),
                    size: 1000.0,
                },
            },
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
fn malformed_and_unadmitted_mark_sequences_refuse_before_shaping() {
    let environment = fixture_environment(BUNGEE, "Bungee");
    for (source, byte_index, character) in [
        ("\u{0301}AX", 0, '\u{0301}'),
        ("Ax\u{0301}\u{0301}Z", 4, '\u{0301}'),
        ("A1\u{0301}Z", 2, '\u{0301}'),
        ("Aé\u{0301}Z", 3, '\u{0301}'),
    ] {
        let error = resolve(
            &AttributedText {
                text: source.to_string(),
                style: Style {
                    family: "Bungee".to_string(),
                    size: 1000.0,
                },
            },
            &environment,
        )
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

    let error = resolve(
        &AttributedText {
            text: "Ax\u{0300}Z".to_string(),
            style: Style {
                family: "Bungee".to_string(),
                size: 1000.0,
            },
        },
        &environment,
    )
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
fn undeclared_family_refuses_by_name() {
    let err = resolve(&ahem("X", 20.0), &Environment::default()).unwrap_err();
    assert_eq!(
        err,
        ResolveError::UnknownFamily {
            family: "Ahem".to_string()
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
