//! The stroke contract: what the one SVG compiler consumes for a stroke, how
//! it resolves, and what still refuses by name.
//!
//! A Web stroke is centred on its geometry, its width is a length in the same
//! space the geometry lives in, and everything about it — the paint, the width
//! with its units, the cap and join keywords, the miter limit — arrives as a
//! typed value from the one cascade. So inheritance through a `<g>`, SVG2
//! precedence, unit resolution and CSS keyword case-insensitivity are the
//! cascade's behaviour and not this compiler's, and the laws below pin the
//! resolved facts plus the refusals that keep the unconsumed half honest.
//!
//! Every pixel claim here was measured in Chromium 149 first; the corpus bakes
//! them (`fixtures/web-first/svg-stroke-*.svg`, 30 of 31 byte-exact — only
//! `svg-stroke-path-closed` carries the declared conic tolerance).

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use rframe::{Geometry, PathCommand, StrokeCap, StrokeJoin};
use support::render_through_n0;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// Strict and best-effort agree, and nothing is declared. A `<style>` element
/// blocks the *sampling* inventory, which is a policy about Sample requests and
/// not a hole in the Base frame these laws read.
fn admit_both(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    let declared: Vec<&websem::Degradation> = best
        .degradations()
        .iter()
        .filter(|d| d.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        declared.is_empty(),
        "an admitted document declares nothing: {declared:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn refusal(source: &str) -> CompileError {
    SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .expect_err("must refuse")
        .clone()
}

fn stroke_of(frame: &rframe::Frame, index: usize) -> &rframe::Stroke {
    frame.nodes[index]
        .stroke
        .as_ref()
        .expect("node carries a resolved stroke")
}

/// A stroked 32x32 rect at (16,16) — the shape every alignment law reads.
fn stroked_rect(extra: &str) -> String {
    document(&format!(
        r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" {extra}/>"##
    ))
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 64 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("pixel")
}

// ─── what a stroke is ────────────────────────────────────────────────────

/// A Web stroke straddles its geometry: half the width inside the outline, half
/// outside. Measured — an 8-wide stroke on an edge at x=16 inks x=12..19 and
/// leaves x=11 and x=20 alone.
#[test]
fn a_stroke_is_centred_on_its_geometry() {
    let frame = admit_both(&stroked_rect(r##"stroke-width="8""##));
    assert_eq!(frame.nodes.len(), 1, "the stroke is not a second node");
    assert_eq!(stroke_of(&frame, 0).width(), 8.0);
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 12, 32), [0, 0, 0, 255], "the outer half paints");
    assert_eq!(at(&pixels, 19, 32), [0, 0, 0, 255], "the inner half paints");
    assert_eq!(at(&pixels, 11, 32), [0, 0, 0, 0], "and no further out");
    assert_eq!(at(&pixels, 20, 32), [0, 0, 0, 0], "and no further in");
}

/// SVG's default paint order is fill, then stroke — so the stroke's inner half
/// covers the fill rather than the other way round.
#[test]
fn a_stroke_paints_over_the_fill() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" stroke="#000000" stroke-width="8"/>"##,
    ));
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 18, 32), [0, 0, 0, 255], "stroke over fill");
    assert_eq!(at(&pixels, 22, 32), [0x16, 0xa3, 0x4a, 255], "fill inside");
}

