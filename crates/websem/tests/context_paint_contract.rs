//! Context paint is selected and fully rebased before `rframe`.
//! Every assertion here is a value-level companion to the Chromium probe
//! matrix in the context-paint capability rung.

use math2::transform::AffineTransform;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

#[allow(dead_code)]
mod support;

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
        .expect("best effort admits");
    let static_degradations: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(static_degradations.is_empty(), "{static_degradations:?}");
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame());
    frame
}

fn refusal(source: &str) -> CompileError {
    SvgFrameSource::from_standalone_svg(source, viewport())
        .expect_err("must refuse")
        .clone()
}

fn fill_color(frame: &rframe::Frame) -> cg::CGColor {
    match frame.nodes()[0].paints.iter().next().expect("fill") {
        cg::Paint::Solid(paint) => paint.color,
        other => panic!("expected solid fill, got {other:?}"),
    }
}

fn stroke_color(frame: &rframe::Frame) -> cg::CGColor {
    match frame.nodes()[0]
        .stroke
        .as_ref()
        .expect("stroke")
        .paints()
        .iter()
        .next()
        .expect("paint")
    {
        cg::Paint::Solid(paint) => paint.color,
        other => panic!("expected solid stroke, got {other:?}"),
    }
}

fn linear_fill(frame: &rframe::Frame, node: usize) -> &cg::LinearGradientPaint {
    match frame.nodes()[node].paints.iter().next().expect("fill") {
        cg::Paint::LinearGradient(paint) => paint,
        other => panic!("expected linear gradient, got {other:?}"),
    }
}

fn radial_fill(frame: &rframe::Frame, node: usize) -> &cg::RadialGradientPaint {
    match frame.nodes()[node].paints.iter().next().expect("fill") {
        cg::Paint::RadialGradient(paint) => paint,
        other => panic!("expected radial gradient, got {other:?}"),
    }
}

#[test]
fn no_context_selects_no_paint_for_both_properties_and_keywords() {
    for paint in ["context-fill", "context-stroke"] {
        let fill = admit_both(&document(&format!(
            r##"<rect x="8" y="8" width="48" height="48" fill="{paint}"/>"##
        )));
        assert!(fill.nodes()[0].paints.is_empty());

        let stroke = admit_both(&document(&format!(
            r##"<rect x="8" y="8" width="48" height="48" fill="none" stroke="{paint}" stroke-width="8"/>"##
        )));
        assert!(stroke.nodes()[0].stroke.is_none());
    }
}

#[test]
fn all_four_property_crossings_select_the_host_paint() {
    let cases = [
        (
            "fill",
            "context-fill",
            "fill",
            "#e11d48",
            cg::CGColor::from_rgb(225, 29, 72),
            false,
        ),
        (
            "fill",
            "context-stroke",
            "stroke",
            "#2563eb",
            cg::CGColor::from_rgb(37, 99, 235),
            false,
        ),
        (
            "stroke",
            "context-fill",
            "fill",
            "#16a34a",
            cg::CGColor::from_rgb(22, 163, 74),
            true,
        ),
        (
            "stroke",
            "context-stroke",
            "stroke",
            "#f59e0b",
            cg::CGColor::from_rgb(245, 158, 11),
            true,
        ),
    ];
    for (destination, context, owner, color, expected, is_stroke) in cases {
        let source = document(&format!(
            r##"<defs><rect id="r" x="16" y="16" width="32" height="32" fill="{}" stroke="{}" stroke-width="8"/></defs>
<use href="#r" fill="{}" stroke="{}"/>"##,
            if destination == "fill" {
                context
            } else {
                "none"
            },
            if destination == "stroke" {
                context
            } else {
                "none"
            },
            if owner == "fill" { color } else { "none" },
            if owner == "stroke" { color } else { "none" },
        ));
        let frame = admit_both(&source);
        let actual = if is_stroke {
            stroke_color(&frame)
        } else {
            fill_color(&frame)
        };
        assert_eq!(actual, expected, "{context}");
    }
}

