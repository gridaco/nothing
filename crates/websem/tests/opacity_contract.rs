//! The group-scope rung's contract: element `opacity`, consumed by the
//! measured fold rule.
//!
//! Chromium's element opacity has distinct routes and each is meaning. A
//! single un-transformed, un-folded opacity pass **folds** opacity into that
//! pass's colour product, quantized once (byte-identical to paint opacity,
//! measured). A single gradient pass applies one separate alpha factor after
//! the gradient's intrinsic opacity materializes. Everything else composites
//! through a real **scope**: an isolated layer in the resolved contract,
//! restored at the group alpha. A valid selected transparent paint remains a
//! pass; `none`, an invalid URL without fallback, zero stroke width, and
//! pruned geometry do not. Nesting never flattens to a product (each layer
//! quantizes — measured one code value apart from the flat fold), a transform
//! strictly *below* the scope element breaks the fold, and a factor lands at
//! most once per pass. The probe matrix behind each law lives with the rung's
//! register addendum.

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

fn admit_inline_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_html_inline_svg(source).expect("strict admits");
    let best =
        SvgFrameSource::from_html_inline_svg_best_effort(source).expect("best-effort admits");
    assert!(
        best.degradations().iter().all(|degradation| matches!(
            degradation.action(),
            websem::DegradationAction::SamplesAsBase
        )),
        "inline Base only declares the still-closed sampled view: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn admit_static_css_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort admits Base");
    assert!(
        best.degradations().iter().all(|degradation| matches!(
            degradation.action(),
            websem::DegradationAction::SamplesAsBase
        )),
        "static CSS only declares the still-closed sampled view: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(
        frame,
        best.base_frame(),
        "Base admissions are frame-identical"
    );
    frame
}