/// The width is a length resolved by the cascade, so units and `em` work, an
/// invalid value falls back the way an invalid CSS declaration does, and the
/// initial value is 1.
#[test]
fn the_stroke_width_is_a_cascaded_length() {
    for (extra, expected) in [
        (r##"stroke-width="8""##, 8.0),
        (r##"stroke-width="8px""##, 8.0),
        (r##"stroke-width="0.5em""##, 8.0),
        (r##"stroke-width="8.5""##, 8.5),
        ("", 1.0),
        // Negative fails the property's non-negative grammar, so the cascade
        // drops the declaration and the initial value stands — which is what
        // Chromium paints (measured byte-identical to the absent case).
        (r##"stroke-width="-8""##, 1.0),
    ] {
        let frame = admit_both(&stroked_rect(extra));
        assert_eq!(stroke_of(&frame, 0).width(), expected, "extra={extra:?}");
    }
    // Inherited through a container, and from a stylesheet, by the one cascade.
    let inherited = admit_both(&document(
        r##"  <g stroke="#000000" stroke-width="8">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##,
    ));
    assert_eq!(stroke_of(&inherited, 0).width(), 8.0);
    let ruled = admit_both(&document(
        r##"  <style>rect { stroke: #000000; stroke-width: 8 }</style>
  <rect x="16" y="16" width="32" height="32" fill="none"/>"##,
    ));
    assert_eq!(stroke_of(&ruled, 0).width(), 8.0);
}

/// A stroke that would paint nothing is `None`, not an empty stroke — so no
/// consumer has to re-derive "is this visible".
#[test]
fn a_stroke_that_paints_nothing_resolves_to_none() {
    for extra in [
        r##"stroke-width="0""##,
        r##"stroke-width="8" stroke="none""##,
    ] {
        let source = document(&format!(
            r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" {extra}/>"##
        ));
        let frame = admit_both(&source);
        assert!(frame.nodes[0].stroke.is_none(), "extra={extra:?}");
    }
    // A shape with neither fill nor stroke is still a node — it has geometry,
    // it simply paints nothing.
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="none"/>"##,
    ));
    assert_eq!(frame.nodes.len(), 1);
    assert!(frame.nodes[0].stroke.is_none());
    assert!(frame.nodes[0].paints.is_empty());
}

/// Caps, joins and the miter limit are cascaded keywords: case-insensitive,
/// with an invalid value falling back to the initial one.
#[test]
fn caps_joins_and_the_miter_limit_come_from_the_one_cascade() {
    for (extra, cap, join, limit) in [
        ("", StrokeCap::Butt, StrokeJoin::Miter, 4.0),
        (
            r##"stroke-linecap="round" stroke-linejoin="round" stroke-miterlimit="7""##,
            StrokeCap::Round,
            StrokeJoin::Round,
            7.0,
        ),
        (
            r##"stroke-linecap="SQUARE" stroke-linejoin="BEVEL""##,
            StrokeCap::Square,
            StrokeJoin::Bevel,
            4.0,
        ),
        (
            r##"stroke-linecap="qqq" stroke-linejoin="qqq""##,
            StrokeCap::Butt,
            StrokeJoin::Miter,
            4.0,
        ),
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        let stroke = stroke_of(&frame, 0);
        assert_eq!(stroke.cap(), cap, "extra={extra:?}");
        assert_eq!(stroke.join(), join, "extra={extra:?}");
        assert_eq!(stroke.miter_limit(), limit, "extra={extra:?}");
    }
}

/// A miter limit below 1 is carried as resolved rather than corrected. No miter
/// can satisfy it, the backend bevels — and that is what Chromium paints for
/// the same value, so "fixing" it here would be the divergence.
#[test]
fn a_miter_limit_below_one_is_carried_not_corrected() {
    let frame = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-miterlimit="0.5""##,
    ));
    assert_eq!(stroke_of(&frame, 0).miter_limit(), 0.5);
    // And the covered area never shrinks below the bevel's.
    assert_eq!(stroke_of(&frame, 0).outset(), 4.0);
}

/// The cap shapes differ in ink, so the resolved value is load-bearing: butt
/// stops at the endpoint while round and square extend by the stroke's radius.
#[test]
fn the_cap_decides_where_a_stroke_ends() {
    let ink = |cap: &str| -> Vec<u8> {
        let frame = admit_both(&document(&format!(
            r##"  <path d="M16 32 L48 32" fill="none" stroke="#000000" stroke-width="16" stroke-linecap="{cap}"/>"##
        )));
        render_through_n0(&frame, 64, 64)
    };
    let butt = ink("butt");
    assert_eq!(
        at(&butt, 15, 32),
        [0, 0, 0, 0],
        "butt stops at the endpoint"
    );
    let round = ink("round");
    assert_eq!(at(&round, 10, 32), [0, 0, 0, 255], "round extends");
    let square = ink("square");
    assert_eq!(at(&square, 8, 32), [0, 0, 0, 255], "square extends further");
    assert_eq!(at(&square, 7, 32), [0, 0, 0, 0], "by exactly the radius");
}

/// The converse of the law above: where a contour is *closed*, the cap has
/// nothing to shape, so changing it must not change one pixel.
///
/// This is a whole-geometry law on purpose. The first version of the closed
/// contour fix covered `<path>` and left `<circle>` and `<ellipse>` painting a
/// cap Chromium does not — measured at 84 to 95 differing pixels of 2304 below
/// about one device pixel of width, in silence. A per-element law would have
/// missed it exactly the way the corpus did, so this one iterates every closed
/// geometry the slice admits and asserts pixel identity across all three caps.
///
/// `<line>` is deliberately absent: it is the open contour, its caps are real,
/// and the law directly above pins that they move ink.
#[test]
fn a_cap_cannot_change_a_closed_contour() {
    let closed = [
        (
            "path",
            r##"<path d="M24 8 C36 8 40 20 40 28 C40 38 33 44 24 44 C15 44 8 38 8 28 C8 20 12 8 24 8 Z" fill="none""##,
        ),
        ("circle", r##"<circle cx="24" cy="24" r="16" fill="none""##),
        (
            "ellipse",
            r##"<ellipse cx="24" cy="24" rx="18" ry="10" fill="none""##,
        ),
        (
            "rect",
            r##"<rect x="10" y="14" width="28" height="20" fill="none""##,
        ),
    ];
    // The divergence tracked the device width and vanished above about one
    // pixel, so the law has to hold the widths where it lived.
    for (element, head) in closed {
        for width in ["0.5", "1", "1.25", "2"] {
            let ink = |cap: &str| -> Vec<u8> {
                let frame = admit_both(&document(&format!(
                    r##"  {head} stroke="#000000" stroke-width="{width}" stroke-linecap="{cap}"/>"##
                )));
                render_through_n0(&frame, 64, 64)
            };
            let butt = ink("butt");
            assert_eq!(
                ink("round"),
                butt,
                "{element} at width {width}: a round cap changed a closed contour"
            );
            assert_eq!(
                ink("square"),
                butt,
                "{element} at width {width}: a square cap changed a closed contour"
            );
        }
    }
}

/// SVG2 §10.1: a zero `width`/`height` on a `<rect>`, or a zero radius on a
/// `<circle>`/`<ellipse>`, disables rendering of **the element** — not just its
/// fill. Chromium paints nothing for a zero-extent stroked rect (measured), so
/// the stroke must not survive the geometry.
#[test]
fn a_zero_extent_box_primitive_disables_its_stroke_too() {
    for body in [
        r##"  <rect x="32" y="16" width="0" height="32" fill="none" stroke="#000000" stroke-width="8"/>"##,
        r##"  <rect x="16" y="32" width="32" height="0" fill="none" stroke="#000000" stroke-width="8"/>"##,
        r##"  <circle cx="32" cy="32" r="0" fill="none" stroke="#000000" stroke-width="8"/>"##,
        r##"  <ellipse cx="32" cy="32" rx="0" ry="12" fill="none" stroke="#000000" stroke-width="8"/>"##,
    ] {
        let frame = admit_both(&document(body));
        assert!(
            frame.nodes[0].stroke.is_none(),
            "rendering is disabled, stroke included: {body}"
        );
        assert!(
            render_through_n0(&frame, 64, 64)
                .iter()
                .all(|byte| *byte == 0),
            "nothing paints at all: {body}"
        );
    }
    // A *path* is different: a zero-extent path is a zero-length segment, which
    // strokes as a cap-shaped dot.
    let dot = admit_both(&document(
        r##"  <path d="M32 32 L32 32" fill="none" stroke="#000000" stroke-width="16" stroke-linecap="round"/>"##,
    ));
    assert!(dot.nodes[0].stroke.is_some());
    assert_eq!(
        at(&render_through_n0(&dot, 64, 64), 32, 32),
        [0, 0, 0, 255],
        "a zero-length segment with a round cap is a dot"
    );
}

// ─── `<line>` ────────────────────────────────────────────────────────────

/// A `<line>` is compiled as a two-command path, not as a geometry kind of its
/// own: Chromium's `<line>` is byte-identical to the equivalent `<path>`
/// (measured), a line has no interior for a fill to cover, and the cap, join
/// and zero-length rules then come out identical for free.
#[test]
fn a_line_is_a_two_point_path() {
    let frame = admit_both(&document(
        r##"  <line x1="8" y1="32" x2="56" y2="32" stroke="#000000" stroke-width="8"/>"##,
    ));
    let Geometry::Path(path) = &frame.nodes[0].geometry else {
        panic!("a line resolves to path geometry");
    };
    assert_eq!(
        path.commands(),
        [
            PathCommand::MoveTo { x: 8.0, y: 32.0 },
            PathCommand::LineTo { x: 56.0, y: 32.0 },
        ]
    );
    assert!(!path.all_contours_closed());
    assert_eq!(
        render_through_n0(&frame, 64, 64),
        render_through_n0(
            &admit_both(&document(
                r##"  <path d="M8 32 L56 32" fill="none" stroke="#000000" stroke-width="8"/>"##,
            )),
            64,
            64
        ),
        "the line and the path are the same pixels"
    );
}

/// A line's fill never paints — its geometry has no area — and its endpoints
/// default to zero, which makes a bare `<line>` a zero-length segment: nothing
/// under the initial butt cap, a dot under a round one.
#[test]
fn a_lines_fill_never_paints_and_its_endpoints_default_to_zero() {
    let filled = admit_both(&document(
        r##"  <line x1="8" y1="8" x2="56" y2="56" fill="#16a34a"/>"##,
    ));
    assert!(
        render_through_n0(&filled, 64, 64)
            .iter()
            .all(|byte| *byte == 0),
        "a filled line with no stroke paints nothing"
    );

    let bare = admit_both(&document(
        r##"  <line stroke="#000000" stroke-width="8"/>"##,
    ));
    let Geometry::Path(path) = &bare.nodes[0].geometry else {
        panic!("path geometry");
    };
    assert_eq!(
        path.commands(),
        [
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::LineTo { x: 0.0, y: 0.0 },
        ],
        "every endpoint defaults to zero"
    );
    assert!(
        render_through_n0(&bare, 64, 64)
            .iter()
            .all(|byte| *byte == 0),
        "zero length under a butt cap paints nothing"
    );
    let capped = admit_both(&document(
        r##"  <line stroke="#000000" stroke-width="16" stroke-linecap="round"/>"##,
    ));
    assert_eq!(
        at(&render_through_n0(&capped, 64, 64), 2, 2),
        [0, 0, 0, 255],
        "and a dot under a round one"
    );
}

// ─── what still refuses ──────────────────────────────────────────────────

/// The stroke properties this slice does not consume refuse by name, through
/// both the attribute patrol and the computed-level read a stylesheet would
/// smuggle them past.
#[test]
fn the_unconsumed_stroke_properties_refuse_by_name() {
    for attr in [
        r##"stroke-dasharray="8 8""##,
        r##"stroke-dashoffset="4""##,
        r##"paint-order="stroke""##,
        r##"vector-effect="non-scaling-stroke""##,
    ] {
        let source = stroked_rect(&format!(r##"stroke-width="8" {attr}"##));
        assert!(
            matches!(refusal(&source), CompileError::UnsupportedAttribute { .. }),
            "{attr} must refuse by name"
        );
    }
    // The same values through a stylesheet, where only a computed-level read or
    // the CSS-name patrol can catch them.
    for css in [
        "stroke-dasharray: 8 8",
        "paint-order: stroke",
        "vector-effect: non-scaling-stroke",
    ] {
        let source = document(&format!(
            r##"  <style>rect {{ stroke: #000; stroke-width: 8; {css} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none"/>"##
        ));
        let error = refusal(&source);
        assert!(
            matches!(
                error,
                CompileError::UnsupportedStroke(_) | CompileError::UnsupportedStyle(_)
            ),
            "{css} must be declared, not silently dropped; got {error}"
        );
    }
}

/// A dash array that would paint nothing is admitted, because Chromium paints a
/// solid stroke for it: `none`, an all-zero array, and an invalid value are all
/// measured identical to no dash array at all. Refusing on a *non-empty* list
/// instead of a *painting* one would drop documents the browser renders solid.
#[test]
fn a_dash_array_that_paints_nothing_is_admitted() {
    for css in [
        "stroke-dasharray: none",
        "stroke-dasharray: 0",
        "stroke-dasharray: qqq",
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <style>rect {{ stroke: #000000; stroke-width: 8; {css} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none"/>"##
        )));
        assert_eq!(stroke_of(&frame, 0).width(), 8.0, "css={css:?}");
    }
}

/// A percentage `stroke-width` resolves against the viewport's normalized
/// diagonal (SVG2 §7.10; measured — `10%` of a 64x64 viewport paints 6.4
/// units), from the attribute and the CSS spellings alike, since both
/// arrive as the same computed percentage.
#[test]
fn a_percentage_stroke_width_resolves_against_the_normalized_diagonal() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000" stroke-width="10%"/>"##,
    ));
    let stroke = frame.nodes[0].stroke.as_ref().expect("a stroke");
    assert_eq!(stroke.width(), 6.4, "10% of sqrt(64² + 64²)/√2");
}

