//! Same-document static SVG marker placement contracts.
//!
//! Chromium-baked cells own the pixel oracle. These laws pin the producer
//! structure and the refusal boundary: authored marker topology and resource
//! references disappear in `websem`, each admitted instance becomes ordinary
//! source-neutral items, and a rejected source removes the whole client in
//! both admissions.

use rframe::{ClipEdgeMode, FrameItem, ScopeEffect};
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
{body}
</svg>"##
    )
}

fn arrow(body: &str) -> String {
    format!(
        r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" markerHeight="10" refX="5" refY="5" orient="auto">{body}</marker></defs>"##
    )
}

fn admit_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort admits");
    assert!(
        best.degradations().is_empty(),
        "admitted source declares nothing"
    );
    assert_eq!(strict.base_frame(), best.base_frame());
    strict.base_frame()
}

fn assert_skipped_by_marker(source: &str, needle: &str) {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport())
        .expect_err("strict must refuse the marker client");
    assert!(
        matches!(strict, CompileError::UnsupportedMarker(_)),
        "stable marker error class; got {strict}"
    );
    assert!(strict.to_string().contains(needle), "got {strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort declares and skips");
    assert!(
        best.base_frame().nodes().is_empty(),
        "a rejected marker source removes the client's base paint too"
    );
    assert_eq!(
        best.degradations().len(),
        1,
        "unexpected declarations: {:?}",
        best.degradations()
    );
    assert_eq!(best.degradations()[0].action(), DegradationAction::Skipped);
    assert!(best.degradations()[0].reason().contains(needle));
}

fn assert_marker_skip_among_declarations(source: &str, needle: &str) {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport())
        .expect_err("strict must refuse the marker client");
    assert!(matches!(strict, CompileError::UnsupportedMarker(_)));
    assert!(strict.to_string().contains(needle), "got {strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort declares and skips");
    assert!(best.base_frame().nodes().is_empty());
    assert!(best.degradations().iter().any(|degradation| {
        degradation.action() == DegradationAction::Skipped && degradation.reason().contains(needle)
    }));
}

#[test]
fn instances_flatten_to_ordinary_nodes_inside_hard_viewport_clips() {
    let source = document(&format!(
        r##"{}
<path d="M10 48L32 12L54 48" fill="none" stroke="#2563eb"
 marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##,
        arrow(r##"<path d="M0 0L10 5L0 10Z" fill="#e11d48"/>"##)
    ));
    let frame = admit_both(&source);
    assert_eq!(frame.nodes().len(), 4, "one client and three instances");

    let clips = frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Clip(clip) => Some(clip),
                ScopeEffect::Opacity(_) | ScopeEffect::Filter(_) => None,
            },
            FrameItem::Node(_)
            | FrameItem::ScopeEnd
            | FrameItem::MaskBegin(_)
            | FrameItem::MaskSource
            | FrameItem::MaskEnd => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(clips.len(), 3);
    assert!(
        clips
            .iter()
            .all(|clip| clip.edge_mode() == ClipEdgeMode::Hard),
        "Chromium marker viewports are hard clips"
    );
}

