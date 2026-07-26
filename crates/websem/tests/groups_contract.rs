//! The container and transform contract: how `<g>` composes and what a
//! `transform` list admits.
//!
//! A container is **flattened**, not represented — it contributes a
//! transform and a place in paint order, both of which the resolved
//! contract already carries per node. These laws pin that flattening
//! (composition order, paint order, nested paths, inherited paint), the
//! transform grammar's admitted set and its refusals, and the boundary
//! that keeps the flattening honest: any construct needing a real group
//! scope still refuses.

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::Geometry;
use support::render_through_n0;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

/// A 64x64 canvas around the markup under test.
fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// Strict and best-effort agree and declare nothing.
fn admit_both(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    assert!(
        best.degradations().is_empty(),
        "an admitted document declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn geometry_box(frame: &rframe::Frame, index: usize) -> Rectangle {
    frame.nodes[index].geometry.local_box()
}

#[test]
fn a_group_contributes_its_transform_to_every_descendant() {
    let frame = admit_both(&document(
        r##"  <g transform="translate(10,4)">
    <rect x="2" y="3" width="8" height="6" fill="#16a34a"/>
    <circle cx="20" cy="20" r="5" fill="#2563eb"/>
  </g>"##,
    ));
    assert_eq!(frame.nodes.len(), 2, "the group itself is not a node");
    let expected = AffineTransform::from_acebdf(1.0, 0.0, 10.0, 0.0, 1.0, 4.0);
    for index in 0..2 {
        assert_eq!(frame.nodes[index].transform, expected, "node {index}");
        assert_eq!(
            frame.nodes[index].bounds,
            math2::rect_transform(geometry_box(&frame, index), &expected),
            "node {index} keeps the exact-bounds law"
        );
    }
    assert_eq!(
        geometry_box(&frame, 0),
        Rectangle::from_xywh(2.0, 3.0, 8.0, 6.0),
        "geometry stays in the authored local space"
    );
}

/// SVG composes outermost-first: an inner transform applies inside its
/// ancestors' mapping. Nesting a translate inside a scale therefore scales
/// the translation.
#[test]
fn nested_group_transforms_compose_outermost_first() {
    let frame = admit_both(&document(
        r##"  <g transform="scale(2)">
    <g transform="translate(5,3)">
      <rect width="4" height="4" fill="#16a34a"/>
    </g>
  </g>"##,
    ));
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 10.0, 0.0, 2.0, 6.0),
        "scale(2) then translate(5,3) maps the origin to (10,6) at 2x"
    );
    assert_eq!(
        frame.nodes[0].bounds,
        Rectangle::from_xywh(10.0, 6.0, 8.0, 8.0)
    );
}

/// A shape's own transform composes inside the mapping it inherits, so the
/// group-then-shape order matches a single equivalent list.
#[test]
fn a_shape_transform_composes_inside_its_inherited_mapping() {
    let nested = admit_both(&document(
        r##"  <g transform="translate(10,0)">
    <rect transform="scale(3)" width="4" height="4" fill="#16a34a"/>
  </g>"##,
    ));
    let flat = admit_both(&document(
        r##"  <rect transform="translate(10,0) scale(3)" width="4" height="4" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        nested.nodes[0].transform, flat.nodes[0].transform,
        "nesting and one list are the same mapping"
    );
    assert_eq!(
        nested.nodes[0].transform,
        AffineTransform::from_acebdf(3.0, 0.0, 10.0, 0.0, 3.0, 0.0)
    );
}

/// Flattening preserves painter order: document order across and into
/// containers, first painted first.
#[test]
fn flattening_preserves_painter_order_across_containers() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  <g>
    <rect width="40" height="40" fill="#16a34a"/>
  </g>
  <g>
    <g>
      <rect width="20" height="20" fill="#2563eb"/>
    </g>
  </g>"##,
    ));
    assert_eq!(frame.nodes.len(), 3);
    let pixels = render_through_n0(&frame, 64, 64);
    let at = |x: i32, y: i32| -> [u8; 4] {
        let offset = ((y * 64 + x) * 4) as usize;
        pixels[offset..offset + 4].try_into().expect("pixel")
    };
    assert_eq!(at(10, 10), [0x25, 0x63, 0xeb, 255], "last painted wins");
    assert_eq!(at(30, 30), [0x16, 0xa3, 0x4a, 255], "middle over the first");
    assert_eq!(at(50, 50), [255, 255, 255, 255], "the background remains");
}

