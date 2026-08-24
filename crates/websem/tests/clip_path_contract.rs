//! Geometric SVG clipping laws at the Web-semantic contract boundary.
//!
//! Chromium probes decide which source branches may reach this file. These
//! tests then pin the source-neutral result: one typed cascade ingress,
//! union/intersection clip layers, exact coordinate maps, and stable named
//! refusals for every branch that would otherwise need a raster mask, a CSS
//! box, external I/O, or a backend contour this pin cannot match.

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{FillRule, Frame, FrameItem, Geometry, ScopeEffect};
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

fn admit_both(source: &str) -> Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort admits");
    assert!(
        best.degradations()
            .iter()
            .all(|degradation| matches!(degradation.action(), DegradationAction::SamplesAsBase)),
        "an admitted clip declares no semantic hole: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn clips(frame: &Frame) -> Vec<&rframe::ClipPath> {
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Clip(clip) => Some(clip),
                ScopeEffect::Opacity(_) => None,
            },
            FrameItem::Node(_)
            | FrameItem::ScopeEnd
            | FrameItem::MaskBegin(_)
            | FrameItem::MaskSource
            | FrameItem::MaskEnd => None,
        })
        .collect()
}

fn assert_target_skip(source: &str, reason: &str) {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport()).expect_err("strict must refuse");
    assert!(strict.to_string().contains(reason), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort declares the target hole");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one affected target: {skipped:?}");
    assert_eq!(skipped[0].path(), "svg/rect[2]");
    assert!(
        skipped[0].reason().contains(reason),
        "{}",
        skipped[0].reason()
    );
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the background survives"
    );
}

fn rectangular_resource() -> &'static str {
    r##"<clipPath id="c"><rect x="16" y="12" width="32" height="40"/></clipPath>"##
}

fn clipped_target(spelling: &str) -> String {
    document(&format!(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  {}
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" {spelling}/>"##,
        rectangular_resource()
    ))
}

