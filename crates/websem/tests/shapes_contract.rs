//! The basic-shapes contract: what the one SVG compiler admits for
//! `<circle>` and `<ellipse>`, how their geometry resolves (defaults, the
//! `rx`/`ry` auto matrix, the degenerate-radius disables), and what refuses
//! or skips by name. Shape-level failures are element-level: strict refuses
//! the document, best-effort declares-and-skips the shape — the split every
//! law here exercises through both admissions.
//!
//! The resolved-geometry laws pin structure (local geometry boxes, the one
//! viewport transform, exact bounds); the pixel claims are Chromium-baked in
//! `reftest_oracle.rs` once the corpus step of this rung lands its fixtures.

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::Geometry;
use support::render_through_n0;
use websem::{CompileError, InitialViewport, SvgFrameSource, compile_standalone_svg};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

/// The local geometry box of node `index`, asserting the ellipse variant.
fn ellipse_box(frame: &rframe::Frame, index: usize) -> Rectangle {
    match &frame.nodes()[index].geometry {
        Geometry::Ellipse(rect) => *rect,
        other => panic!("expected ellipse geometry, got {other:?}"),
    }
}

/// A white 64x64 canvas around one shape under test.
fn on_canvas(shape: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <rect width="64" height="64" fill="#ffffff"/>
  {shape}
</svg>"##
    )
}

/// Strict and best-effort on one admitted source: identical frames, zero
/// degradations — the admission-invariance shape of every admit law here.
fn admit_both(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    assert!(
        best.degradations().is_empty(),
        "an admitted shape declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

/// Strict refuses and best-effort skips-and-declares the same shape-level
/// failure; returns the strict error and the declared reason. Only skips
/// are counted: a `style`-attribute source also carries the animation
/// inventory's own `SamplesAsBase` blocker, which is a separate contract.
fn shape_failure(source: &str) -> (CompileError, String) {
    let error =
        compile_standalone_svg(source, viewport(64.0, 64.0)).expect_err("strict refuses the shape");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort still compiles the document");
    let declared: Vec<&websem::Degradation> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == websem::DegradationAction::Skipped)
        .collect();
    assert_eq!(declared.len(), 1, "the failure is skipped exactly once");
    assert_eq!(
        declared[0].reason(),
        error.to_string(),
        "both admissions name the same reason"
    );
    (error, declared[0].reason().to_string())
}

#[test]
fn circle_resolves_to_its_inscribing_box() {
    let frame = admit_both(&on_canvas(
        r##"<circle cx="32" cy="32" r="12" fill="#16a34a"/>"##,
    ));
    assert_eq!(frame.nodes().len(), 2);
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(20.0, 20.0, 24.0, 24.0),
        "the circle's local geometry is the box inscribing it"
    );
    assert_eq!(frame.nodes()[1].transform, AffineTransform::identity());
    assert_eq!(
        frame.nodes()[1].bounds,
        Rectangle::from_xywh(20.0, 20.0, 24.0, 24.0)
    );
}