/// Paint inherits through a container by the one cascade, not by any
/// group-local paint state this compiler owns.
#[test]
fn paint_inherits_through_a_container() {
    let frame = admit_both(&document(
        r##"  <g fill="#16a34a">
    <rect width="8" height="8"/>
  </g>"##,
    ));
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(&pixels[0..4], &[0x16, 0xa3, 0x4a, 255]);
}

/// Every transform function the slice admits, with its exact mapping.
/// Quarter turns come from their integer matrices: `cos(90°)` in f32
/// carries a residue that would put every edge on a different subpixel
/// than the oracle.
#[test]
fn the_admitted_transform_functions_map_exactly() {
    for (list, expected) in [
        (
            "translate(7)",
            AffineTransform::from_acebdf(1.0, 0.0, 7.0, 0.0, 1.0, 0.0),
        ),
        (
            "translate(7,-3)",
            AffineTransform::from_acebdf(1.0, 0.0, 7.0, 0.0, 1.0, -3.0),
        ),
        (
            "scale(2)",
            AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 2.0, 0.0),
        ),
        (
            "scale(2,3)",
            AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 3.0, 0.0),
        ),
        (
            "rotate(90)",
            AffineTransform::from_acebdf(0.0, -1.0, 0.0, 1.0, 0.0, 0.0),
        ),
        (
            "rotate(180)",
            AffineTransform::from_acebdf(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0),
        ),
        (
            "rotate(90,10,10)",
            AffineTransform::from_acebdf(0.0, -1.0, 20.0, 1.0, 0.0, 0.0),
        ),
        (
            "matrix(1,0,0,1,5,6)",
            AffineTransform::from_acebdf(1.0, 0.0, 5.0, 0.0, 1.0, 6.0),
        ),
        (
            "matrix(2,0,0,4,1,2)",
            AffineTransform::from_acebdf(2.0, 0.0, 1.0, 0.0, 4.0, 2.0),
        ),
        (
            // Comma-or-whitespace separated, and composed left to right.
            "translate(4 4) scale(2),translate(1,1)",
            AffineTransform::from_acebdf(2.0, 0.0, 6.0, 0.0, 2.0, 6.0),
        ),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <g transform="{list}"><rect width="4" height="4" fill="#16a34a"/></g>"##
        )));
        assert_eq!(frame.nodes[0].transform, expected, "{list}");
    }
}