fn scope_opacities(frame: &rframe::Frame) -> Vec<f32> {
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => {
                let ScopeEffect::Opacity(opacity) = scope.effect else {
                    panic!("opacity fixture emitted a non-opacity scope");
                };
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

fn painted_node_count(frame: &rframe::Frame) -> usize {
    frame
        .nodes()
        .iter()
        .filter(|node| !node.paints.is_empty() || node.stroke.is_some())
        .count()
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

/// A `<line>` has no visible fill area, but a valid selected fill still records
/// an opacity pass in Chromium. Its default fill plus visible stroke therefore
/// requires a scope; explicit `fill="none"` leaves the sole stroke pass and
/// folds exactly like `stroke-opacity`. Attribute, CSS, container, and
/// instance ingress all consume the same structural fact.
#[test]
fn a_line_counts_selected_fill_for_opacity_even_without_visible_fill_area() {
    for (label, static_css, body) in [
        (
            "attribute",
            false,
            r##"  <line x1="8" y1="8" x2="24" y2="24" stroke="#2563eb" stroke-width="3" opacity=".75"/>"##,
        ),
        (
            "style",
            true,
            r##"  <line x1="8" y1="8" x2="24" y2="24" stroke="#2563eb" stroke-width="3" style="opacity:.75"/>"##,
        ),
        (
            "stylesheet",
            true,
            r##"  <style>.target { opacity: .75 }</style>
  <line class="target" x1="8" y1="8" x2="24" y2="24" stroke="#2563eb" stroke-width="3"/>"##,
        ),
        (
            "container",
            false,
            r##"  <g opacity=".75"><line x1="8" y1="8" x2="24" y2="24" stroke="#2563eb" stroke-width="3"/></g>"##,
        ),
        (
            "use",
            false,
            r##"  <defs><line id="l" x1="8" y1="8" x2="24" y2="24" stroke="#2563eb" stroke-width="3"/></defs><use href="#l" opacity=".75"/>"##,
        ),
    ] {
        let source = document(body);
        let frame = if static_css {
            admit_static_css_both(&source)
        } else {
            admit_both(&source)
        };
        assert_eq!(scope_opacities(&frame), [0.75], "{label}");
        assert_eq!(
            frame.nodes().len(),
            1,
            "{label}: only the stroke is visible"
        );
    }

    let element = admit_both(&document(
        r##"  <line x1="8" y1="8" x2="24" y2="24" fill="none" stroke="#2563eb" stroke-width="3" opacity=".75"/>"##,
    ));
    let paint = admit_both(&document(
        r##"  <line x1="8" y1="8" x2="24" y2="24" fill="none" stroke="#2563eb" stroke-width="3" stroke-opacity=".75"/>"##,
    ));
    assert_eq!(element, paint, "one selected stroke pass folds");
    assert!(scope_opacities(&element).is_empty());
}

/// Paint selection, not resolved visible alpha, decides whether a pass exists.
/// Transparent colours and valid servers that resolve to no ink still block a
/// one-pass fold. `none`, an invalid URL without fallback, and a zero-width
/// stroke do not.
#[test]
fn transparent_selected_paints_are_passes_but_absent_paints_are_not() {
    for (label, sibling) in [
        (
            "transparent colour",
            r##"<rect x="2" y="2" width="8" height="8" fill="transparent"/>"##,
        ),
        (
            "zero-alpha colour",
            r##"<rect x="2" y="2" width="8" height="8" fill="#ef444400"/>"##,
        ),
        (
            "zero fill opacity",
            r##"<rect x="2" y="2" width="8" height="8" fill="#ef4444" fill-opacity="0"/>"##,
        ),
        (
            "stopless gradient",
            r##"<rect x="2" y="2" width="8" height="8" fill="url(#empty)"/>"##,
        ),
        (
            "transparent gradient",
            r##"<rect x="2" y="2" width="8" height="8" fill="url(#transparent)"/>"##,
        ),
        (
            "dash with no visible ink",
            r##"<path d="M2 2L10 10" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="butt" stroke-dasharray="0 4"/>"##,
        ),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <defs>
    <linearGradient id="empty"/>
    <linearGradient id="transparent"><stop stop-color="#ef4444" stop-opacity="0"/><stop offset="1" stop-color="#2563eb" stop-opacity="0"/></linearGradient>
  </defs>
  <g opacity=".5"><rect x="16" y="16" width="32" height="32" fill="#16a34a"/>{sibling}</g>"##
        )));
        assert_eq!(scope_opacities(&frame), [0.5], "{label}");
        assert!(
            (1..=2).contains(&frame.nodes().len()),
            "{label}: only the visible sibling and an optional transparent shader node"
        );
    }

    for (label, sibling) in [
        (
            "none",
            r##"<rect x="2" y="2" width="8" height="8" fill="none"/>"##,
        ),
        (
            "missing URL",
            r##"<rect x="2" y="2" width="8" height="8" fill="url(#missing)"/>"##,
        ),
        (
            "zero stroke width",
            r##"<path d="M2 2L10 10" fill="none" stroke="#ef4444" stroke-width="0"/>"##,
        ),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <g opacity=".5"><rect x="16" y="16" width="32" height="32" fill="#16a34a"/>{sibling}</g>"##
        )));
        assert!(
            scope_opacities(&frame).is_empty(),
            "{label}: sole pass folds"
        );
        assert_eq!(fill_alpha(&frame, 0), 128, "{label}");
    }
}

