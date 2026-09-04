//! Direct non-root SVG viewport laws at the Web-semantic contract boundary.
//!
//! Chromium decides the source behavior. These tests pin the source-neutral
//! lowering: immutable nearest-viewport bases, composed child transforms, an
//! ordinary antialiased rectangle clip, measured effect order, and stable
//! refusal ownership. No SVG viewport object crosses `rframe`.

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{ClipEdgeMode, Frame, FrameItem, Geometry, ScopeEffect};
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

fn document(body: &str) -> String {
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{body}</svg>"##)
}

fn nested(body: &str) -> String {
    document(&format!(
        r##"<svg x="4" y="8" width="40" height="20" viewBox="0 0 20 10" preserveAspectRatio="none">{body}</svg>"##
    ))
}

fn admit_both_named(source: &str, name: &str) -> Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport())
        .unwrap_or_else(|error| panic!("{name}: strict must admit: {error}"));
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .unwrap_or_else(|error| panic!("{name}: best effort must admit: {error}"));
    let departures: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        departures.is_empty(),
        "{name}: admitted viewport declares no hole: {departures:?}"
    );
    assert_eq!(
        strict.base_frame(),
        best.base_frame(),
        "{name}: admissions are frame-identical"
    );
    strict.base_frame()
}

fn admit_both(source: &str) -> Frame {
    admit_both_named(source, "nested viewport")
}

fn item_tags(frame: &Frame) -> Vec<&'static str> {
    frame
        .items
        .iter()
        .map(|item| match item {
            FrameItem::Node(_) => "node",
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Clip(_) => "clip-begin",
                ScopeEffect::Filter(_) => "filter-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
            },
            FrameItem::ScopeEnd => "scope-end",
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
        })
        .collect()
}

fn assert_nested_skip(attributes: &str, reason: &str) {
    let source = document(&format!(
        r##"<rect width="8" height="8" fill="#16a34a"/><svg {attributes}><rect width="16" height="16" fill="#e11d48"/></svg><rect x="56" y="56" width="8" height="8" fill="#2563eb"/>"##
    ));
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("strict must refuse the nested viewport");
    assert!(strict.to_string().contains(reason), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort declares the nested viewport hole");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one nested subtree: {skipped:?}");
    assert_eq!(skipped[0].path(), "svg/svg[1]");
    assert!(skipped[0].reason().contains(reason), "{:?}", skipped[0]);
    assert_eq!(best.base_frame().nodes().len(), 2, "both siblings survive");
}

#[test]
fn mapping_order_lowers_to_the_child_transform_only() {
    let frame = admit_both(&document(
        r##"<svg x="8" y="4" width="24" height="16" viewBox="0 0 12 8"
             preserveAspectRatio="none" transform="scale(2)" overflow="visible">
          <rect x="2" y="1" width="4" height="3" fill="#16a34a"/>
        </svg>"##,
    ));
    assert_eq!(frame.items.len(), 1, "visible overflow needs no scope");
    let nodes = frame.nodes();
    let node = nodes.first().expect("one child node");
    assert_eq!(
        node.transform,
        AffineTransform::from_acebdf(4.0, 0.0, 16.0, 0.0, 4.0, 8.0),
        "element transform is outside x/y placement and viewBox mapping"
    );
    assert_eq!(
        node.geometry,
        Geometry::Rect(Rectangle::from_xywh(2.0, 1.0, 4.0, 3.0))
    );
}

