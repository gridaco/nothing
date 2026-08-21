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
//! them (`fixtures/web-first/svg-stroke-*.svg`, 114 of 115 byte-exact — only
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
    admit_both_at(source, 64.0, 64.0)
}

fn admit_both_at(source: &str, width: f32, height: f32) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport(width, height))
        .expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(width, height))
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

fn assert_percentage_precision_alias(source: &str, label: &str) {
    let error = refusal(source);
    let rendered = error.to_string();
    assert!(
        rendered.contains("stroke-width percentage precision alias")
            && rendered.contains("loses Chromium used-value provenance"),
        "{label}: got {error}"
    );
}

fn assert_dashoffset_percentage_precision_alias(source: &str, label: &str) {
    let error = refusal(source);
    let rendered = error.to_string();
    assert!(
        rendered.contains("stroke-dashoffset percentage precision alias")
            && rendered.contains("loses Chromium used-value provenance"),
        "{label}: got {error}"
    );
}

fn stroke_of(frame: &rframe::Frame, index: usize) -> &rframe::Stroke {
    frame.nodes()[index]
        .stroke
        .as_ref()
        .expect("node carries a resolved stroke")
}

fn dash_phase(frame: &rframe::Frame, index: usize) -> f32 {
    stroke_of(frame, index)
        .dash()
        .expect("stroke carries an active dash pattern")
        .phase()
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
    assert_eq!(frame.nodes().len(), 1, "the stroke is not a second node");
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
        // The same length grammar in the CSS-property spelling.
        (r##"style="stroke-width: 0.5em""##, 8.0),
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
    // The whole absolute-unit family folds to px by the cascade's own
    // constants (this rung's lesson from the cq* family: a unit no test ever
    // spells is a unit whose parse can silently rot at the next Stylo pin).
    // The physical units are inexact in binary — 1cm is 96/2.54 px — so this
    // sweep pins them to a hair, not to the bit.
    let cm = 96.0 / 2.54;
    for (extra, expected) in [
        (r##"stroke-width="6pt""##, 8.0),
        (r##"stroke-width="0.5pc""##, 8.0),
        (r##"stroke-width="0.25in""##, 24.0),
        (r##"stroke-width="1cm""##, cm),
        (r##"stroke-width="10mm""##, cm),
        (r##"stroke-width="40Q""##, cm),
    ] {
        let frame = admit_both(&stroked_rect(extra));
        let width = stroke_of(&frame, 0).width();
        assert!(
            (width - expected).abs() < 0.01,
            "extra={extra:?}: width {width} is not {expected}"
        );
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

/// Blink clamps pure resolved Web lengths to its fixed-point layout ceiling
/// before stroke construction. The integer ceiling (33,554,429) rounds to
/// 33,554,428 in the frame's f32 vocabulary; both authored spellings must
/// produce that exact fact instead of reaching the stroke-reach refusal.
#[test]
fn a_huge_pure_stroke_width_clamps_to_the_web_used_length_ceiling() {
    for extra in [
        r##"stroke-width="3.4e38""##,
        r##"style="stroke-width: 3.4e38px""##,
    ] {
        let frame = admit_both(&stroked_rect(extra));
        assert_eq!(
            stroke_of(&frame, 0).width(),
            33_554_428.0,
            "extra={extra:?}"
        );
    }
}

/// The cascade's percentage fact has one fewer observable rounding step than
/// Blink's: Stylo stores `N / 100` as f32, while Blink first stores `N` as f32
/// and then evaluates `basis * N / 100`. These two adjacent authored floats
/// become the same Stylo fraction, but on the 64-unit diagonal only the latter
/// overflows Blink's intermediate. No computed-value consumer can tell which
/// used value won, so both spellings refuse under one stable capability name
/// through every cascade ingress, including a pure calc wrapper.
#[test]
fn a_percentage_precision_alias_straddling_blink_overflow_refuses_by_name() {
    for authored in ["5.3169116662270134e36%", "5.3169119831396635e36%"] {
        for value in [authored.to_string(), format!("calc({authored})")] {
            let sources = [
                (
                    "presentation attribute",
                    stroked_rect(&format!(r##"stroke-width="{value}""##)),
                ),
                (
                    "winning style attribute",
                    stroked_rect(&format!(
                        r##"stroke-width="6" style="stroke-width: {value}""##
                    )),
                ),
                (
                    "winning stylesheet",
                    document(&format!(
                        r##"  <style>rect {{ stroke-width: {value} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="6"/>"##
                    )),
                ),
                (
                    "inheritance",
                    document(&format!(
                        r##"  <g stroke="#000000" style="stroke-width: {value}">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##
                    )),
                ),
            ];

            for (ingress, source) in sources {
                assert_percentage_precision_alias(
                    &source,
                    &format!("{authored} via {value:?}, {ingress}"),
                );
            }
        }
    }
}

/// The typed preimage check cannot reconstruct every source that cssparser
/// folded into the same percentage fraction. A raw f64 literal can normalize
/// to Stylo's lower bucket even though Blink rounds the authored number to the
/// higher f32 first; percentage arithmetic erases still more operation
/// history. The authored-source patrol therefore guards all four cascade
/// ingresses under the same stable refusal instead of adding a second CSS
/// evaluator.
#[test]
fn authored_percentage_provenance_that_the_cascade_erases_refuses_by_name() {
    for value in [
        "57384.267578125007%",
        "calc(57384.265625% + 0.001953125007%)",
        "calc(28692.1337890625035% * 2)",
        "calc(57384.267578125007% + 0px)",
        "calc(57384.267578125007% + (1px - 1px))",
        "calc(57384.267578125007% + 0 * 1px)",
    ] {
        let sources = [
            (
                "presentation attribute",
                stroked_rect(&format!(r##"stroke-width="{value}""##)),
            ),
            (
                "winning style attribute",
                stroked_rect(&format!(
                    r##"stroke-width="6" style="stroke-width: {value}""##
                )),
            ),
            (
                "winning stylesheet",
                document(&format!(
                    r##"  <style>rect {{ stroke-width: {value} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="6"/>"##
                )),
            ),
            (
                "inheritance",
                document(&format!(
                    r##"  <g stroke="#000000" style="stroke-width: {value}">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##
                )),
            ),
        ];

        for (ingress, source) in sources {
            assert_percentage_precision_alias(&source, &format!("{value}, {ingress}"));
        }
    }
}

/// CSS priority is declaration syntax, not part of the percentage. The
/// authored-source patrol strips either spacing before it classifies the
/// winning value; otherwise an unsafe literal could hide behind the suffix or
/// a safe literal could be refused merely for being important.
#[test]
fn percentage_precision_patrol_reads_unsafe_values_before_css_priority() {
    let hidden_high = "57384.267578125007%";
    for (ingress, source) in [
        (
            "important style attribute",
            stroked_rect(&format!(
                r##"stroke-width="6" style="stroke-width: {hidden_high} ! important""##
            )),
        ),
        (
            "important stylesheet",
            document(&format!(
                r##"  <style>rect {{ stroke-width: {hidden_high} !important }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="6"/>"##
            )),
        ),
    ] {
        assert_percentage_precision_alias(&source, ingress);
    }
}

/// The provenance patrol is a refusal boundary, not a magnitude patrol. These
/// direct percentages preserve their numeric source through cssparser's
/// normalization and remain admitted; `calc(N%)` is the same identity source,
/// with no arithmetic history to reconstruct.
#[test]
fn recoverable_direct_percentage_sources_remain_admitted() {
    for value in ["3.4e38%", "5e36%", "10%", "1e9%", "calc(3.4e38%)"] {
        for (ingress, source) in [
            (
                "presentation attribute",
                stroked_rect(&format!(r##"stroke-width="{value}""##)),
            ),
            (
                "important style attribute",
                stroked_rect(&format!(r##"style="stroke-width: {value} !important""##)),
            ),
            (
                "important stylesheet",
                document(&format!(
                    r##"  <style>rect {{ stroke-width: {value} ! important }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
                )),
            ),
            (
                "inheritance",
                document(&format!(
                    r##"  <g stroke="#000000" style="stroke-width: {value}">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##
                )),
            ),
        ] {
            let frame = admit_both(&source);
            assert!(
                stroke_of(&frame, 0).width().is_finite(),
                "{value}, {ingress}"
            );
        }
    }
}

/// Percentages take a different used-value path from fixed lengths. Blink
/// multiplies the authored percentage by the viewport basis before dividing by
/// 100; positive overflow saturates to `f32::MAX`. That is still one ordinary
/// finite resolved stroke width. Its eventual ink depends on the complete
/// geometry/transform/stroke fact, so the producer carries the exact fact
/// rather than deriving a renderer-specific absence.
#[test]
fn a_percentage_product_saturation_is_one_finite_resolved_stroke() {
    let sources = vec![
        (
            "presentation attribute",
            stroked_rect(r##"stroke-width="3.4e38%""##),
        ),
        (
            "winning style attribute",
            stroked_rect(r##"stroke-width="6" style="stroke-width: 3.4e38%""##),
        ),
        (
            "calc presentation attribute",
            stroked_rect(r##"stroke-width="calc(3.4e38%)""##),
        ),
        (
            "winning calc style attribute",
            stroked_rect(r##"stroke-width="6" style="stroke-width: calc(3.4e38%)""##),
        ),
        (
            "winning stylesheet",
            document(
                r##"  <style>rect { stroke-width: 3.4e38% }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="6"/>"##,
            ),
        ),
        (
            "winning calc stylesheet",
            document(
                r##"  <style>rect { stroke-width: calc(3.4e38%) }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="6"/>"##,
            ),
        ),
        (
            "inheritance",
            document(
                r##"  <g stroke-width="3.4e38%">
    <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>
  </g>"##,
            ),
        ),
        (
            "calc inheritance",
            document(
                r##"  <g style="stroke-width: calc(3.4e38%)">
    <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>
  </g>"##,
            ),
        ),
    ];

    for (name, source) in sources {
        let frame = admit_both(&source);
        assert_eq!(
            stroke_of(&frame, 0).width().to_bits(),
            f32::MAX.to_bits(),
            "{name}"
        );
    }

    // A large percentage whose intermediate product is still finite remains
    // distinct. 1e9% of the normalized 64x64 diagonal is 640,000,000 — well
    // beyond the fixed-length ceiling but nowhere near the saturation fact.
    let finite = admit_both(&stroked_rect(r##"stroke-width="1e9%""##));
    assert_eq!(stroke_of(&finite, 0).width(), 640_000_000.0);

    // The overflow event is operation-order and basis dependent. The same
    // authored percentage remains finite against a small nonzero normalized
    // diagonal and saturates against a larger one.
    let small_basis = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 0.5 0.5">
  <path d="M0 0.25 H0.5" fill="none" stroke="#000" stroke-width="3.4e38%" stroke-linejoin="round"/>
</svg>"##,
    );
    let small_width = stroke_of(&small_basis, 0).width();
    assert!(
        small_width.is_finite() && small_width < f32::MAX,
        "the 0.5 basis keeps the multiply finite: {small_width}"
    );

    // Cap, join, miter and dash facts stay independent of the width's numeric
    // magnitude. The painter, not the producer, consumes their cross-product.
    let crossed = admit_both(&stroked_rect(
        r##"stroke-width="3.4e38%" stroke-linecap="square" stroke-linejoin="miter" stroke-miterlimit="3.4e38" stroke-dasharray="8 4""##,
    ));
    let stroke = stroke_of(&crossed, 0);
    assert_eq!(stroke.width(), f32::MAX);
    assert_eq!(stroke.cap(), StrokeCap::Square);
    assert_eq!(stroke.join(), StrokeJoin::Miter);
    assert_eq!(stroke.miter_limit(), 3.4e38_f32);
    assert_eq!(
        stroke
            .dash_intervals()
            .expect("ordinary dash cycle")
            .as_slice(),
        [8.0, 4.0]
    );
    assert!(stroke.outset().is_finite());
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
        assert!(frame.nodes()[0].stroke.is_none(), "extra={extra:?}");
    }
    // A shape with neither fill nor stroke is still a node — it has geometry,
    // it simply paints nothing.
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="none"/>"##,
    ));
    assert_eq!(frame.nodes().len(), 1);
    assert!(frame.nodes()[0].stroke.is_none());
    assert!(frame.nodes()[0].paints.is_empty());
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
            frame.nodes()[0].stroke.is_none(),
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
    assert!(dot.nodes()[0].stroke.is_some());
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
    let Geometry::Path(path) = &frame.nodes()[0].geometry else {
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
    let Geometry::Path(path) = &bare.nodes()[0].geometry else {
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

/// The stroke properties this slice does not consume refuse by name through
/// both authored spellings. Dasharray and the guarded dashoffset capability
/// have left this list; paint ordering and non-scaling geometry have not.
#[test]
fn the_unconsumed_stroke_properties_refuse_by_name() {
    for attr in [
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
    for css in ["paint-order: stroke", "vector-effect: non-scaling-stroke"] {
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

/// `pathLength` calibrates every SVGGeometryElement's distance space in
/// Chromium, including rect, circle, and ellipse. The zero-calibration dash
/// contract cannot carry that fact, so all seven admitted geometry elements
/// keep the same named patrol rather than silently scaling only some cycles.
#[test]
fn pathlength_refuses_on_every_admitted_geometry_element() {
    for shape in [
        r##"<rect x="8" y="8" width="48" height="48"/>"##,
        r##"<circle cx="32" cy="32" r="24"/>"##,
        r##"<ellipse cx="32" cy="32" rx="24" ry="16"/>"##,
        r##"<path d="M8 32 H56"/>"##,
        r##"<line x1="8" y1="32" x2="56" y2="32"/>"##,
        r##"<polygon points="8,8 56,8 32,56"/>"##,
        r##"<polyline points="8,8 32,56 56,8"/>"##,
    ] {
        let shape = shape.replacen(
            "/>",
            r##" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="2" pathLength="24"/>"##,
            1,
        );
        let error = refusal(&document(&format!("  {shape}")));
        assert!(
            matches!(error, CompileError::UnsupportedAttribute { ref attr, .. } if attr == "pathLength"),
            "{shape}: got {error}"
        );
    }
}

/// Dasharray is one cascaded, resolved cycle: both source spellings enter the
/// same longhand, percentages use the normalized diagonal, calc() resolves on
/// that basis, commas and spaces are separators, and an odd authored list is
/// repeated once before the frame boundary.
#[test]
fn a_dash_array_resolves_to_one_even_local_space_cycle() {
    for (extra, expected) in [
        (r##"stroke-dasharray="8 4""##, vec![8.0, 4.0]),
        (r##"style="stroke-dasharray: 8px, 4px""##, vec![8.0, 4.0]),
        (
            r##"stroke-dasharray="5 3 2""##,
            vec![5.0, 3.0, 2.0, 5.0, 3.0, 2.0],
        ),
        (r##"stroke-dasharray="10% 5%""##, vec![6.4, 3.2]),
        (
            r##"stroke-dasharray="calc(10% + 1.6px) 4""##,
            vec![8.0, 4.0],
        ),
        (r##"stroke-dasharray="0.5em 0.25em""##, vec![8.0, 4.0]),
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        assert_eq!(
            stroke_of(&frame, 0)
                .dash_intervals()
                .expect("active dash cycle")
                .as_slice(),
            expected,
            "extra={extra:?}"
        );
    }
}

/// The same pure-length used-value ceiling applies member-by-member before an
/// odd dash list is doubled. SVG unitless numbers, CSS lengths, and a calc()
/// simplified to a pure length therefore cross the frame boundary as the same
/// exact finite interval.
#[test]
fn huge_pure_dash_lengths_clamp_before_the_cycle_is_formed() {
    for extra in [
        r##"stroke-dasharray="3.4e38 3.4e38""##,
        r##"style="stroke-dasharray: 3.4e38px 3.4e38px""##,
        r##"style="stroke-dasharray: calc(3.4e38px) calc(3.4e38px)""##,
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        assert_eq!(
            stroke_of(&frame, 0)
                .dash_intervals()
                .expect("active finite cycle")
                .as_slice(),
            [33_554_428.0, 33_554_428.0],
            "extra={extra:?}"
        );
    }

    let odd = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-dasharray="3.4e38 0 0""##,
    ));
    assert_eq!(
        stroke_of(&odd, 0)
            .dash_intervals()
            .expect("active doubled cycle")
            .as_slice(),
        [33_554_428.0, 0.0, 0.0, 33_554_428.0, 0.0, 0.0]
    );
}

/// The cascade owns precedence and inheritance for the property: a rule beats
/// its presentation hint, an inline declaration beats the rule, an invalid CSS
/// declaration exposes the valid hint, and a container passes the computed
/// cycle to its descendant.
#[test]
fn dasharray_precedence_and_inheritance_are_the_cascades() {
    let frame = admit_both(&document(
        r##"  <style>
    #rule { stroke-dasharray: 6 2 }
    #inline { stroke-dasharray: 6 2 }
    #invalid { stroke-dasharray: qqq }
  </style>
  <rect id="rule" x="4" y="4" width="8" height="8" fill="none" stroke="#000" stroke-dasharray="8 4"/>
  <rect id="inline" x="16" y="4" width="8" height="8" fill="none" stroke="#000" stroke-dasharray="8 4" style="stroke-dasharray: 10 5"/>
  <rect id="invalid" x="28" y="4" width="8" height="8" fill="none" stroke="#000" stroke-dasharray="8 4"/>
  <g stroke="#000" stroke-dasharray="12 3"><rect x="40" y="4" width="8" height="8" fill="none"/></g>"##,
    ));
    let cycles: Vec<&[f32]> = frame
        .nodes()
        .iter()
        .map(|node| {
            node.stroke
                .as_ref()
                .expect("stroke")
                .dash_intervals()
                .expect("cycle")
                .as_slice()
        })
        .collect();
    assert_eq!(
        cycles,
        [&[6.0, 2.0][..], &[10.0, 5.0], &[8.0, 4.0], &[12.0, 3.0]]
    );
}

#[test]
fn dasharray_inherits_from_the_root() {
    let frame = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" stroke="#000000" stroke-dasharray="8 4">
  <path d="M8 32 H56" fill="none" stroke-width="8"/>
</svg>"##,
    );
    assert_eq!(
        stroke_of(&frame, 0)
            .dash_intervals()
            .expect("root-inherited cycle")
            .as_slice(),
        [8.0, 4.0]
    );
}

/// `<use>` establishes the inherited style for its instantiated subtree. The
/// clone receives the computed dash cycle from the use site; no source-facing
/// dash syntax leaks across the resolved frame boundary.
#[test]
fn dasharray_inherits_from_a_use_site() {
    let frame = admit_both(&document(
        r##"  <defs><path id="p" d="M8 32 H56" fill="none"/></defs>
  <use href="#p" stroke="#000000" stroke-width="8" stroke-dasharray="8 4"/>"##,
    ));
    assert_eq!(frame.nodes().len(), 1);
    assert_eq!(
        stroke_of(&frame, 0)
            .dash_intervals()
            .expect("use-site cycle")
            .as_slice(),
        [8.0, 4.0]
    );
}

/// `none`, an all-zero list, and an invalid or negative declaration all paint
/// the solid fallback in Chromium. They therefore normalize to dash absence,
/// not to a second present-cycle spelling. A zero-painted butt cycle paints
/// nothing, while round caps make its zero-length painted entries visible.
#[test]
fn neutral_and_zero_painted_dash_cycles_keep_their_measured_meaning() {
    for extra in [
        r##"stroke-dasharray="none""##,
        r##"stroke-dasharray="0 0""##,
        r##"stroke-dasharray="qqq""##,
        r##"stroke-dasharray="-1 4""##,
        r##"style="stroke-dasharray: none""##,
        r##"style="stroke-dasharray: 0 0""##,
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        assert!(
            stroke_of(&frame, 0).dash_intervals().is_none(),
            "extra={extra:?}"
        );
    }

    let butt = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-dasharray="0 8""##,
    ));
    assert!(butt.nodes()[0].stroke.is_none(), "butt dots paint nothing");

    let round = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-dasharray="0 8" stroke-linecap="round""##,
    ));
    assert_eq!(
        stroke_of(&round, 0)
            .dash_intervals()
            .expect("round dots remain active")
            .as_slice(),
        [0.0, 8.0]
    );
}

/// Percentages do not take the pure-length ceiling. When resolving them makes
/// the repeated f32 cycle non-finite, Chromium drops the dash path effect: the
/// result is a solid stroke, with the authored cap still intact. Attribute and
/// CSS declarations produce the same normalized dash absence.
#[test]
fn a_nonfinite_percentage_dash_cycle_normalizes_to_solid() {
    for extra in [
        r##"stroke-dasharray="3.4e38% 3.4e38%""##,
        r##"style="stroke-dasharray: 3.4e38% 3.4e38%""##,
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        assert!(
            stroke_of(&frame, 0).dash_intervals().is_none(),
            "extra={extra:?}"
        );
    }

    for extra in [
        r##"stroke-linecap="round" stroke-dasharray="0 3.4e38% 0 3.4e38%""##,
        r##"style="stroke-linecap: round; stroke-dasharray: 0 3.4e38% 0 3.4e38%""##,
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        let stroke = stroke_of(&frame, 0);
        assert_eq!(stroke.cap(), StrokeCap::Round, "extra={extra:?}");
        assert!(
            stroke.dash_intervals().is_none(),
            "zero-first overflow is solid, not an initial cap dot: extra={extra:?}"
        );
    }

    // A negative CSS member remains invalid grammar. It drops at the cascade
    // and exposes the valid presentation hint; overflow normalization must not
    // turn invalid declarations into an accepted solid value.
    let fallback = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-dasharray="8 4" style="stroke-dasharray: -1 2""##,
    ));
    assert_eq!(
        stroke_of(&fallback, 0)
            .dash_intervals()
            .expect("valid presentation fallback")
            .as_slice(),
        [8.0, 4.0]
    );
}

/// Dashoffset is one signed local-space fact paired with the checked interval
/// cycle. Numbers and lengths share the grammar, pure length math folds before
/// the compiler reads it, and the contract owns sign and multi-cycle modulo.
#[test]
fn a_dashoffset_resolves_to_one_canonical_local_space_phase() {
    for (extra, expected) in [
        (r##"stroke-dashoffset="4""##, 4.0),
        (r##"style="stroke-dashoffset: 4px""##, 4.0),
        (r##"stroke-dashoffset="4e0""##, 4.0),
        (r##"stroke-dashoffset="16""##, 4.0),
        (r##"stroke-dashoffset="-8""##, 4.0),
        (r##"stroke-dashoffset="-4""##, 8.0),
        (r##"stroke-dashoffset="12""##, 0.0),
        (r##"stroke-dashoffset="-12""##, 0.0),
        (r##"stroke-dashoffset="-0""##, 0.0),
        (r##"style="stroke-dashoffset: calc(2px + 2px)""##, 4.0),
        (r##"style="stroke-dashoffset: min(6px, 4px)""##, 4.0),
        (r##"style="stroke-dashoffset: max(2px, 4px)""##, 4.0),
        (r##"style="stroke-dashoffset: clamp(2px, 4px, 6px)""##, 4.0),
    ] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" {extra}"##
        )));
        assert_eq!(dash_phase(&frame, 0), expected, "extra={extra:?}");
    }

    let omitted = admit_both(&stroked_rect(
        r##"stroke-width="8" stroke-dasharray="8 4""##,
    ));
    assert_eq!(dash_phase(&omitted, 0), 0.0, "the initial phase is zero");
}

/// Percentage phases use the current viewport's normalized diagonal, retain
/// their sign, and keep an identity calc wrapper. A deliberately non-square
/// viewBox distinguishes that basis from both axes: sqrt(70² + 10²)/sqrt(2)
/// is exactly 50, so 20% is ten local-space units.
#[test]
fn a_percentage_dashoffset_uses_the_normalized_diagonal() {
    for (extra, expected) in [
        (r##"stroke-dashoffset="10%""##, 6.4),
        (r##"style="stroke-dashoffset: 10%""##, 6.4),
        (r##"style="stroke-dashoffset: calc(10%)""##, 6.4),
        (r##"stroke-dashoffset="-10%""##, 5.6),
    ] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" {extra}"##
        )));
        assert!(
            (dash_phase(&frame, 0) - expected).abs() < 0.000_01,
            "extra={extra:?}: got {}",
            dash_phase(&frame, 0)
        );
    }

    let non_square = r##"<svg xmlns="http://www.w3.org/2000/svg" width="70" height="10" viewBox="0 0 70 10">
  <path d="M2 5 H68" fill="none" stroke="#000" stroke-width="2"
        stroke-dasharray="8 4" stroke-dashoffset="20%"/>
</svg>"##;
    let frame = admit_both_at(non_square, 70.0, 10.0);
    assert_eq!(dash_phase(&frame, 0), 10.0);
}

/// The one cascade owns dashoffset precedence, inheritance, and CSS-wide
/// values. In particular, `revert` removes the presentation hint with the
/// author origin while `revert-layer` exposes it; these are measured browser
/// identities, not producer-side keyword handling.
#[test]
fn dashoffset_precedence_inheritance_and_css_wide_values_are_the_cascades() {
    let frame = admit_both(&document(
        r##"  <style>
    #rule { stroke-dashoffset: 6 }
    #inline { stroke-dashoffset: 6 }
    #invalid { stroke-dashoffset: qqq }
    #revert { stroke-dashoffset: revert }
    @layer top { #revert-layer { stroke-dashoffset: revert-layer } }
  </style>
  <rect id="rule" x="2" y="2" width="6" height="6" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="4"/>
  <rect id="inline" x="10" y="2" width="6" height="6" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="4" style="stroke-dashoffset: 10"/>
  <rect id="invalid" x="18" y="2" width="6" height="6" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="4"/>
  <g stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="8">
    <rect x="26" y="2" width="6" height="6" fill="none"/>
    <rect x="34" y="2" width="6" height="6" fill="none" style="stroke-dashoffset: inherit"/>
    <rect x="42" y="2" width="6" height="6" fill="none" style="stroke-dashoffset: unset"/>
    <rect x="50" y="2" width="6" height="6" fill="none" style="stroke-dashoffset: initial"/>
  </g>
  <rect id="revert" x="2" y="12" width="6" height="6" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="4"/>
  <rect id="revert-layer" x="10" y="12" width="6" height="6" fill="none" stroke="#000" stroke-dasharray="8 4" stroke-dashoffset="4"/>
"##,
    ));
    let phases: Vec<f32> = frame
        .nodes()
        .iter()
        .map(|node| {
            node.stroke
                .as_ref()
                .expect("stroke")
                .dash()
                .expect("dash")
                .phase()
        })
        .collect();
    assert_eq!(phases, [6.0, 10.0, 4.0, 8.0, 8.0, 8.0, 0.0, 0.0, 4.0]);
}

/// `<use>` establishes the inherited style of its instance. The target path
/// receives the use site's phase exactly like its dash intervals and paint.
#[test]
fn dashoffset_inherits_from_a_use_site() {
    let frame = admit_both(&document(
        r##"  <defs><path id="p" d="M8 32 H56" fill="none"/></defs>
  <use href="#p" stroke="#000" stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="4"/>"##,
    ));
    assert_eq!(frame.nodes().len(), 1);
    assert_eq!(dash_phase(&frame, 0), 4.0);
}

/// A phase cannot change a solid stroke. The producer therefore does not
/// inspect even an otherwise unsupported offset when no positive dash cycle
/// survives; one neutral dash spelling remains one resolved absence.
#[test]
fn dashoffset_is_inert_without_an_active_cycle() {
    for extra in [
        r##"stroke-dashoffset="57384.267578125007%""##,
        r##"stroke-dasharray="none" stroke-dashoffset="1vw""##,
        r##"stroke-dasharray="0 0" style="--p: 4px; stroke-dashoffset: var(--p)""##,
        r##"stroke-dasharray="3.4e38% 3.4e38%" stroke-dashoffset="4""##,
    ] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="8" {extra}"##)));
        assert!(
            stroke_of(&frame, 0).dash().is_none(),
            "extra={extra:?} stays one solid stroke"
        );
    }
}

/// Blink clamps fixed lengths before phase modulo. The lower fixed-point bound
/// is asymmetric: +3.4e38 becomes 33,554,428 (phase 4 on this cycle), while
/// -3.4e38 becomes -33,554,430 (canonical phase 6). Percentage overflow uses
/// the finite f32 ceiling instead and lands at the cycle boundary.
#[test]
fn huge_dashoffsets_keep_blinks_used_value_route() {
    for (extra, expected) in [
        (r##"stroke-dashoffset="3.4e38""##, 4.0),
        (r##"style="stroke-dashoffset: 3.4e38px""##, 4.0),
        (r##"stroke-dashoffset="-3.4e38""##, 6.0),
        (r##"style="stroke-dashoffset: -3.4e38px""##, 6.0),
        (r##"stroke-dashoffset="3.4e38%""##, 0.0),
        (r##"style="stroke-dashoffset: -3.4e38%""##, 0.0),
    ] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" {extra}"##
        )));
        assert_eq!(dash_phase(&frame, 0), expected, "extra={extra:?}");
    }
}

/// Distinct valid authored percentages can collapse into one pinned-Stylo f32
/// while Chromium retains distinct used phases. The unsafe member through
/// every attributable ingress, its signed mirror, the overflow boundary, and
/// percentage-bearing math therefore refuse under one stable capability name.
/// This is the deliberate SPLIT that keeps both dashoffset checklist twins
/// open; the recoverable member remains admitted rather than broad-refused.
#[test]
fn dashoffset_percentage_precision_aliases_refuse_by_name() {
    for value in [
        "57384.267578125007%",
        "-57384.267578125007%",
        "5.3169116662270134e36%",
        "5.3169119831396635e36%",
        "calc(57384.265625% + 0.001953125007%)",
        "calc(57384.265625% + 0px)",
        "min(57384.265625%, 57384.267578125007%)",
        "max(10%, 20%)",
        "clamp(10%, 20%, 30%)",
    ] {
        for (ingress, source) in [
            (
                "presentation attribute",
                stroked_rect(&format!(
                    r##"stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{value}""##
                )),
            ),
            (
                "style attribute",
                stroked_rect(&format!(
                    r##"stroke-width="8" stroke-dasharray="8 4" style="stroke-dashoffset: {value}""##
                )),
            ),
            (
                "inherited attribute",
                document(&format!(
                    r##"  <g stroke="#000" stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{value}">
    <path d="M8 32 H56" fill="none"/>
  </g>"##
                )),
            ),
            (
                "stylesheet",
                document(&format!(
                    r##"  <style>path {{ stroke-dashoffset: {value} }}</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##
                )),
            ),
        ] {
            assert_dashoffset_percentage_precision_alias(&source, &format!("{value}, {ingress}"));
        }
    }
}

#[test]
fn recoverable_direct_dashoffset_percentages_remain_admitted() {
    // Chromium 149 exact controls (measured, not celled): on this 64-unit
    // percentage basis and 12-unit cycle, the positive source is phase
    // 5.9296875 and its signed mirror is phase 6.0703125. Nearby phase controls
    // discriminate the positive result; the separate alias matrix
    // discriminates both signs at this source threshold.
    for (value, expected_phase) in [("57384.265625%", 5.9296875), ("-57384.265625%", 6.0703125)] {
        let attr = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{value}""##
        )));
        let css = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" style="stroke-dashoffset: {value}""##
        )));
        assert_eq!(dash_phase(&attr, 0), expected_phase, "value={value}");
        assert_eq!(dash_phase(&css, 0), expected_phase, "value={value}");
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
    let stroke = frame.nodes()[0].stroke.as_ref().expect("a stroke");
    assert_eq!(stroke.width(), 6.4, "10% of sqrt(64² + 64²)/√2");
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
        "1vi",
        "1vb",
        "10vmin",
        "10vmax",
        "1dvw",
        "1lvh",
        "1ex",
        "1ch",
        "1cap",
        "1lh",
        // The root-relative twins are their own tokens — `1rex` never matches
        // an `ex` entry (the `e` is preceded by `r`, not a digit), so each
        // needs its own list row; `1rex` painted a silent 8.0 where Chromium
        // paints the root ex-height before these were listed.
        "1rex",
        "1rch",
        "1ric",
        "1rcap",
        // Container-query units: the pinned Stylo drops them to the initial 1
        // where Chromium resolves the small-viewport fallback (12.5cqw of a
        // 64px document is 8) — a silent 1-versus-8 before these were listed.
        "12.5cqw",
        "12.5cqh",
        "12.5cqi",
        "12.5cqb",
        "12.5cqmin",
        "12.5cqmax",
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

/// Dash lengths share the width rung's authored-basis patrol. The computed
/// list has already forgotten whether a number came from a viewport/container
/// basis, var() substitution, an escaped token, or an em basis poisoned by an
/// unrepresentable font-size; every measured silent-divergence class therefore
/// refuses through both source spellings before a cycle reaches rframe.
#[test]
fn a_dasharray_with_an_untrustworthy_basis_refuses_by_name() {
    for unit in [
        "1vw",
        "1vh",
        "1vi",
        "1vb",
        "10vmin",
        "10vmax",
        "1dvw",
        "1lvh",
        "1ex",
        "1ch",
        "1cap",
        "1lh",
        "1rex",
        "1rch",
        "1ric",
        "1rcap",
        "12.5cqw",
        "12.5cqh",
        "12.5cqi",
        "12.5cqb",
        "12.5cqmin",
        "12.5cqmax",
        "calc(1vw + 2px)",
    ] {
        for source in [
            stroked_rect(&format!(r##"stroke-dasharray="{unit} 4""##)),
            document(&format!(
                r##"  <style>rect {{ stroke-dasharray: {unit} 4 }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
            )),
        ] {
            let error = refusal(&source);
            assert!(
                matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("basis"))
                    || matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("basis")),
                "{unit} must refuse by name; got {error}"
            );
        }
    }

    for extra in [
        r##"style="--d: 8px; stroke-dasharray: var(--d) 4""##,
        r##"stroke-dasharray="1\76 w 4""##,
        r##"font-size="2vw" stroke-dasharray="1em 4""##,
    ] {
        let error = refusal(&stroked_rect(extra));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(ref reason)
                if reason.contains("var()") || reason.contains("escape") || reason.contains("font-size")),
            "{extra} must refuse by name; got {error}"
        );
    }

    for source in [
        // Presentation attributes accept substitution too; the authored
        // provenance must be patrolled before the typed value forgets it.
        stroked_rect(r##"style="--d: 8px" stroke-dasharray="var(--d) 4""##),
        // The property inherits, so the ancestor's authored provenance must
        // remain visible when the descendant resolves its stroke.
        document(
            r##"  <g stroke="#000000" style="--d: 8px; stroke-dasharray: var(--d) 4">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##,
        ),
        // A comment can split the property name at either authored CSS
        // ingress; stripping it is what lets the patrol see the unit.
        document(
            r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" style="stroke-/**/dasharray: 1vw 4"/>"##,
        ),
        document(
            r##"  <style>rect { stroke-/**/dasharray: 1vw 4 }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
        ),
        // A sheet can carry both halves of the poisoned em relationship.
        document(
            r##"  <style>rect { font-size: 2vw; stroke-dasharray: 1em 4 }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
        ),
        // A sheet is scanned before selector matching, so its var() leg has
        // its own document-level guard rather than relying on element text.
        document(
            r##"  <style>rect { --d: 8px; stroke-dasharray: var(--d) 4 }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
        ),
        // An escaped property name can hide any declaration from a coarse
        // name scan, so it takes the earlier blanket CSS-escape refusal.
        document(
            r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" style="stroke-\64 asharray: 8 4"/>"##,
        ),
    ] {
        let error = refusal(&source);
        assert!(
            error.to_string().contains("stroke-dasharray")
                || error.to_string().contains("escape")
                || error.to_string().contains("\\64 asharray"),
            "every hidden dasharray ingress must refuse by name; got {error}"
        );
    }
}

/// Absolute units and the two trustworthy font-relative units are resolved by
/// the cascade before the dash contract. This sweep guards the admitted half
/// of the unit split while the other unit classes retain their own rows.
#[test]
fn dasharray_admits_the_trustworthy_length_unit_family() {
    for (value, expected) in [
        ("8px 4px", [8.0, 4.0]),
        ("6pt 3pt", [8.0, 4.0]),
        ("0.5pc 0.25pc", [8.0, 4.0]),
        ("0.083333333in 0.041666667in", [8.0, 4.0]),
        ("0.211666667cm 0.105833333cm", [8.0, 4.0]),
        ("2.11666667mm 1.05833333mm", [8.0, 4.0]),
        ("8.46666667Q 4.23333333Q", [8.0, 4.0]),
        ("0.5em 0.25em", [8.0, 4.0]),
        ("0.5rem 0.25rem", [8.0, 4.0]),
    ] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="{value}""##
        )));
        let actual = stroke_of(&frame, 0)
            .dash_intervals()
            .expect("cycle")
            .as_slice();
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| (*actual - expected).abs() < 0.001),
            "value={value:?}: got {actual:?}"
        );
    }
}

/// Dashoffset shares the trustworthy resolved-length surface: absolute units
/// and em/rem cross the frame boundary only after the cascade has reduced them
/// to one local-space value.
#[test]
fn dashoffset_admits_the_trustworthy_length_unit_family() {
    for value in [
        "4px",
        "3pt",
        "0.25pc",
        "0.041666667in",
        "0.105833333cm",
        "1.05833333mm",
        "4.23333333Q",
        "0.25em",
        "0.25rem",
    ] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{value}""##
        )));
        assert!(
            (dash_phase(&frame, 0) - 4.0).abs() < 0.001,
            "value={value:?}: got {}",
            dash_phase(&frame, 0)
        );
    }
}

/// The computed phase has forgotten whether its length used a viewport,
/// container, or placeholder font-metric basis. The generic stroke-length
/// patrol must therefore cover dashoffset through attribute, inline CSS,
/// inheritance, and stylesheet routes, plus var(), escaped tokens, and a
/// poisoned em/rem font basis.
#[test]
fn a_dashoffset_with_an_untrustworthy_basis_refuses_by_name() {
    for unit in [
        "1vw",
        "1vh",
        "1vi",
        "1vb",
        "10vmin",
        "10vmax",
        "1dvw",
        "1lvh",
        "1ex",
        "1ch",
        "1cap",
        "1lh",
        "1rex",
        "1rch",
        "1ric",
        "1rcap",
        "12.5cqw",
        "12.5cqh",
        "12.5cqi",
        "12.5cqb",
        "12.5cqmin",
        "12.5cqmax",
        "calc(1vw + 2px)",
    ] {
        for source in [
            stroked_rect(&format!(
                r##"stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{unit}""##
            )),
            stroked_rect(&format!(
                r##"stroke-width="8" stroke-dasharray="8 4" style="stroke-dashoffset: {unit}""##
            )),
            document(&format!(
                r##"  <g stroke="#000" stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="{unit}">
    <path d="M8 32 H56" fill="none"/>
  </g>"##
            )),
            document(&format!(
                r##"  <style>path {{ stroke-dashoffset: {unit} }}</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##
            )),
        ] {
            let error = refusal(&source);
            assert!(
                error.to_string().contains("stroke-dashoffset")
                    && error.to_string().contains("basis"),
                "{unit} must refuse by name; got {error}"
            );
        }
    }

    for source in [
        stroked_rect(
            r##"stroke-width="8" stroke-dasharray="8 4" style="--p: 4px; stroke-dashoffset: var(--p)""##,
        ),
        stroked_rect(
            r##"stroke-width="8" stroke-dasharray="8 4" style="--p: 4px" stroke-dashoffset="var(--p)""##,
        ),
        document(
            r##"  <g stroke="#000" stroke-width="8" stroke-dasharray="8 4" style="--p: 4px; stroke-dashoffset: var(--p)">
    <path d="M8 32 H56" fill="none"/>
  </g>"##,
        ),
        stroked_rect(r##"stroke-width="8" stroke-dasharray="8 4" stroke-dashoffset="1\76 w""##),
        stroked_rect(
            r##"stroke-width="8" stroke-dasharray="8 4" font-size="2vw" stroke-dashoffset="1em""##,
        ),
        document(
            r##"  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4" style="stroke-/**/dashoffset: 1vw"/>"##,
        ),
        document(
            r##"  <style>path { stroke-/**/dashoffset: 1vw }</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##,
        ),
        document(
            r##"  <style>path { font-size: 2vw; stroke-dashoffset: 1em }</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##,
        ),
        document(
            r##"  <style>path { --p: 4px; stroke-dashoffset: var(--p) }</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##,
        ),
        document(
            r##"  <style>path { stroke-\64 ashoffset: 4 }</style>
  <path d="M8 32 H56" fill="none" stroke="#000" stroke-width="8" stroke-dasharray="8 4"/>"##,
        ),
    ] {
        let error = refusal(&source);
        assert!(
            error.to_string().contains("stroke-dashoffset") || error.to_string().contains("escape"),
            "every hidden dashoffset ingress must refuse by name; got {error}"
        );
    }
}

/// A `calc()` mixing lengths and percentages is the one `<length-percentage>`
/// that survives to computed-value time unresolved: pure-length math folds to
/// an absolute length in the cascade, and a pure percentage resolves against
/// the normalized diagonal, but the mixed sum needs both bases at once and the
/// resolve reads neither through a calc tree. Chromium resolves it (measured:
/// `calc(10% + 0.8px)` is byte-identical to an authored `7.2` on a 64x64
/// document), so a silent drop of either term would be a wrong pixel.
///
/// Unlike the basis-less units above, no authored-text patrol is involved:
/// the mixed value is caught at resolve, where the element is known — so all
/// four spellings refuse under the stroke's own name, the `<style>` sheet
/// included.
#[test]
fn a_stroke_width_calc_mixing_lengths_and_percentages_refuses_by_name() {
    for value in ["calc(10% + 2px)", "min(10%, 12px)"] {
        for source in [
            stroked_rect(&format!(r##"stroke-width="{value}""##)),
            document(&format!(
                r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" style="stroke-width: {value}"/>"##
            )),
            // The property inherits, so an ancestor's value reaches the shape.
            document(&format!(
                r##"  <g stroke="#000000" stroke-width="{value}">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##
            )),
            document(&format!(
                r##"  <style>rect {{ stroke-width: {value} }}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
            )),
        ] {
            let error = refusal(&source);
            assert!(
                matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("mixing lengths and percentages")),
                "{value} must refuse by name; got {error}"
            );
        }
    }

    // The fold boundaries stay admitted: pure-length math is absolute by
    // computed-value time, and a pure percentage has the diagonal basis —
    // both resolve to the same 8-unit stroke (Chromium-baked as
    // `svg-stroke-width-calc`, `svg-stroke-width-css-min`, and the percent
    // cells).
    for value in ["calc(4px + 4px)", "min(8px, 12px)"] {
        let frame = admit_both(&stroked_rect(&format!(r##"stroke-width="{value}""##)));
        assert_eq!(stroke_of(&frame, 0).width(), 8.0, "value={value:?}");
    }
}

/// `var()` hides the unit from the authored-text patrol: `--w: 1vw` fed
/// through a sheet's `stroke-width: var(--w)` painted a silent 12.8 where
/// Chromium paints 0.64 (measured) — the exact wrong number the device-pin
/// doc warns about, reached through a spelling the unit scan cannot see.
/// Which declaration feeds a substitution is a resolver question, not a
/// patrol question, so every `var(` in stroke-width-bearing text refuses —
/// including one that would have resolved to an honest length (Chromium
/// substitutes `var()` in all four spellings, the presentation attribute
/// included; measured).
#[test]
fn a_stroke_width_through_var_indirection_refuses_by_name() {
    for extra in [
        // Undefined: the substitution fails to the inherited/initial width in
        // both engines — but the *next* author edit defines it, so the patrol
        // refuses the spelling, not the outcome.
        r##"stroke-width="var(--w)""##,
        r##"style="--w: 8px; stroke-width: var(--w)""##,
    ] {
        let error = refusal(&stroked_rect(extra));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("var()")),
            "{extra} must refuse by name; got {error}"
        );
    }

    // The property inherits, so an ancestor's var()-spelled width reaches the
    // shape; the walk sees the ancestor's style attribute.
    let error = refusal(&document(
        r##"  <g stroke="#000000" style="stroke-width: var(--w)">
    <rect x="16" y="16" width="32" height="32" fill="none"/>
  </g>"##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("var()")),
        "an inherited var() width must refuse by name; got {error}"
    );

    // The sheet spelling — the one that painted the measured 12.8 — refuses
    // under the document's name, like every sheet finding.
    for sheet in [
        r##"rect { --w: 1vw; stroke-width: var(--w) }"##,
        // A var() that would have resolved honestly refuses too: over-refusal
        // is the contract, a resolver is not.
        r##"rect { --w: 8px; stroke-width: var(--w) }"##,
    ] {
        let error = refusal(&document(&format!(
            r##"  <style>{sheet}</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##
        )));
        assert!(
            matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("var()")),
            "{sheet} must refuse by name; got {error}"
        );
    }
}

/// `em` and `rem` are admitted because `font-size` is a basis the cascade
/// has — which makes the *font-size* an ingress for every basis the cascade
/// lacks. `font-size: 2vw` under `stroke-width: 1em` painted a silent ~25.6
/// where Chromium paints 1.28 (2vw of the 64px viewport = 1.28, measured).
/// So when a stroke-width in scope is font-relative, every authored
/// font-size — attribute, style attribute (the `font` shorthand included),
/// ancestor, or sheet — must be free of basis-less units, `var()`, and
/// escapes; and a sheet's own em-width refuses when a poisoned font-size is
/// authored anywhere.
#[test]
fn a_font_relative_stroke_width_under_a_poisoned_font_size_refuses_by_name() {
    // The attributable combinations refuse at the element.
    for extra in [
        r##"style="font-size: 2vw" stroke-width="1em""##,
        r##"font-size="2vw" style="stroke-width: 1em""##,
        r##"style="font: 2vw sans-serif" stroke-width="1em""##,
        r##"style="font-size: var(--fs)" stroke-width="1em""##,
        r##"font-size="2vw" stroke-width="0.5rem""##,
    ] {
        let error = refusal(&stroked_rect(extra));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("font-size")),
            "{extra} must refuse by name; got {error}"
        );
    }

    // The basis inherits, so an ancestor's poisoned font-size reaches the
    // shape's em width.
    let error = refusal(&document(
        r##"  <g style="font-size: 2vw">
    <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="1em"/>
  </g>"##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("font-size")),
        "an inherited poisoned font-size must refuse by name; got {error}"
    );

    // A sheet can set the font-size of any ancestor without being
    // attributable to one — the element's em width refuses on a descent.
    let error = refusal(&document(
        r##"  <style>rect { font-size: 2vw }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" stroke-width="1em"/>"##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("font-size")),
        "a sheet-poisoned em width must refuse by name; got {error}"
    );

    // And a sheet's own em-spelled width refuses when the poison is authored
    // anywhere — on an element or in the same sheet.
    for body in [
        r##"  <style>rect { stroke-width: 1em }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000" font-size="2vw"/>"##,
        r##"  <style>rect { stroke-width: 1em; font-size: 2vw }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
    ] {
        let error = refusal(&document(body));
        assert!(
            matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("font-size")),
            "a sheet em width under authored poison must refuse by name; got {error}"
        );
    }

    // The honest half stays admitted: an authored absolute font-size IS the
    // em basis (Chromium-baked as `svg-stroke-width-em-font-size` at 8px,
    // where the default 16px basis would paint double and fail the raster),
    // and a poisoned font-size with no font-relative width in scope is inert.
    for (font_size, expected) in [("8px", 8.0), ("32px", 32.0)] {
        let frame = admit_both(&stroked_rect(&format!(
            r##"style="font-size: {font_size}" stroke-width="1em""##
        )));
        assert_eq!(stroke_of(&frame, 0).width(), expected);
    }
    let frame = admit_both(&stroked_rect(
        r##"style="font-size: 2vw" stroke-width="8""##,
    ));
    assert_eq!(stroke_of(&frame, 0).width(), 8.0);
}

/// A CSS escape is the tokenizer's spelling, not this patrol's: `1\76 w` is
/// `1vw` to the cascade and nothing to a text scan — measured painting the
/// same silent 12.8 in all three ingresses — and an escape can hide the
/// *property name* as well as the unit. So an escape anywhere in
/// stroke-width-bearing text refuses: the attribute's own value, any `style`
/// attribute in scope (scanned whole, before the name filter an escaped name
/// would fool), and any sheet.
#[test]
fn a_stroke_width_spelled_through_css_escapes_refuses_by_name() {
    for extra in [
        r##"stroke-width="1\76 w""##,
        r##"style="stroke-width: 1\76 w""##,
    ] {
        let error = refusal(&stroked_rect(extra));
        assert!(
            matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("escape")),
            "{extra} must refuse by name; got {error}"
        );
    }

    // The escaped property name: no scan that filters on "stroke-width" can
    // see this declaration — the style attribute's *name* patrol wins the
    // race and refuses it as a property it cannot check, which is the same
    // loud outcome through an earlier door.
    let error = refusal(&stroked_rect(
        r##"style="stroke-\77 idth: 1vw" stroke-width="8""##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("stroke-\\77 idth"))
            || matches!(error, CompileError::UnsupportedStroke(ref reason) if reason.contains("escape")),
        "an escaped property name must refuse by name; got {error}"
    );

    let error = refusal(&document(
        r##"  <style>rect { stroke-width: 1\76 w }</style>
  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#000000"/>"##,
    ));
    assert!(
        matches!(error, CompileError::UnsupportedStyle(ref reason) if reason.contains("escape")),
        "a sheet escape must refuse by name; got {error}"
    );
}

/// A cap is a per-*contour* property on a solid stroke, and the consumer holds
/// one cap per draw. It can serve a path whose solid contours are all closed
/// (the cap is inert, so it strokes them under butt) and a path with none. A
/// solid path that mixes them needs both caps at once, so a non-butt cap refuses
/// by name. With dashing, every painted segment has ends even on a closed
/// contour, so the same authored cap correctly applies to both kinds.
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

    let dashed = admit_both(&document(&format!(
        r##"  <path d="{mixed}" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-dasharray="4 4"/>"##
    )));
    let stroke = stroke_of(&dashed, 0);
    assert_eq!(stroke.cap(), rframe::StrokeCap::Round);
    assert_eq!(
        stroke
            .dash_intervals()
            .expect("the dashed mixed path is active")
            .as_slice(),
        [4.0, 4.0]
    );
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

/// A finite stroke stays one resolved fact even when its conservative reach
/// exceeds `f32`. The contract widens only that derived arithmetic, while the
/// Web fixed-length ceiling and the authored miter limit remain exact carried
/// facts.
#[test]
fn a_wide_derived_reach_does_not_change_the_resolved_stroke() {
    let frame = admit_both(&stroked_rect(
        r##"stroke-width="3.4e38" stroke-miterlimit="3.4e38""##,
    ));
    let stroke = stroke_of(&frame, 0);
    assert_eq!(stroke.width(), 33_554_428.0);
    assert_eq!(stroke.miter_limit(), 3.4e38_f32);
    assert!(stroke.outset().is_finite());
    assert!(stroke.outset() > f64::from(f32::MAX));
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
    assert_eq!(frame.nodes().len(), 2, "both elements are admitted");
    assert!(frame.nodes()[0].stroke.is_none(), "rendering is disabled");
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
