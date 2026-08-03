//! The group-scope rung's contract: element `opacity`, consumed by the
//! measured fold rule.
//!
//! Chromium's element opacity has two byte-distinct routes, one code value
//! apart, and both are meaning: content that is a single un-transformed,
//! un-folded draw **folds** the opacity into that draw's paint — one float
//! product with the colour's alpha and `fill-opacity`/`stroke-opacity`,
//! quantized once (byte-identical to the paint-level fold, measured) —
//! and everything else composites through a real **scope**: an isolated
//! layer in the resolved contract, restored at the group alpha. Nesting
//! never flattens to a product (each layer quantizes — measured one code
//! value apart from the flat fold), a transform strictly *below* the
//! scope element breaks the fold, and the fold fires at most once per
//! draw. The probe matrix behind each law lives with the rung's register
//! addendum.

// This binary consumes only the compiler half of the shared plumbing.
#[allow(dead_code)]
mod support;

use rframe::{FrameItem, ScopeEffect};
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
        "an admitted opacity declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn scope_opacities(frame: &rframe::Frame) -> Vec<f32> {
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => {
                let ScopeEffect::Opacity(opacity) = scope.effect;
                Some(opacity.get())
            }
            _ => None,
        })
        .collect()
}

fn fill_alpha(frame: &rframe::Frame, index: usize) -> u8 {
    match frame.nodes()[index]
        .paints
        .iter()
        .next()
        .expect("a fill paint")
    {
        cg::Paint::Solid(solid) => solid.color.a(),
        other => panic!("expected a solid paint, got {other:?}"),
    }
}

/// A lone unstroked shape folds its element opacity into the fill exactly
/// as `fill-opacity` folds — the two spellings resolve to the identical
/// frame (measured byte-identical in Chromium), and no scope appears.
#[test]
fn a_lone_fill_folds_like_fill_opacity() {
    let element = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="0.5"/>"##,
    ));
    let paint = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" fill-opacity="0.5"/>"##,
    ));
    assert_eq!(element, paint);
    assert!(
        scope_opacities(&element).is_empty(),
        "a fold is not a scope"
    );
    assert_eq!(fill_alpha(&element, 0), 128);
}

/// Element opacity joins the one float product: `opacity` × `fill-opacity`
/// on one shape quantizes once — the multiply-once law of the translucency
/// rung, extended by measurement to the element factor.
#[test]
fn element_and_fill_opacity_multiply_once() {
    let stacked = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="0.5" fill-opacity="0.5"/>"##,
    ));
    let flat = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" fill-opacity="0.25"/>"##,
    ));
    assert_eq!(stacked, flat);
    assert_eq!(fill_alpha(&stacked, 0), 64);
}

/// The percentage spelling is the number spelling — one <alpha-value>
/// grammar through the one cascade, for the element property too.
#[test]
fn the_percentage_spelling_is_the_number_spelling() {
    let number = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="0.5"/>"##,
    ));
    let percentage = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="50%"/>"##,
    ));
    assert_eq!(number, percentage);
}

/// A stroke-only shape folds into the stroke's paint — the one draw is the
/// stroke, and its ink outside the geometry bounds still paints (a fold
/// clamps nothing).
#[test]
fn a_lone_stroke_folds_into_the_stroke_paint() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="none" stroke="#2563eb" stroke-width="8" opacity="0.5"/>"##,
    ));
    assert!(scope_opacities(&frame).is_empty());
    let nodes = frame.nodes();
    let stroke = nodes[0].stroke.as_ref().expect("a stroke");
    assert_eq!(
        match stroke.paints().iter().next().expect("a stroke paint") {
            cg::Paint::Solid(solid) => solid.color.a(),
            other => panic!("expected a solid paint, got {other:?}"),
        },
        128
    );
}