/// A stroke paint outside the gated value surface refuses exactly as the same
/// fill value does — one paint boundary, read twice.
#[test]
fn stroke_paint_beyond_the_gated_surface_refuses_by_name() {
    // `stroke: url(#…)` left this list with the gradient rung — a resolvable
    // reference strokes the gradient and an unresolvable one strokes the
    // measured nothing (see `gradients_contract`). The context paints remain
    // the named boundary.
    for css in ["stroke: context-fill", "stroke: context-stroke"] {
        let error = refusal(&document(&format!(
            r##"  <style>rect {{ {css}; stroke-width: 8 }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none"/>"##
        )));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(_)),
            "{css} must refuse under the stroke's own name; got {error}"
        );
    }
}

// ─── the unit surface ────────────────────────────────────────────────────

/// A cascaded length is only as good as its basis, and this build lacks two.
///
/// A **viewport-relative** width is the sharp one: Chromium resolves `1vw`
/// against the SVG viewport (0.64 units on a 64x64 document, measured
/// byte-identical to an authored `0.64`), while the cascade's device is pinned
/// at 1280x720 and computes 12.8 — a twentyfold error that painted silently.
/// A **font-metric** width (`ex`, `ch`, …) resolves from placeholder metrics
/// rather than measured ones. Both refuse by name; the computed value is
/// already absolute px, so the authored text is what carries the answer.
#[test]
fn a_stroke_width_whose_basis_this_build_lacks_refuses_by_name() {
    for unit in [
        "1vw",
        "1vh",
        "10vmin",
        "10vmax",
        "1dvw",
        "1lvh",
        "1ex",
        "1ch",
        "1cap",
        "1lh",
        // A calc carries the same basis, so it refuses with it.
        "calc(1vw + 2px)",
    ] {
        for source in [
            stroked_rect(&format!(r##"stroke-width="{unit}""##)),
            document(&format!(
                r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" style="stroke-width: {unit}"/>"##
            )),
            // The property inherits, so an ancestor's value reaches the shape.
            document(&format!(
                r##"  <g stroke="#000000" stroke-width="{unit}">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##
            )),
        ] {
            let error = refusal(&source);
            assert!(
                matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("basis")),
                "{unit} must refuse by name; got {error}"
            );
        }

        // The fourth spelling is a `<style>` sheet, and it is the one that
        // painted a silently wrong width: the attribute patrol walks
        // ancestors, so it cannot see a rule. The sheet refuses under the
        // document's name rather than the stroke's — a sheet is not
        // attributable to one element without selector matching — but refuse
        // it does, on the same unit list, so no spelling of the declaration
        // gets through quietly.
        let error = refusal(&document(&format!(
            r##"  <style>rect {{ stroke-width: {unit} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
        )));
        assert!(
            matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("basis")),
            "{unit} in a sheet must refuse by name; got {error}"
        );
    }
}