#[test]
fn marker_only_geometry_is_not_discarded_with_the_base_shape() {
    let marker = arrow(r##"<circle cx="5" cy="5" r="3" fill="#e11d48"/>"##);
    let move_only = admit_both(&document(&format!(
        r##"{marker}<path d="M20 24" fill="none" stroke="none" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##
    )));
    assert_eq!(
        move_only.nodes().len(),
        1,
        "a lone path move selects only end"
    );

    let one_polyline = admit_both(&document(&format!(
        r##"{marker}<polyline points="20,24" fill="none" stroke="none" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##
    )));
    assert_eq!(
        one_polyline.nodes().len(),
        1,
        "one open point selects only end"
    );

    let one_polygon = admit_both(&document(&format!(
        r##"{marker}<polygon points="20,24" fill="none" stroke="none" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##
    )));
    let hard_clips = one_polygon
        .items
        .iter()
        .filter(|item| {
            matches!(
                item,
                FrameItem::ScopeBegin(scope)
                    if matches!(&scope.effect, ScopeEffect::Clip(clip) if clip.edge_mode() == ClipEdgeMode::Hard)
            )
        })
        .count();
    assert_eq!(hard_clips, 2, "one closed point selects start and end");
}

#[test]
fn chromium_inapplicable_shape_attributes_and_bare_marker_are_inert() {
    for shape in [
        r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##,
        r##"<circle cx="32" cy="32" r="20" fill="#16a34a" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##,
        r##"<ellipse cx="32" cy="32" rx="22" ry="16" fill="#16a34a" marker-start="url(#m)" marker-mid="url(#m)" marker-end="url(#m)"/>"##,
        r##"<line x1="12" y1="32" x2="52" y2="32" stroke="#16a34a" marker="url(#m)"/>"##,
    ] {
        let source = document(&format!(
            "{}{}",
            arrow(r##"<path d="M0 0L10 5L0 10Z" fill="#e11d48"/>"##),
            shape
        ));
        assert_eq!(admit_both(&source).nodes().len(), 1, "{shape}");
    }
}

#[test]
fn direct_inheritance_and_none_reset_are_resolved_before_the_frame() {
    let marker = arrow(r##"<path d="M0 0L10 5L0 10Z" fill="#e11d48"/>"##);
    let inherited = admit_both(&document(&format!(
        r##"{marker}<g marker-end="url(#m)"><line x1="12" y1="24" x2="52" y2="24" stroke="#2563eb"/></g>"##
    )));
    assert_eq!(inherited.nodes().len(), 2, "line plus inherited end marker");

    let reset = admit_both(&document(&format!(
        r##"{marker}<g marker-end="url(#m)"><line x1="12" y1="24" x2="52" y2="24" stroke="#2563eb" marker-end="none"/></g>"##
    )));
    assert_eq!(reset.nodes().len(), 1, "explicit none stops inheritance");
}

#[test]
fn only_a_selected_resolved_marker_enters_the_combined_opacity_span() {
    let opacity_scopes = |source: &str| {
        admit_both(source)
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    FrameItem::ScopeBegin(scope)
                        if matches!(scope.effect, ScopeEffect::Opacity(_))
                )
            })
            .count()
    };
    let client = |defs: &str, property: &str, path_data: &str| {
        document(&format!(
            r##"{defs}<path d="{path_data}" fill="#e11d48" opacity=".47" {property}/>"##
        ))
    };

    assert_eq!(
        opacity_scopes(&client(
            "",
            r##"marker-end="url(#missing)""##,
            "M8 48L32 8L56 48Z"
        )),
        0,
        "a missing target is opacity-equivalent to none"
    );
    assert_eq!(
        opacity_scopes(&client(
            r##"<defs><g id="wrong"/></defs>"##,
            r##"marker-end="url(#wrong)""##,
            "M8 48L32 8L56 48Z",
        )),
        0,
        "a wrong-type target is opacity-equivalent to none"
    );
    assert_eq!(
        opacity_scopes(&client(
            r##"<defs><marker id="m" markerWidth="8" markerHeight="8"><title>empty</title></marker></defs>"##,
            r##"marker-end="url(#m)""##,
            "M8 48L32 8L56 48Z",
        )),
        1,
        "a selected marker resource keeps combined opacity even when its source is empty"
    );
    assert_eq!(
        opacity_scopes(&client(
            r##"<defs><marker id="m" markerWidth="0" markerHeight="8"><circle cx="4" cy="4" r="4"/></marker></defs>"##,
            r##"marker-end="url(#m)""##,
            "M8 48L32 8L56 48Z",
        )),
        1,
        "a selected marker resource keeps combined opacity when its viewport suppresses paint"
    );
    assert_eq!(
        opacity_scopes(&document(
            r##"<defs><marker id="m" markerWidth="8" markerHeight="8"><circle cx="4" cy="4" r="4"/></marker></defs>
<path d="M8 32L56 32" fill="none" stroke="#e11d48" opacity=".47" marker-mid="url(#m)"/>"##,
        )),
        0,
        "an unselected marker kind is opacity-equivalent to none"
    );
}