#[test]
fn default_overflow_is_one_antialiased_rectangular_scope() {
    let default = admit_both(&document(
        r##"<svg x="8.5" y="8.5" width="15" height="15"><rect x="-8" y="-8" width="32" height="32" fill="#16a34a"/></svg>"##,
    ));
    let hidden = admit_both(&document(
        r##"<svg x="8.5" y="8.5" width="15" height="15" overflow="hidden"><rect x="-8" y="-8" width="32" height="32" fill="#16a34a"/></svg>"##,
    ));
    assert_eq!(default, hidden, "the nested UA default is hidden");
    assert_eq!(item_tags(&default), ["clip-begin", "node", "scope-end"]);

    let FrameItem::ScopeBegin(scope) = default.items.iter().next().expect("clip") else {
        panic!("first item must begin the viewport clip");
    };
    let ScopeEffect::Clip(clip) = &scope.effect else {
        panic!("first scope must be a clip");
    };
    assert_eq!(clip.edge_mode(), ClipEdgeMode::AntiAliased);
    let geometry = &clip.layers()[0].geometries()[0];
    assert_eq!(
        geometry.transform(),
        AffineTransform::from_acebdf(1.0, 0.0, 8.5, 0.0, 1.0, 8.5)
    );
    assert_eq!(
        geometry.geometry(),
        &Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, 15.0, 15.0))
    );

    for (name, spelling) in [
        ("attribute", r##"overflow="visible""##),
        ("inline style", r##"style="overflow:visible""##),
        (
            "axis coupling",
            r##"style="overflow-x:auto;overflow-y:hidden""##,
        ),
    ] {
        let frame = admit_both_named(
            &document(&format!(
                r##"<svg x="8" y="8" width="16" height="16" {spelling}><rect x="-8" y="-8" width="32" height="32" fill="#16a34a"/></svg>"##
            )),
            name,
        );
        assert_eq!(item_tags(&frame), ["node"], "{name} opens the viewport");
    }
}

#[test]
fn auto_and_non_positive_extents_follow_the_nested_dimension_table() {
    let omitted = admit_both(&document(
        r##"<svg x="4" y="8" overflow="visible"><rect x="75%" y="50%" width="8" height="8" fill="#16a34a"/></svg>"##,
    ));
    let auto = admit_both(&document(
        r##"<svg x="4" y="8" width="auto" height="auto" overflow="visible"><rect x="75%" y="50%" width="8" height="8" fill="#16a34a"/></svg>"##,
    ));
    let explicit = admit_both(&document(
        r##"<svg x="4" y="8" width="64" height="64" overflow="visible"><rect x="75%" y="50%" width="8" height="8" fill="#16a34a"/></svg>"##,
    ));
    assert_eq!(omitted, auto, "explicit auto uses the omitted 100% default");
    assert_eq!(auto, explicit, "auto extents use the parent viewport axes");

    let zero = admit_both(&document(
        r##"<svg width="0" height="24"><rect width="24" height="24" fill="#e11d48"/></svg><svg width="24" height="0"><rect width="24" height="24" fill="#e11d48"/></svg><rect x="56" y="56" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        zero.nodes().len(),
        1,
        "either zero extent suppresses only that nested subtree"
    );
}

#[test]
fn nearest_viewport_bases_reach_every_admitted_consumer_family() {
    let pairs = [
        (
            "geometry",
            nested(r##"<rect x="25%" y="20%" width="50%" height="50%" fill="#16a34a"/>"##),
            nested(r##"<rect x="5" y="2" width="10" height="5" fill="#16a34a"/>"##),
        ),
        (
            "gradient",
            nested(
                r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x2="100%"><stop stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs><rect width="20" height="10" fill="url(#g)"/>"##,
            ),
            nested(
                r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x2="20"><stop stop-color="#e11d48"/><stop offset="1" stop-color="#2563eb"/></linearGradient></defs><rect width="20" height="10" fill="url(#g)"/>"##,
            ),
        ),
        (
            "clip path",
            nested(
                r##"<defs><clipPath id="c"><rect width="50%" height="100%"/></clipPath></defs><rect width="20" height="10" fill="#16a34a" clip-path="url(#c)"/>"##,
            ),
            nested(
                r##"<defs><clipPath id="c"><rect width="10" height="10"/></clipPath></defs><rect width="20" height="10" fill="#16a34a" clip-path="url(#c)"/>"##,
            ),
        ),
        (
            "pattern",
            nested(
                r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="25%" height="100%"><rect width="2" height="10" fill="#16a34a"/></pattern></defs><rect width="20" height="10" fill="url(#p)"/>"##,
            ),
            nested(
                r##"<defs><pattern id="p" patternUnits="userSpaceOnUse" width="5" height="10"><rect width="2" height="10" fill="#16a34a"/></pattern></defs><rect width="20" height="10" fill="url(#p)"/>"##,
            ),
        ),
        (
            "filter region",
            nested(
                r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="25%" y="20%" width="50%" height="60%"><feFlood flood-color="#16a34a"/></filter></defs><rect width="20" height="10" fill="#e11d48" filter="url(#f)"/>"##,
            ),
            nested(
                r##"<defs><filter id="f" filterUnits="userSpaceOnUse" x="5" y="2" width="10" height="6"><feFlood flood-color="#16a34a"/></filter></defs><rect width="20" height="10" fill="#e11d48" filter="url(#f)"/>"##,
            ),
        ),
        (
            "use placement",
            nested(
                r##"<defs><rect id="tile" width="4" height="4" fill="#16a34a"/></defs><use href="#tile" x="50%" y="20%"/>"##,
            ),
            nested(
                r##"<defs><rect id="tile" width="4" height="4" fill="#16a34a"/></defs><use href="#tile" x="10" y="2"/>"##,
            ),
        ),
    ];
    for (name, percent, explicit) in pairs {
        assert_eq!(
            admit_both_named(&percent, name),
            admit_both_named(&explicit, name),
            "{name} must consume the child viewBox bases"
        );
    }

    let stroked = admit_both(&nested(
        r##"<path d="M2 5H18" fill="none" stroke="#16a34a" stroke-width="10%"/>"##,
    ));
    assert_eq!(
        stroked.nodes()[0]
            .stroke
            .as_ref()
            .expect("percentage stroke")
            .width(),
        1.5811387_f32,
        "stroke-width uses the child viewBox normalized diagonal"
    );

    let mask_percent = document(
        r##"<svg x="4" y="8" width="10" height="5" viewBox="0 0 20 10" preserveAspectRatio="none"><defs><mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="25%" y="20%" width="50%" height="60%"><rect x="25%" y="20%" width="50%" height="60%" fill="white"/></mask></defs><rect width="20" height="10" fill="#16a34a" mask="url(#m)"/></svg>"##,
    );
    let mask_explicit = document(
        r##"<svg x="4" y="8" width="10" height="5" viewBox="0 0 20 10" preserveAspectRatio="none"><defs><mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="5" y="2" width="10" height="6"><rect x="5" y="2" width="10" height="6" fill="white"/></mask></defs><rect width="20" height="10" fill="#16a34a" mask="url(#m)"/></svg>"##,
    );
    assert_eq!(
        admit_both_named(&mask_percent, "mask bases"),
        admit_both_named(&mask_explicit, "mask bases")
    );

    let marker_percent = nested(
        r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="25%" markerHeight="40%" refX="10%" refY="20%" orient="0"><rect width="5" height="4" fill="#16a34a"/></marker></defs><path d="M5 5H15" fill="none" stroke="#2563eb" marker-end="url(#m)"/>"##,
    );
    let marker_explicit = nested(
        r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="5" markerHeight="4" refX="2" refY="2" orient="0"><rect width="5" height="4" fill="#16a34a"/></marker></defs><path d="M5 5H15" fill="none" stroke="#2563eb" marker-end="url(#m)"/>"##,
    );
    assert_eq!(
        admit_both_named(&marker_percent, "marker bases"),
        admit_both_named(&marker_explicit, "marker bases")
    );

    // The nested element's own percentage transform is resolved in its
    // parent viewport, while descendants switch to the child bases.
    let percentage = document(
        r##"<svg x="4" y="8" width="40" height="20" viewBox="0 0 20 10" preserveAspectRatio="none" overflow="visible" style="transform:translate(50%,20%)"><rect width="4" height="3" fill="#16a34a"/></svg>"##,
    );
    let parent_control = document(
        r##"<svg x="4" y="8" width="40" height="20" viewBox="0 0 20 10" preserveAspectRatio="none" overflow="visible" transform="translate(32 12.8)"><rect width="4" height="3" fill="#16a34a"/></svg>"##,
    );
    assert_eq!(admit_both(&percentage), admit_both(&parent_control));
}

#[test]
fn viewport_clip_separates_descendant_and_own_filter_stages() {
    let defs = r##"<defs><filter id="blur" x="-100%" y="-100%" width="300%" height="300%"><feGaussianBlur stdDeviation="3"/></filter></defs>"##;
    let own = admit_both(&document(&format!(
        r##"{defs}<svg x="16" y="16" width="16" height="16" overflow="hidden" filter="url(#blur)"><rect x="2" y="2" width="12" height="12" fill="#16a34a"/></svg>"##
    )));
    let child = admit_both(&document(&format!(
        r##"{defs}<svg x="16" y="16" width="16" height="16" overflow="hidden"><rect x="2" y="2" width="12" height="12" fill="#16a34a" filter="url(#blur)"/></svg>"##
    )));
    assert_eq!(
        item_tags(&own),
        [
            "filter-begin",
            "clip-begin",
            "node",
            "scope-end",
            "scope-end"
        ]
    );
    assert_eq!(
        item_tags(&child),
        [
            "clip-begin",
            "filter-begin",
            "node",
            "scope-end",
            "scope-end"
        ]
    );
}

#[test]
fn invalid_nested_geometry_is_attributable_and_does_not_leak_bases() {
    for (attributes, reason) in [
        (r##"x="8px" width="16" height="16""##, "attribute x=\"8px\""),
        (
            r##"x="calc(4px + 4px)" width="16" height="16""##,
            "attribute x=\"calc(4px + 4px)\"",
        ),
        (
            r##"x="var(--x)" style="--x:8px" width="16" height="16""##,
            "attribute x=\"var(--x)\"",
        ),
        (
            r##"x="initial" width="16" height="16""##,
            "attribute x=\"initial\"",
        ),
        (
            r##"x="8" width="16" height="16" style="width:32px""##,
            "CSS width",
        ),
        (
            r##"x="57384.267578125007%" width="16" height="16""##,
            "x numeric precision alias",
        ),
        (
            r##"x="8" width="16" height="16" viewBox="0 0 8 6,""##,
            "viewBox",
        ),
    ] {
        assert_nested_skip(attributes, reason);
    }

    let negative = admit_both(&document(
        r##"<rect width="8" height="8" fill="#16a34a"/><svg x="8" y="8" width="-1" height="16"><rect width="16" height="16" fill="#e11d48"/></svg><rect x="56" y="56" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        negative.nodes().len(),
        2,
        "a negative extent is a local no-render error"
    );
}

#[test]
fn nesting_and_use_keep_the_child_mapping_and_bounded_failure() {
    let nested_source = document(
        r##"<defs><g id="target"><svg x="3" y="2" width="8" height="6"><rect width="8" height="6" fill="#16a34a"/></svg></g></defs><g transform="translate(4 6)"><use href="#target" x="10" y="12"/></g>"##,
    );
    let frame = admit_both(&nested_source);
    let nodes = frame.nodes();
    let node = nodes.first().expect("one expanded child");
    assert_eq!(
        node.transform,
        AffineTransform::from_acebdf(1.0, 0.0, 17.0, 0.0, 1.0, 20.0)
    );

    let viewport_target = document(
        r##"<defs><svg id="target" viewBox="0 0 10 10"><rect width="10" height="10" fill="#16a34a"/></svg></defs><use href="#target" x="8" y="8" width="32" height="16"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(viewport_target.as_str(), viewport())
        .expect_err("a direct SVG use target remains a separate viewport contract");
    assert!(
        strict.to_string().contains("instance-sized viewport"),
        "{strict}"
    );
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(viewport_target.as_str(), viewport())
            .expect("best effort declares the instance-viewport hole");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(declared.len(), 1, "one use subtree: {declared:?}");
    assert!(
        declared[0].reason().contains("instance-sized viewport"),
        "{:?}",
        declared[0]
    );

    let mut body = String::new();
    for _ in 0..65 {
        body.push_str(r##"<svg width="64" height="64" overflow="visible">"##);
    }
    body.push_str(r##"<rect width="8" height="8" fill="#16a34a"/>"##);
    for _ in 0..65 {
        body.push_str("</svg>");
    }
    let source = document(&body);
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("bounded nesting must refuse");
    assert!(strict.to_string().contains("deeper than 64"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort contains the bounded hole");
    assert_eq!(best.base_frame().nodes().len(), 0);
    assert_eq!(
        best.degradations()
            .iter()
            .filter(|degradation| degradation.action() == DegradationAction::Skipped)
            .count(),
        1
    );
}

#[test]
fn inline_html_and_standalone_entries_share_the_nested_semantics() {
    let body = r##"<rect width="64" height="64" fill="#ffffff"/><svg x="8" y="12" width="20" height="16"><rect width="20" height="16" fill="#16a34a"/></svg>"##;
    let standalone = admit_both(&document(body));
    let html = format!(
        r##"<!doctype html><style>html,body{{margin:0}}</style><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{body}</svg>"##
    );
    let strict = SvgFrameSource::from_html_inline_svg(html.as_str()).expect("inline strict admits");
    let best = SvgFrameSource::from_html_inline_svg_best_effort(html.as_str())
        .expect("inline best effort admits");
    assert!(
        best.degradations()
            .iter()
            .all(|degradation| degradation.action() == DegradationAction::SamplesAsBase),
        "inline Base-only declaration is the only allowed departure: {:?}",
        best.degradations()
    );
    assert_eq!(strict.base_frame(), best.base_frame());
    assert_eq!(strict.base_frame(), standalone);
}