/// A cap is a per-*contour* property, and the consumer holds one cap per draw.
/// It can serve a path whose contours are all closed (the cap is inert, so it
/// strokes them under butt, byte-exact at every width) and a path with none.
/// A path that mixes them needs both caps at once, so a non-butt cap on one
/// refuses by name — over-refusal, since the divergence only appears once the
/// *device* width falls to about a pixel, which the compiler cannot know.
#[test]
fn a_mixed_contour_path_refuses_a_cap_it_cannot_apply_per_contour() {
    // One closed contour and one open contour, in one `d`.
    let mixed = "M16 16 L32 16 L32 32 Z M8 40 L40 40";
    for cap in ["round", "square"] {
        let error = refusal(&document(&format!(
            r##"  <path d="{mixed}" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="{cap}"/>"##
        )));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("mixes open and closed contours")),
            "{cap} on a mixed path must refuse by name; got {error}"
        );
    }

    // Butt needs no per-contour treatment, so the same geometry is admitted.
    let frame = admit_both(&document(&format!(
        r##"  <path d="{mixed}" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="butt"/>"##
    )));
    assert_eq!(stroke_of(&frame, 0).cap(), rframe::StrokeCap::Butt);

    // So does a non-butt cap on a path that is wholly closed, or wholly open —
    // those the consumer can serve.
    for d in ["M16 16 L32 16 L32 32 Z", "M8 40 L40 40"] {
        let frame = admit_both(&document(&format!(
            r##"  <path d="{d}" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"/>"##
        )));
        assert_eq!(
            stroke_of(&frame, 0).cap(),
            rframe::StrokeCap::Round,
            "d={d:?}"
        );
    }
}