/// A non-identity opacity stage on non-pruned geometry remains a structural
/// fold barrier even when that geometry selects no paint or opacity is zero.
/// Empty geometry, hidden geometry, and an empty opacity container are pruned
/// and do not block the one visible sibling's fold.
#[test]
fn paintless_opacity_stages_block_outer_folds_until_their_geometry_is_pruned() {
    for (label, sibling) in [
        (
            "partial paintless shape",
            r##"<rect x="2" y="2" width="8" height="8" fill="none" opacity=".5"/>"##,
        ),
        (
            "zero paintless shape",
            r##"<rect x="2" y="2" width="8" height="8" fill="none" opacity="0"/>"##,
        ),
        (
            "nested zero container",
            r##"<g opacity="0"><rect x="2" y="2" width="8" height="8" fill="none"/></g>"##,
        ),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <g opacity=".5"><rect x="16" y="16" width="32" height="32" fill="#16a34a"/>{sibling}</g>"##
        )));
        assert_eq!(scope_opacities(&frame), [0.5], "{label}");
        assert_eq!(painted_node_count(&frame), 1, "{label}");
    }

    for (label, sibling) in [
        (
            "zero extent",
            r##"<rect x="2" y="2" width="0" height="8" fill="#ef4444" opacity=".5"/>"##,
        ),
        (
            "empty path",
            r##"<path d="" fill="#ef4444" opacity=".5"/>"##,
        ),
        (
            "hidden geometry",
            r##"<rect x="2" y="2" width="8" height="8" fill="#ef4444" opacity=".5" display="none"/>"##,
        ),
        ("empty container", r##"<g opacity=".5"/>"##),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <g opacity=".5"><rect x="16" y="16" width="32" height="32" fill="#16a34a"/>{sibling}</g>"##
        )));
        assert!(
            scope_opacities(&frame).is_empty(),
            "{label}: sole pass folds"
        );
        assert_eq!(fill_alpha(&frame, 0), 128, "{label}");
    }
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

/// A lone gradient keeps its intrinsic paint opacity and carries element
/// opacity as a separate post-paint factor. A gradient under a *real* group
/// scope is also admitted, but that remains a distinct raster operation.
#[test]
fn a_lone_gradient_carries_post_paint_opacity_for_fill_and_stroke() {
    let fill = admit_both(&document(
        r##"  <defs><linearGradient id="lg"><stop offset="0" stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#lg)" fill-opacity="0.25" opacity="0.5"/>"##,
    ));
    assert!(scope_opacities(&fill).is_empty());
    let fill_stack = &fill.nodes()[0].paints;
    assert_eq!(fill_stack.alpha_factor().get().to_bits(), 0.5f32.to_bits());
    match fill_stack.iter().next().expect("one fill paint") {
        cg::Paint::LinearGradient(gradient) => {
            assert_eq!(gradient.opacity.to_bits(), 0.25f32.to_bits())
        }
        other => panic!("expected a linear gradient, got {other:?}"),
    }

    let stroke = admit_both(&document(
        r##"  <defs><linearGradient id="lg"><stop offset="0" stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <rect x="12" y="12" width="40" height="40" fill="none" stroke="url(#lg)" stroke-width="8" stroke-opacity="0.25" opacity="0.5"/>"##,
    ));
    assert!(scope_opacities(&stroke).is_empty());
    let stroke_stack = stroke.nodes()[0]
        .stroke
        .as_ref()
        .expect("one stroke")
        .paints();
    assert_eq!(
        stroke_stack.alpha_factor().get().to_bits(),
        0.5f32.to_bits()
    );
    match stroke_stack.iter().next().expect("one stroke paint") {
        cg::Paint::LinearGradient(gradient) => {
            assert_eq!(gradient.opacity.to_bits(), 0.25f32.to_bits())
        }
        other => panic!("expected a linear gradient, got {other:?}"),
    }

    let under_scope = admit_both(&document(
        r##"  <defs><linearGradient id="lg"><stop offset="0" stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  <g opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="url(#lg)"/><rect x="2" y="58" width="4" height="4" fill="#000000"/></g>"##,
    ));
    assert_eq!(
        scope_opacities(&under_scope),
        [0.5],
        "a gradient inside a real layer is admitted"
    );
    assert_eq!(
        under_scope.nodes()[0].paints.alpha_factor().get(),
        1.0,
        "a real scope does not leak into the paint factor"
    );
}