#[test]
fn skew_maps_by_the_tangent_of_its_angle() {
    let frame = admit_both(&document(
        r##"  <g transform="skewX(45)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let matrix = frame.nodes[0].transform.matrix;
    assert!(
        (matrix[0][1] - 1.0).abs() < 1e-6,
        "tan(45°) = 1: {matrix:?}"
    );
    assert_eq!(matrix[1][0], 0.0, "skewX leaves the other axis alone");
}

/// A malformed transform refuses by name in both admissions — the posture
/// `viewBox` and `preserveAspectRatio` already set. The frozen donor
/// instead filters unparseable arguments out of the list, silently mapping
/// a subset; that is the divergence this refusal exists to prevent.
#[test]
fn malformed_transform_lists_refuse_by_name() {
    for list in [
        "translate(10, abc)",
        "translate()",
        "translate(1,2,3)",
        "scale(1,2,3)",
        "rotate(45,1)",
        "rotate(45,1,2,3)",
        "skewX()",
        "skewX(10,20)",
        "matrix(1,0,0,1,0)",
        "matrix(1,0,0,1,0,0,0)",
        "translate(10.)",
        "shear(10)",
        "translate 10",
        "translate(10",
        "translate(NaN)",
        "translate(1e999)",
        // Separator grammar: SVG's comma-wsp permits whitespace and at
        // most one comma, never a leading or doubled one. Skipping empty
        // tokens — the obvious implementation — would read each of these
        // as a well-formed shorter list, while Chromium rejects them and
        // paints the element untransformed.
        "translate(1,,2)",
        "translate(,1)",
        "translate(1,)",
        "translate(1 2,)",
        "matrix(1,0,0,1,,0)",
        ",translate(1,2)",
        "translate(1,2),,scale(2)",
    ] {
        let source = document(&format!(
            r##"  <g transform="{list}"><rect width="4" height="4" fill="#16a34a"/></g>"##
        ));
        let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .err()
            .unwrap_or_else(|| panic!("{list}: strict must refuse"));
        assert!(
            matches!(&error, CompileError::BadTransform { element, .. } if element == "g"),
            "{list}: {error:?}"
        );

        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
                .unwrap_or_else(|e| panic!("{list}: best-effort compiles the document: {e}"));
        assert_eq!(best.base_frame().nodes.len(), 0, "{list}: nothing paints");
        assert_eq!(best.degradations().len(), 1, "{list}");
        assert_eq!(best.degradations()[0].path(), "svg/g[1]", "{list}");
        assert_eq!(
            best.degradations()[0].reason(),
            error.to_string(),
            "{list}: one reason, both admissions"
        );
    }
}

/// An empty or whitespace-only list authored no function and maps as the
/// identity, exactly as an absent attribute does.
#[test]
fn an_empty_transform_list_is_the_identity() {
    for list in ["", " ", "\t\n"] {
        let frame = admit_both(&document(&format!(
            r##"  <g transform="{list}"><rect width="4" height="4" fill="#16a34a"/></g>"##
        )));
        assert_eq!(frame.nodes[0].transform, AffineTransform::identity());
    }
}

/// Flattening is only honest while every construct needing a real group
/// scope refuses: a container carrying one is a declared hole, not a
/// silently un-scoped paint.
#[test]
fn scope_bearing_containers_still_refuse() {
    for (label, attrs, named) in [
        ("group opacity", r#"opacity="0.5""#, "opacity"),
        ("group clip-path", r#"clip-path="url(#c)""#, "clip-path"),
        ("group mask", r#"mask="url(#m)""#, "mask"),
        ("group filter", r#"filter="url(#f)""#, "filter"),
        (
            "cascaded transform",
            r#"style="transform: translate(4px,0)""#,
            "transform",
        ),
    ] {
        let source = document(&format!(
            r##"  <g {attrs}><rect width="8" height="8" fill="#16a34a"/></g>"##
        ));
        let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .err()
            .unwrap_or_else(|| panic!("{label}: strict must refuse"));
        assert!(error.to_string().contains(named), "{label}: {error}");

        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
                .unwrap_or_else(|e| panic!("{label}: best-effort compiles: {e}"));
        let skipped: Vec<_> = best
            .degradations()
            .iter()
            .filter(|d| d.action() == DegradationAction::Skipped)
            .collect();
        assert_eq!(skipped.len(), 1, "{label}");
        assert_eq!(skipped[0].path(), "svg/g[1]", "{label}");
        assert_eq!(
            best.base_frame().nodes.len(),
            0,
            "{label}: the whole subtree is one hole — nothing inside it can be \
             placed or composited without the construct"
        );
    }
}

/// A beyond-slice *descendant* is its own hole: its siblings still paint,
/// and the skip names it at its nested path. That is what keeps
/// best-effort useful inside real illustrations, where one unsupported
/// child would otherwise drop a whole group.
#[test]
fn a_beyond_slice_descendant_is_its_own_declared_hole() {
    let source = document(
        r##"  <g transform="translate(4,4)">
    <rect width="8" height="8" fill="#16a34a"/>
    <polygon points="0,0 4,0 4,4" fill="#000000"/>
    <circle cx="20" cy="20" r="4" fill="#2563eb"/>
  </g>"##,
    );
    SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .expect_err("strict refuses at the polygon");

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort");
    assert_eq!(
        best.base_frame().nodes.len(),
        2,
        "the rect and the circle still paint"
    );
    assert_eq!(best.degradations().len(), 1);
    assert_eq!(
        best.degradations()[0].path(),
        "svg/g[1]/polygon[1]",
        "the skip names its nested structural path"
    );
    assert_eq!(
        best.degradations()[0].reason(),
        "unsupported element <polygon>"
    );
}

/// Ordinals are per parent, so the same tag at different depths keeps
/// distinct, stable paths.
#[test]
fn nested_degradation_paths_number_per_parent() {
    let source = document(
        r##"  <polygon points="0,0 1,0"/>
  <g>
    <polygon points="0,0 1,0"/>
    <g>
      <polygon points="0,0 1,0"/>
    </g>
  </g>
  <g>
    <polygon points="0,0 1,0"/>
  </g>"##,
    );
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort");
    let paths: Vec<&str> = best.degradations().iter().map(|d| d.path()).collect();
    assert_eq!(
        paths,
        vec![
            "svg/polygon[1]",
            "svg/g[1]/polygon[1]",
            "svg/g[1]/g[1]/polygon[1]",
            "svg/g[2]/polygon[1]",
        ]
    );
}

/// Non-rendering elements contribute no geometry *and no hole*: Chromium
/// paints nothing for them either, so declaring them would report a
/// difference that does not exist.
#[test]
fn non_rendering_elements_are_neither_compiled_nor_declared() {
    let frame = admit_both(&document(
        r##"  <title>A drawing</title>
  <desc>With a description</desc>
  <metadata><whatever/></metadata>
  <g>
    <title>A group title</title>
    <rect width="8" height="8" fill="#16a34a"/>
  </g>"##,
    ));
    assert_eq!(frame.nodes.len(), 1, "only the rect materializes");
}

/// The recursive walk bounds its depth explicitly instead of exhausting
/// the stack on a generated or adversarial document.
#[test]
fn container_nesting_beyond_the_bound_refuses() {
    let deep = format!(
        "{}<rect width=\"4\" height=\"4\" fill=\"#16a34a\"/>{}",
        "<g>".repeat(200),
        "</g>".repeat(200)
    );
    let source = document(&deep);
    let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .err()
        .expect("strict refuses past the bound");
    assert!(
        matches!(error, CompileError::ContainerTooDeep(_)),
        "{error:?}"
    );

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort declares instead of crashing");
    assert_eq!(best.base_frame().nodes.len(), 0);
    assert!(
        best.degradations()
            .iter()
            .any(|d| d.reason().contains("nesting deeper than")),
        "{:?}",
        best.degradations()
    );
}

/// An `<animate>` under a shape nested in a container stays a declared
/// blocker: the sampling inventory's candidate set is the root's own
/// materialized rects, and admitting overrides deeper would widen the
/// slice past what the animation corpus bakes.
#[test]
fn animate_inside_a_container_stays_a_declared_blocker() {
    let source = document(
        r##"  <g>
    <rect x="4" y="8" width="8" height="16" fill="#000000">
      <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
    </rect>
  </g>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .expect("strict Base compiles");
    strict
        .sample_frame(animation_sampling::SampleTime::ZERO)
        .expect_err("strict sampling refuses");

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::SamplesAsBase)
        .collect();
    assert_eq!(declared.len(), 1);
    assert!(
        declared[0]
            .reason()
            .contains("materialized top-level <rect>"),
        "{}",
        declared[0].reason()
    );
    assert_eq!(
        best.sample_frame(animation_sampling::SampleTime::from_nanoseconds(
            1_000_000_000
        ))
        .expect("samples as base"),
        best.base_frame()
    );
}

/// `skewY` maps by the tangent of its angle on the other axis — the
/// mirror of `skewX`, and the arm a transposed matrix slot would silently
/// break.
#[test]
fn skew_y_maps_on_the_other_axis() {
    let frame = admit_both(&document(
        r##"  <g transform="skewY(45)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let matrix = frame.nodes[0].transform.matrix;
    assert!(
        (matrix[1][0] - 1.0).abs() < 1e-6,
        "tan(45°) = 1: {matrix:?}"
    );
    assert_eq!(matrix[0][1], 0.0, "skewY leaves the other axis alone");
}

/// Quarter turns are exact at every multiple, including negative and
/// beyond a full turn: the special case reduces the angle rather than
/// matching literal values.
#[test]
fn quarter_turns_are_exact_at_every_multiple() {
    for (list, expected) in [
        (
            "rotate(-90)",
            AffineTransform::from_acebdf(0.0, 1.0, 0.0, -1.0, 0.0, 0.0),
        ),
        (
            "rotate(270)",
            AffineTransform::from_acebdf(0.0, 1.0, 0.0, -1.0, 0.0, 0.0),
        ),
        ("rotate(360)", AffineTransform::identity()),
        (
            "rotate(450)",
            AffineTransform::from_acebdf(0.0, -1.0, 0.0, 1.0, 0.0, 0.0),
        ),
        ("rotate(0)", AffineTransform::identity()),
    ] {
        let frame = admit_both(&document(&format!(
            r##"  <g transform="{list}"><rect width="4" height="4" fill="#16a34a"/></g>"##
        )));
        assert_eq!(frame.nodes[0].transform, expected, "{list}");
    }
}

/// A container transform composes *inside* the viewport mapping: its
/// numbers are user units, scaled by the viewBox, never device pixels.
/// This is the rung's least reversible decision, so it is pinned in
/// pixels as well as in the matrix.
#[test]
fn container_transforms_are_user_units_under_a_viewbox() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 32 32">
  <g transform="translate(4,2)">
    <rect width="8" height="8" fill="#16a34a"/>
  </g>
</svg>"##;
    let frame = admit_both(source);
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 8.0, 0.0, 2.0, 4.0),
        "the viewBox's 2x scales the group's translate: (4,2) user units -> (8,4) device"
    );
    assert_eq!(
        frame.nodes[0].bounds,
        Rectangle::from_xywh(8.0, 4.0, 16.0, 16.0)
    );
    let pixels = render_through_n0(&frame, 64, 64);
    let at = |x: i32, y: i32| -> [u8; 4] {
        let offset = ((y * 64 + x) * 4) as usize;
        pixels[offset..offset + 4].try_into().expect("pixel")
    };
    assert_eq!(at(10, 6), [0x16, 0xa3, 0x4a, 255], "inside the mapped rect");
    assert_eq!(at(6, 2), [0, 0, 0, 0], "outside it, before the translate");
}

/// The root `<svg>`'s own `transform` is not a container transform:
/// Chromium applies it to the element's CSS box, outside the viewBox
/// mapping, so composing it like a `<g>` would place content wrongly. It
/// refuses by name in both admissions rather than being silently dropped.
#[test]
fn a_transform_on_the_root_svg_refuses_in_both_admissions() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" transform="translate(20,0)"><rect width="16" height="16" fill="#16a34a"/></svg>"##;
    for (admission, result) in [
        (
            "strict",
            SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)),
        ),
        (
            "best-effort",
            SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0)),
        ),
    ] {
        let error = result
            .err()
            .unwrap_or_else(|| panic!("{admission} must refuse the root transform"));
        assert!(
            matches!(
                &error,
                CompileError::UnsupportedAttribute { element, attr }
                    if element == "svg" && attr == "transform"
            ),
            "{admission}: {error:?}"
        );
    }
}

