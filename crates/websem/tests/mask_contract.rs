//! SVG image-mask laws at the Web-semantic contract boundary.
//!
//! Chromium probes decide the source grammar and effect order. These tests pin
//! the resolved result: one source-neutral target/source stream, exact region
//! coordinate systems, alpha versus luminance, and stable named refusals for
//! every route that cannot reach that stream without a wrong pixel.

#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{Frame, FrameItem, Geometry, MaskMode, ScopeEffect};
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
        "an admitted mask declares nothing: {declared:?}"
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
fn a_mask_resolves_to_one_two_phase_source_neutral_contract() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <mask id="m" maskUnits="userSpaceOnUse" x="8" y="8" width="48" height="48"
        mask-type="alpha">
    <rect x="8" y="8" width="24" height="48" fill="white" fill-opacity=".5"/>
  </mask>
  <rect x="8" y="8" width="48" height="48" fill="black" mask="url(#m)"/>"##,
    ));

    let tags: Vec<_> = frame
        .items
        .iter()
        .map(|item| match item {
            FrameItem::Node(_) => "node",
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
            FrameItem::ScopeBegin(_) => "scope-begin",
            FrameItem::ScopeEnd => "scope-end",
        })
        .collect();
    assert_eq!(
        tags,
        [
            "node",
            "mask-begin",
            "node",
            "mask-source",
            "node",
            "mask-end"
        ]
    );
    let FrameItem::MaskBegin(mask) = frame.items.iter().nth(1).expect("mask") else {
        panic!("resolved mask begin")
    };
    assert_eq!(mask.mode(), MaskMode::Alpha);
    let region = &mask.region().layers()[0].geometries()[0];
    assert_eq!(region.transform(), AffineTransform::identity());
    assert_eq!(
        region.geometry(),
        &Geometry::Rect(Rectangle::from_xywh(8.0, 8.0, 48.0, 48.0))
    );

    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 16, 32), [127, 127, 127, 255]);
    assert_eq!(at(&pixels, 40, 32), [255, 255, 255, 255]);
}

#[test]
fn default_luminance_and_default_object_box_region_are_exact_facts() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <mask id="m"><rect x="-128" y="-128" width="512" height="512" fill="red"/></mask>
  <rect x="16" y="16" width="32" height="32" fill="black" mask="url(#m)"/>"##,
    ));
    let mask = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::MaskBegin(mask) => Some(mask),
            _ => None,
        })
        .expect("resolved mask");
    assert_eq!(mask.mode(), MaskMode::Luminance);
    let region = &mask.region().layers()[0].geometries()[0];
    assert_eq!(region.transform(), AffineTransform::identity());
    assert_eq!(
        region.geometry(),
        &Geometry::Rect(Rectangle::from_xywh(12.8, 12.8, 38.4, 38.4))
    );
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 32, 32), [201, 201, 201, 255]);
}

#[test]
fn same_element_order_is_clip_then_opacity_then_mask() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <clipPath id="c"><rect x="8" y="8" width="48" height="48"/></clipPath>
  <mask id="m"><rect x="8" y="8" width="24" height="48" fill="white"/></mask>
  <g clip-path="url(#c)" opacity=".5" mask="url(#m)">
    <rect x="8" y="8" width="48" height="48" fill="black"/>
    <rect x="24" y="8" width="32" height="48" fill="black"/>
  </g>"##,
    ));
    let tags: Vec<_> = frame
        .items
        .iter()
        .skip(1)
        .map(|item| match item {
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Clip(_) => "clip-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
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
            "node",
            "node",
            "mask-source",
            "node",
            "mask-end",
            "scope-end",
            "scope-end",
        ]
    );
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 16, 16), [126, 126, 126, 255]);
    assert_eq!(at(&pixels, 40, 16), [255, 255, 255, 255]);
}

#[test]
fn the_mask_resources_own_clip_path_is_inert_in_both_ingresses() {
    let source = |mask_extra: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <clipPath id="c"><rect x="8" y="8" width="24" height="48"/></clipPath>
  <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
        mask-type="alpha" {mask_extra}>
    <rect x="8" y="8" width="48" height="48" fill="white"/>
  </mask>
  <rect x="8" y="8" width="48" height="48" fill="black" mask="url(#m)"/>"##,
        ))
    };
    let baseline = render_through_n0(&admit_both(&source("")), 64, 64);
    for extra in [
        r##"clip-path="url(#c)""##,
        r##"style="clip-path: url(#c)""##,
    ] {
        assert_eq!(
            render_through_n0(&admit_both(&source(extra)), 64, 64),
            baseline
        );
    }
    assert_eq!(at(&baseline, 48, 32), [0, 0, 0, 255]);
}