#[test]
fn rejected_source_is_one_transaction_in_strict_and_best_effort() {
    let source = document(
        r##"<defs>
  <linearGradient id="g"><stop stop-color="#e11d48"/><stop offset="1" stop-color="#16a34a"/></linearGradient>
  <marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" markerHeight="10"><rect width="10" height="10" fill="url(#g)"/></marker>
</defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="#2563eb" stroke-width="5" marker-end="url(#m)"/>"##,
    );
    assert_skipped_by_marker(&source, "paint server");
}

#[test]
fn resource_root_rendering_declarations_refuse_before_inheritance_is_lost() {
    for marker_declaration in [
        r##"shape-rendering="crispEdges""##,
        r##"style="shape-rendering:crispEdges""##,
        r##"paint-order="stroke fill""##,
        r##"style="paint-order:stroke fill""##,
    ] {
        let source = document(&format!(
            r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="24" markerHeight="24" refX="12" refY="12" {marker_declaration}><path d="M1.3 2.7L22.2 12.4L1.3 21.1Z" fill="#e11d48" stroke="#2563eb" stroke-width="2.4"/></marker></defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="none" marker-end="url(#m)"/>"##
        ));
        assert_marker_skip_among_declarations(&source, "resource-root rendering declaration");
    }
}

#[test]
fn external_and_unresolved_resource_grammar_refuse_by_stable_marker_name() {
    let external = document(
        r##"<line x1="8" y1="32" x2="56" y2="32" stroke="#2563eb" marker-end="url(https://example.com/a.svg#m)"/>"##,
    );
    assert_skipped_by_marker(&external, "external");

    let calc = document(
        r##"<defs><marker id="m" markerWidth="calc(4px + 4px)" markerHeight="8"><rect width="8" height="8" fill="#e11d48"/></marker></defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="#2563eb" marker-end="url(#m)"/>"##,
    );
    assert_skipped_by_marker(&calc, "markerWidth uses calc()");

    let nested = document(
        r##"<defs>
  <marker id="inner" markerUnits="userSpaceOnUse" markerWidth="4" markerHeight="4"><circle cx="2" cy="2" r="2" fill="#16a34a"/></marker>
  <marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" markerHeight="10"><path d="M0 5L10 5" stroke="#e11d48" marker-end="url(#inner)"/></marker>
</defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="#2563eb" marker-end="url(#m)"/>"##,
    );
    assert_skipped_by_marker(&nested, "nested marker source");
}

#[test]
fn position_and_source_fanout_limits_refuse_before_partial_paint() {
    let mut path = String::from("M0 32");
    for index in 0..4096 {
        path.push_str(&format!("L{} {}", index % 64, (index / 64) % 64));
    }
    let too_many_positions = document(&format!(
        r##"{}<path d="{path}" fill="none" stroke="#2563eb" marker-end="url(#m)"/>"##,
        arrow(r##"<circle cx="5" cy="5" r="2" fill="#e11d48"/>"##)
    ));
    assert_skipped_by_marker(&too_many_positions, "authored positions");

    let source_nodes = (0..65)
        .map(|index| {
            format!(
                r##"<rect x="{}" y="{}" width="1" height="1" fill="#e11d48"/>"##,
                index % 10,
                index / 10
            )
        })
        .collect::<String>();
    let too_many_source_items = document(&format!(
        r##"{}<line x1="8" y1="32" x2="56" y2="32" stroke="#2563eb" marker-end="url(#m)"/>"##,
        arrow(&source_nodes)
    ));
    assert_skipped_by_marker(&too_many_source_items, "source emits 65 frame items");
}
