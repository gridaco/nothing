//! Direct SVG `paint-order` contract.
//!
//! The pinned Stylo build has no Servo longhand for this inherited property,
//! so `websem` resolves only the direct presentation attribute. Non-default
//! operations become ordinary source-neutral frame items; CSS declarations
//! remain a separate named refusal.

#[allow(dead_code)]
mod support;

use std::collections::BTreeSet;

use rframe::{FrameItem, FrameNode, ScopeEffect};
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

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

fn marker() -> &'static str {
    r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="12" markerHeight="12" refX="6" refY="6" orient="0"><circle cx="6" cy="6" r="5" fill="#dc2626"/></marker></defs>"##
}

fn three_operations(value: &str) -> String {
    document(&format!(
        r##"{}
<path d="M16 16H48V48H16Z" fill="#16a34a" stroke="#2563eb" stroke-width="12" marker-end="url(#m)" paint-order="{value}"/>"##,
        marker()
    ))
}

fn admit_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort admits");
    assert!(
        best.degradations().is_empty(),
        "admitted source declares nothing: {:?}",
        best.degradations()
    );
    assert_eq!(strict.base_frame(), best.base_frame());
    strict.base_frame()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    Combined,
    Fill,
    Stroke,
    Marker,
}

fn channel(node: &FrameNode) -> Channel {
    match (!node.paints.is_empty(), node.stroke.is_some()) {
        (true, true) => Channel::Combined,
        (false, true) => Channel::Stroke,
        (true, false) => {
            let color = node
                .paints
                .iter()
                .next()
                .and_then(cg::Paint::solid_color)
                .expect("test nodes use one solid fill");
            if color.r == 0xdc && color.g == 0x26 && color.b == 0x26 {
                Channel::Marker
            } else {
                Channel::Fill
            }
        }
        (false, false) => panic!("the order fixture emits no paintless node"),
    }
}

fn channels(frame: &rframe::Frame) -> Vec<Channel> {
    frame.nodes().into_iter().map(channel).collect()
}

#[test]
fn normal_and_default_normalized_values_preserve_the_established_frame() {
    let missing = document(&format!(
        r##"{}
<path d="M16 16H48V48H16Z" fill="#16a34a" stroke="#2563eb" stroke-width="12" marker-end="url(#m)"/>"##,
        marker()
    ));
    let expected = admit_both(&missing);
    for value in ["normal", "fill", "fill stroke", "fill stroke markers"] {
        assert_eq!(admit_both(&three_operations(value)), expected, "{value}");
    }
    assert_eq!(channels(&expected), [Channel::Combined, Channel::Marker]);
}

#[test]
fn the_six_orders_lower_to_the_checked_item_stream() {
    for (value, expected) in [
        ("normal", vec![Channel::Combined, Channel::Marker]),
        (
            "fill markers",
            vec![Channel::Fill, Channel::Marker, Channel::Stroke],
        ),
        (
            "stroke",
            vec![Channel::Stroke, Channel::Fill, Channel::Marker],
        ),
        (
            "stroke markers",
            vec![Channel::Stroke, Channel::Marker, Channel::Fill],
        ),
        (
            "markers",
            vec![Channel::Marker, Channel::Fill, Channel::Stroke],
        ),
        (
            "markers stroke",
            vec![Channel::Marker, Channel::Stroke, Channel::Fill],
        ),
    ] {
        let frame = admit_both(&three_operations(value));
        assert_eq!(channels(&frame), expected, "{value}");
    }
}

#[test]
fn split_channels_have_unique_identities_and_common_source_provenance() {
    let frame = admit_both(&three_operations("markers stroke"));
    let nodes = frame.nodes();
    let fill = nodes
        .iter()
        .find(|node| channel(node) == Channel::Fill)
        .expect("fill fragment");
    let stroke = nodes
        .iter()
        .find(|node| channel(node) == Channel::Stroke)
        .expect("stroke fragment");
    assert_ne!(fill.owner.identity(), stroke.owner.identity());
    assert_eq!(fill.owner.provenance(), stroke.owner.provenance());

    let owners = frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::Node(node) => Some(node.owner),
            FrameItem::ScopeBegin(scope) => Some(scope.owner),
            FrameItem::MaskBegin(mask) => Some(mask.owner),
            FrameItem::ScopeEnd | FrameItem::MaskSource | FrameItem::MaskEnd => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        owners.iter().copied().collect::<BTreeSet<_>>().len(),
        owners.len()
    );

    let first = n0::glyphless::compile(frame.clone()).expect("split frame compiles");
    let fresh = n0::glyphless::compile(frame).expect("fresh split frame compiles");
    assert!(
        n0::glyphless::diff_frame(&first, &fresh).is_empty(),
        "fresh projection preserves every ordered fact"
    );
}