#[test]
fn recursion_uses_the_nearest_context_and_keeps_the_eventual_outer_owner() {
    let nearest = admit_both(&document(
        r##"<defs><rect id="leaf" x="16" y="16" width="32" height="32" fill="context-fill"/><use id="mid" href="#leaf" fill="#16a34a"/></defs>
<use href="#mid" fill="#e11d48"/>"##,
    ));
    assert_eq!(fill_color(&nearest), cg::CGColor::from_rgb(22, 163, 74));

    let url = admit_both(&document(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient><rect id="leaf" x="4" y="12" width="20" height="40" fill="context-fill"/><g id="outer"><use href="#leaf" fill="context-fill"/><rect x="40" y="12" width="20" height="40" visibility="hidden"/></g></defs><use href="#outer" fill="url(#g)"/>"##,
    ));
    let transform = linear_fill(&url, 0).transform;
    assert_eq!(
        transform,
        AffineTransform::from_acebdf(2.8, 0.0, 0.0, 0.0, 1.0, 0.0),
        "outer 56-wide owner box rebased into the 20-wide leaf"
    );
}

#[test]
fn nested_context_coordinates_accumulate_every_use_translation() {
    let context = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="104" height="96"><defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="48" y2="0"><stop offset="0" stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient><rect id="leaf" x="4" y="6" width="24" height="20" fill="context-fill"/><use id="mid" href="#leaf" x="10" y="8" fill="context-fill"/></defs><use href="#mid" x="20" y="18" fill="url(#g)"/></svg>"##,
    );
    let control = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="104" height="96"><defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="30" y1="0" x2="78" y2="0"><stop offset="0" stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs><rect x="34" y="32" width="24" height="20" fill="url(#g)"/></svg>"##,
    );
    assert_eq!(
        support::render_through_n0(&context, 104, 96),
        support::render_through_n0(&control, 104, 96),
        "the ultimate outer owner persists while outer and intermediate x/y both accumulate"
    );
}

#[test]
fn immediate_inner_url_owner_does_not_double_its_own_translation() {
    // Exact Chromium probe pair: pre-own-x/y OBB plus mid+outer once is
    // identical; inner-double, outer-only, and untranslated controls all
    // discriminate.
    let context = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="104" height="96"><defs><linearGradient id="g"><stop offset="0" stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient><rect id="leaf" x="4" y="6" width="24" height="20" fill="context-fill"/><use id="mid" href="#leaf" x="10" y="8" fill="url(#g)"/></defs><use href="#mid" x="20" y="18"/></svg>"##,
    );
    let control = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="104" height="96"><defs><linearGradient id="g"><stop offset="0" stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs><rect x="34" y="32" width="24" height="20" fill="url(#g)"/></svg>"##,
    );
    assert_eq!(
        support::render_through_n0(&context, 104, 96),
        support::render_through_n0(&control, 104, 96),
        "the immediate owner's box is pre-x/y while both use translations move the resolved paint"
    );
}

#[test]
fn eventual_owner_current_color_and_destination_opacity_do_not_cross_owners() {
    let frame = admit_both(&document(
        r##"<defs><rect id="r" x="12" y="12" width="40" height="40" color="#2563eb" fill="context-fill" fill-opacity=".5"/></defs>
<use href="#r" color="#e11d48" fill="currentColor" fill-opacity=".2"/>"##,
    ));
    assert_eq!(
        fill_color(&frame),
        cg::CGColor::from_rgba(225, 29, 72, 128),
        "currentColor belongs to the eventual paint owner; fill-opacity stays on the leaf"
    );
}