/// The depth bound admits exactly as many container levels as it declares
/// and refuses the next one — the off-by-one a bound like this invites.
#[test]
fn the_container_depth_bound_admits_its_limit_and_refuses_past_it() {
    let nest = |depth: usize| {
        document(&format!(
            "{}<rect width=\"4\" height=\"4\" fill=\"#16a34a\"/>{}",
            "<g>".repeat(depth),
            "</g>".repeat(depth)
        ))
    };
    let admitted = nest(64);
    let frame = admit_both(&admitted);
    assert_eq!(frame.nodes.len(), 1, "64 levels compile");

    let refused = nest(65);
    let error = SvgFrameSource::from_standalone_svg(refused.as_str(), viewport(64.0, 64.0))
        .err()
        .expect("65 levels refuse");
    assert!(
        matches!(error, CompileError::ContainerTooDeep(64)),
        "{error:?}"
    );
}

/// Every sheet is inspected, not just the first: a second stylesheet
/// declaring a different unrepresentable property is declared too, and the
/// action says what actually happened — the element rendered, the
/// declaration did not apply. Nothing was skipped.
#[test]
fn every_stylesheet_declaring_an_unrepresentable_property_is_declared() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>rect { transform: translate(4px,0) }</style>
  <style>circle { filter: blur(2px) }</style>
  <rect width="16" height="16" fill="#16a34a"/>