/// A CSS property name is case-insensitive, so every ingress must be too.
///
/// This is the shape of a real leak, not a hypothetical: the `style` leg used a
/// case-sensitive `contains`, so `style="STROKE-WIDTH:1vw"` compiled to a stroke
/// 12.8 units wide — the cascade's pinned 1280px device — where Chromium paints
/// 0.64. The cascade honours the declaration whatever its case; the patrol
/// looking for it did not.
#[test]
fn the_unit_patrol_reads_a_property_name_case_insensitively() {
    for name in [
        "stroke-width",
        "STROKE-WIDTH",
        "Stroke-Width",
        "sTrOkE-wIdTh",
    ] {
        let attribute = refusal(&document(&format!(
            r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" style="{name}: 1vw"/>"##
        )));
        assert!(
            matches!(attribute, CompileError::UnsupportedStroke(ref reason) if reason.contains("basis")),
            "style attribute spelled {name} must refuse; got {attribute}"
        );

        let sheet = refusal(&document(&format!(
            r##"  <style>rect {{ {name}: 1vw }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
        )));
        assert!(
            matches!(sheet, CompileError::UnsupportedStyle(ref reason) if reason.contains("basis")),
            "sheet spelled {name} must refuse; got {sheet}"
        );
    }
}

/// The sheet leg refuses on the unit, not on the word `stroke-width`: a width
/// whose basis this build *does* have still renders from a sheet, and a
/// basis-less unit belonging to some other property in the same rule is not
/// this patrol's business.
#[test]
fn the_sheet_leg_refuses_the_unit_rather_than_the_property() {
    for (css, width) in [("8", 8.0), ("0.5em", 8.0), ("2px", 2.0)] {
        let frame = admit_both(&document(&format!(
            r##"  <style>rect {{ stroke-width: {css} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
        )));
        assert_eq!(stroke_of(&frame, 0).width(), width, "css={css:?}");
    }

    let frame = admit_both(&document(
        r##"  <style>rect { stroke-width: 2; font-size: 1ex }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
    ));
    assert_eq!(stroke_of(&frame, 0).width(), 2.0);
}