/// A shape with fill *and* stroke composites both through one isolated
/// scope — the double-blend fact that kept element opacity a refusal until
/// this rung: per-paint folding would blend the stroke-over-fill overlap
/// twice (measured 57 code values apart).
#[test]
fn fill_and_stroke_composite_through_one_scope() {
    let frame = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" stroke="#2563eb" stroke-width="8" opacity="0.5"/>"##,
    ));
    assert_eq!(scope_opacities(&frame), [0.5]);
    assert_eq!(fill_alpha(&frame, 0), 255, "the paint alpha is untouched");
    let per_paint = admit_both(&document(
        r##"  <rect x="16" y="16" width="32" height="32" fill="#16a34a" stroke="#2563eb" stroke-width="8" fill-opacity="0.5" stroke-opacity="0.5"/>"##,
    ));
    assert_ne!(frame, per_paint, "a scope is not a per-paint fold");
}

/// A group whose content is more than one draw composites through one
/// scope; a group whose one visible draw is plain folds instead — the
/// hidden sibling paints nothing and does not break the fold (measured:
/// Chromium's bytes are the fold's).
#[test]
fn a_group_scopes_many_draws_and_folds_one() {
    let many = admit_both(&document(
        r##"  <g opacity="0.5"><rect x="8" y="8" width="32" height="32" fill="#16a34a"/><rect x="24" y="24" width="32" height="32" fill="#2563eb"/></g>"##,
    ));
    assert_eq!(scope_opacities(&many), [0.5]);
    assert_eq!(many.nodes().len(), 2);

    let one = admit_both(&document(
        r##"  <g opacity="0.5"><rect x="8" y="8" width="24" height="24" fill="#16a34a" visibility="hidden"/><rect x="40" y="40" width="16" height="16" fill="#2563eb"/></g>"##,
    ));
    assert!(scope_opacities(&one).is_empty(), "one visible draw folds");
    assert_eq!(fill_alpha(&one, 0), 128);
}

/// The fold reaches through a plain container and past a zero-draw
/// sibling (measured byte-identical to the lone fold), and fires at most
/// once: a group over an already-folded draw is a real scope, never a
/// multiplied scalar — nested layers quantize per layer in Chromium, one
/// code value apart from the flat product.
#[test]
fn the_fold_fires_once_and_nesting_never_flattens() {
    let through_plain = admit_both(&document(
        r##"  <g opacity="0.5"><g><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></g></g>"##,
    ));
    assert!(scope_opacities(&through_plain).is_empty());
    assert_eq!(fill_alpha(&through_plain, 0), 128);

    for (label, body) in [
        (
            "nested groups",
            r##"  <g opacity="0.5"><g opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></g></g>"##,
        ),
        (
            "group over a shape's own fold",
            r##"  <g opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="0.5"/></g>"##,
        ),
    ] {
        let frame = admit_both(&document(body));
        assert_eq!(scope_opacities(&frame), [0.5], "{label}: the outer layers");
        assert_eq!(fill_alpha(&frame, 0), 128, "{label}: the inner folded");
    }
}

/// A transform strictly below the scope element breaks the fold (measured:
/// an intermediate transformed container, or a transformed draw, is one
/// code value from the fold — Chromium runs the real layer); the scope
/// element's own transform does not.
#[test]
fn a_transform_below_the_scope_breaks_the_fold() {
    let below = admit_both(&document(
        r##"  <g opacity="0.5"><g transform="translate(4,4)"><rect x="4" y="4" width="48" height="48" fill="#16a34a"/></g></g>"##,
    ));
    assert_eq!(scope_opacities(&below), [0.5]);

    let on_draw = admit_both(&document(
        r##"  <g opacity="0.5"><rect x="4" y="4" width="48" height="48" fill="#16a34a" transform="translate(4,4)"/></g>"##,
    ));
    assert_eq!(scope_opacities(&on_draw), [0.5]);

    let on_scope_element = admit_both(&document(
        r##"  <rect x="4" y="4" width="48" height="48" fill="#16a34a" transform="translate(4,4)" opacity="0.5"/>"##,
    ));
    assert!(
        scope_opacities(&on_scope_element).is_empty(),
        "transform and opacity on one element still fold"
    );
    assert_eq!(fill_alpha(&on_scope_element, 0), 128);
}

