//! SVG image-filter laws at the Web-semantic contract boundary.
//!
//! Chromium probes decide the source grammar, graph fallback, color space,
//! region, and effect order. These tests pin the resolved result and the
//! stable refusals that keep every unrepresented graph branch from becoming
//! an unfiltered silent fallback.

#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    FilterColorSpace, FilterComposite, FilterInput, FilterPrimitive, Frame, FrameItem, ScopeEffect,
};
use support::render_through_n0;
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

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

fn admit_both(source: &str) -> Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort admits");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        declared.is_empty(),
        "an admitted filter declares nothing: {declared:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn assert_target_skip(source: &str, reason: &str) {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport()).expect_err("strict must refuse");
    assert!(strict.to_string().contains(reason), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort declares the affected target");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one affected target: {skipped:?}");
    assert!(
        skipped[0].reason().contains(reason),
        "{}",
        skipped[0].reason()
    );
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the white backdrop survives"
    );
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 64 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

#[test]
fn gaussian_blur_resolves_to_one_source_neutral_checked_graph() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" primitiveUnits="objectBoundingBox">
    <feGaussianBlur stdDeviation=".125" result="blurred"/>
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##,
    ));
    let tags: Vec<_> = frame
        .items
        .iter()
        .map(|item| match item {
            FrameItem::Node(_) => "node",
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Filter(_) => "filter-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
                ScopeEffect::Clip(_) => "clip-begin",
            },
            FrameItem::ScopeEnd => "scope-end",
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
        })
        .collect();
    assert_eq!(tags, ["node", "filter-begin", "node", "scope-end"]);

    let filter = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("resolved filter scope");
    assert_eq!(filter.transform(), AffineTransform::identity());
    assert_eq!(
        filter.region(),
        Rectangle::from_xywh(17.6, 17.6, 28.8, 28.8)
    );
    let node = filter.program().iter().next().expect("one blur node");
    assert_eq!(node.inputs(), [FilterInput::Source]);
    assert_eq!(node.color_space(), FilterColorSpace::LinearRgb);
    assert_eq!(
        node.primitive(),
        FilterPrimitive::GaussianBlur {
            sigma_x: 3.0,
            sigma_y: 3.0
        }
    );

    let pixels = render_through_n0(&frame, 64, 64);
    assert_ne!(at(&pixels, 17, 32), [255, 255, 255, 255]);
    assert_eq!(at(&pixels, 16, 32), [255, 255, 255, 255]);
}

#[test]
fn hard_shadow_graph_resolves_zero_one_two_and_n_input_operations() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feOffset in="SourceAlpha" dx="5" dy="4" result="o"/>
    <feFlood flood-color="#7c3aed" flood-opacity=".65" result="f"/>
    <feComposite in="f" in2="o" operator="in" result="s"/>
    <feMerge><feMergeNode in="s"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#0ea5e9" filter="url(#f)"/>"##,
    ));
    let filter = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("one resolved filter");
    let nodes: Vec<_> = filter.program().iter().collect();
    assert_eq!(nodes.len(), 4);
    assert_eq!(nodes[0].inputs(), [FilterInput::SourceAlpha]);
    assert_eq!(
        nodes[0].primitive(),
        FilterPrimitive::Offset { dx: 5.0, dy: 4.0 }
    );
    assert!(nodes[1].inputs().is_empty());
    let FilterPrimitive::SolidColor { color } = nodes[1].primitive() else {
        panic!("second node is the resolved solid source")
    };
    assert_eq!(color.to_rgba8(), cg::CGColor::from_rgba(124, 58, 237, 166));
    assert_eq!(
        nodes[2].inputs(),
        [FilterInput::Node(1), FilterInput::Node(0)]
    );
    assert_eq!(
        nodes[2].primitive(),
        FilterPrimitive::Composite {
            operator: FilterComposite::In
        }
    );
    assert_eq!(
        nodes[3].inputs(),
        [FilterInput::Node(2), FilterInput::Source]
    );
    assert_eq!(nodes[3].primitive(), FilterPrimitive::Merge);
    assert!(
        nodes
            .iter()
            .all(|node| node.color_space() == FilterColorSpace::Srgb)
    );

    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 24, 24), [14, 165, 233, 255]);
    assert_ne!(at(&pixels, 47, 32), [255, 255, 255, 255]);
}

#[test]
fn flood_opacity_percentage_keeps_css_parser_normalization_order() {
    let source = |opacity: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">
    <feFlood flood-color="red" flood-opacity="{opacity}"/>
  </filter>
  <rect width="64" height="64" filter="url(#f)"/>"##
        ))
    };

    let percentage = admit_both(&source("57.384267578125007%"));
    let equivalent_number = admit_both(&source(".57384267578125007"));
    let lower_f32_neighbor = admit_both(&source(".5738426446914673"));
    assert_eq!(
        percentage, equivalent_number,
        "CSS percentage normalization must divide before narrowing to f32"
    );
    assert_ne!(
        percentage, lower_f32_neighbor,
        "the authored percentage must not collapse onto the lower f32 neighbor"
    );
}