#[test]
fn circle_center_defaults_to_the_origin() {
    let frame = admit_both(&on_canvas(r##"<circle r="24" fill="#16a34a"/>"##));
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(-24.0, -24.0, 48.0, 48.0),
        "cx/cy default to 0; the frame clip crops the overhang"
    );
}

#[test]
fn negative_circle_centers_remain_authored_geometry() {
    let frame = admit_both(&on_canvas(
        r##"<circle cx="-4" cy="-6" r="8" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(-12.0, -14.0, 16.0, 16.0),
        "negative cx/cy are valid coordinates, never clamped"
    );
}

#[test]
fn circle_geometry_stays_local_under_a_scaling_viewport() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 32 32">
  <circle cx="16" cy="16" r="8" fill="#16a34a"/>
</svg>"##;
    let frame = admit_both(source);
    assert_eq!(
        ellipse_box(&frame, 0),
        Rectangle::from_xywh(8.0, 8.0, 16.0, 16.0),
        "geometry is authored user units"
    );
    assert_eq!(
        frame.nodes()[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 2.0, 0.0),
        "the one viewport transform carries the scale"
    );
    assert_eq!(
        frame.nodes()[0].bounds,
        Rectangle::from_xywh(16.0, 16.0, 32.0, 32.0)
    );
}

/// SVG2 §10.3: a negative `r` is invalid and the element is not rendered; a
/// computed value of zero likewise disables rendering. Missing, zero, and
/// negative radii therefore produce no frame node and no degradation.
#[test]
fn degenerate_circle_radius_is_an_admitted_nothing() {
    for (label, shape) in [
        ("missing r", r##"<circle cx="32" cy="32" fill="#16a34a"/>"##),
        (
            "zero r",
            r##"<circle cx="32" cy="32" r="0" fill="#16a34a"/>"##,
        ),
        (
            "negative r",
            r##"<circle cx="32" cy="32" r="-12" fill="#16a34a"/>"##,
        ),
        (
            "extreme negative r",
            r##"<circle cx="32" cy="32" r="-2.176e38" fill="#16a34a"/>"##,
        ),
    ] {
        let frame = admit_both(&on_canvas(shape));
        assert_eq!(
            frame.nodes().len(),
            1,
            "{label}: only the white backdrop materializes"
        );
        let pixels = render_through_n0(&frame, 64, 64);
        assert!(
            pixels.chunks_exact(4).all(|p| p == [255, 255, 255, 255]),
            "{label}: every pixel stays the white background"
        );
    }
}

#[test]
fn ellipse_resolves_distinct_radii() {
    let frame = admit_both(&on_canvas(
        r##"<ellipse cx="32" cy="32" rx="24" ry="12" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(8.0, 20.0, 48.0, 24.0)
    );
}

/// SVG2 §10.4 + geometry: `rx`/`ry` are initially `auto`, which adopts the
/// other radius; a negative value is invalid and must be ignored, which
/// Chromium implements as that same `auto` (live-probed against the pinned
/// bake version: a single negative radius renders exactly as the adopting
/// circle).
#[test]
fn ellipse_radius_auto_matrix_resolves_as_chromium() {
    let circle_box = Rectangle::from_xywh(20.0, 20.0, 24.0, 24.0);
    for (label, shape) in [
        (
            "rx absent adopts ry",
            r##"<ellipse cx="32" cy="32" ry="12" fill="#16a34a"/>"##,
        ),
        (
            "ry absent adopts rx",
            r##"<ellipse cx="32" cy="32" rx="12" fill="#16a34a"/>"##,
        ),
        (
            "negative rx treated as auto",
            r##"<ellipse cx="32" cy="32" rx="-5" ry="12" fill="#16a34a"/>"##,
        ),
        (
            "negative ry treated as auto",
            r##"<ellipse cx="32" cy="32" rx="12" ry="-5" fill="#16a34a"/>"##,
        ),
    ] {
        let frame = admit_both(&on_canvas(shape));
        assert_eq!(ellipse_box(&frame, 1), circle_box, "{label}");
    }
}

/// The `auto` **keyword** is a CSS value, not an attribute value: Blink
/// parses geometry presentation attributes with the SVGLength grammar,
/// where the keyword is invalid and maps an explicit `0px` hint — Chromium
/// renders nothing (live-probed: computed `rx: 0px`, zero-width bbox, white
/// center pixel), which is the opposite of the absent attribute's adopting
/// `auto`. Reading it as absent here would paint an ellipse the browser
/// does not, so it refuses loudly as a bad number instead.
#[test]
fn authored_auto_radius_keyword_refuses_instead_of_adopting() {
    for (label, shape) in [
        (
            "auto keyword",
            r##"<ellipse cx="32" cy="32" rx="auto" ry="12" fill="#16a34a"/>"##,
        ),
        (
            "case-folded auto keyword",
            r##"<ellipse cx="32" cy="32" rx="AUTO" ry="12" fill="#16a34a"/>"##,
        ),
    ] {
        let (error, _) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(&error, CompileError::BadNumber { attr, .. } if attr == "rx"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn degenerate_ellipse_radii_are_an_admitted_nothing() {
    for (label, shape, expected) in [
        (
            "both auto",
            r##"<ellipse cx="32" cy="32" fill="#16a34a"/>"##,
            Rectangle::from_xywh(32.0, 32.0, 0.0, 0.0),
        ),
        (
            "both negative",
            r##"<ellipse cx="32" cy="32" rx="-4" ry="-8" fill="#16a34a"/>"##,
            Rectangle::from_xywh(32.0, 32.0, 0.0, 0.0),
        ),
        (
            "zero rx disables",
            r##"<ellipse cx="32" cy="32" rx="0" ry="12" fill="#16a34a"/>"##,
            Rectangle::from_xywh(32.0, 20.0, 0.0, 24.0),
        ),
    ] {
        let frame = admit_both(&on_canvas(shape));
        assert_eq!(ellipse_box(&frame, 1), expected, "{label}");
        let pixels = render_through_n0(&frame, 64, 64);
        assert!(
            pixels.chunks_exact(4).all(|p| p == [255, 255, 255, 255]),
            "{label}: a zero-extent oval paints nothing"
        );
    }
}

/// The Rust float grammar superset stays refused on the new elements: a
/// trailing-dot number parses as f32 but is an invalid length to Chromium,
/// which resolves the property to an explicit zero (live-probed: computed
/// `rx: 0px`, rendering nothing) rather than to the parsed number — so it
/// refuses/skips as a bad number instead of silently resolving to a
/// different geometry than the oracle.
#[test]
fn rust_float_superset_radii_refuse_as_bad_numbers() {
    for (label, shape) in [
        (
            "circle r",
            r##"<circle cx="32" cy="32" r="12." fill="#16a34a"/>"##,
        ),
        (
            "ellipse rx",
            r##"<ellipse cx="32" cy="32" rx="12." ry="8" fill="#16a34a"/>"##,
        ),
        (
            "junk r",
            r##"<circle cx="32" cy="32" r="abc" fill="#16a34a"/>"##,
        ),
    ] {
        let (error, _) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(error, CompileError::BadNumber { .. }),
            "{label}: {error:?}"
        );
    }
}

/// Chromium reaches geometry presentation values through its CSS number
/// parser while this compiler owns a raw `f32` route. Amplified browser probes
/// established two distinct rounding classes: percentage normalization and a
/// direct decimal just above an exact f32 midpoint. Every affected attribute
/// refuses by the same stable geometry reason instead of choosing a neighbour.
#[test]
fn geometry_numeric_precision_aliases_refuse_for_every_raw_geometry_attribute() {
    const PERCENTAGE_ALIAS: &str = "57384.267578125007%";
    const DIRECT_ALIAS: &str = "1.000000059604644775390625000000000000000000000001";

    for (kind, attr, shape) in [
        (
            "percentage",
            "cx",
            format!(r##"<circle cx="{PERCENTAGE_ALIAS}" cy="32" r="8" fill="#16a34a"/>"##),
        ),
        (
            "percentage",
            "cy",
            format!(r##"<circle cx="32" cy="{PERCENTAGE_ALIAS}" r="8" fill="#16a34a"/>"##),
        ),
        (
            "percentage",
            "r",
            format!(r##"<circle cx="32" cy="32" r="{PERCENTAGE_ALIAS}" fill="#16a34a"/>"##),
        ),
        (
            "percentage",
            "x",
            format!(
                r##"<rect x="{PERCENTAGE_ALIAS}" y="8" width="8" height="8" fill="#16a34a"/>"##
            ),
        ),
        (
            "percentage",
            "y",
            format!(
                r##"<rect x="8" y="{PERCENTAGE_ALIAS}" width="8" height="8" fill="#16a34a"/>"##
            ),
        ),
        (
            "percentage",
            "width",
            format!(
                r##"<rect x="8" y="8" width="{PERCENTAGE_ALIAS}" height="8" fill="#16a34a"/>"##
            ),
        ),
        (
            "percentage",
            "height",
            format!(
                r##"<rect x="8" y="8" width="8" height="{PERCENTAGE_ALIAS}" fill="#16a34a"/>"##
            ),
        ),
        (
            "direct",
            "cx",
            format!(r##"<circle cx="{DIRECT_ALIAS}" cy="32" r="8" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "cy",
            format!(r##"<circle cx="32" cy="{DIRECT_ALIAS}" r="8" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "r",
            format!(r##"<circle cx="32" cy="32" r="{DIRECT_ALIAS}" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "x",
            format!(r##"<rect x="{DIRECT_ALIAS}" y="8" width="8" height="8" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "y",
            format!(r##"<rect x="8" y="{DIRECT_ALIAS}" width="8" height="8" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "width",
            format!(r##"<rect x="8" y="8" width="{DIRECT_ALIAS}" height="8" fill="#16a34a"/>"##),
        ),
        (
            "direct",
            "height",
            format!(r##"<rect x="8" y="8" width="8" height="{DIRECT_ALIAS}" fill="#16a34a"/>"##),
        ),
    ] {
        let (error, reason) = shape_failure(&on_canvas(&shape));
        assert!(
            matches!(&error, CompileError::UnsupportedGeometry(named)
                if named.contains(attr) && named.contains("numeric precision alias")),
            "{kind} {attr}: {error:?}"
        );
        assert!(
            reason.contains("loses Chromium used-value provenance"),
            "{kind} {attr}: {reason}"
        );
    }
}

/// A finite percentage token can still overflow this producer's f32
/// basis-resolution operation. Chromium drops each amplified source exactly
/// to empty, while finite direct controls reach its used-value clamp and
/// paint. The producer cannot represent that range contract yet, so it must
/// refuse at the attribute instead of leaking infinity into the frame.
#[test]
fn geometry_percentage_resolution_overflow_refuses_for_every_raw_geometry_attribute() {
    for (attr, shape) in [
        (
            "cx",
            r##"<circle cx="3.4e38%" cy="32" r="8" fill="#16a34a"/>"##,
        ),
        (
            "cy",
            r##"<circle cx="32" cy="3.4e38%" r="8" fill="#16a34a"/>"##,
        ),
        (
            "r",
            r##"<circle cx="32" cy="32" r="3.4e38%" fill="#16a34a"/>"##,
        ),
        (
            "x",
            r##"<rect x="3.4e38%" y="8" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "y",
            r##"<rect x="8" y="3.4e38%" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "width",
            r##"<rect x="8" y="8" width="3.4e38%" height="8" fill="#16a34a"/>"##,
        ),
        (
            "height",
            r##"<rect x="8" y="8" width="8" height="3.4e38%" fill="#16a34a"/>"##,
        ),
    ] {
        let (error, reason) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(&error, CompileError::UnsupportedGeometry(named)
                if named.contains(attr) && named.contains("finite frame range")),
            "{attr}: {error:?}"
        );
        assert!(reason.contains("resolves outside"), "{attr}: {reason}");
    }
}

/// Chromium carries extreme finite geometry through its Web used-value clamp;
/// this producer does not yet reproduce that clamp for shapes. All three
/// consumed attributes therefore refuse at the established Web range instead
/// of reaching a backend path that silently drops their transformed pixels.
#[test]
fn geometry_used_value_range_refuses_for_every_drawable_raw_geometry_attribute() {
    for (attr, shape) in [
        (
            "cx",
            r##"<circle cx="2.176e38" cy="32" r="8" fill="#16a34a"/>"##,
        ),
        (
            "cy",
            r##"<circle cx="32" cy="2.176e38" r="8" fill="#16a34a"/>"##,
        ),
        (
            "r",
            r##"<circle cx="32" cy="32" r="2.176e38" fill="#16a34a"/>"##,
        ),
        (
            "x",
            r##"<rect x="2.176e38" y="8" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "x",
            r##"<rect x="-2.176e38" y="8" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "y",
            r##"<rect x="8" y="2.176e38" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "y",
            r##"<rect x="8" y="-2.176e38" width="8" height="8" fill="#16a34a"/>"##,
        ),
        (
            "width",
            r##"<rect x="8" y="8" width="2.176e38" height="8" fill="#16a34a"/>"##,
        ),
        (
            "height",
            r##"<rect x="8" y="8" width="8" height="2.176e38" fill="#16a34a"/>"##,
        ),
    ] {
        let (error, reason) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(&error, CompileError::UnsupportedGeometry(named)
                if named.contains(attr) && named.contains("Web used-value range")),
            "{attr}: {error:?}"
        );
        assert!(reason.contains("exceeds the admitted"), "{attr}: {reason}");
    }
}

/// A negative box extent is invalid element geometry at every magnitude. It
/// must keep the admitted no-paint meaning rather than trip the positive
/// used-range patrol that protects drawable extents.
#[test]
fn extreme_negative_box_extents_remain_admitted_nothings() {
    for (attr, shape) in [
        (
            "width",
            r##"<rect x="8" y="8" width="-2.176e38" height="16" fill="#16a34a" stroke="#111827" stroke-width="4"/>"##,
        ),
        (
            "height",
            r##"<rect x="8" y="8" width="16" height="-2.176e38" fill="#16a34a" stroke="#111827" stroke-width="4"/>"##,
        ),
    ] {
        let frame = admit_both(&on_canvas(shape));
        let pixels = render_through_n0(&frame, 64, 64);
        assert!(
            pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [255, 255, 255, 255]),
            "negative {attr} paints no fill or stroke"
        );
    }
}

/// Unit-bearing values, CSS math, custom-property substitution, CSS-wide
/// keywords, and comments are valid CSS presentation-value forms Chromium
/// consumes. The first four families retain independent open checklist rows;
/// comments are a registered no-own-row gap. This raw parser over-refuses each
/// shape by its exact attribute rather than falling back to an authored/default
/// geometry and painting silently.
#[test]
fn geometry_css_value_families_refuse_by_attribute_name() {
    for (family, attr, shape) in [
        (
            "unit",
            "cx",
            r##"<circle cx="12pt" cy="32" r="8" fill="#16a34a"/>"##,
        ),
        (
            "calc",
            "cy",
            r##"<circle cx="32" cy="calc(16px + 16px)" r="8" fill="#16a34a"/>"##,
        ),
        (
            "var",
            "r",
            r##"<circle cx="32" cy="32" r="var(--r)" style="--r: 8px" fill="#16a34a"/>"##,
        ),
        (
            "initial",
            "cx",
            r##"<circle cx="initial" cy="32" r="8" fill="#16a34a"/>"##,
        ),
        (
            "inherit",
            "cy",
            r##"<circle cx="32" cy="inherit" r="8" fill="#16a34a"/>"##,
        ),
        (
            "unset",
            "r",
            r##"<circle cx="32" cy="32" r="unset" fill="#16a34a"/>"##,
        ),
        (
            "revert",
            "r",
            r##"<circle cx="32" cy="32" r="revert" fill="#16a34a"/>"##,
        ),
        (
            "CSS comment",
            "cx",
            r##"<circle cx="/**/32" cy="32" r="8" fill="#16a34a"/>"##,
        ),
    ] {
        let (error, reason) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(&error, CompileError::BadNumber { attr: named, .. } if named == attr),
            "{family} {attr}: {error:?}"
        );
        assert!(
            reason.contains(&format!("attribute {attr}=")),
            "{family} {attr} names its ingress: {reason}"
        );
    }
}

/// The rect-side consumers share the same raw parser as circle geometry. Every
/// value family the parser intentionally over-refuses must keep the exact
/// attribute name in both admission paths; otherwise one rect could silently
/// fall back while a sibling still made the document look successful.
#[test]
fn rect_geometry_css_value_families_refuse_by_attribute_name() {
    let shape = |attr: &str, value: &str, style: &str| {
        let (x, y, width, height) = match attr {
            "x" => (value, "8", "16", "16"),
            "y" => ("8", value, "16", "16"),
            "width" => ("8", "8", value, "16"),
            "height" => ("8", "8", "16", value),
            _ => unreachable!("the matrix names only rect geometry attributes"),
        };
        format!(
            r##"<rect x="{x}" y="{y}" width="{width}" height="{height}" {style} fill="#16a34a"/>"##
        )
    };
    let assert_named = |family: &str, attr: &str, source: String| {
        let (error, reason) = shape_failure(&on_canvas(&source));
        assert!(
            matches!(&error, CompileError::BadNumber { attr: named, .. } if named == attr),
            "{family} {attr}: {error:?}"
        );
        assert!(
            reason.contains(&format!("attribute {attr}=")),
            "{family} {attr} names its ingress: {reason}"
        );
    };

    for attr in ["x", "y", "width", "height"] {
        for (family, value, style) in [
            ("unit", "32px", ""),
            ("calc", "calc(16px + 16px)", ""),
            ("var", "var(--v)", r##"style="--v: 32px""##),
            ("CSS comment", "/**/32/**/", ""),
        ] {
            assert_named(family, attr, shape(attr, value, style));
        }
        for keyword in ["initial", "unset", "revert", "revert-layer", "inherit"] {
            assert_named(keyword, attr, shape(attr, keyword, ""));
        }
    }

    for attr in ["width", "height"] {
        assert_named("auto", attr, shape(attr, "auto", ""));
    }
}

/// Percentage geometry resolves against the one viewport's user-unit
/// extent (SVG2 §7.10): x-axis lengths against its width, y-axis against
/// its height, and a radius against the normalized diagonal — on a square
/// 64x64 viewport all three bases are 64, so `50%` is 32 everywhere, and
/// the Chromium-baked percentage cells pin the distinct-axis cases.
#[test]
fn percentage_geometry_resolves_against_the_axis_bases() {
    let frame = admit_both(&on_canvas(
        r##"<circle cx="50%" cy="50%" r="25%" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(16.0, 16.0, 32.0, 32.0),
        "cx/cy against the axes, r against the normalized diagonal"
    );

    // With a viewBox, the basis is the viewBox's user-unit extent, not the
    // root's pixel extent.
    let viewbox = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 32 32">
  <rect x="25%" y="25%" width="50%" height="50%" fill="#16a34a"/>
</svg>"##,
    );
    let rframe::Geometry::Rect(rect) = &viewbox.nodes()[0].geometry else {
        panic!("a rect");
    };
    assert_eq!(
        *rect,
        Rectangle::from_xywh(8.0, 8.0, 16.0, 16.0),
        "50% of the 32-unit viewBox, in user units"
    );

    // The grammar stays the number scanner's: junk around the sign refuses
    // as a bad number, exactly as it would without the percent.
    let (error, _) = shape_failure(&on_canvas(
        r##"<rect x="4" y="4" width="5 0%" height="8" fill="#16a34a"/>"##,
    ));
    assert!(
        matches!(error, CompileError::BadNumber { .. }),
        "malformed percentage: {error:?}"
    );
}

/// Attribute values are read from the SVG's own no-namespace attributes.
/// A prefixed lookalike is a foreign attribute Chromium ignores: consuming
/// it as geometry would paint a shape the browser does not.
#[test]
fn foreign_namespaced_geometry_attributes_are_not_consumed() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:a="urn:x" width="64" height="64">
  <rect width="64" height="64" fill="#ffffff"/>
  <circle a:r="30" cx="32" cy="32" fill="#16a34a"/>
</svg>"##;
    let frame = admit_both(source);
    assert_eq!(
        frame.nodes().len(),
        1,
        "the prefixed attribute is not `r`; only the backdrop materializes"
    );
    let pixels = render_through_n0(&frame, 64, 64);
    assert!(
        pixels.chunks_exact(4).all(|p| p == [255, 255, 255, 255]),
        "nothing paints, exactly as in Chromium"
    );
}

/// Only the five ASCII characters the SVG grammar calls whitespace may pad
/// a numeric attribute. Rust's Unicode `trim` would silently parse a value
/// Chromium rejects — and Chromium's rejection resolves to the property's
/// initial value, different geometry than the parsed number.
#[test]
fn non_ascii_whitespace_padding_refuses_as_a_bad_number() {
    for (label, shape) in [
        (
            "leading NBSP",
            "<circle cx=\"32\" cy=\"32\" r=\"\u{00A0}12\" fill=\"#16a34a\"/>",
        ),
        (
            "ideographic space",
            "<circle cx=\"32\" cy=\"32\" r=\"\u{3000}12\" fill=\"#16a34a\"/>",
        ),
        (
            "trailing line separator",
            "<circle cx=\"32\" cy=\"32\" r=\"12\u{2028}\" fill=\"#16a34a\"/>",
        ),
    ] {
        let (error, _) = shape_failure(&on_canvas(shape));
        assert!(
            matches!(&error, CompileError::BadNumber { attr, .. } if attr == "r"),
            "{label}: {error:?}"
        );
    }

    // The ASCII set still passes through to the number.
    let frame = admit_both(&on_canvas(
        "<circle cx=\"32\" cy=\"32\" r=\" \t\n12\r\" fill=\"#16a34a\"/>",
    ));
    assert_eq!(
        ellipse_box(&frame, 1),
        Rectangle::from_xywh(20.0, 20.0, 24.0, 24.0)
    );
}

/// A CSS property this cascade cannot represent — every one of them
/// `engine = "gecko"`-gated in the pinned Stylo build, so the declaration
/// is dropped at parse and no computed value survives — moves, clips, or
/// recolors Chromium's pixels. The `style` attribute leg is element-level
/// (skip-and-declare under best-effort); the stylesheet leg is
/// document-level, refusing in both admissions.
#[test]
fn cascade_properties_the_build_cannot_represent_refuse_by_name() {
    for (label, shape, property) in [
        // `transform` sat first in this table until its rung consumed it;
        // `transform-origin` holds the family's place — it changes where
        // every transform pivots (measured), and stays unread.
        (
            "cx",
            r##"<circle cx="8" cy="8" r="3" style="cx: 32px" fill="#16a34a"/>"##,
            "cx",
        ),
        (
            "cy",
            r##"<circle cx="8" cy="8" r="3" style="cy: 32px" fill="#16a34a"/>"##,
            "cy",
        ),
        (
            "r",
            r##"<circle cx="8" cy="8" r="3" style="r: 12px" fill="#16a34a"/>"##,
            "r",
        ),
        (
            "x",
            r##"<rect x="8" y="8" width="16" height="16" style="x: 32px" fill="#16a34a"/>"##,
            "x",
        ),
        (
            "y",
            r##"<rect x="8" y="8" width="16" height="16" style="y: 32px" fill="#16a34a"/>"##,
            "y",
        ),
        (
            "transform-origin",
            r##"<circle cx="20" cy="32" r="10" style="transform: rotate(90deg); transform-origin: 20px 32px" fill="#16a34a"/>"##,
            "transform-origin",
        ),
        (
            "clip-path",
            r##"<circle cx="32" cy="32" r="12" style="clip-path: inset(0 50% 0 0)" fill="#16a34a"/>"##,
            "clip-path",
        ),
        (
            "filter",
            r##"<ellipse cx="32" cy="32" rx="12" ry="8" style="filter: blur(4px)" fill="#16a34a"/>"##,
            "filter",
        ),
        (
            "mix-blend-mode",
            r##"<rect x="4" y="4" width="8" height="8" style="mix-blend-mode: multiply" fill="#16a34a"/>"##,
            "mix-blend-mode",
        ),
    ] {
        let (error, reason) = shape_failure(&on_canvas(shape));
        if label == "clip-path" {
            assert!(
                matches!(error, CompileError::UnsupportedClipPath(_)),
                "{label}: {error:?}"
            );
        } else {
            assert!(
                matches!(error, CompileError::UnsupportedStyle(_)),
                "{label}: {error:?}"
            );
        }
        assert!(reason.contains(property), "{label} names it: {reason}");
    }

    // The stylesheet leg is document-level, because a sheet is not
    // attributable to one element without selector matching. `clip-path`
    // has now left that scanner: Stylo computes it and the compiler can name
    // the unsupported basic-shape route on the affected element itself.
    let styled = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>circle { clip-path: inset(0 50% 0 0); }</style>
  <rect width="64" height="64" fill="#ffffff"/>
  <circle cx="20" cy="32" r="10" fill="#16a34a"/>
</svg>"##;
    let strict = SvgFrameSource::from_standalone_svg(styled, viewport(64.0, 64.0))
        .err()
        .expect("strict refuses the basic-shape route");
    assert!(strict.to_string().contains("clip-path"), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(styled, viewport(64.0, 64.0))
        .expect("best-effort renders and declares");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.reason().contains("basic-shape clip-path"))
        .collect();
    assert_eq!(declared.len(), 1, "declared exactly once");
    assert_eq!(declared[0].path(), "svg/circle[1]", "named at the target");
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the admitted background still renders while the affected target is skipped"
    );
}

#[test]
fn geometry_stylesheet_properties_refuse_at_the_sheet() {
    for (property, value, selector, shape) in [
        (
            "cx",
            "32px",
            "circle",
            r##"<circle cx="8" cy="8" r="3" fill="#16a34a"/>"##,
        ),
        (
            "cy",
            "32px",
            "circle",
            r##"<circle cx="8" cy="8" r="3" fill="#16a34a"/>"##,
        ),
        (
            "r",
            "12px",
            "circle",
            r##"<circle cx="8" cy="8" r="3" fill="#16a34a"/>"##,
        ),
        (
            "x",
            "32px",
            "rect.target",
            r##"<rect class="target" x="8" y="8" width="16" height="16" fill="#16a34a"/>"##,
        ),
        (
            "y",
            "32px",
            "rect.target",
            r##"<rect class="target" x="8" y="8" width="16" height="16" fill="#16a34a"/>"##,
        ),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>{selector} {{ {property}: {value}; }}</style>
  <rect width="64" height="64" fill="#ffffff"/>
  {shape}
</svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .expect_err("strict refuses an unrepresented geometry property");
        assert!(
            strict
                .to_string()
                .contains(&format!("stylesheet declares {property}")),
            "{strict}"
        );

        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
                .expect("best-effort renders with a sheet declaration");
        let declared: Vec<_> = best
            .degradations()
            .iter()
            .filter(|d| d.reason().contains("stylesheet declares"))
            .collect();
        assert_eq!(declared.len(), 1, "{property} is declared exactly once");
        assert!(
            declared[0]
                .reason()
                .contains(&format!("declares {property}")),
            "the sheet names {property}: {declared:?}"
        );
        assert_eq!(
            declared[0].path(),
            "svg/style[1]",
            "sheet-level declarations keep one structural path"
        );
    }
}

/// An unconsumed rendering attribute on the new shape refuses/skips by name.
#[test]
fn transform_origin_refuses_on_ellipse() {
    let shape =
        r##"<ellipse cx="32" cy="32" rx="12" ry="8" fill="#16a34a" transform-origin="center"/>"##;
    let (error, _) = shape_failure(&on_canvas(shape));
    assert!(
        matches!(error, CompileError::UnsupportedAttribute { .. }),
        "transform-origin on ellipse: {error:?}"
    );
}

/// A stylesheet-set `stroke-dasharray` resolves on both basic shapes exactly as
/// on `<rect>`. The property is inherited and consumed from the one computed
/// style; the `<style>` element carries only the separate Sample blocker.
#[test]
fn stylesheet_stroke_dasharray_resolves_on_the_new_shapes() {
    for (element, shape) in [
        (
            "circle",
            r##"<circle cx="32" cy="32" r="12" fill="#16a34a"/>"##,
        ),
        (
            "ellipse",
            r##"<ellipse cx="32" cy="32" rx="12" ry="8" fill="#16a34a"/>"##,
        ),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>{element} {{ stroke: #000000; stroke-dasharray: 4 4; }}</style>
  <rect width="64" height="64" fill="#ffffff"/>
  {shape}
</svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport(64.0, 64.0))
            .expect("strict admits the stylesheet dash");
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
                .expect("best-effort admits the stylesheet dash");
        assert!(
            best.degradations()
                .iter()
                .all(|d| d.action() == websem::DegradationAction::SamplesAsBase),
            "only the separate stylesheet sampling blocker remains"
        );
        assert_eq!(strict.base_frame(), best.base_frame());
        let frame = strict.base_frame();
        assert_eq!(
            frame.nodes()[1]
                .stroke
                .as_ref()
                .expect("stroke")
                .dash_intervals()
                .expect("dash cycle")
                .as_slice(),
            [4.0, 4.0],
            "{element}"
        );
    }
}

/// SVG2's applicability table makes CSS `width`/`height` inert on
/// `<circle>`/`<ellipse>` (they are geometry only on the root, `<rect>`,
/// and the image-like elements), so a cascaded value there is not a
/// smuggled size: the document admits and renders exactly as without the
/// rule — no over-refusal. The geometry properties that *would* apply
/// (`cx`/`cy`/`r`/`rx`/`ry`) do not exist in the pinned Stylo build and
/// stay a named open boundary.
#[test]
fn inert_css_sizing_on_the_new_shapes_admits() {
    for (element, shape) in [
        (
            "circle",
            r##"<circle cx="32" cy="32" r="12" fill="#16a34a"/>"##,
        ),
        (
            "ellipse",
            r##"<ellipse cx="32" cy="32" rx="12" ry="8" fill="#16a34a"/>"##,
        ),
    ] {
        let styled = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>{element} {{ width: 10px; height: 10px; }}</style>
  <rect width="64" height="64" fill="#ffffff"/>
  {shape}
</svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(styled.as_str(), viewport(64.0, 64.0))
            .expect("strict admits the inert rule");
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(styled.as_str(), viewport(64.0, 64.0))
                .expect("best-effort admits the inert rule");
        assert!(
            best.degradations()
                .iter()
                .all(|d| d.action() != websem::DegradationAction::Skipped),
            "{element} skips nothing: {:?}",
            best.degradations()
        );
        let styled_frame = strict.base_frame();
        assert_eq!(styled_frame, best.base_frame());
        let plain_frame = admit_both(&on_canvas(shape));
        assert_eq!(
            render_through_n0(&styled_frame, 64, 64),
            render_through_n0(&plain_frame, 64, 64),
            "{element}: the inert rule changes no pixel"
        );
    }
}

/// `<animate>` under a circle stays outside the sampling inventory: the
/// slice is rect-x, and a materialized circle must not silently admit an
/// override no shape read consumes. Outside the inventory it is treated as
/// a load-active override — a deliberate over-refusal here, since `x` does
/// not apply to a circle and Chromium ignores it, but the inventory owns no
/// per-element applicability model and a declared hole is never a wrong
/// pixel.
#[test]
fn animate_under_a_circle_is_a_load_active_override() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <rect width="64" height="64" fill="#ffffff"/>
  <circle cx="32" cy="32" r="12" fill="#16a34a">
    <animate attributeName="x" from="0" to="16" dur="2s" fill="freeze"/>
  </circle>
</svg>"##;
    let strict = SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .expect_err("strict refuses the override at construction");
    assert!(
        strict.to_string().contains("materialized top-level <rect>"),
        "the refusal names the narrow slice: {strict}"
    );

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort");
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the circle is a declared hole; the backdrop renders"
    );
    let declared: Vec<_> = best.degradations().iter().collect();
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].action(), websem::DegradationAction::Skipped);
    assert_eq!(declared[0].path(), "svg/circle[1]");
    assert!(
        declared[0]
            .reason()
            .contains("materialized top-level <rect>"),
        "the declaration names the narrow slice: {}",
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

/// Both grammar entries reach the new shapes through the one compiler: the
/// inline-SVG-in-HTML entry and the standalone entry resolve a circle to
/// the same SVG-local frame (the equivalence law extended to the rung).
#[test]
fn inline_and_standalone_circles_resolve_to_the_same_frame() {
    let svg_body = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#ffffff"/><circle cx="32" cy="32" r="12" fill="#16a34a"/></svg>"##;
    let html = format!("<html><body>{svg_body}</body></html>");
    let inline = websem::compile_html_inline_svg(&html).expect("compile inline entry");
    let standalone =
        compile_standalone_svg(svg_body, viewport(64.0, 64.0)).expect("compile standalone entry");
    assert_eq!(
        inline, standalone,
        "one compiler, one resolved frame across entries"
    );
}