#[test]
fn the_mask_resources_own_css_filter_is_inert() {
    let source = |mask_extra: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs>
    <filter id="f"><feOffset dx="24"/></filter>
    <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          mask-type="alpha" {mask_extra}>
      <rect x="8" y="8" width="24" height="48" fill="white"/>
    </mask>
  </defs>
  <rect x="8" y="8" width="48" height="48" fill="black" mask="url(#m)"/>"##,
        ))
    };
    let baseline = render_through_n0(&admit_both(&source("")), 64, 64);
    assert_eq!(
        render_through_n0(&admit_both(&source(r##"style="filter: url(#f)""##)), 64, 64,),
        baseline
    );
}

#[test]
fn unsupported_mask_routes_skip_the_whole_target_by_stable_name() {
    let target = |mask: &str, target_extra: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  {mask}
  <rect x="8" y="8" width="48" height="48" fill="black" mask="url(#m)" {target_extra}/>"##
        ))
    };
    for (source, reason) in [
        (
            target(
                r##"<mask id="m"><rect width="64" height="64" fill="white"/></mask>"##,
                r##"transform="scale(1.01)""##,
            ),
            "translation/positive-downscale precision envelope",
        ),
        (
            target(
                r##"<mask id="m" maskUnits="userSpaceOnUse" x="1000000000"><rect width="64" height="64" fill="white"/></mask>"##,
                "",
            ),
            "unimplemented Web used-length range",
        ),
        (
            target(
                r##"<mask id="m" maskUnits="userSpaceOnUse" x="1e100"><rect width="64" height="64" fill="white"/></mask>"##,
                "",
            ),
            "unimplemented Web used-length range",
        ),
        (
            target(
                r##"<mask id="m"><pattern id="p"/><rect width="64" height="64" fill="url(#p)"/></mask>"##,
                "",
            ),
            "mask source cannot be compiled completely",
        ),
        (
            target(
                r##"<mask id="m" style="shape-rendering: crispEdges"><rect x="8.25" width="24" height="64" fill="white"/></mask>"##,
                "",
            ),
            "source-side cascade effect is not represented",
        ),
        (
            target(
                r##"<mask id="m" style="color-interpolation: linearRGB"><rect width="64" height="64" fill="white"/></mask>"##,
                "",
            ),
            "source-side cascade effect is not represented",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <rect x="8" y="8" width="48" height="48" fill="black"
        mask="url(https://example.test/mask.svg#m)"/>"##,
            ),
            "external",
        ),
    ] {
        assert_target_skip(&source, reason);
    }
}

#[test]
fn pinned_css_mask_ingress_refuses_instead_of_bypassing_the_cascade() {
    for (property, value) in [
        ("mask", "url(#m)"),
        ("mask-image", "url(#m)"),
        ("-webkit-mask-image", "url(#m)"),
        ("mask-mode", "alpha"),
        ("mask-repeat", "no-repeat"),
        ("mask-position", "8px 8px"),
        ("mask-clip", "fill-box"),
        ("mask-origin", "fill-box"),
        ("mask-size", "32px 32px"),
        ("mask-composite", "exclude"),
        ("mask-border", "url(#m) 1"),
        ("mask-border-source", "url(#m)"),
        ("mask-border-mode", "luminance"),
        ("mask-border-slice", "1"),
        ("mask-border-width", "1"),
        ("mask-border-outset", "1"),
        ("mask-border-repeat", "round"),
    ] {
        let source = document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <mask id="m"><rect width="64" height="64" fill="white"/></mask>
  <rect x="8" y="8" width="48" height="48" fill="black"
        style="{property}: {value}"/>"##
        ));
        assert_target_skip(&source, &format!("declares {property}"));
    }

    let source = document(
        r##"  <rect width="64" height="64" fill="white"/>
  <mask id="m" style="mask-type: alpha">
    <rect width="64" height="64" fill="white"/>
  </mask>
  <rect x="8" y="8" width="48" height="48" fill="black" mask="url(#m)"/>"##,
    );
    assert_target_skip(&source, "CSS mask-type");
}

#[test]
fn active_root_mask_is_a_document_boundary_in_both_admissions() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" mask="url(#m)">
  <mask id="m"><rect width="64" height="64" fill="white"/></mask>
  <rect width="64" height="64" fill="black"/>
</svg>"##;
    for result in [
        SvgFrameSource::from_standalone_svg(source, viewport()),
        SvgFrameSource::from_standalone_svg_best_effort(source, viewport()),
    ] {
        let error = result.expect_err("root mask cannot become a local skip");
        assert!(error.to_string().contains("root <svg>"), "{error}");
    }
}