/// A valid paint server keeps the post-paint stage. A one-stop ramp remains a
/// constant gradient for rasterization; a geometric degeneracy whose selected
/// stop alpha is an endpoint can resolve to a solid and still keep the later
/// element factor. A non-endpoint stop alpha refuses instead of flattening the
/// shader-alpha and element-alpha stages. An invalid URL's authored solid
/// fallback is an ordinary direct colour and folds all opacity factors into
/// that colour. The distinction is visible away from half-value rounding
/// coincidences.
#[test]
fn paint_server_outcomes_keep_their_alpha_stage() {
    let one_stop = admit_both(&document(
        r##"  <defs><linearGradient id="g"><stop stop-color="#16a34a" stop-opacity="0.30196078431372547"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" fill-opacity=".7" opacity=".6"/>"##,
    ));
    let stack = &one_stop.nodes()[0].paints;
    assert_eq!(stack.alpha_factor().get().to_bits(), 0.6f32.to_bits());
    match stack.iter().next().expect("one resolved paint") {
        cg::Paint::LinearGradient(gradient) => {
            assert_eq!(gradient.opacity.to_bits(), 0.7f32.to_bits());
            assert_eq!(gradient.stops.len(), 2);
            assert!(
                gradient
                    .stops
                    .iter()
                    .all(|stop| stop.color.a().to_bits() == (77.0f32 / 255.0).to_bits())
            );
        }
        other => panic!("one stop: expected a constant gradient, got {other:?}"),
    }

    let degenerate = admit_both(&document(
        r##"  <defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="0"><stop stop-color="#2563eb"/><stop offset="1" stop-color="#16a34a"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" opacity=".6"/>"##,
    ));
    let stack = &degenerate.nodes()[0].paints;
    assert_eq!(stack.alpha_factor().get().to_bits(), 0.6f32.to_bits());
    match stack.iter().next().expect("one resolved paint") {
        cg::Paint::Solid(solid) => assert_eq!(solid.color.a(), 255),
        other => panic!("degenerate geometry: expected a solid, got {other:?}"),
    }

    let staged_source = document(
        r##"  <defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="0"><stop stop-color="#2563eb"/><stop offset="1" stop-color="#16a34a" stop-opacity="0.30196078431372547"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" opacity=".6"/>"##,
    );
    let staged = SvgFrameSource::from_standalone_svg(staged_source, viewport())
        .expect_err("a collapsed non-endpoint shader alpha cannot carry later opacity");
    let websem::CompileError::UnsupportedFill(reason) = staged else {
        panic!("expected a fill refusal, got {staged:?}");
    };
    assert!(
        reason.contains("collapses before post-paint opacity"),
        "{reason}"
    );

    let fallback = admit_both(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="url(#missing) #16a34a4d" fill-opacity=".7" opacity=".6"/>"##,
    ));
    let stack = &fallback.nodes()[0].paints;
    assert_eq!(stack.alpha_factor().get(), 1.0);
    match stack.iter().next().expect("one fallback paint") {
        cg::Paint::Solid(solid) => assert_eq!(solid.color.a(), 32),
        other => panic!("expected a solid fallback, got {other:?}"),
    }
}

/// Root opacity is the same source-neutral isolated composite as a group,
/// enclosing the complete item stream. Attribute and CSS spellings meet in
/// one computed value, and standalone/inline entries state the same frame.
#[test]
fn root_opacity_wraps_the_complete_frame_in_both_entries() {
    let attribute = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" opacity="0.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg>"##;
    let css = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" style="opacity: 50%"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg>"##;
    let attribute_frame = admit_both(attribute);
    let css_frame = admit_static_css_both(css);
    assert_eq!(attribute_frame, css_frame);
    assert_eq!(scope_opacities(&attribute_frame), [0.5]);
    assert_eq!(fill_alpha(&attribute_frame, 0), 255);

    let inline = format!("<!doctype html><html><body>{attribute}</body></html>");
    assert_eq!(attribute_frame, admit_inline_both(&inline));
}