#[test]
fn offset_only_graphs_can_exceed_the_old_two_operation_boundary() {
    let source = |body: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{body}</filter>
  <rect x="20" y="20" width="24" height="24" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let chained = render_through_n0(
        &admit_both(&source(
            r##"<feOffset dx="1" result="a"/><feOffset in="a" dx="1" result="b"/><feOffset in="b" dx="1"/>"##,
        )),
        64,
        64,
    );
    let direct = render_through_n0(&admit_both(&source(r##"<feOffset dx="3"/>"##)), 64, 64);
    assert_eq!(chained, direct);
}

#[test]
fn safe_sigma_blur_graphs_can_exceed_the_retired_depth_boundary() {
    let source = |body: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{body}</filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##
        ))
    };
    let direct = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur in="a" stdDeviation="2" result="b"/><feGaussianBlur in="b" stdDeviation="2"/>"##,
        )),
        64,
        64,
    );
    let through_merges = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feMerge result="m1"><feMergeNode in="a"/></feMerge><feGaussianBlur in="m1" stdDeviation="2" result="b"/><feMerge result="m2"><feMergeNode in="b"/></feMerge><feGaussianBlur in="m2" stdDeviation="2"/>"##,
        )),
        64,
        64,
    );
    assert_eq!(direct, through_merges);

    admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feGaussianBlur stdDeviation="1"/>
  </filter>
  <rect x="10" y="10" width="12" height="12" transform="scale(2)" fill="#16a34a" filter="url(#f)"/>"##,
    ));
}

#[test]
fn graph_inputs_names_units_and_color_spaces_follow_the_measured_blink_fallbacks() {
    let source = |primitive: &str, filter_extra: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" {filter_extra}>
    {primitive}
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##
        ))
    };
    let previous = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur stdDeviation="2"/>"##,
            "",
        )),
        64,
        64,
    );
    for second in [r##"in="a""##, r##"in="missing""##] {
        let pixels = render_through_n0(
            &admit_both(&source(
                &format!(
                    r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur {second} stdDeviation="2"/>"##
                ),
                "",
            )),
            64,
            64,
        );
        assert_eq!(pixels, previous, "{second} selects the previous result");
    }

    let user = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="3"/>"##,
            r##"primitiveUnits="userSpaceOnUse""##,
        )),
        64,
        64,
    );
    let object = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation=".125"/>"##,
            r##"primitiveUnits="objectBoundingBox""##,
        )),
        64,
        64,
    );
    assert_eq!(user, object, "object-box sigma resolves per target axis");

    let linear = render_through_n0(
        &admit_both(&source(r##"<feGaussianBlur stdDeviation="3"/>"##, "")),
        64,
        64,
    );
    let auto = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="3" color-interpolation-filters="auto"/>"##,
            "",
        )),
        64,
        64,
    );
    assert_ne!(
        linear, auto,
        "missing is linearRGB while explicit auto is sRGB"
    );
}

#[test]
fn empty_invalid_and_wrong_kind_references_are_measured_nothings() {
    let source = |defs: &str, filter: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs>{defs}</defs>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" {filter}/>"##
        ))
    };
    let plain = render_through_n0(&admit_both(&source("", "")), 64, 64);
    for document in [
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(#missing)""##,
        ),
        source(r##"<linearGradient id="f"/>"##, r##"filter="url(#f)""##),
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(#f) trailing""##,
        ),
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(/**/#f/**/)""##,
        ),
    ] {
        assert_eq!(render_through_n0(&admit_both(&document), 64, 64), plain);
    }

    let hidden = render_through_n0(
        &admit_both(&source(r##"<filter id="f"/>"##, r##"filter="url(#f)""##)),
        64,
        64,
    );
    assert_eq!(at(&hidden, 32, 32), [255, 255, 255, 255]);
}

#[test]
fn quoted_urls_share_the_url_branch_and_lists_refuse_by_name() {
    let source = |filter: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="3"/></filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="{filter}"/>"##
        ))
    };

    assert_eq!(
        admit_both(&source("url('#f')")),
        admit_both(&source("url(#f)")),
        "quoted and unquoted URL tokens resolve the same resource"
    );

    for filter in [
        "url('#f') url('#f')",
        "url('#f') blur(1px)",
        "url('#f'), url('#f')",
    ] {
        assert_target_skip(&source(filter), "multiple filter operations");
    }

    let plain = render_through_n0(&admit_both(&source("none")), 64, 64);
    let invalid = render_through_n0(&admit_both(&source("url('#f') trailing")), 64, 64);
    assert_eq!(
        invalid, plain,
        "an invalid trailing ident drops the whole hint"
    );
}