</svg>"##;
    SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .err()
        .expect("strict refuses on the first");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort renders and declares");
    let ignored: Vec<(&str, &str)> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::DeclarationIgnored)
        .map(|d| (d.path(), d.reason()))
        .collect();
    assert_eq!(ignored.len(), 2, "both sheets: {ignored:?}");
    assert_eq!(ignored[0].0, "svg/style[1]");
    assert_eq!(ignored[1].0, "svg/style[2]");
    assert!(ignored[0].1.contains("transform"), "{}", ignored[0].1);
    assert!(ignored[1].1.contains("filter"), "{}", ignored[1].1);
    assert_eq!(
        best.base_frame().nodes.len(),
        1,
        "the rect rendered — nothing was skipped"
    );
}

/// The inline-HTML entry's stylesheet lives outside the compiled SVG
/// subtree, and its declared path is the document's real structure — not a
/// fabricated one rooted at `svg`.
#[test]
fn an_inline_html_stylesheet_is_declared_at_its_real_path() {
    let html = r##"<!doctype html><html><head><style>rect { filter: blur(2px) }</style></head><body><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="16" height="16" fill="#16a34a"/></svg></body></html>"##;
    let best = SvgFrameSource::from_html_inline_svg_best_effort(html)
        .expect("best-effort renders and declares");
    let ignored: Vec<&str> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::DeclarationIgnored)
        .map(|d| d.path())
        .collect();
    assert_eq!(
        ignored,
        vec!["html/head[1]/style[1]"],
        "the path names the document's own structure"
    );
}