#[test]
fn every_represented_clip_path_ingress_resolves_to_one_contract_fact() {
    let attribute = admit_both(&clipped_target(r##"clip-path="url(#c)""##));
    for spelling in [
        r##"style="clip-path:url(#c)""##,
        r##"style="-webkit-clip-path:url(#c)""##,
        r##"clip-path="var(--clip)" style="--clip:url(#c)""##,
    ] {
        assert_eq!(
            admit_both(&clipped_target(spelling)),
            attribute,
            "{spelling}"
        );
    }

    let stylesheet = document(&format!(
        r##"  <style>.target {{ clip-path: url(#c); }}</style>
  <rect width="64" height="64" fill="#ffffff"/>
  {}
  <rect class="target" x="8" y="8" width="48" height="48" fill="#16a34a"/>"##,
        rectangular_resource()
    ));
    assert_eq!(admit_both(&stylesheet), attribute);

    let none_wins = document(&format!(
        r##"  <style>.target {{ clip-path: none; }}</style>
  <rect width="64" height="64" fill="#ffffff"/>
  {}
  <rect class="target" x="8" y="8" width="48" height="48" fill="#16a34a"
        clip-path="url(#c)"/>"##,
        rectangular_resource()
    ));
    assert!(
        clips(&admit_both(&none_wins)).is_empty(),
        "author CSS beats the hint"
    );
}

#[test]
fn object_bounding_box_and_chains_are_resolved_before_the_frame() {
    let object_box = admit_both(&document(
        r##"  <clipPath id="c" clipPathUnits="objectBoundingBox">
    <rect x=".25" y=".2" width=".5" height=".6"/>
  </clipPath>
  <rect x="8" y="10" width="40" height="30" fill="#16a34a" clip-path="url(#c)"/>"##,
    ));
    let object_clip = clips(&object_box);
    assert_eq!(object_clip.len(), 1);
    let geometry = &object_clip[0].layers()[0].geometries()[0];
    assert_eq!(
        geometry.transform(),
        AffineTransform::from_acebdf(40.0, 0.0, 8.0, 0.0, 30.0, 10.0)
    );
    assert_eq!(
        geometry.geometry(),
        &Geometry::Rect(Rectangle::from_xywh(0.25, 0.2, 0.5, 0.6))
    );

    let chained = admit_both(&document(
        r##"  <clipPath id="a"><rect x="8" y="16" width="48" height="32"/></clipPath>
  <clipPath id="b" clip-path="url(#a)"><rect x="16" y="8" width="32" height="48"/></clipPath>
  <rect x="4" y="4" width="56" height="56" fill="#16a34a" clip-path="url(#b)"/>"##,
    ));
    assert_eq!(
        clips(&chained)[0].layers().len(),
        2,
        "a chain is layer intersection"
    );
}

#[test]
fn clip_rule_is_inherited_and_child_paint_is_inert() {
    let frame = admit_both(&document(
        r##"  <clipPath id="c" clip-rule="evenodd">
    <path d="M8 8H56V56H8Z M20 20H44V44H20Z"
          fill="none" stroke="red" stroke-width="20" opacity=".2"/>
  </clipPath>
  <rect x="4" y="4" width="56" height="56" fill="#16a34a" clip-path="url(#c)"/>"##,
    ));
    let geometry = clips(&frame)[0].layers()[0].geometries()[0].geometry();
    let Geometry::Path(path) = geometry else {
        panic!("expected resolved path contributor")
    };
    assert_eq!(path.fill_rule(), FillRule::EvenOdd);
}

#[test]
fn invalid_references_mean_no_clip_and_empty_resources_clip_everything() {
    for body in [
        r##"  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#missing)"/>"##,
        r##"  <rect id="not-a-clip"/><rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#not-a-clip)"/>"##,
    ] {
        assert!(clips(&admit_both(&document(body))).is_empty());
    }

    let empty = admit_both(&document(
        r##"  <clipPath id="c"/>
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#c)"/>"##,
    ));
    assert_eq!(clips(&empty)[0].layers()[0].geometries().len(), 0);
    assert_eq!(
        empty.nodes().len(),
        1,
        "the node remains enclosed by clip-all"
    );
}

#[test]
fn every_non_geometric_or_unproven_branch_refuses_by_name() {
    let target = |resources: &str, value: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="#ffffff"/>
  {resources}
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="{value}"/>"##
        ))
    };

    for (source, reason) in [
        (
            target(
                r##"<clipPath id="c"><text x="8" y="32">X</text></clipPath>"##,
                "url(#c)",
            ),
            "raster-mask strategy",
        ),
        (
            target(
                r##"<clipPath id="inner"><rect x="8" y="8" width="48" height="48"/></clipPath><clipPath id="c"><rect x="12" y="12" width="40" height="40" clip-path="url(#inner)"/></clipPath>"##,
                "url(#c)",
            ),
            "own clip-path",
        ),
        (target("", "circle(12px at 32px 32px)"), "basic-shape"),
        (target("", "fill-box"), "geometry-box"),
        (
            target(
                r##"<clipPath id="a" clip-path="url(#b)"><rect width="48" height="48"/></clipPath><clipPath id="b" clip-path="url(#a)"><rect x="8" y="8" width="48" height="48"/></clipPath>"##,
                "url(#a)",
            ),
            "cyclic clip-path chain",
        ),
        (
            target("", "url(https://example.test/clip.svg#c)"),
            "external",
        ),
        (
            target(
                r##"<clipPath id="c"><path clip-rule="/**/evenodd" d="M8 8H56V56H8Z M20 20H44V44H20Z"/></clipPath>"##,
                "url(#c)",
            ),
            "CSS comment",
        ),
        (
            target(
                r##"<clipPath id="c"><path clip-rule="even\6f dd" d="M8 8H56V56H8Z M20 20H44V44H20Z"/></clipPath>"##,
                "url(#c)",
            ),
            "CSS escape",
        ),
    ] {
        assert_target_skip(&source, reason);
    }

    let many = (0..43)
        .map(|index| format!(r##"<rect x="{index}" width="2" height="64"/>"##))
        .collect::<String>();

    let path_limit = (0..rframe::MAX_CLIP_GEOMETRIES_PER_LAYER)
        .map(|index| format!(r##"<rect x="{index}" width="2" height="64"/>"##))
        .collect::<String>();
    let admitted_limit = admit_both(&target(
        &format!(r##"<clipPath id="c">{path_limit}</clipPath>"##),
        "url(#c)",
    ));
    assert_eq!(
        clips(&admitted_limit)[0].layers()[0].geometries().len(),
        rframe::MAX_CLIP_GEOMETRIES_PER_LAYER,
        "the measured path-strategy side of the 42/43 boundary stays admitted"
    );

    assert_target_skip(
        &target(
            &format!(r##"<clipPath id="c">{many}</clipPath>"##),
            "url(#c)",
        ),
        "43 visible path contributors",
    );
}

#[test]
fn source_bounds_refuse_instead_of_recursing_or_panicking() {
    let mut resources = String::new();
    for index in 0..=rframe::MAX_CLIP_LAYERS {
        let next = index + 1;
        let link = if index < rframe::MAX_CLIP_LAYERS {
            format!(r##" clip-path="url(#c{next})""##)
        } else {
            String::new()
        };
        resources.push_str(&format!(
            r##"<clipPath id="c{index}"{link}><rect x="8" y="8" width="48" height="48"/></clipPath>"##
        ));
    }
    let chain = document(&format!(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  {resources}
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#c0)"/>"##
    ));
    assert_target_skip(&chain, "reference chain exceeds");

    let mut body = format!(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  {}"##,
        rectangular_resource()
    );
    for _ in 0..33 {
        body.push_str(r##"<g opacity=".5" clip-path="url(#c)">"##);
    }
    body.push_str(r##"<rect x="8" y="8" width="48" height="48" fill="#16a34a"/>"##);
    for _ in 0..33 {
        body.push_str("</g>");
    }
    let deep = document(&body);
    for result in [
        SvgFrameSource::from_standalone_svg(deep.as_str(), viewport()),
        SvgFrameSource::from_standalone_svg_best_effort(deep.as_str(), viewport()),
    ] {
        let error = result.expect_err("effect depth refuses in both admissions");
        assert!(
            matches!(
                error,
                CompileError::EffectScopeTooDeep(rframe::MAX_SCOPE_DEPTH)
            ),
            "{error:?}"
        );
    }
}

#[test]
fn root_clip_uses_a_separate_css_layer_route() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" clip-path="url(#c)">
  <clipPath id="c"><rect x="8" y="8" width="48" height="48"/></clipPath>
  <rect width="64" height="64" fill="#16a34a"/>
</svg>"##;
    for result in [
        SvgFrameSource::from_standalone_svg(source, viewport()),
        SvgFrameSource::from_standalone_svg_best_effort(source, viewport()),
    ] {
        let error = result.expect_err("root route refuses in both admissions");
        assert!(error.to_string().contains("root <svg>"), "{error}");
    }
}

#[test]
fn html_host_clips_and_resources_outside_the_compiled_svg_never_leak_in() {
    let host_clip = r##"<!doctype html><html><body>
<div style="clip-path:url(#c)">
  <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
    <clipPath id="c"><rect x="8" y="8" width="48" height="48"/></clipPath>
    <rect width="64" height="64" fill="#16a34a"/>
  </svg>
</div></body></html>"##;
    for result in [
        SvgFrameSource::from_html_inline_svg(host_clip),
        SvgFrameSource::from_html_inline_svg_best_effort(host_clip),
    ] {
        let error = result.expect_err("an HTML host clip is outside the SVG-local frame");
        assert!(error.to_string().contains("HTML ancestor"), "{error}");
    }

    let outside_resource = r##"<!doctype html><html><body>
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <rect width="64" height="64" fill="#ffffff"/>
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#outside)"/>
</svg>
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <clipPath id="outside"><rect x="16" y="16" width="32" height="32"/></clipPath>
</svg>
</body></html>"##;
    let strict = SvgFrameSource::from_html_inline_svg(outside_resource)
        .expect_err("strict refuses cross-subtree resource resolution");
    assert!(
        strict
            .to_string()
            .contains("outside the compiled SVG subtree"),
        "{strict}"
    );

    let best = SvgFrameSource::from_html_inline_svg_best_effort(outside_resource)
        .expect("best-effort skips only the affected target");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one cross-subtree target: {skipped:?}");
    assert_eq!(skipped[0].path(), "svg/rect[2]");
    assert!(
        skipped[0]
            .reason()
            .contains("outside the compiled SVG subtree"),
        "{}",
        skipped[0].reason()
    );
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the background survives"
    );
}

#[test]
fn load_active_animation_never_leaks_stale_clip_geometry() {
    let source = document(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  <clipPath id="c">
    <rect x="8" y="8" width="24" height="48">
      <animate attributeName="x" from="8" to="32" dur="1s"/>
    </rect>
  </clipPath>
  <rect x="8" y="8" width="48" height="48" fill="#16a34a" clip-path="url(#c)"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("strict refuses the active animation");
    assert!(strict.to_string().contains("animation"), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best-effort skips the referencing target");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one referencing target is the hole");
    assert_eq!(skipped[0].path(), "svg/rect[2]");
    assert!(
        skipped[0]
            .reason()
            .contains("authored geometry is overridden at document load"),
        "{}",
        skipped[0].reason()
    );
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "only the background remains"
    );

    // The same skip must leave no stale geometry in an objectBoundingBox
    // target measurement. The remaining static child defines the complete
    // box, exactly as if the animated sibling were absent.
    let animated_group = document(
        r##"  <clipPath id="c" clipPathUnits="objectBoundingBox"><rect width=".5" height="1"/></clipPath>
  <g clip-path="url(#c)">
    <rect width="32" height="64" fill="#ef4444">
      <animate attributeName="x" from="0" to="32" dur="1s"/>
    </rect>
    <rect x="32" width="32" height="64" fill="#16a34a"/>
  </g>"##,
    );
    let control = admit_both(&document(
        r##"  <clipPath id="c" clipPathUnits="objectBoundingBox"><rect width=".5" height="1"/></clipPath>
  <g clip-path="url(#c)"><rect x="32" width="32" height="64" fill="#16a34a"/></g>"##,
    ));
    let best = SvgFrameSource::from_standalone_svg_best_effort(animated_group.as_str(), viewport())
        .expect("best-effort skips the animated sibling");
    assert_eq!(best.base_frame(), control);
}