#[test]
fn same_element_order_is_clip_then_opacity_then_mask_then_filter() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <clipPath id="c"><rect x="8" y="8" width="48" height="48"/></clipPath>
  <mask id="m" mask-type="alpha"><rect width="64" height="64" fill="white"/></mask>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feGaussianBlur stdDeviation="2"/>
  </filter>
  <g clip-path="url(#c)" opacity=".5" mask="url(#m)" filter="url(#f)">
    <rect x="16" y="16" width="32" height="32" fill="black"/>
    <rect x="24" y="16" width="24" height="32" fill="black"/>
  </g>"##,
    ));
    let tags: Vec<_> = frame
        .items
        .iter()
        .skip(1)
        .map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Clip(_) => "clip-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
                ScopeEffect::Filter(_) => "filter-begin",
            },
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::Node(_) => "node",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
            FrameItem::ScopeEnd => "scope-end",
        })
        .collect();
    assert_eq!(
        tags,
        [
            "clip-begin",
            "opacity-begin",
            "mask-begin",
            "filter-begin",
            "node",
            "node",
            "scope-end",
            "mask-source",
            "node",
            "mask-end",
            "scope-end",
            "scope-end",
        ]
    );
}

#[test]
fn unsupported_filter_routes_skip_the_whole_target_by_stable_name() {
    let target = |filter: &str, attribute: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  {filter}
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" {attribute}/>"##
        ))
    };
    for (source, reason) in [
        (
            target(
                r##"<filter id="f"><feOffset dx="2.5"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "fractional displacement",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="128" height="128">
    <feOffset dx="1"/>
  </filter>
  <g transform="scale(.5)">
    <rect x="40" y="40" width="48" height="48" fill="#16a34a" filter="url(#f)"/>
  </g>"##,
            ),
            "fractional device-space displacement",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/><feOffset dx="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "combines feOffset with Gaussian blur",
        ),
        (
            target(
                r##"<filter id="f"><feFlood style="flood-color:red"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS flood-color on <feFlood>",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="calc(1 / 2)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS function",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="var(--o)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-opacity resolves through var()",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="inherit"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-opacity uses inherit",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="var(--c)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-color resolves through var()",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="inherit"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-color uses inherit",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="hsl(0 100% 50%)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "outside the admitted color slice",
        ),
        (
            target(
                r##"<filter id="f"><feDropShadow dx="2" dy="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "unsupported primitive <feDropShadow>",
        ),
        (
            target(
                r##"<filter id="f" href="#base"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "href inheritance",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur width="0" stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "transparent graph result",
        ),
        (
            target(
                r##"<filter id="f" x="1em"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "unit, whose basis is not admitted",
        ),
        (
            target("", r##"filter="blur(2px)""##),
            "CSS filter functions",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url('#f') url('#f')""##,
            ),
            "multiple filter operations",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="var(--fx)" style="--fx:url(#f)""##,
            ),
            "filter presentation attribute uses var()",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <g filter="url(#f)">
    <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="inherit"/>
  </g>"##,
            ),
            "filter presentation attribute uses inherit",
        ),
        (
            target("", r##"filter="url(https://example.test/f.svg#f)""##),
            "external",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="/*x*/linearRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "contains a CSS comment",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="l\69 nearRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "contains a CSS escape",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="var(--space)" style="--space:sRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "color-interpolation-filters presentation attribute uses var()",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" style="color-interpolation-filters:sRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS color-interpolation-filters",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="1"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "small-kernel precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="3"/></filter>
  <g transform="scale(.5)">
    <rect x="40" y="40" width="24" height="24" fill="#16a34a" filter="url(#f)"/>
  </g>"##,
            ),
            "small-kernel precision boundary",
        ),
        (
            target(
                r##"<filter id="f" x="33554432"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "crosses the unimplemented Web used-length range",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur x="128" y="128" width="8" height="8" stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "outside the effect region",
        ),
    ] {
        assert_target_skip(&source, reason);
    }
}

#[test]
fn root_and_css_filter_routes_remain_named_separate_boundaries() {
    let root = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" filter="url(#f)">
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <rect width="64" height="64" fill="black"/>
</svg>"##;
    for result in [
        SvgFrameSource::from_standalone_svg(root, viewport()),
        SvgFrameSource::from_standalone_svg_best_effort(root, viewport()),
    ] {
        let error = result.expect_err("root filter is a document-level boundary");
        assert!(error.to_string().contains("root <svg>"), "{error}");
    }

    let css = document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <rect x="20" y="20" width="24" height="24" fill="black"
        filter="url(#f)" style="filter:none"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(css.as_str(), viewport())
        .expect_err("CSS property ingress remains quarantined");
    assert!(strict.to_string().contains("declares filter"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(css.as_str(), viewport())
        .expect("best effort declares the CSS boundary");
    assert!(best.degradations().iter().any(|degradation| {
        degradation.action() == DegradationAction::Skipped
            && degradation.reason().contains("declares filter")
    }));
}