/// A malformed transform on a *shape* refuses naming that shape, not the
/// container it happens to sit in — the shape call site has its own
/// element name.
#[test]
fn a_malformed_shape_transform_names_the_shape() {
    for (element, shape) in [
        (
            "rect",
            r##"<rect transform="translate(1,,2)" width="4" height="4" fill="#16a34a"/>"##,
        ),
        (
            "circle",
            r##"<circle transform="shear(2)" cx="8" cy="8" r="4" fill="#16a34a"/>"##,
        ),
        (
            "ellipse",
            r##"<ellipse transform="scale()" cx="8" cy="8" rx="4" ry="2" fill="#16a34a"/>"##,
        ),
    ] {
        let source = document(&format!("  {shape}"));
        let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .err()
            .unwrap_or_else(|| panic!("{element}: strict must refuse"));
        assert!(
            matches!(&error, CompileError::BadTransform { element: named, .. } if named == element),
            "{element}: {error:?}"
        );
    }
}

/// A composed transform that overflows to a non-finite matrix refuses at
/// the element, where best-effort can leave one declared hole. The
/// downstream contract would otherwise refuse the whole frame with nothing
/// named, turning one bad list into a blank render.
#[test]
fn an_overflowing_transform_composition_refuses_at_its_element() {
    let source = document(
        r##"  <g transform="scale(1e38)">
    <g transform="scale(1e38)">
      <rect width="4" height="4" fill="#16a34a"/>
    </g>
  </g>
  <rect x="8" y="8" width="8" height="8" fill="#2563eb"/>"##,
    );
    let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .err()
        .expect("strict refuses the overflow");
    assert!(
        matches!(&error, CompileError::BadTransform { element, .. } if element == "g"),
        "{error:?}"
    );

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort still compiles the document");
    assert_eq!(
        best.base_frame().nodes.len(),
        1,
        "the sibling rect still renders — the overflow is one hole"
    );
    assert_eq!(best.degradations().len(), 1);
    assert_eq!(best.degradations()[0].path(), "svg/g[1]/g[1]");
}

