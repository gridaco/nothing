//! The path contract: what the one SVG compiler admits for `<path>`, how the
//! `d` grammar resolves into the resolved contract's canonical command stream,
//! and what refuses by name.
//!
//! Every answer these laws pin was measured against Chromium 149, not read off
//! SVG's BNF — the two disagree in places (a trailing dot is valid path-data
//! grammar and Chromium rejects it), and where they disagree the browser wins.
//! The pixel claims are Chromium-baked in `reftest_oracle.rs`; these laws pin
//! the structure that produces them: Blink's ordered `f32` source-number
//! evaluation, complete-segment prefix finalization after an error, and the
//! pinned Skia conics used for elliptical arcs.

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use math2::transform::AffineTransform;
use rframe::{FillRule, Geometry, PathCommand};
use support::render_through_n0;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

/// A 64x64 canvas around the markup under test.
fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

fn path_document(d: &str) -> String {
    document(&format!(r##"  <path fill="#16a34a" d="{d}"/>"##))
}

/// Strict and best-effort agree, and nothing is declared.
fn admit_both(source: &str) -> rframe::Frame {
    admit(source, false)
}

/// The same, for a document carrying a `<style>` element. A stylesheet blocks
/// the *sampling* inventory — it could declare a CSS animation — which is a
/// policy about Sample requests, not a hole in the Base frame these laws read.
fn admit_both_with_stylesheet(source: &str) -> rframe::Frame {
    admit(source, true)
}

fn admit(source: &str, stylesheet: bool) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    let declared: Vec<&websem::Degradation> = best
        .degradations()
        .iter()
        .filter(|d| !(stylesheet && d.action() == DegradationAction::SamplesAsBase))
        .collect();
    assert!(
        declared.is_empty(),
        "an admitted document declares nothing: {declared:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

/// The resolved path of node `index`, asserting the path variant.
fn path_of(frame: &rframe::Frame, index: usize) -> &rframe::PathData {
    match &frame.nodes()[index].geometry {
        Geometry::Path(path) => path,
        other => panic!("expected path geometry, got {other:?}"),
    }
}

fn commands(d: &str) -> Vec<PathCommand> {
    let frame = admit_both(&path_document(d));
    path_of(&frame, 0).commands().to_vec()
}

/// The strict refusal for one document, or a panic if it was admitted.
fn refusal(source: &str) -> CompileError {
    SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .expect_err("must refuse")
        .clone()
}

fn move_to(x: f32, y: f32) -> PathCommand {
    PathCommand::MoveTo { x, y }
}

fn line_to(x: f32, y: f32) -> PathCommand {
    PathCommand::LineTo { x, y }
}

// ─── the command stream ──────────────────────────────────────────────────

#[test]
fn a_closed_polygon_resolves_to_its_absolute_command_stream() {
    let frame = admit_both(&path_document("M10 10 L54 10 L54 54 Z"));
    assert_eq!(frame.nodes().len(), 1);
    let path = path_of(&frame, 0);
    assert_eq!(
        path.commands(),
        [
            move_to(10.0, 10.0),
            line_to(54.0, 10.0),
            line_to(54.0, 54.0),
            PathCommand::Close,
        ]
    );
    assert_eq!(path.fill_rule(), FillRule::NonZero, "the initial fill rule");
    assert!(path.all_contours_closed());
    assert_eq!(
        path.local_bounds(),
        math2::Rectangle::from_xywh(10.0, 10.0, 44.0, 44.0),
        "the tight extent of the geometry"
    );
    assert_eq!(
        frame.nodes()[0].bounds,
        math2::rect_transform(path.local_bounds(), &frame.nodes()[0].transform),
        "the exact-bounds law holds for a path like any other geometry"
    );
}

/// Relative commands, implicit repeats, and the `H`/`V` shorthands are
/// spellings of the same absolute stream — the producer resolves them, and
/// nothing downstream sees the difference.
#[test]
fn every_spelling_of_one_shape_resolves_to_the_same_stream() {
    let closed = [
        move_to(10.0, 10.0),
        line_to(54.0, 10.0),
        line_to(54.0, 54.0),
        PathCommand::Close,
    ];
    for d in [
        "M10 10 L54 10 L54 54 Z",
        "m10 10 l44 0 0 44 z",
        "M10 10 54 10 54 54 Z",
        "M10 10L54 10 54 54Z",
        "M10 10 H54 V54 Z",
        "M10 10 h44 v44 z",
        "M+10 +10 L54 10 L54 54 Z",
        "M1e1 1e1 L54 10 L54 54 Z",
        "  M10 10 L54 10 L54 54 Z",
        "M10 10,L54 10,L54 54,Z",
    ] {
        assert_eq!(commands(d), closed, "d={d:?}");
    }
}

/// An unclosed contour is carried as unclosed — the fill covers the same area
/// either way (Chromium-baked twice, once for each spelling), and the
/// difference is a stroke's, so the fact is resolved not discarded.
#[test]
fn an_unclosed_contour_keeps_its_open_fact_and_fills_the_same() {
    let open = admit_both(&path_document("M10 10 L54 10 L54 54"));
    let closed = admit_both(&path_document("M10 10 L54 10 L54 54 Z"));
    assert!(!path_of(&open, 0).all_contours_closed());
    assert!(path_of(&closed, 0).all_contours_closed());
    assert_eq!(
        render_through_n0(&open, 64, 64),
        render_through_n0(&closed, 64, 64),
        "a fill closes an open contour implicitly"
    );
}

/// `S` reflects the previous cubic's second control point about the current
/// point; after a non-cubic there is nothing to reflect, so the control point
/// *is* the current point. Both are pinned against the explicit `C` spelling.
#[test]
fn a_smooth_cubic_reflects_only_a_previous_cubic() {
    assert_eq!(
        commands("M8 56 C8 32 20 20 32 20 S56 32 56 56 Z"),
        commands("M8 56 C8 32 20 20 32 20 C44 20 56 32 56 56 Z"),
        "S continues the previous cubic"
    );
    assert_eq!(
        commands("M8 56 L20 30 S56 32 56 56 Z"),
        commands("M8 56 L20 30 C20 30 56 32 56 56 Z"),
        "after a line, S reflects about the current point"
    );
}

#[test]
fn a_smooth_quadratic_reflects_only_a_previous_quadratic() {
    assert_eq!(
        commands("M8 56 Q20 20 32 32 T56 56 Z"),
        commands("M8 56 Q20 20 32 32 Q44 44 56 56 Z"),
        "T continues the previous quadratic"
    );
    assert_eq!(
        commands("M8 56 T56 56 Z"),
        commands("M8 56 Q8 56 56 56 Z"),
        "with no previous quadratic, T reflects about the current point"
    );
}

/// SVG leaves the move implicit: after a `Z` the current point is the closed
/// contour's start, and a drawing command there begins a new contour. The
/// canonical stream says so explicitly.
#[test]
fn a_drawing_command_after_a_close_gets_its_explicit_move() {
    assert_eq!(
        commands("M10 10 L30 10 L30 30 Z L54 54 L10 54 Z"),
        [
            move_to(10.0, 10.0),
            line_to(30.0, 10.0),
            line_to(30.0, 30.0),
            PathCommand::Close,
            move_to(10.0, 10.0),
            line_to(54.0, 54.0),
            line_to(10.0, 54.0),
            PathCommand::Close,
        ]
    );
}

/// Three shapes SVG allows and the canonical form does not: a redundant `Z`,
/// a contour that only moves, and an implicit move after a close. Each was
/// measured pixel-neutral in Chromium *on anti-aliased geometry* before being
/// normalized away — integer axis-aligned edges hide this class of difference,
/// as the law below shows.
#[test]
fn normalization_drops_only_what_paints_nothing() {
    let normalized = [
        move_to(10.0, 10.0),
        line_to(54.0, 10.0),
        line_to(54.0, 54.0),
        PathCommand::Close,
    ];
    for d in [
        "M10 10 L54 10 L54 54 ZZ",
        "M10 10 L54 10 L54 54 Z Z",
        "M10 10 L54 10 L54 54 Z M2 2",
        "M10 10 L54 10 L54 54 z m0 0",
        "M2 2 M10 10 L54 10 L54 54 Z",
    ] {
        assert_eq!(commands(d), normalized, "d={d:?}");
    }
}

/// **`M x y Z` is not a contour that draws nothing.** It is a zero-length
/// *closed* contour, and dropping it is measurably wrong twice over: it strokes
/// as a cap-shaped dot, and it changes how the rest of the path is *filled* —
/// an extra contour is an extra contour to the scan converter. Measured in
/// Chromium on anti-aliased geometry, dropping it moves 96 pixels of the
/// surviving triangle; the same document rendered with the contour spelled as
/// an explicit zero-length segment is **byte-identical** to the authored form.
/// So the producer resolves it into that spelling instead of discarding it.
///
/// The earlier claim that all three normalizations were "measured" held only
/// for the integer axis-aligned coordinates the laws sampled — which is exactly
/// why the corpus cell for this one (`svg-path-closed-move-only-contour.svg`)
/// uses fractional coordinates.
#[test]
fn a_closed_move_only_contour_is_a_zero_length_contour_not_nothing() {
    assert_eq!(
        commands("M2 2 Z M10 10 L54 10 L54 54 Z"),
        [
            move_to(2.0, 2.0),
            line_to(2.0, 2.0),
            PathCommand::Close,
            move_to(10.0, 10.0),
            line_to(54.0, 10.0),
            line_to(54.0, 54.0),
            PathCommand::Close,
        ]
    );
    // Alone, it is the whole path — a node, not an absence.
    let frame = admit_both(&path_document("M20 20 Z"));
    assert_eq!(frame.nodes().len(), 1);
    let path = path_of(&frame, 0);
    assert_eq!(
        path.commands(),
        [move_to(20.0, 20.0), line_to(20.0, 20.0), PathCommand::Close,]
    );
    assert!(path.all_contours_closed());
    assert_eq!(
        path.local_bounds(),
        math2::Rectangle::from_xywh(20.0, 20.0, 0.0, 0.0),
        "zero extent: it has no fill area, only stroke geometry"
    );
    // A second `Z`, closing nothing, really is inert (measured) — so the two
    // cases never trade places.
    assert_eq!(commands("M20 20 Z Z"), commands("M20 20 Z"));
    // After the close, the current point is the subpath start, so a following
    // command continues from there and not from the origin.
    assert_eq!(
        commands("M20 20 Z L30 30"),
        [
            move_to(20.0, 20.0),
            line_to(20.0, 20.0),
            PathCommand::Close,
            move_to(20.0, 20.0),
            line_to(30.0, 30.0),
        ]
    );
}

/// A zero-length contour *does* draw — it is a segment, not a bare move — so
/// normalization keeps it. Its fill covers nothing, which is what Chromium
/// paints; a stroke with a round cap would paint a dot, which is why the fact
/// must survive.
#[test]
fn a_zero_length_segment_is_not_normalized_away() {
    assert_eq!(
        commands("M10 10 L10 10 Z"),
        [move_to(10.0, 10.0), line_to(10.0, 10.0), PathCommand::Close,]
    );
}

/// A `d` that draws nothing resolves to no node at all: the element is
/// admitted (it is not a hole — Chromium paints nothing for it either), and
/// there is no visual fact to carry.
#[test]
fn a_path_that_draws_nothing_resolves_to_no_node() {
    for body in [
        r##"  <path fill="#16a34a" d=""/>"##,
        r##"  <path fill="#16a34a" d="   "/>"##,
        r##"  <path fill="#16a34a"/>"##,
        r##"  <path fill="#16a34a" d="none"/>"##,
        r##"  <path fill="#16a34a" d="  NoNe  "/>"##,
        r##"  <path fill="#16a34a" d="initial"/>"##,
        r##"  <path fill="#16a34a" d="unset"/>"##,
        r##"  <path fill="#16a34a" d="revert"/>"##,
        r##"  <path fill="#16a34a" d="revert-layer"/>"##,
        r##"  <path fill="#16a34a" d="path('M8 8 H56 V56 H8 Z')"/>"##,
        r##"  <path fill="#16a34a" style="--p: M8 8 H56 V56 H8 Z" d="var(--p)"/>"##,
        r##"  <path fill="#16a34a" d="M20 20"/>"##,
        r##"  <path fill="#16a34a" d="M20 20 M30 30"/>"##,
    ] {
        let source = document(body);
        let frame = if body.contains("style=") {
            admit_both_with_stylesheet(&source)
        } else {
            admit_both(&source)
        };
        assert!(frame.nodes().is_empty(), "body={body:?}");
    }
}

// ─── the fill rule ───────────────────────────────────────────────────────

/// `fill-rule` is read as a typed computed value, so every CSS ingress and
/// SVG2's precedence come from the one cascade: the presentation attribute,
/// inheritance through a container, an author rule, ASCII-case-insensitive
/// keywords, and an invalid value falling back to the initial `nonzero`.
#[test]
fn the_fill_rule_comes_from_the_one_cascade() {
    let star = "M32 8 L44 50 L8 22 L56 22 L20 50 Z";
    for (body, expected) in [
        (
            format!(r##"  <path fill="#16a34a" d="{star}"/>"##),
            FillRule::NonZero,
        ),
        (
            format!(r##"  <path fill="#16a34a" fill-rule="evenodd" d="{star}"/>"##),
            FillRule::EvenOdd,
        ),
        (
            format!(r##"  <path fill="#16a34a" fill-rule="EVENODD" d="{star}"/>"##),
            FillRule::EvenOdd,
        ),
        (
            format!(r##"  <path fill="#16a34a" fill-rule="qqq" d="{star}"/>"##),
            FillRule::NonZero,
        ),
        (
            format!(r##"  <g fill-rule="evenodd"><path fill="#16a34a" d="{star}"/></g>"##),
            FillRule::EvenOdd,
        ),
        (
            format!(
                r##"  <style>path {{ fill-rule: evenodd }}</style>
  <path fill="#16a34a" d="{star}"/>"##
            ),
            FillRule::EvenOdd,
        ),
        (
            format!(
                r##"  <style>path {{ fill-rule: nonzero }}</style>
  <path fill="#16a34a" fill-rule="evenodd" d="{star}"/>"##
            ),
            FillRule::NonZero,
        ),
    ] {
        let source = document(body.as_str());
        let frame = if body.contains("<style>") {
            admit_both_with_stylesheet(source.as_str())
        } else {
            admit_both(source.as_str())
        };
        assert_eq!(path_of(&frame, 0).fill_rule(), expected, "body={body}");
    }
}

/// The two rules paint differently, so the resolved value is load-bearing:
/// the star's core is filled under `nonzero` and hollow under `evenodd`.
#[test]
fn the_fill_rule_decides_the_interior() {
    let star = "M32 8 L44 50 L8 22 L56 22 L20 50 Z";
    let nonzero = admit_both(&path_document(star));
    let evenodd = admit_both(&document(&format!(
        r##"  <path fill="#16a34a" fill-rule="evenodd" d="{star}"/>"##
    )));
    let at = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] {
        let offset = (y * 64 + x) * 4;
        pixels[offset..offset + 4].try_into().expect("pixel")
    };
    let nonzero = render_through_n0(&nonzero, 64, 64);
    let evenodd = render_through_n0(&evenodd, 64, 64);
    assert_eq!(at(&nonzero, 32, 30), [0x16, 0xa3, 0x4a, 255], "core filled");
    assert_eq!(at(&evenodd, 32, 30), [0, 0, 0, 0], "core hollow");
    assert_eq!(
        at(&nonzero, 32, 15),
        at(&evenodd, 32, 15),
        "a single-wound point is filled by both rules"
    );
}

// ─── source grammar and error finalization ───────────────────────────────

/// The path-data number grammar, measured. SVG's BNF allows a trailing dot
/// (`digit-sequence "."`); Chromium's parser requires a digit after the dot
/// and stops at `M10. 10 …`. An error retains only fully emitted segments, so
/// one before the first complete moveto is clean empty geometry while one
/// after a visible contour retains that contour.
#[test]
fn the_number_grammar_is_chromiums_not_the_bnfs() {
    for d in [
        "M10. 10 L54 10 L54 54 Z",
        "M10 10 L54. 10 L54 54 Z",
        "M. 10 L54 10 L54 54 Z",
        "M1e 10 L54 10 L54 54 Z",
    ] {
        let frame = admit_both(&path_document(d));
        assert!(
            frame.nodes().is_empty(),
            "d={d:?} has an empty valid prefix"
        );
    }

    let prefix = "M8 8 H56 V56 H8 Z";
    for d in [
        format!("{prefix} M10 10 L54. 10"),
        format!("{prefix} M10 10 L1e40 10"),
        format!("{prefix} M10 10 L340282346638528859811704183484516925440 10"),
    ] {
        assert_eq!(
            commands(&d),
            commands(prefix),
            "d={d:?} retains the last complete segment"
        );
    }
    // A finite huge number is not an overflow.
    admit_both(&path_document("M10 10 L1e30 10 L54 54 Z"));
}

/// Blink does not parse a source token through an ideal decimal and round
/// once. It accumulates integer and fraction digits in ordered `f32`
/// operations. These two valid tokens exercise both directions in which
/// Rust's former `parse::<f32>()` route selected the other neighbour.
#[test]
fn source_numbers_use_blinks_ordered_f32_accumulation() {
    for (source, expected_bits) in [
        ("1188.679260273", 0x4494_95bc),
        ("5186.454833937", 0x45a2_13a4),
    ] {
        let stream = commands(&format!("M{source} 8 h1 v1 Z"));
        let PathCommand::MoveTo { x, .. } = stream[0] else {
            panic!("a path begins with the resolved moveto");
        };
        assert_eq!(x.to_bits(), expected_bits, "source={source}");
    }
}

/// A number consumes trailing whitespace and **at most one** comma. That one
/// rule reproduces every measured case, and the asymmetry is real: a comma
/// before a command letter parses (the preceding number ate it), while a comma
/// right after a command letter, a doubled comma, and a leading comma are all
/// errors Chromium reports by finalizing the valid prefix.
#[test]
fn the_separator_grammar_admits_exactly_one_comma() {
    admit_both(&path_document("M10 10 L54 10, 54 54 Z"));
    admit_both(&path_document("M10 10,L54 10 L54 54 Z"));
    for d in [
        "M,10 10 L54 10 L54 54 Z",
        "M10,,10 L54 10 L54 54 Z",
        ",M10 10 L54 10 L54 54 Z",
    ] {
        assert!(admit_both(&path_document(d)).nodes().is_empty(), "d={d:?}");
    }

    let prefix = "M8 8 H56 V56 H8 Z";
    assert_eq!(
        commands(&format!("{prefix} M10,,10")),
        commands(prefix),
        "a separator error after a completed contour retains it"
    );
}

/// Only the five ASCII whitespace characters separate path-data tokens. A
/// value padded with U+00A0 is invalid to Chromium, which paints nothing.
#[test]
fn non_ascii_whitespace_is_not_a_separator() {
    assert!(
        admit_both(&path_document("M10\u{00a0}10 L54 10 L54 54 Z"))
            .nodes()
            .is_empty()
    );
}

/// SVG2 and Chromium finalize an erroneous path after its last fully defined
/// segment. This matrix crosses every argument arity, an unknown command, an
/// error after close, and a trailing move-only contour. No half-defined curve
/// or arc may leak into the resolved command stream.
#[test]
fn malformed_path_data_finalizes_its_complete_segment_prefix() {
    let prefix = "M8 8 H56 V56 H8 Z";
    let expected = commands(prefix);
    for suffix in [
        "BOGUS L60 60",
        "0",
        "M2 2 L",
        "M2 2 H",
        "M2 2 V",
        "M2 2 C1 2 3 4 5",
        "M2 2 S1 2 3",
        "M2 2 Q1 2 3",
        "M2 2 T1",
        "M2 2 A8 8 0 0 1 20",
        "M2 2 A8 8 0 2 1 20 20",
    ] {
        let d = format!("{prefix} {suffix}");
        assert_eq!(commands(&d), expected, "suffix={suffix:?}");
    }

    assert_eq!(
        commands("M8 8 L56 8 56 56 8"),
        [move_to(8.0, 8.0), line_to(56.0, 8.0), line_to(56.0, 56.0)],
        "complete implicit repeats survive an incomplete final pair"
    );
    assert_eq!(
        commands("M8 32 C16 8 48 8 56 32 48"),
        commands("M8 32 C16 8 48 8 56 32"),
        "a repeated cubic is emitted only after all six arguments"
    );
}

/// Path data must begin with a moveto. Chromium's valid prefix is empty in
/// that case, so it paints nothing without a declaration.
#[test]
fn path_data_must_begin_with_a_moveto() {
    for d in ["L10 10 L54 54 Z", "M8 BOGUS L56 56"] {
        let frame = admit_both(&path_document(d));
        assert!(frame.nodes().is_empty(), "d={d:?}");
    }
}

/// **The second deliberate divergence is repaid.** The conic rung resolved
/// it: an arc's grammar parses and emits the rational conics Chromium
/// rasterizes it through — never Blink's cubic *normalizer*, whose explicit
/// cubics differ from Chromium's own `A` by 77 pixels at up to a
/// 170-per-channel delta. The half-ellipse sweep resolves to two quarter-turn
/// conics of weight `cos 45°`, and the authored endpoint is reused exactly on
/// the last segment so the current-point chain never drifts.
#[test]
fn an_elliptical_arc_resolves_to_conics_in_both_admissions() {
    for d in ["M8 28 A24 20 0 0 1 56 28 Z", "M8 28 a24 20 0 0 1 48 0 Z"] {
        let stream = commands(d);
        assert_eq!(stream.len(), 4, "move, two conics, close: {stream:?}");
        assert_eq!(stream[0], move_to(8.0, 28.0));
        let (mid, end) = match (stream[1], stream[2]) {
            (
                PathCommand::ConicTo {
                    x, y, weight: w1, ..
                },
                PathCommand::ConicTo {
                    x: x2,
                    y: y2,
                    weight: w2,
                    ..
                },
            ) => {
                assert!(
                    (w1 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6
                        && (w2 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6,
                    "a quarter-turn segment's weight is cos 45°"
                );
                ((x, y), (x2, y2))
            }
            other => panic!("expected two conics, got {other:?}"),
        };
        assert!(
            (mid.0 - 32.0).abs() < 1e-3 && (mid.1 - 8.0).abs() < 1e-3,
            "the split lands on the ellipse's apex, got {mid:?}"
        );
        assert_eq!(end, (56.0, 28.0), "the authored endpoint, exactly");
        assert_eq!(stream[3], PathCommand::Close);
    }
    // A malformed repeated arc emits none of that arc, but keeps the complete
    // conics before it.
    assert_eq!(
        commands("M8 28 A24 20 0 0 1 56 28 A24 20 0 2 1 8 28"),
        commands("M8 28 A24 20 0 0 1 56 28")
    );
}

/// The arc's degenerate and out-of-range rules, each the measured Chromium
/// behavior — byte-identical to the equivalent explicit spelling, so the
/// resolved streams must be identical too:
///
/// - coincident endpoints elide the segment;
/// - a zero radius degenerates to the authored line;
/// - negative radii take their absolute value;
/// - radii too small to span the endpoints scale up uniformly;
/// - a smooth cubic after an arc reflects about the current point
///   (the arc resets both reflections);
/// - the rotation is fed through as authored. An ellipse makes that rotation
///   structurally observable without relying on platform-libm residue from a
///   rotationally invariant circle, and `390` is not reduced to `30`.
#[test]
fn arc_degenerates_and_corrections_resolve_as_chromium_paints_them() {
    assert_eq!(
        commands("M8 8 L56 8 A10 10 0 0 1 56 8 L56 56 Z"),
        commands("M8 8 L56 8 L56 56 Z"),
        "coincident endpoints elide"
    );
    assert_eq!(
        commands("M8 8 A0 20 0 0 1 56 40 L56 56 Z"),
        commands("M8 8 L56 40 L56 56 Z"),
        "zero radius is the authored line"
    );
    assert_eq!(
        commands("M8 28 A-24 -20 0 0 1 56 28 Z"),
        commands("M8 28 A24 20 0 0 1 56 28 Z"),
        "negative radii take their absolute value"
    );
    assert_eq!(
        commands("M12 32 A5 5 0 0 1 52 32 Z"),
        commands("M12 32 A20 20 0 0 1 52 32 Z"),
        "too-small radii scale up uniformly"
    );
    assert_eq!(
        commands("M8 32 A12 12 0 0 1 32 32 S44 44 56 32"),
        commands("M8 32 A12 12 0 0 1 32 32 C32 32 44 44 56 32"),
        "a smooth cubic after an arc reflects about the current point"
    );
    assert_ne!(
        commands("M12 32 A20 12 45 0 1 52 32 Z"),
        commands("M12 32 A20 12 0 0 1 52 32 Z"),
        "the authored angle reaches pinned Skia's f32 construction"
    );
    assert_ne!(
        commands("M12 40 A24 12 390 0 1 52 40 Z"),
        commands("M12 40 A24 12 30 0 1 52 40 Z"),
        "the angle is not reduced before f32 trigonometry"
    );
}

/// Finite source numbers can produce non-finite derived coordinates. Ordinary
/// path verbs poison Skia's path and therefore erase prior ink; an extreme arc
/// can instead return before appending a verb, preserving the prior prefix.
/// Those outcomes are visually opposite and must not share one blanket rule.
#[test]
fn numeric_extremes_preserve_skias_poison_and_arc_noop_split() {
    for d in [
        "M8 8 H56 V56 H8 Z M3.4e38 32 h3.4e38",
        "M8 8 H56 V56 H8 Z M3.4e38 3.4e38 C3.4e38 3.4e38 3.4e38 -3.4e38 3.4e38 3.4e38 S3.4e38 3.4e38 3.4e38 3.4e38",
    ] {
        let frame = admit_both(&path_document(d));
        assert!(
            frame.nodes().is_empty(),
            "ordinary non-finite verb poisons d={d:?}"
        );
    }

    let prefix = "M8 8 H56 V56 H8 Z";
    assert_eq!(
        commands(&format!("{prefix} M3.4e38 32 a8 8 0 0 1 3.4e38 0")),
        commands(prefix),
        "a non-finite arc construction appends no verb and retains prior ink"
    );

    for d in [
        "M8 32 A3.4e38 3.4e38 0 0 1 56 32",
        "M8 32 A1e-45 1e-45 0 0 1 56 32",
    ] {
        assert!(
            admit_both(&path_document(d)).nodes().is_empty(),
            "an isolated extreme no-op arc leaves only a neutral move: d={d:?}"
        );
    }
    assert_eq!(
        commands("M8 32 A3.4e38 3.4e38 0 0 1 56 32 l0 16"),
        [move_to(8.0, 32.0), line_to(56.0, 48.0)],
        "the arc advances the logical point without appending a path verb"
    );
    assert_eq!(
        commands("M8 8 L8 56 A3.4e38 3.4e38 0 0 1 56 56 Z"),
        [move_to(8.0, 8.0), line_to(8.0, 56.0), PathCommand::Close],
        "close still targets the authored contour start after an arc no-op"
    );
    assert!(
        commands("M8 32 A24 12 3.4e38 0 1 56 32").len() > 1,
        "a huge finite rotation still constructs a stable path"
    );
}

// ─── the patrols around `<path>` ─────────────────────────────────────────

/// Chromium honors a stylesheet's `d: path(…)` in place of the attribute
/// (measured), and the pinned Stylo build drops the declaration entirely —
/// so it would silently paint the attribute's geometry, or nothing, where the
/// browser paints the sheet's. A sheet is document-level; a `style` attribute
/// is the element's own hole.
#[test]
fn the_css_d_property_is_declared_never_silently_dropped() {
    let sheet = document(
        r##"  <style>path { d: path("M10 10 L54 10 L54 54 Z") }</style>
  <path fill="#16a34a"/>"##,
    );
    assert!(matches!(
        refusal(&sheet),
        CompileError::UnsupportedStyle(reason) if reason.contains("declares d")
    ));
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(sheet.as_str(), viewport(64.0, 64.0))
            .expect("best-effort declares the sheet");
    assert_eq!(
        best.degradations()[0].action(),
        DegradationAction::DeclarationIgnored
    );

    let inline =
        document(r##"  <path fill="#16a34a" style="d: path('M10 10 L54 10 L54 54 Z')"/>"##);
    assert!(matches!(
        refusal(&inline),
        CompileError::UnsupportedStyle(reason) if reason.contains("<path> declares d")
    ));
}

/// `marker-start`/`-mid`/`-end` are refused by name on `<path>`. Nothing "reads"
/// a marker property — the property *is* the paint trigger, so this refusal is
/// what keeps Chromium's arrowhead from becoming a silent hole.
#[test]
fn marker_patrols_are_load_bearing() {
    for attr in [
        r##"marker-start="url(#a)""##,
        r##"marker-mid="url(#a)""##,
        r##"marker-end="url(#a)""##,
    ] {
        let source = document(&format!(
            r##"  <path fill="#16a34a" {attr} d="M10 10 L54 10 L54 54 Z"/>"##
        ));
        let error = refusal(&source);
        assert!(
            matches!(error, CompileError::UnsupportedAttribute { ref element, .. } if element == "path"),
            "{attr} must refuse by name; got {error}"
        );
    }
}

/// A path's own `transform` composes inside its inherited mapping exactly as
/// any other shape's does, and the geometry stays in the authored user space.
#[test]
fn a_path_transform_composes_like_any_other_shape() {
    let frame = admit_both(&document(
        r##"  <g transform="scale(2)">
    <path transform="translate(1,1)" fill="#16a34a" d="M5 5 L27 5 L27 27 Z"/>
  </g>"##,
    ));
    assert_eq!(
        frame.nodes()[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 2.0, 0.0, 2.0, 2.0)
    );
    assert_eq!(
        path_of(&frame, 0).commands()[0],
        move_to(5.0, 5.0),
        "the transform never enters the resolved geometry"
    );
    assert_eq!(
        frame.nodes()[0].bounds,
        math2::rect_transform(
            path_of(&frame, 0).local_bounds(),
            &frame.nodes()[0].transform
        ),
        "the exact-bounds law survives composition"
    );
}

/// Curve extrema are solved, not approximated by the control hull: a control
/// point lies outside its own curve, so a hull bound would claim ink where
/// there is none and break the exact-bounds law's meaning.
#[test]
fn curve_bounds_are_the_solved_extent() {
    let frame = admit_both(&path_document("M0 0 C0 100 100 100 100 0"));
    assert_eq!(
        path_of(&frame, 0).local_bounds(),
        math2::Rectangle::from_xywh(0.0, 0.0, 100.0, 75.0),
        "the cubic's apex is at three quarters of the control height"
    );
    let frame = admit_both(&path_document("M0 0 Q50 100 100 0"));
    assert_eq!(
        path_of(&frame, 0).local_bounds(),
        math2::Rectangle::from_xywh(0.0, 0.0, 100.0, 50.0),
        "the quadratic's apex is at half the control height"
    );
}

// ─── the CSS patrol's ingresses ──────────────────────────────────────────

/// Every measured way a stylesheet can change Chromium's pixels through a
/// property this cascade cannot represent. Each case below was verified in
/// Chromium 149 to actually paint differently from the unstyled document, and
/// each must be *declared* — the text scan is not a CSS tokenizer, and every
/// rule in it exists because one of these leaked.
#[test]
fn every_measured_css_ingress_is_declared() {
    let triangle = r##"  <path fill="#16a34a" d="M10 10 L54 10 L54 54 Z"/>"##;
    for (css, what) in [
        // The shorthand that names none of its longhands: Chromium resets `d`
        // and paints nothing at all.
        ("all: initial", "the all shorthand"),
        ("all: unset", "the all shorthand, unset"),
        // Vendor aliases of names already on the list — a one-character
        // bypass. (`-webkit-transform` and `-webkit-clip-path` both left this
        // table when their represented Stylo longhands became compiler
        // ingresses; their feature contracts pin those aliases now.)
        ("-webkit-filter: blur(3px)", "a -webkit- filter"),
        // CSS motion path moves the shape off its authored position.
        (
            "offset-path: path(\"M0 0 L30 30\"); offset-distance: 100%",
            "motion path",
        ),
        ("offset: path(\"M0 0 L30 30\") 100%", "the offset shorthand"),
        // A comment adjacent to the property name keeps the declaration valid
        // for Chromium's tokenizer while splitting the scanned text.
        (
            "d/**/: path(\"M2 2 L20 2 L20 20 Z\")",
            "a comment before the colon",
        ),
        (
            "/**/d: path(\"M2 2 L20 2 L20 20 Z\")",
            "a comment before the name",
        ),
        // An ident escape spells the name without spelling it.
        ("\\000064: path(\"M2 2 L20 2 L20 20 Z\")", "an ident escape"),
    ] {
        let sheet = document(&format!("  <style>path {{ {css} }}</style>\n{triangle}"));
        assert!(
            matches!(refusal(&sheet), CompileError::UnsupportedStyle(_)),
            "{what} must be declared, not silently dropped: {css}"
        );
        // The same declaration inside an XML attribute: CSS strings take
        // single quotes, which is what keeps the attribute well-formed.
        let inline = document(&format!(
            r##"  <path fill="#16a34a" style="{}" d="M10 10 L54 10 L54 54 Z"/>"##,
            css.replace('"', "'")
        ));
        assert!(
            matches!(refusal(&inline), CompileError::UnsupportedStyle(_)),
            "{what} must be declared through the style attribute too: {css}"
        );
    }
}

/// A vendor alias of a *consumed* property is consumed, not refused: the
/// scan strips the prefix and finds no refusal, and the pinned Stylo
/// implements the `-webkit-transform` alias, so the spelling Chromium
/// applies (measured: it moves an SVG rect) is the spelling the cascade
/// carries. A graduating name takes its aliases with it — checked, not
/// assumed.
#[test]
fn a_webkit_transform_alias_composes_like_the_unprefixed_property() {
    // A style attribute is the same sampling-only blocker a sheet is; the
    // stylesheet-tolerant helper reads the identical Base frame.
    let aliased = admit_both_with_stylesheet(&document(
        r##"  <path fill="#16a34a" style="-webkit-transform: translate(20px, 20px)" d="M10 10 L20 10 L20 20 Z"/>"##,
    ));
    let unprefixed = admit_both_with_stylesheet(&document(
        r##"  <path fill="#16a34a" style="transform: translate(20px, 20px)" d="M10 10 L20 10 L20 20 Z"/>"##,
    ));
    assert_eq!(aliased, unprefixed, "one property under two spellings");
}

/// A `<style>` element's CSS is patrolled as the **concatenation** of its text
/// children, which is what the cascade compiles and what a browser's
/// `textContent` yields. A comment node between two text nodes otherwise splits
/// a declaration so that neither fragment names a listed property, while the
/// sheet Chromium sees is perfectly valid (measured: it paints the CSS `d`).
#[test]
fn a_stylesheet_split_across_text_nodes_is_patrolled_as_one_sheet() {
    let split = document(
        r##"  <style>path{d:pa<!---->th("M2 2 L20 2 L20 20 Z")}</style>
  <path fill="#16a34a" d="M10 10 L54 10 L54 54 Z"/>"##,
    );
    assert!(
        matches!(refusal(&split), CompileError::UnsupportedStyle(reason) if reason.contains("declares d")),
        "the sheet in force is what must be patrolled"
    );
}

/// A property the cascade *does* represent and this compiler consumes is not
/// swept up by the patrol: the scan must not over-refuse the two properties the
/// slice actually reads, nor an unrelated one it can ignore safely.
#[test]
fn the_patrol_leaves_consumed_and_harmless_properties_alone() {
    let star = "M32 8 L44 50 L8 22 L56 22 L20 50 Z";
    admit_both_with_stylesheet(&document(&format!(
        r##"  <style>path {{ fill: #16a34a; fill-rule: evenodd }}</style>
  <path d="{star}"/>"##
    )));
    admit_both_with_stylesheet(&document(&format!(
        r##"  <style>/* a comment naming nothing */ path {{ fill: #16a34a }}</style>
  <path d="{star}"/>"##
    )));
}
