//! The translucency contract: `fill-opacity`, `stroke-opacity`, and
//! translucent sRGB paint.
//!
//! One rule generates the rung: paint alpha is the product of the colour's
//! own alpha and the paint-level opacity, multiplied in float and quantized
//! **once** — Chromium composites the product, not the quantized factors,
//! and the multiplied corpus cell (`svg-fill-opacity-times-alpha`) pins the
//! rounding byte-exactly. Element `opacity` is deliberately absent: it
//! composites fill and stroke together through a layer, which needs the
//! group scope, and it stays a named refusal until that rung.

// This binary consumes only the compiler half of the shared plumbing.
#[allow(dead_code)]
mod support;

use websem::{InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

fn admit_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort admits");
    assert!(
        best.degradations().is_empty(),
        "an admitted paint declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn fill_alpha(frame: &rframe::Frame, index: usize) -> u8 {
    match frame.nodes[index]
        .paints
        .iter()
        .next()
        .expect("a fill paint")
    {
        cg::Paint::Solid(solid) => solid.color.a(),
        other => panic!("expected a solid paint, got {other:?}"),
    }
}

/// The number and percentage spellings of `fill-opacity` are one grammar
/// (CSS <alpha-value>), so `0.5` and `50%` resolve to the same frame.
#[test]
fn the_percentage_spelling_is_the_number_spelling() {
    let number = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" fill-opacity="0.5"/>"##,
    ));
    let percentage = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" fill-opacity="50%"/>"##,
    ));
    assert_eq!(number, percentage);
    assert_eq!(fill_alpha(&number, 0), 128);
}

/// `fill-opacity` inherits: a `<g>` value reaches every descendant that
/// does not override it, through the one cascade.
#[test]
fn fill_opacity_inherits_through_a_container() {
    let frame = admit_both(&document(
        r##"  <g fill-opacity="0.5" fill="#16a34a"><rect x="8" y="8" width="24" height="24"/><rect x="40" y="8" width="16" height="16" fill-opacity="1"/></g>"##,
    ));
    assert_eq!(fill_alpha(&frame, 0), 128, "inherited");
    assert_eq!(
        fill_alpha(&frame, 1),
        255,
        "overridden by the element's own"
    );
}

/// `stroke-opacity` folds into the stroke paint's alpha exactly as
/// `fill-opacity` folds into the fill's — the two stacks are separate, so
/// a translucent stroke over an opaque fill composites without ever
/// multiplying the wrong paint.
#[test]
fn stroke_opacity_folds_into_the_stroke_alone() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" stroke="#2563eb" stroke-width="8" stroke-opacity="0.5"/>"##,
    ));
    assert_eq!(fill_alpha(&frame, 0), 255, "the fill stays opaque");
    let stroke = frame.nodes[0].stroke.as_ref().expect("a stroke");
    assert_eq!(
        match stroke.paints().iter().next().expect("a stroke paint") {
            cg::Paint::Solid(solid) => solid.color.a(),
            other => panic!("expected a solid paint, got {other:?}"),
        },
        128
    );
}

/// A paint-level opacity of zero folds to an invisible paint, which the
/// contract drops: `fill-opacity="0"` is `fill="none"`'s picture — an
/// admitted nothing, with the stroke (if any) still painting.
#[test]
fn zero_opacity_is_an_admitted_nothing() {
    let invisible = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" fill-opacity="0"/>"##,
    ));
    assert!(
        invisible.nodes[0].paints.is_empty(),
        "an invisible paint never enters the stack"
    );
    let none = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="none"/>"##,
    ));
    assert_eq!(
        invisible.nodes[0].paints.len(),
        none.nodes[0].paints.len(),
        "fill-opacity: 0 and fill: none are the same resolved fact"
    );
}

/// The colour's own alpha and the paint opacity multiply in float and
/// quantize once: `rgba(a=0.5)` under `fill-opacity="0.5"` is alpha 64
/// (0.25), not 128-then-halved-with-rounding-drift. The corpus cell bakes
/// the composite; this law pins the resolved value.
#[test]
fn color_alpha_and_paint_opacity_multiply_once() {
    let frame = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="rgba(22, 163, 74, 0.5)" fill-opacity="0.5"/>"##,
    ));
    assert_eq!(fill_alpha(&frame, 0), 64);
}

/// Element `opacity` stays a named refusal until the group scope: it
/// composites fill and stroke through one layer, which no per-paint alpha
/// fold can express without double-blending where they overlap.
#[test]
fn element_opacity_stays_a_named_refusal() {
    let strict = SvgFrameSource::from_standalone_svg(
        document(r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" opacity="0.5"/>"##),
        viewport(),
    )
    .expect_err("strict refuses");
    assert!(
        strict.to_string().contains("opacity"),
        "named; got {strict}"
    );
}