#[test]
fn obb_and_userspace_gradients_rebase_from_the_context_owner() {
    let obb = admit_both(&document(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><rect x="4" y="12" width="20" height="40" fill="context-fill"/><rect x="40" y="12" width="20" height="40" fill="context-fill"/></g></defs><use href="#p" fill="url(#g)"/>"##,
    ));
    assert_eq!(
        linear_fill(&obb, 0).transform,
        AffineTransform::from_acebdf(2.8, 0.0, 0.0, 0.0, 1.0, 0.0)
    );
    assert_eq!(
        linear_fill(&obb, 1).transform,
        AffineTransform::from_acebdf(2.8, 0.0, -1.8, 0.0, 1.0, 0.0)
    );

    let user = admit_both(&document(
        r##"<defs><radialGradient id="g" gradientUnits="userSpaceOnUse" cx="32" cy="32" r="32"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient><g id="p"><rect x="4" y="8" width="20" height="20" fill="context-fill"/><rect x="4" y="36" width="20" height="20" transform="translate(32 0)" fill="context-fill"/></g></defs><use href="#p" fill="url(#g)"/>"##,
    ));
    assert_eq!(
        radial_fill(&user, 0).transform,
        AffineTransform::from_acebdf(3.2, 0.0, -0.2, 0.0, 3.2, -0.4)
    );
    let second = radial_fill(&user, 1).transform.matrix;
    assert_eq!(second[0][0].to_bits(), 3.2f32.to_bits());
    assert_eq!(second[1][1].to_bits(), 3.2f32.to_bits());
    assert!((second[0][2] + 1.8).abs() < 1e-6, "{second:?}");
    assert!((second[1][2] + 1.8).abs() < 1e-6, "{second:?}");

    // Value facts alone can agree while a transform is composed on the
    // wrong side. The measured Chromium control is the equivalent explicit
    // host-space gradient; raster equality proves the rebase paints right.
    let user_control = admit_both(&document(
        r##"<defs><radialGradient id="g" gradientUnits="userSpaceOnUse" cx="32" cy="32" r="32"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient></defs><rect x="4" y="8" width="20" height="20" fill="url(#g)"/><rect x="36" y="36" width="20" height="20" fill="url(#g)"/>"##,
    ));
    assert_eq!(
        support::render_through_n0(&user, 64, 64),
        support::render_through_n0(&user_control, 64, 64),
        "context userSpace gradient paints in the use host's coordinates"
    );
}

