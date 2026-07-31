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

/// The same, for markup carrying a `style` attribute or `<style>` sheet:
/// either blocks the *sampling* inventory (it could carry a CSS animation),
/// which is a policy about Sample requests, not a hole in the Base frame
/// these laws read — the static degradation set must still be empty.
fn admit_both_styled(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    let static_degradations: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        static_degradations.is_empty(),
        "an admitted document declares nothing static: {static_degradations:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn geometry_box(frame: &rframe::Frame, index: usize) -> Rectangle {
    frame.nodes[index].geometry.local_box()
}

/// `<a>` is a container exactly like `<g>` (SVG2 §16.2: `href` is
/// interaction, not paint): it flattens, its transform composes, and the
/// equivalent `<g>` resolves to the identical frame. Chromium-baked as
/// `svg-anchor-container`.
#[test]
fn an_anchor_is_a_container_like_a_group() {
    let anchor = admit_both(&document(
        r##"  <a href="https://example.com" transform="translate(8,8)"><rect width="24" height="24" fill="#16a34a"/></a>"##,
    ));
    let group = admit_both(&document(
        r##"  <g transform="translate(8,8)"><rect width="24" height="24" fill="#16a34a"/></g>"##,
    ));
    assert_eq!(anchor.nodes, group.nodes, "one container semantics");
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

/// A malformed transform list drops the whole attribute and renders the
/// element untransformed — no refusal and no declared hole, because the
/// drop *is* Chromium's measured behavior for every list here (the
/// transform rung's probe matrix baked all of them) and the pixels agree
/// exactly. This flipped from a refusal when the attribute became a
/// presentation hint: a malformed list contributes no hint, which is the
/// same computed `none` Chromium resolves. The frozen donor instead
/// filters unparseable arguments out of the list, silently mapping a
/// subset — a *different transform* than any browser computes, which is
/// the divergence the whole-list drop exists to prevent. The
/// accepted-vs-dropped boundary itself is pinned in csscascade's
/// `svg_transform` tests, one row per probe.
#[test]
fn a_malformed_transform_list_drops_the_attribute() {
    let baseline = admit_both(&document(
        r##"  <g><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
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
        // most one comma, never a leading, doubled, or trailing one.
        // Skipping empty tokens — the obvious implementation — would read
        // each of these as a well-formed shorter list, while Chromium
        // rejects them and paints the element untransformed.
        "translate(1,,2)",
        "translate(,1)",
        "translate(1,)",
        "translate(1 2,)",
        "matrix(1,0,0,1,,0)",
        ",translate(1,2)",
        "translate(1,2),,scale(2)",
        "translate(1,2),",
        // Function names are case-sensitive, and the CSS-only spellings —
        // units, `!important` — are not attribute grammar.
        "Translate(1,2)",
        "translate(10px, 10px)",
        "translate(1 2) !important",
    ] {
        let source = document(&format!(
            r##"  <g transform="{list}"><rect width="4" height="4" fill="#16a34a"/></g>"##
        ));
        let frame = admit_both(source.as_str());
        assert_eq!(
            frame, baseline,
            "{list}: a dropped list ≡ an absent attribute"
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
        // A cascaded `transform` sat beside these until its rung consumed
        // it — a transform needs no compositing scope, it was only ever
        // waiting on the computed read. The four below are the real
        // scope-bearers, refused until the group scope exists.
        ("group opacity", r#"opacity="0.5""#, "opacity"),
        ("group clip-path", r#"clip-path="url(#c)""#, "clip-path"),
        ("group mask", r#"mask="url(#m)""#, "mask"),
        ("group filter", r#"filter="url(#f)""#, "filter"),
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

/// The CSS-spelled transform is the same property as the attribute (CSS
/// Transforms L1 §7), so both spellings must produce the same frame — a
/// container's cascaded translate composes for every descendant exactly as
/// the attribute spelling does (probe: the two Chromium bakes are
/// byte-identical).
#[test]
fn a_cascaded_transform_composes_like_the_attribute() {
    let css = admit_both_styled(&document(
        r##"  <g style="transform: translate(10px, 10px)"><rect x="8" y="8" width="20" height="12" fill="#16a34a"/></g>"##,
    ));
    let attr = admit_both(&document(
        r##"  <g transform="translate(10 10)"><rect x="8" y="8" width="20" height="12" fill="#16a34a"/></g>"##,
    ));
    assert_eq!(css, attr, "one property, two spellings, one frame");
}

/// Precedence is the cascade's, measured against Chromium: any author rule
/// beats the attribute — `transform: none` included — and an *invalid* CSS
/// declaration never enters, so the attribute stands. (The cascade-level
/// rows live in csscascade's precedence laws; these pin that the compiler
/// composes the winner.)
#[test]
fn author_css_wins_the_transform_attribute_and_invalid_css_falls_back() {
    // The style attribute beats the attribute: the rect lands at the CSS
    // translate, not the attribute's.
    let both = admit_both_styled(&document(
        r##"  <g transform="translate(10 10)" style="transform: translate(30px, 0px)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let css_only = admit_both_styled(&document(
        r##"  <g style="transform: translate(30px, 0px)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    assert_eq!(both, css_only, "the author CSS is the one transform");

    // `transform: none` from a sheet un-transforms the attribute.
    let none = admit_both_styled(&document(
        r##"  <style>g { transform: none }</style>
  <g transform="translate(10 10)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let bare = admit_both_styled(&document(
        r##"  <style>g { transform: none }</style>
  <g><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    assert_eq!(none, bare, "authored none beats the attribute");

    // An invalid CSS declaration (unitless lengths are CSS-invalid) drops
    // at parse, so the attribute hint stands.
    let invalid = admit_both_styled(&document(
        r##"  <g transform="translate(10 10)" style="transform: translate(30, 0)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    let attr_only = admit_both(&document(
        r##"  <g transform="translate(10 10)"><rect width="4" height="4" fill="#16a34a"/></g>"##,
    ));
    assert_eq!(
        invalid, attr_only,
        "invalid CSS falls back to the attribute"
    );
}

/// Percentage translations resolve against the viewport's user-unit extent
/// — the `viewBox` when one maps the viewport (measured: `translate(50%,
/// 25%)` in a 64-unit viewBox moves exactly (+32, +16), and the same
/// percentages in a 128-unit viewBox move (+64, +32) regardless of the
/// raster size).
#[test]
fn percent_translations_resolve_against_the_viewbox() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect x="8" y="8" width="20" height="12" fill="#16a34a" style="transform: translate(50%, 25%)"/>
</svg>"##;
    let percent = admit_both_styled(source);
    let explicit = admit_both_styled(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect x="8" y="8" width="20" height="12" fill="#16a34a" style="transform: translate(32px, 16px)"/>
</svg>"##,
    );
    assert_eq!(
        percent, explicit,
        "50%/25% of a 64-unit viewBox is (32, 16)"
    );
}

/// The beyond-2D function family refuses by its CSS spelling. Chromium
/// composes these on SVG content (measured: `translate3d` moves a rect),
/// so a silent drop would move nothing where Chromium moves — the element
/// is a declared hole until a flattening rung measures the family.
#[test]
fn the_beyond_2d_transform_family_refuses_by_name() {
    let source = document(
        r##"  <rect width="4" height="4" fill="#16a34a" style="transform: translate3d(10px, 10px, 0px)"/>"##,
    );
    let error = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .err()
        .expect("strict refuses the 3D form");
    assert!(
        error.to_string().contains("translate3d()"),
        "the refusal names the function: {error}"
    );

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort compiles");
    assert_eq!(best.base_frame().nodes.len(), 0);
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].path(), "svg/rect[1]");
    assert_eq!(skipped[0].reason(), error.to_string());
}

/// The CSS spelling reaches the root too — a stylesheet can select the
/// outermost `<svg>` — and the root's transform applies to its CSS box
/// outside the viewBox mapping, exactly why the attribute spelling is a
/// root refusal. Both spellings refuse in both admissions, document-level.
#[test]
fn a_css_transform_on_the_root_svg_refuses_in_both_admissions() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>svg { transform: translate(4px, 0px) }</style>
  <rect width="16" height="16" fill="#16a34a"/>
</svg>"##;
    for result in [
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).err(),
        SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0)).err(),
    ] {
        let error = result.expect("document-level refusal in both admissions");
        assert!(
            error.to_string().contains("root <svg>"),
            "the refusal names the root: {error}"
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
    <text x="0" y="60" fill="#000000">hi</text>
    <circle cx="20" cy="20" r="4" fill="#2563eb"/>
  </g>"##,
    );
    SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .expect_err("strict refuses at the text");

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
        "svg/g[1]/text[1]",
        "the skip names its nested structural path"
    );
    assert_eq!(
        best.degradations()[0].reason(),
        "unsupported element <text>"
    );
}

/// Ordinals are per parent, so the same tag at different depths keeps
/// distinct, stable paths.
#[test]
fn nested_degradation_paths_number_per_parent() {
    let source = document(
        r##"  <text>a</text>
  <g>
    <text>b</text>
    <g>
      <text>c</text>
    </g>
  </g>
  <g>
    <text>d</text>
  </g>"##,
    );
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort");
    let paths: Vec<&str> = best.degradations().iter().map(|d| d.path()).collect();
    assert_eq!(
        paths,
        vec![
            "svg/text[1]",
            "svg/g[1]/text[1]",
            "svg/g[1]/g[1]/text[1]",
            "svg/g[2]/text[1]",
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

/// An `<animate>` under a shape nested in a container stays outside the
/// sampling inventory: the candidate set is the root's own materialized
/// rects, and admitting overrides deeper would widen the slice past what
/// the animation corpus bakes. Being outside the inventory, it is a
/// load-active authored-state override like any other — strict refuses at
/// construction, and best-effort skips the nested target at its nested
/// path, in every view.
#[test]
fn animate_inside_a_container_is_a_load_active_override() {
    let source = document(
        r##"  <g>
    <rect x="4" y="8" width="8" height="16" fill="#000000">
      <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
    </rect>
  </g>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
        .expect_err("strict refuses the override at construction");
    assert!(
        strict.to_string().contains("materialized top-level <rect>"),
        "the refusal carries the inventory's reason; got {strict}"
    );

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort");
    assert_eq!(
        best.base_frame().nodes.len(),
        0,
        "the nested target is a declared hole"
    );
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(declared.len(), 1);
    assert_eq!(
        declared[0].path(),
        "svg/g[1]/rect[1]",
        "declared at the target's nested path"
    );
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
        .expect("best-effort sampling never refuses a retained source"),
        best.base_frame(),
        "every view shares the skip"
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
  <style>rect { transform-origin: 4px 4px }</style>
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
    assert!(
        ignored[0].1.contains("transform-origin"),
        "{}",
        ignored[0].1
    );
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

/// A malformed transform on a *shape* drops on that shape exactly as it
/// does on a container: the shape renders untransformed, identical to the
/// same shape with no attribute at all.
#[test]
fn a_malformed_shape_transform_drops_on_the_shape() {
    for (bad, clean) in [
        (
            r##"<rect transform="translate(1,,2)" width="4" height="4" fill="#16a34a"/>"##,
            r##"<rect width="4" height="4" fill="#16a34a"/>"##,
        ),
        (
            r##"<circle transform="shear(2)" cx="8" cy="8" r="4" fill="#16a34a"/>"##,
            r##"<circle cx="8" cy="8" r="4" fill="#16a34a"/>"##,
        ),
        (
            r##"<ellipse transform="scale()" cx="8" cy="8" rx="4" ry="2" fill="#16a34a"/>"##,
            r##"<ellipse cx="8" cy="8" rx="4" ry="2" fill="#16a34a"/>"##,
        ),
    ] {
        let frame = admit_both(&document(&format!("  {bad}")));
        let baseline = admit_both(&document(&format!("  {clean}")));
        assert_eq!(frame, baseline, "{bad}: dropped ≡ absent");
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
        matches!(&error, CompileError::NonFiniteTransform { element } if element == "g"),
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