/// The inline entry keeps each HTML ancestor's computed opacity as a distinct
/// outer scope around the selected SVG-local raster. `opacity` is not
/// inherited by default, while an explicit `inherit` on the SVG compounds
/// with the ancestor layer rather than replacing it.
#[test]
fn html_ancestor_opacity_wraps_and_compounds_the_inline_svg() {
    let body_half = r##"<!doctype html><html><body style="opacity:.5"><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg></body></html>"##;
    let root_half = r##"<!doctype html><html><body><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" style="opacity:.5"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg></body></html>"##;
    assert_eq!(admit_inline_both(body_half), admit_inline_both(root_half));

    let inherited = r##"<!doctype html><html><body style="opacity:.5"><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" style="opacity:inherit"><rect x="8" y="8" width="48" height="48" fill="#16a34a"/></svg></body></html>"##;
    assert_eq!(scope_opacities(&admit_inline_both(inherited)), [0.5, 0.5]);
}

/// A zero-opacity HTML ancestor composites the selected SVG to nothing, so
/// the compiler does not inspect a rendering construct in that invisible
/// subtree. This is the host analogue of the admitted zero group/root law.
#[test]
fn zero_html_ancestor_is_empty_without_inspecting_the_svg_subtree() {
    let source = r##"<!doctype html><html><body style="opacity:0"><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><g marker-start="url(#m)"><rect width="64" height="64" fill="#16a34a"/></g></svg></body></html>"##;
    let frame = admit_inline_both(source);
    assert!(frame.items.is_empty());
    assert!(frame.nodes().is_empty());
}

/// A zero root is the correct empty frame. Like a zero-opacity group, it does
/// not descend into a child whose rendering construct would otherwise refuse.
#[test]
fn zero_root_is_empty_without_inspecting_its_rendering_subtree() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" opacity="0"><g marker-start="url(#m)"><rect width="64" height="64" fill="#16a34a"/></g></svg>"##;
    let frame = admit_both(source);
    assert!(frame.items.is_empty());
    assert!(frame.nodes().is_empty());
}

/// Root opacity consumes one slot of the checked scope budget: 63 nested
/// translucent containers plus the root are admitted; the next container
/// refuses before `FrameItems` construction instead of panicking at 65 scopes.
#[test]
fn root_opacity_counts_toward_the_scope_depth_budget() {
    let source = |depth: usize| {
        let mut body = String::new();
        for _ in 0..depth {
            body.push_str(r##"<g opacity="0.5"><rect width="1" height="1" fill="#16a34a"/>"##);
        }
        body.push_str(r##"<rect x="2" width="1" height="1" fill="#2563eb"/>"##);
        body.push_str(&"</g>".repeat(depth));
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" opacity="0.5">{body}</svg>"##
        )
    };

    let admitted = admit_both(&source(63));
    assert_eq!(scope_opacities(&admitted).len(), 64);

    let error = SvgFrameSource::from_standalone_svg(source(64), viewport())
        .expect_err("the 65th possible scope refuses before construction");
    assert!(
        error.to_string().contains("nesting deeper than 64"),
        "{error}"
    );
}

/// Host ancestors consume the same checked scope budget as SVG-local layers.
#[test]
fn html_ancestor_opacity_counts_toward_the_scope_depth_budget() {
    let source = |depth: usize| {
        let mut body = String::new();
        for _ in 0..depth {
            body.push_str(r##"<div style="opacity:.5">"##);
        }
        body.push_str(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="1" height="1" fill="#16a34a"/></svg>"##,
        );
        body.push_str(&"</div>".repeat(depth));
        format!("<!doctype html><html><body>{body}</body></html>")
    };

    let admitted = admit_inline_both(&source(64));
    assert_eq!(scope_opacities(&admitted).len(), 64);

    let error = SvgFrameSource::from_html_inline_svg(source(65))
        .expect_err("the 65th host scope refuses before construction");
    assert!(
        error.to_string().contains("nesting deeper than 64"),
        "{error}"
    );
}