#[test]
fn painter_order_changes_are_attributed_with_bounded_damage() {
    let before = n0::glyphless::compile(admit_both(&three_operations("fill markers")))
        .expect("first order compiles");
    let after = n0::glyphless::compile(admit_both(&three_operations("markers stroke")))
        .expect("second order compiles");
    let damage = n0::glyphless::diff_frame(&before, &after);
    assert!(
        !damage.is_empty(),
        "reordered visual facts are attributable"
    );
    let union = damage.union_frame.expect("reordered ink has coverage");
    assert!(union.width > 0.0 && union.height > 0.0);

    let fresh = n0::glyphless::compile(admit_both(&three_operations("markers stroke")))
        .expect("fresh equivalent compiles");
    assert!(
        n0::glyphless::diff_frame(&after, &fresh).is_empty(),
        "equivalent fresh products carry no damage"
    );
}

#[test]
fn invalid_and_css_wide_values_follow_inherited_property_rules() {
    let parent = document(
        r##"<g paint-order="markers stroke"><rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12"/></g>"##,
    );
    let inherited = admit_both(&parent);
    for child in ["banana", "", "inherit", "unset", "revert", "revert-layer"] {
        let source = document(&format!(
            r##"<g paint-order="markers stroke"><rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12" paint-order="{child}"/></g>"##
        ));
        assert_eq!(admit_both(&source), inherited, "{child:?}");
    }

    let initial = document(
        r##"<g paint-order="markers stroke"><rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12" paint-order="initial"/></g>"##,
    );
    let normal = document(
        r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12"/>"##,
    );
    assert_eq!(admit_both(&initial), admit_both(&normal));
}