/// The quarter-turn shortcut is bounded: an angle large enough that every
/// f32 quotient is integral must not be snapped onto an exact matrix.
#[test]
fn huge_rotation_angles_do_not_snap_to_a_quarter_turn() {
    let frame = admit_both(&document(
        r##"  <g transform="rotate(1e30)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let matrix = frame.nodes[0].transform.matrix;
    let exact = [
        AffineTransform::identity().matrix,
        AffineTransform::from_acebdf(0.0, -1.0, 0.0, 1.0, 0.0, 0.0).matrix,
        AffineTransform::from_acebdf(-1.0, 0.0, 0.0, 0.0, -1.0, 0.0).matrix,
        AffineTransform::from_acebdf(0.0, 1.0, 0.0, -1.0, 0.0, 0.0).matrix,
    ];
    assert!(
        !exact.contains(&matrix),
        "a huge angle took the exact quarter-turn path: {matrix:?}"
    );
}

/// `display: contents` generates no box in Chromium: the element is
/// dropped and its children paint in the parent's place, so a container
/// loses its transform and a shape never paints. Both refuse.
#[test]
fn display_contents_refuses_on_containers_and_shapes() {
    for (label, body) in [
        (
            "container",
            r##"  <style>g { display: contents }</style>
  <g transform="translate(10,0)"><rect width="8" height="8" fill="#16a34a"/></g>"##,
        ),
        (
            "shape",
            r##"  <style>rect { display: contents }</style>
  <rect width="8" height="8" fill="#16a34a"/>"##,
        ),
    ] {
        let source = document(body);
        let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .err()
            .unwrap_or_else(|| panic!("{label}: strict must refuse"));
        assert!(
            error.to_string().contains("display: contents"),
            "{label}: {error}"
        );
    }
}

/// A rotated shape reaches the painter, not just the matrix: the frame
/// compiles through the n0 downstream and the pixels land where the
/// rotation puts them. (`rotate(90)` about a pivot keeps every edge
/// pixel-aligned, so this is an exact pixel claim without a tolerance.)
#[test]
fn a_rotated_shape_paints_through_the_downstream() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="#ffffff"/>
  <g transform="rotate(90,32,32)">
    <rect x="32" y="0" width="16" height="16" fill="#16a34a"/>
  </g>"##,
    ));
    let pixels = render_through_n0(&frame, 64, 64);
    let at = |x: i32, y: i32| -> [u8; 4] {
        let offset = ((y * 64 + x) * 4) as usize;
        pixels[offset..offset + 4].try_into().expect("pixel")
    };
    // rotate(90) about (32,32) maps (32..48, 0..16) to (48..64, 32..48).
    assert_eq!(at(56, 40), [0x16, 0xa3, 0x4a, 255], "the rotated position");
    assert_eq!(at(40, 8), [255, 255, 255, 255], "the authored position");
}

/// `<style>` keeps the non-rendering treatment the old flat walk gave it
/// by special case: its CSS still reaches the cascade, and it declares
/// nothing.
#[test]
fn a_style_element_contributes_cascade_but_no_hole() {
    let source = document(
        r##"  <style>rect { fill: #16a34a }</style>
  <rect width="8" height="8"/>"##,
    );
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort admits");
    // The sheet's own sampling blocker is a separate standing policy; what
    // matters here is that the element leaves no hole.
    assert!(
        best.degradations()
            .iter()
            .all(|d| d.action() == DegradationAction::SamplesAsBase),
        "the style element declares no hole: {:?}",
        best.degradations()
    );
    let frame = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .expect("strict admits")
        .base_frame();
    assert_eq!(frame, best.base_frame());
    assert_eq!(frame.nodes.len(), 1, "only the rect materializes");
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(
        &pixels[0..4],
        &[0x16, 0xa3, 0x4a, 255],
        "the stylesheet still cascaded its fill"
    );
}
