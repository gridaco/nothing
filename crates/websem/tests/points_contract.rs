//! The points-shapes contract: what `<polygon>` and `<polyline>` admit.
//!
//! Both lower to the line-segment path the contract already carries —
//! `MoveTo` + `LineTo`* — exactly as `<line>` does; closure is the one
//! semantic difference between them. The `points` grammar runs through the
//! same number scanner as path data so the two cannot drift, and its
//! separator rules are Blink's, measured against Chromium: a trailing
//! separator after the last complete pair is accepted (unlike `viewBox`),
//! a leading or doubled comma is an error, and Chromium renders the valid
//! pair prefix of an erroneous list where this slice refuses the whole
//! element by name — the paths rung's declared divergence, restated here.

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use rframe::{Geometry, PathCommand};
use support::render_through_n0;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

/// A 64x64 canvas around the markup under test.
fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// Strict and best-effort agree and declare nothing.
fn admit_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort admits");
    assert!(
        best.degradations().is_empty(),
        "an admitted document declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn commands(frame: &rframe::Frame, index: usize) -> &[PathCommand] {
    let Geometry::Path(path) = &frame.nodes[index].geometry else {
        panic!("points shapes lower to paths");
    };
    path.commands()
}

#[test]
fn a_polygon_lowers_to_a_closed_line_path() {
    let frame = admit_both(&document(
        r##"  <polygon points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    assert_eq!(frame.nodes.len(), 1);
    assert_eq!(
        commands(&frame, 0),
        &[
            PathCommand::MoveTo { x: 8.0, y: 8.0 },
            PathCommand::LineTo { x: 56.0, y: 8.0 },
            PathCommand::LineTo { x: 32.0, y: 56.0 },
            PathCommand::Close,
        ]
    );
}

#[test]
fn a_polyline_stays_open() {
    let frame = admit_both(&document(
        r##"  <polyline points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        commands(&frame, 0),
        &[
            PathCommand::MoveTo { x: 8.0, y: 8.0 },
            PathCommand::LineTo { x: 56.0, y: 8.0 },
            PathCommand::LineTo { x: 32.0, y: 56.0 },
        ],
        "no closing command: the open contour is the polyline's identity"
    );
}

/// A `<polygon>` and its equivalent `<path>` are the same resolved fact —
/// the shape needs no geometry kind of its own.
#[test]
fn a_polygon_equals_its_equivalent_path() {
    let polygon = admit_both(&document(
        r##"  <polygon points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    let path = admit_both(&document(
        r##"  <path d="M8 8L56 8L32 56Z" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        polygon.nodes[0].geometry, path.nodes[0].geometry,
        "one geometry, two grammars"
    );
    assert_eq!(
        render_through_n0(&polygon, 64, 64),
        render_through_n0(&path, 64, 64),
        "and one raster"
    );
}

/// `fill-rule` is read from the one cascade, so the presentation attribute
/// and a stylesheet rule resolve to the same frame.
#[test]
fn fill_rule_comes_from_the_cascade() {
    let attribute = admit_both(&document(
        r##"  <polygon points="32,4 12,60 60,24 4,24 52,60" fill="#2563eb" fill-rule="evenodd"/>"##,
    ));
    // Not `admit_both`: a `<style>` sheet is a declared sampling-only
    // blocker (the CSS animation inventory is not owned), which is exactly
    // the Base-honest class — the Base frame is what this law compares.
    let sheet = SvgFrameSource::from_standalone_svg(
        document(
            r##"  <style>polygon { fill-rule: evenodd }</style>
  <polygon points="32,4 12,60 60,24 4,24 52,60" fill="#2563eb"/>"##,
        ),
        viewport(),
    )
    .expect("strict admits the sheet at Base")
    .base_frame();
    assert_eq!(
        attribute.nodes[0].geometry, sheet.nodes[0].geometry,
        "both spellings reach the shape through the cascade"
    );
    let Geometry::Path(path) = &attribute.nodes[0].geometry else {
        panic!("a polygon lowers to a path");
    };
    assert_eq!(path.fill_rule(), rframe::FillRule::EvenOdd);
}

/// A filled polyline paints as if closed (the fill of an open contour), so
/// it matches the same polygon's fill exactly; their strokes differ, since
/// only the polygon paints the closing segment.
#[test]
fn a_filled_polyline_paints_as_if_closed_and_strokes_stay_open() {
    let filled_polyline = admit_both(&document(
        r##"  <polyline points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    let filled_polygon = admit_both(&document(
        r##"  <polygon points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        render_through_n0(&filled_polyline, 64, 64),
        render_through_n0(&filled_polygon, 64, 64),
        "fill sees a closed contour either way"
    );

    let stroked_polyline = admit_both(&document(
        r##"  <polyline points="16,12 48,12 48,44" fill="none" stroke="#2563eb" stroke-width="4"/>"##,
    ));
    let stroked_polygon = admit_both(&document(
        r##"  <polygon points="16,12 48,12 48,44" fill="none" stroke="#2563eb" stroke-width="4"/>"##,
    ));
    assert_ne!(
        render_through_n0(&stroked_polyline, 64, 64),
        render_through_n0(&stroked_polygon, 64, 64),
        "the closing segment is the polygon's stroke alone"
    );
}

/// The `points` number grammar is the path scanner's: mixed separators,
/// exponents, dots carrying digits, and a sign starting a new number.
#[test]
fn the_points_number_grammar_is_the_path_scanners() {
    let frame = admit_both(&document(
        r##"  <polygon points="8 8, 5.6e1,8 32-56" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        commands(&frame, 0),
        &[
            PathCommand::MoveTo { x: 8.0, y: 8.0 },
            PathCommand::LineTo { x: 56.0, y: 8.0 },
            PathCommand::LineTo { x: 32.0, y: -56.0 },
            PathCommand::Close,
        ]
    );

    let dots = admit_both(&document(
        r##"  <polyline points="8,8 56.5.5 32,56" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        commands(&dots, 0)[1],
        PathCommand::LineTo { x: 56.5, y: 0.5 },
        "a second dot starts a new number, as in path data"
    );
}

/// A trailing separator after the last complete pair is admitted — Blink
/// accepts it (measured; its cell is Chromium-baked), unlike the `viewBox`
/// grammar, whose trailing comma stays a refusal.
#[test]
fn a_trailing_separator_after_the_last_pair_is_admitted() {
    let trailing = admit_both(&document(
        r##"  <polygon points="8,8 56,8 32,56," fill="#16a34a"/>"##,
    ));
    let clean = admit_both(&document(
        r##"  <polygon points="8,8 56,8 32,56" fill="#16a34a"/>"##,
    ));
    assert_eq!(trailing.nodes[0].geometry, clean.nodes[0].geometry);
}

/// An erroneous list refuses the whole element by name, with the byte
/// offset — Chromium renders the valid pair prefix instead, and that
/// divergence is declared, never silent.
#[test]
fn an_erroneous_points_list_refuses_the_whole_element_by_name() {
    for (label, points) in [
        ("leading comma", ",8,8 56,8 32,56"),
        ("doubled comma", "8,8,,56,8 32,56"),
        ("trailing dot", "8,8 56,8 32.,56"),
        ("odd coordinate count", "8,8 56,8 32,56 40"),
        ("percentage", "8,8 56,8 50%,56"),
    ] {
        let source = document(&format!(
            r##"  <polygon points="{points}" fill="#16a34a"/>"##
        ));
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            matches!(strict, CompileError::BadPoints { .. }),
            "{label}: expected BadPoints, got {strict}"
        );
        assert!(
            strict.to_string().contains("points on <polygon>")
                && strict.to_string().contains("invalid at byte"),
            "{label}: the refusal names the construct and the offset; got {strict}"
        );

        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        assert_eq!(best.base_frame().nodes.len(), 0, "{label}: a declared hole");
        assert_eq!(best.degradations().len(), 1, "{label}");
        assert!(
            best.degradations()[0]
                .reason()
                .contains("points on <polygon>"),
            "{label}: the skip names the construct; got {}",
            best.degradations()[0].reason()
        );
        assert_eq!(
            best.degradations()[0].action(),
            DegradationAction::Skipped,
            "{label}"
        );
    }
}

/// A single point splits by closure: the polygon is the zero-length closed
/// contour in the contract's canonical `M x y L x y Z` spelling (its cap
/// decides whether it paints — the Chromium-baked dot cell), while the
/// polyline is a neutral move-only contour that is admitted and is not a
/// node.
#[test]
fn a_single_point_splits_by_closure() {
    let polygon = admit_both(&document(
        r##"  <polygon points="32,32" fill="none" stroke="#000000" stroke-width="8" stroke-linecap="square"/>"##,
    ));
    assert_eq!(polygon.nodes.len(), 1);
    assert_eq!(
        commands(&polygon, 0),
        &[
            PathCommand::MoveTo { x: 32.0, y: 32.0 },
            PathCommand::LineTo { x: 32.0, y: 32.0 },
            PathCommand::Close,
        ],
        "the canonical zero-length closed spelling, cap preserved"
    );

    let polyline = admit_both(&document(
        r##"  <polyline points="32,32" fill="none" stroke="#000000" stroke-width="8" stroke-linecap="square"/>"##,
    ));
    assert_eq!(
        polyline.nodes.len(),
        0,
        "a move-only open contour paints nothing under any cap (measured)"
    );
}

/// An empty or missing `points` is valid and renders nothing — the `d`
/// grammar's empty-value admission, restated for the points shapes.
#[test]
fn empty_and_missing_points_render_nothing() {
    for body in [
        r##"  <polygon points="" fill="#16a34a"/>"##,
        r##"  <polygon fill="#16a34a"/>"##,
        r##"  <polyline points="   " fill="#16a34a"/>"##,
    ] {
        let frame = admit_both(&document(body));
        assert_eq!(frame.nodes.len(), 0, "{body}: an admitted nothing");
    }
}

/// The points shapes inherit the path patrols: `pathLength` and the marker
/// properties refuse or skip by name, never silently.
#[test]
fn points_shapes_are_patrolled_like_paths() {
    for (label, attrs, named) in [
        ("pathLength", r#"pathLength="100""#, "pathLength"),
        ("marker-end", r#"marker-end="url(#m)""#, "marker-end"),
    ] {
        let source = document(&format!(
            r##"  <polygon points="8,8 56,8 32,56" fill="#16a34a" {attrs}/>"##
        ));
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains(named),
            "{label}: named; got {strict}"
        );
        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        assert_eq!(best.base_frame().nodes.len(), 0, "{label}: a declared hole");
        assert!(
            best.degradations()[0].reason().contains(named),
            "{label}: named; got {}",
            best.degradations()[0].reason()
        );
    }
}

/// Both grammar entries reach the points shapes through the one compiler
/// (the equivalence law extended to the rung).
#[test]
fn inline_and_standalone_points_resolve_to_the_same_frame() {
    let svg_body = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><polygon points="8,8 56,8 32,56" fill="#16a34a"/></svg>"##;
    let html = format!("<html><body>{svg_body}</body></html>");
    let inline = websem::compile_html_inline_svg(&html).expect("compile inline entry");
    let standalone =
        websem::compile_standalone_svg(svg_body, viewport()).expect("compile standalone entry");
    assert_eq!(inline, standalone);
}