#[test]
fn context_box_includes_hidden_and_zero_opacity_but_excludes_display_none() {
    for disposition in [r##"visibility="hidden""##, r##"opacity="0""##] {
        let frame = admit_both(&document(&format!(
            r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><rect x="4" y="12" width="20" height="40" fill="context-fill"/><rect x="40" y="12" width="20" height="40" {disposition}/></g></defs><use href="#p" fill="url(#g)"/>"##
        )));
        assert_eq!(linear_fill(&frame, 0).transform.matrix[0][0], 2.8);
    }
    let pruned = admit_both(&document(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><rect x="4" y="12" width="20" height="40" fill="context-fill"/><rect x="40" y="12" width="20" height="40" display="none"/></g></defs><use href="#p" fill="url(#g)"/>"##,
    ));
    assert_eq!(
        linear_fill(&pruned, 0).transform,
        AffineTransform::identity()
    );

    let empty_and_inert = admit_both(&document(
        r##"<defs><g id="p"><g/><g display="none"><path d="not path data"/></g><rect x="4" y="4" width="20" height="20" fill="context-fill"/></g></defs><g display="none"><use href="#p" fill="url(#missing)"/></g><use href="#p" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        fill_color(&empty_and_inert),
        cg::CGColor::from_rgb(22, 163, 74)
    );
}

#[test]
fn nonstandard_context_fallback_refuses_from_every_ingress() {
    for (body, property) in [
        (
            r##"<rect width="20" height="20" fill="context-fill red"/>"##,
            "fill",
        ),
        (
            r##"<rect width="20" height="20" style="fill:context-fill red"/>"##,
            "fill",
        ),
        (
            r##"<style>rect { fill: context-fill red }</style><rect width="20" height="20"/>"##,
            "fill",
        ),
        (
            r##"<rect width="20" height="20" fill="none" stroke="context-stroke red"/>"##,
            "stroke",
        ),
        (
            r##"<rect width="20" height="20" fill="none" style="stroke:context-stroke red"/>"##,
            "stroke",
        ),
        (
            r##"<style>rect { stroke: context-stroke red }</style><rect width="20" height="20" fill="none"/>"##,
            "stroke",
        ),
    ] {
        let error = refusal(&document(body));
        assert!(
            matches!((&error, property),
                (CompileError::UnsupportedFill(reason), "fill")
                    | (CompileError::UnsupportedStroke(reason), "stroke")
                    if reason.contains("non-standard fallback")),
            "{error}"
        );
    }
}

#[test]
fn pattern_and_external_urls_keep_their_own_refusal_through_context() {
    let pattern = refusal(&document(
        r##"<defs><pattern id="p" width="8" height="8"><rect width="4" height="4" fill="red"/></pattern><rect id="r" width="20" height="20" fill="context-fill"/></defs><use href="#r" fill="url(#p)"/>"##,
    ));
    assert!(
        matches!(pattern, CompileError::UnsupportedFill(ref reason) if reason.contains("<pattern>")),
        "{pattern}"
    );

    let external = refusal(&document(
        r##"<defs><rect id="r" width="20" height="20" fill="context-fill"/></defs><use href="#r" fill="url(https://example.invalid/g.svg#g)"/>"##,
    ));
    assert!(
        matches!(external, CompileError::UnsupportedFill(ref reason) if reason.contains("external resources")),
        "{external}"
    );
}

#[test]
fn transformed_context_boxes_use_the_measured_local_aabb_rule() {
    for shape in [
        r##"<ellipse cx="24" cy="24" rx="12" ry="8" fill="context-fill"/>"##,
        r##"<rect x="8" y="8" width="32" height="24" rx="8" fill="context-fill"/>"##,
        r##"<path d="M8 24 24 8 40 24 24 40Z" fill="context-fill"/>"##,
    ] {
        let frame = admit_both(&document(&format!(
            r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p" transform="rotate(20)">{shape}</g></defs><use href="#p" fill="url(#g)"/>"##
        )));
        assert!(
            matches!(
                frame.nodes()[0].paints.iter().next(),
                Some(cg::Paint::LinearGradient(_))
            ),
            "the transformed local AABB is measured meaning, including for curves"
        );
    }
}

#[test]
fn singular_context_gradient_consumers_paint_nothing() {
    for transform in ["scale(0)", "scale(0 1)", "scale(1 0)"] {
        let fill = admit_both(&document(&format!(
            r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><rect x="8" y="8" width="24" height="24" transform="{transform}" fill="context-fill"/><rect x="40" y="8" width="16" height="24" visibility="hidden"/></g></defs><use href="#p" fill="url(#g)"/>"##
        )));
        assert!(fill.nodes()[0].paints.is_empty(), "fill {transform}");

        let stroke = admit_both(&document(&format!(
            r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><path d="M8 20H32" transform="{transform}" fill="none" stroke="context-stroke" stroke-width="8" stroke-linecap="round"/><rect x="40" y="8" width="16" height="24" visibility="hidden"/></g></defs><use href="#p" stroke="url(#g)"/>"##
        )));
        assert!(stroke.nodes()[0].stroke.is_none(), "stroke {transform}");
    }
}

#[test]
fn unknown_geometry_never_becomes_a_partial_context_box() {
    let source = document(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><g id="p"><rect x="4" y="8" width="20" height="32" fill="context-fill"/><image x="40" y="8" width="20" height="32" href="data:image/png;base64,"/></g></defs><use href="#p" fill="url(#g)"/>"##,
    );
    let strict = refusal(&source);
    assert!(
        matches!(strict, CompileError::UnsupportedFill(ref reason) if reason.contains("geometry box is incomplete")),
        "{strict}"
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort declares both holes");
    assert!(best.base_frame().nodes().is_empty());
    assert!(
        best.degradations()
            .iter()
            .any(|d| d.reason().contains("geometry box is incomplete")),
        "{:?}",
        best.degradations()
    );

    let solid = document(
        r##"<defs><g id="p"><rect x="4" y="8" width="20" height="32" fill="context-fill"/><image x="40" y="8" width="20" height="32" href="data:image/png;base64,"/></g></defs><use href="#p" fill="#16a34a"/>"##,
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(solid.as_str(), viewport())
        .expect("an unused box error stays quarantined");
    assert_eq!(
        fill_color(&best.base_frame()),
        cg::CGColor::from_rgb(22, 163, 74)
    );
    assert!(
        best.degradations()
            .iter()
            .any(|d| d.reason().contains("unsupported element <image>")),
        "{:?}",
        best.degradations()
    );
}

#[test]
fn an_unindexed_nested_use_is_unknown_not_empty() {
    // The odd `points` coordinate is a registered whole-element departure.
    // Its measurement error deliberately prevents the outer group's prepass
    // entry; the nested use must not reinterpret that absence as an empty box.
    let source = document(
        r##"<defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><rect id="leaf" x="8" y="8" width="24" height="24" fill="context-fill"/><g id="outer"><polygon points="8,8 20"/><use href="#leaf" fill="url(#g)"/></g></defs><use href="#outer"/>"##,
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort names both holes");
    assert!(best.base_frame().nodes().is_empty());
    assert!(
        best.degradations()
            .iter()
            .any(|d| d.reason().contains("geometry box is incomplete")),
        "an absent prepass entry must not masquerade as an empty box: {:?}",
        best.degradations()
    );
}

#[test]
fn one_stop_context_gradient_does_not_demand_an_unknown_box() {
    let source = document(
        r##"<defs><linearGradient id="g"><stop offset=".5" stop-color="#e11d48"/></linearGradient><g id="p"><rect x="4" y="8" width="20" height="32" fill="context-fill"/><image x="40" y="8" width="20" height="32" href="data:image/png;base64,"/></g></defs><use href="#p" fill="url(#g)"/>"##,
    );
    let strict = refusal(&source);
    assert!(
        matches!(strict, CompileError::UnsupportedElement(ref element) if element == "image"),
        "{strict}"
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("the constant gradient and unsupported sibling are independent");
    let frame = best.base_frame();
    let gradient = linear_fill(&frame, 0);
    assert_eq!(gradient.stops.len(), 2);
    assert!(
        gradient
            .stops
            .iter()
            .all(|stop| stop.color == cg::CGColor::from_rgb(225, 29, 72).into())
    );
    assert_eq!(best.degradations().len(), 1, "{:?}", best.degradations());
    assert!(
        best.degradations()[0]
            .reason()
            .contains("unsupported element <image>")
    );
}

#[test]
fn degenerate_context_gradients_do_not_demand_an_unknown_box() {
    for (server, expected) in [
        (
            r##"<linearGradient id="g" x1=".5" x2=".5"><stop offset="0" stop-color="#2563eb"/><stop offset="1" stop-color="#e11d48"/></linearGradient>"##,
            cg::CGColor::from_rgb(225, 29, 72),
        ),
        (
            r##"<radialGradient id="g" r="0"><stop offset="0" stop-color="#2563eb"/><stop offset="1" stop-color="#e11d48"/></radialGradient>"##,
            cg::CGColor::from_rgb(225, 29, 72),
        ),
    ] {
        let source = document(&format!(
            r##"<defs>{server}<g id="p"><rect x="4" y="8" width="20" height="32" fill="context-fill"/><image x="40" y="8" width="20" height="32" href="data:image/png;base64,"/></g></defs><use href="#p" fill="url(#g)"/>"##
        ));
        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
            .expect("the degenerate solid and unsupported sibling are independent");
        assert_eq!(fill_color(&best.base_frame()), expected, "{server}");
        assert_eq!(best.degradations().len(), 1, "{:?}", best.degradations());
        assert!(
            best.degradations()[0]
                .reason()
                .contains("unsupported element <image>")
        );
    }
}