#[test]
fn substitution_functions_refuse_only_when_order_can_change_pixels() {
    for (name, function) in [
        ("var", "stroke var(--po, markers)"),
        ("env", "stroke env(--missing-po, markers)"),
        (
            "attr",
            "stroke attr(data-po type(&lt;custom-ident&gt;), fill)",
        ),
        ("if", "stroke if(style(--po-on): markers; else: fill)"),
    ] {
        let source = document(&format!(
            r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12" data-po="markers" style="--po: markers; --po-on: yes" paint-order="{function}"/>"##
        ));
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
            .expect_err("live function refuses");
        assert!(
            matches!(&strict, CompileError::UnsupportedStyle(reason)
                if reason.contains("paint-order presentation attribute")
                    && reason.contains(&format!("{name}()"))),
            "{function}: {strict}"
        );
        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
            .expect("best effort declares the skip");
        let skips = best
            .degradations()
            .iter()
            .filter(|degradation| degradation.action() == DegradationAction::Skipped)
            .collect::<Vec<_>>();
        assert_eq!(skips.len(), 1, "{function}");
        assert_eq!(skips[0].reason(), strict.to_string(), "{function}");
    }

    for (with_function, control) in [
        (
            r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="none" paint-order="var(--po, stroke)"/>"##,
            r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="none"/>"##,
        ),
        (
            r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="0" paint-order="var(--po, stroke)"/>"##,
            r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="0"/>"##,
        ),
        (
            r##"<rect x="12" y="12" width="40" height="40" fill="transparent" stroke="#2563eb" stroke-width="8" paint-order="var(--po, stroke)"/>"##,
            r##"<rect x="12" y="12" width="40" height="40" fill="transparent" stroke="#2563eb" stroke-width="8"/>"##,
        ),
        (
            r##"<line x1="8" y1="32" x2="56" y2="32" fill="#16a34a" stroke="#2563eb" stroke-width="8" opacity=".55" paint-order="var(--po, stroke)"/>"##,
            r##"<line x1="8" y1="32" x2="56" y2="32" fill="#16a34a" stroke="#2563eb" stroke-width="8" opacity=".55"/>"##,
        ),
    ] {
        assert_eq!(
            admit_both(&document(with_function)),
            admit_both(&document(control))
        );
    }

    let open_path = document(
        r##"<path d="M8 32H56" fill="#16a34a" stroke="#2563eb" stroke-width="8" opacity=".55" paint-order="var(--po, stroke)"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(open_path.as_str(), viewport())
        .expect_err("a potentially fillable path keeps the function patrol");
    assert!(
        matches!(&strict, CompileError::UnsupportedStyle(reason)
            if reason.contains("paint-order presentation attribute uses var()")),
        "{strict}"
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(open_path.as_str(), viewport())
        .expect("best effort names the conservative skip");
    assert_eq!(best.degradations().len(), 1);
    assert_eq!(best.degradations()[0].reason(), strict.to_string());
}

#[test]
fn element_opacity_encloses_the_complete_reordered_span() {
    let source = document(&format!(
        r##"{}
<path d="M16 16H48V48H16Z" fill="#16a34a" stroke="#2563eb" stroke-width="12" marker-end="url(#m)" paint-order="fill markers stroke" opacity=".55"/>"##,
        marker()
    ));
    let frame = admit_both(&source);
    assert!(matches!(
        frame.items.iter().next(),
        Some(FrameItem::ScopeBegin(scope))
            if matches!(scope.effect, ScopeEffect::Opacity(opacity) if opacity.get() == 0.55)
    ));
    assert!(matches!(
        frame.items.iter().last(),
        Some(FrameItem::ScopeEnd)
    ));
    assert_eq!(
        channels(&frame),
        [Channel::Fill, Channel::Marker, Channel::Stroke]
    );
}

#[test]
fn a_marker_source_failure_rolls_back_every_client_channel() {
    let source = document(
        r##"<defs><marker id="bad" markerWidth="10" markerHeight="10"><image width="10" height="10" href="missing.png"/></marker></defs>
<path d="M16 16H48V48H16Z" fill="#16a34a" stroke="#2563eb" stroke-width="12" marker-end="url(#bad)" paint-order="markers stroke"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("unsupported source refuses the whole client");
    assert!(matches!(strict, CompileError::UnsupportedMarker(_)));
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort declares the client skip");
    assert!(best.base_frame().nodes().is_empty());
    assert_eq!(best.degradations().len(), 1);
    assert_eq!(best.degradations()[0].action(), DegradationAction::Skipped);
}

#[test]
fn marker_resource_root_order_inherits_into_its_source() {
    let rooted = document(
        r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="24" markerHeight="24" refX="12" refY="12" paint-order="stroke"><path d="M1.3 2.7L22.2 12.4L1.3 21.1Z" fill="#e11d48" stroke="#2563eb" stroke-width="2.4"/></marker></defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="none" marker-end="url(#m)"/>"##,
    );
    let child = document(
        r##"<defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="24" markerHeight="24" refX="12" refY="12"><path d="M1.3 2.7L22.2 12.4L1.3 21.1Z" fill="#e11d48" stroke="#2563eb" stroke-width="2.4" paint-order="stroke"/></marker></defs>
<line x1="8" y1="32" x2="56" y2="32" stroke="none" marker-end="url(#m)"/>"##,
    );
    assert_eq!(admit_both(&rooted), admit_both(&child));
}

#[test]
fn inline_html_stops_inheritance_at_the_svg_namespace_boundary() {
    fn admit_html(source: &str) -> rframe::Frame {
        let strict = SvgFrameSource::from_html_inline_svg(source).expect("inline strict admits");
        let best = SvgFrameSource::from_html_inline_svg_best_effort(source)
            .expect("inline best effort admits");
        assert_eq!(best.degradations().len(), 1);
        assert_eq!(
            best.degradations()[0].action(),
            DegradationAction::SamplesAsBase,
            "the inline-HTML entry keeps its established temporal declaration"
        );
        assert!(
            best.degradations()[0].reason().contains("inline HTML"),
            "the only declaration is unrelated to paint-order: {:?}",
            best.degradations()
        );
        assert_eq!(strict.base_frame(), best.base_frame());
        strict.base_frame()
    }

    let shape = r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12"/>"##;
    let outside = format!(
        r##"<!doctype html><div paint-order="stroke"><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{shape}</svg></div>"##
    );
    let plain = format!(
        r##"<!doctype html><div><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{shape}</svg></div>"##
    );
    assert_eq!(admit_html(&outside), admit_html(&plain));

    let rooted = format!(
        r##"<!doctype html><div paint-order="normal"><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" paint-order="stroke">{shape}</svg></div>"##
    );
    let child = format!(
        r##"<!doctype html><div><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12" paint-order="stroke"/></svg></div>"##
    );
    assert_eq!(admit_html(&rooted), admit_html(&child));
}

#[test]
fn authored_css_property_remains_at_the_pinned_cascade_boundary() {
    for body in [
        r##"<rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12" style="paint-order:stroke"/>"##,
        r##"<style>rect { paint-order: stroke }</style><rect x="12" y="12" width="40" height="40" fill="#16a34a" stroke="#2563eb" stroke-width="12"/>"##,
    ] {
        let source = document(body);
        let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
            .expect_err("CSS property refuses");
        assert!(error.to_string().contains("paint-order"), "{error}");
    }
}