/// Zero composites nothing — an admitted nothing for a shape and for a
/// whole group, with siblings painting and nothing declared. The clamp is
/// Chromium's: a negative value is zero, a value above one is opaque.
#[test]
fn zero_and_the_clamp_are_admitted_nothings() {
    for body in [
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" opacity="0"/><rect x="40" y="40" width="16" height="16" fill="#2563eb"/>"##,
        r##"  <g opacity="0"><rect x="8" y="8" width="24" height="24" fill="#16a34a"/></g><rect x="40" y="40" width="16" height="16" fill="#2563eb"/>"##,
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" opacity="-0.5"/><rect x="40" y="40" width="16" height="16" fill="#2563eb"/>"##,
    ] {
        let frame = admit_both(&document(body));
        assert_eq!(frame.nodes().len(), 1, "only the sibling paints");
        assert!(scope_opacities(&frame).is_empty());
    }
    let above_one = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" opacity="1.5"/>"##,
    ));
    assert_eq!(
        fill_alpha(&above_one, 0),
        255,
        "clamped to opaque, no scope"
    );
    assert!(scope_opacities(&above_one).is_empty());
}

/// `<use>` and `<a>` scope exactly as `<g>` does: an instance composites
/// as one group, and opacity through a translucent target compounds
/// per-layer (measured byte-identical to the nested-group bytes).
#[test]
fn use_and_anchor_scope_like_a_group() {
    let via_use = admit_both(&document(
        r##"  <defs><g id="pair"><rect x="8" y="8" width="32" height="32" fill="#16a34a"/><rect x="24" y="24" width="32" height="32" fill="#2563eb"/></g></defs>
  <use href="#pair" opacity="0.5"/>"##,
    ));
    assert_eq!(scope_opacities(&via_use), [0.5]);

    let compound = admit_both(&document(
        r##"  <defs><g id="half" opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></g></defs>
  <use href="#half" opacity="0.5"/>"##,
    ));
    assert_eq!(scope_opacities(&compound), [0.5], "the outer layers");
    assert_eq!(fill_alpha(&compound, 0), 128, "the inner folded");

    let via_anchor = admit_both(&document(
        r##"  <a opacity="0.5"><rect x="8" y="8" width="32" height="32" fill="#16a34a"/><rect x="24" y="24" width="32" height="32" fill="#2563eb"/></a>"##,
    ));
    assert_eq!(scope_opacities(&via_anchor), [0.5]);
}

/// What still refuses, by name: element opacity folding over a gradient
/// paint (one quantized alpha slot cannot carry Chromium's
/// fold-after-quantization — measured one code value apart), and the
/// root's opacity (it composites the whole canvas, which an opaque raster
/// surface cannot express). A gradient under a *real* scope is admitted —
/// the layer modulates the composite, not the paint.
#[test]
fn the_remaining_refusals_name_their_constructs() {
    let gradient = document(
        r##"  <defs><linearGradient id="lg"><stop offset="0" stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#lg)" opacity="0.5"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(gradient.as_str(), viewport())
        .expect_err("strict refuses the gradient fold");
    assert!(
        strict.to_string().contains("gradient"),
        "named; got {strict}"
    );

    let under_scope = admit_both(&document(
        r##"  <defs><linearGradient id="lg"><stop offset="0" stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <g opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="url(#lg)"/><rect x="2" y="58" width="4" height="4" fill="#000000"/></g>"##,
    ));
    assert_eq!(
        scope_opacities(&under_scope),
        [0.5],
        "a gradient inside a real layer is admitted"
    );

    let root = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg>"##;
    let strict = SvgFrameSource::from_standalone_svg(root, viewport())
        .expect_err("strict refuses the root's opacity");
    assert!(
        strict.to_string().contains("root <svg>"),
        "named; got {strict}"
    );
    SvgFrameSource::from_standalone_svg_best_effort(root, viewport())
        .expect_err("the root contract refuses in both admissions");
}
