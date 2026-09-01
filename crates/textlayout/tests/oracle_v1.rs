//! Oracle v1 against measured ground truth.
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
const PT_SERIF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/PT_Serif/PTSerif-Regular.ttf"
);

/// Tests exercise resolution, not host-side digest verification, so the key
/// is a stand-in identity the artifact must nonetheless carry through.
const TEST_KEY: FontKey = FontKey::new([0xAB; 32]);

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

    assert_eq!(textlayout::ORACLE_VERSION, "textlayout-v1");
    assert_eq!(layout.oracle_version(), textlayout::ORACLE_VERSION);
    assert_eq!(layout.face().key, TEST_KEY);
    assert_eq!(layout.face().units_per_em, 1000);

    // Measured: 'X' is glyph 58, advance 1000 font units.
    let glyphs = layout.glyphs();
    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].glyph_id, 58);
    assert_eq!(glyphs[0].x, 0.0);
    assert_eq!(glyphs[0].advance, 50.0);
    assert_eq!(glyphs[0].cluster_index, 0);
    assert_eq!(layout.clusters().len(), 1);
    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[0].source_utf16(), 0..1);
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
                cluster.glyphs()
            ))
            .collect::<Vec<_>>(),
        vec![(0..1, 0..1, 0..1), (1..2, 1..2, 1..2), (2..3, 2..3, 2..3)]
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
    .expect("one-to-one kerning is inside oracle v1");

    assert_eq!(layout.advance(), 4685.0);
    assert_eq!(layout.glyphs().len(), 2);
    assert_eq!(layout.clusters().len(), 2);
    assert_eq!(layout.glyphs()[0].glyph_id, 70);
    assert_eq!(layout.glyphs()[0].advance, 2330.0);
    assert_eq!(layout.glyphs()[1].glyph_id, 70);
    assert_eq!(layout.glyphs()[1].advance, 2355.0);
    assert_eq!(layout.clusters()[0].source_utf8(), 0..1);
    assert_eq!(layout.clusters()[0].source_utf16(), 0..1);
    assert_eq!(layout.clusters()[0].glyphs(), 0..1);
    assert_eq!(layout.clusters()[1].source_utf8(), 1..2);
    assert_eq!(layout.clusters()[1].source_utf16(), 1..2);
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

// MissingGlyph is unreachable through Ahem at the v1 profile — Ahem covers
// all of printable ASCII — so its pin arrives with the first environment
// font that does not, rather than pretending a reachable test exists today.

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