/// `em` and `rem` are *not* in that family: they resolve against `font-size`,
/// which this build represents and which the cascade now admits as a
/// presentation attribute — so an `em` width is exact rather than refused.
/// Chromium paints `0.5em` under `font-size="32"` as 16 units (measured); before
/// the hint was admitted this engine painted 8.
#[test]
fn an_em_stroke_width_resolves_against_the_authored_font_size() {
    let frame = admit_both(&document(
        r##"  <g font-size="32">
    <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="0.5em"/>
  </g>"##,
    ));
    assert_eq!(stroke_of(&frame, 0).width(), 16.0);
    // The CSS spelling of the same thing agrees, which is what makes the
    // presentation-attribute ingress a fidelity question rather than a feature.
    let ruled = admit_both(&document(
        r##"  <style>g { font-size: 32px }</style>
  <g>
    <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="0.5em"/>
  </g>"##,
    ));
    assert_eq!(stroke_of(&ruled, 0).width(), 16.0);
    // And with no authored font size, the initial 16px basis stands.
    let plain = admit_both(&stroked_rect(r##"stroke-width="0.5em""##));
    assert_eq!(stroke_of(&plain, 0).width(), 8.0);
}

/// A width whose *reach* cannot be represented refuses at the contract, so no
/// consumer receives an infinite coverage rectangle.
#[test]
fn a_stroke_whose_reach_overflows_refuses_by_name() {
    let error = refusal(&stroked_rect(
        r##"stroke-width="3.4e38" stroke-miterlimit="3.4e38""##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("representable")),
        "got {error}"
    );
}

/// A negative `width`/`height` on a `<rect>` disables rendering of that element
/// and **the rest of the document still renders** — measured in Chromium, where
/// a sibling rect paints normally. Carrying the negative extent instead would
/// reach the downstream's geometry validation and abort the whole render with an
/// internal message naming no element.
#[test]
fn a_negative_box_extent_disables_one_element_not_the_document() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="-32" height="32" fill="#16a34a" stroke="#000000" stroke-width="8"/>
  <rect x="4" y="4" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(frame.nodes.len(), 2, "both elements are admitted");
    assert!(frame.nodes[0].stroke.is_none(), "rendering is disabled");
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(
        at(&pixels, 6, 6),
        [0x25, 0x63, 0xeb, 255],
        "the sibling paints"
    );
    assert_eq!(
        at(&pixels, 20, 32),
        [0, 0, 0, 0],
        "the invalid rect does not"
    );
}

/// A stroke paint outside the gated surface is declared as a *stroke* problem.
/// The same unusable colour is an unsupported `fill` when it arrives as one —
/// a declared hole that names the wrong property misdirects whoever reads it.
/// (Translucent sRGB graduated with the translucency rung, so the pair here
/// is a colour space the slice still refuses.)
#[test]
fn an_unusable_stroke_paint_is_declared_under_the_strokes_name() {
    let error = refusal(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" stroke="color(display-p3 0 0 0)" stroke-width="8"/>"##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("color space")),
        "the stroke's own name: {error}"
    );
    let fill = refusal(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="color(display-p3 0 0 0)"/>"##,
    ));
    assert!(
        matches!(fill, CompileError::UnsupportedFill(_)),
        "and the fill's, for the same colour: {fill}"
    );
}
