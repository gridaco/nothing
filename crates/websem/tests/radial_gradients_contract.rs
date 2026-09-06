//! Ordered radial circles: value and refusal laws. Chromium PNG cells own
//! pixel truth; these assertions attribute the source branch that produced it.
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn source(attributes: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32"><defs><radialGradient id="g" {attributes}><stop stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient></defs><rect width="64" height="32" fill="url(#g)"/></svg>"##
    )
}

fn paint(source: &str) -> cg::RadialGradientPaint {
    let viewport = InitialViewport::new(64.0, 32.0);
    let strict = SvgFrameSource::from_standalone_svg(source, viewport).unwrap();
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport).unwrap();
    assert!(
        best.degradations()
            .iter()
            .all(|d| d.action() == DegradationAction::SamplesAsBase)
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame());
    let cg::Paint::RadialGradient(paint) = frame.nodes()[0].paints.iter().next().unwrap() else {
        panic!("radial leaf")
    };
    paint.clone()
}

#[test]
fn default_and_explicit_default_keep_the_old_leaf_while_radii_remain_ordered() {
    assert_eq!(
        paint(&source("")),
        paint(&source(r#"fx=".5" fy=".5" fr="0""#))
    );
    assert!(paint(&source("")).geometry.is_none());
    for (attrs, start, end) in [
        (
            r#"fx="-.25" fy=".75" fr=".125""#,
            (-0.25, 0.75, 0.125),
            (0.5, 0.5, 0.5),
        ),
        (
            r#"fx=".25" fy=".375" fr=".75""#,
            (0.25, 0.375, 0.75),
            (0.5, 0.5, 0.5),
        ),
        (
            r#"fx=".25" fy=".375" fr=".125" r="0""#,
            (0.25, 0.375, 0.125),
            (0.5, 0.5, 0.0),
        ),
        (
            r#"fx=".25" fy=".375" fr=".125" r="-.25""#,
            (0.25, 0.375, 0.125),
            (0.5, 0.5, 0.0),
        ),
        (r#"fx=".25" fr="-.25""#, (0.25, 0.5, 0.0), (0.5, 0.5, 0.5)),
        (r#"fr=".5""#, (0.5, 0.5, 0.5), (0.5, 0.5, 0.5)),
    ] {
        let paint = paint(&source(attrs));
        let g = paint.geometry.unwrap();
        assert_eq!((g.start.center.0, g.start.center.1, g.start.radius), start);
        assert_eq!((g.end.center.0, g.end.center.1, g.end.radius), end);
        assert_eq!(
            paint.transform,
            math2::transform::AffineTransform::identity()
        );
    }
}

#[test]
fn user_percentages_resolve_before_placement_with_a_diagonal_radius_basis() {
    let g = paint(&source(
        r#"gradientUnits="userSpaceOnUse" fx="25%" fy="37.5%" fr="12.5%""#,
    ))
    .geometry
    .unwrap();
    assert_eq!(g.start.center, (16.0, 12.0));
    // Pin the contract's multiply-before-divide arithmetic, independently of
    // the resolver. The Chromium percentage/numeric PNG pair proves the
    // diagonal basis, not equality of their least-significant float bits.
    assert_eq!(g.start.radius.to_bits(), 1_087_005_379);
    assert_ne!(g.start.radius, 8.0, "width is not the radius basis");
    assert_ne!(g.start.radius, 4.0, "height is not the radius basis");
}

#[test]
fn every_retained_value_family_names_the_exact_attribute_in_both_admissions() {
    for attr in ["fx", "fy", "fr"] {
        for (value, needle) in [
            ("57384.267578125007", "numeric precision alias"),
            ("57384.267578125007%", "numeric precision alias"),
            (
                "8388608.500000000000000000000000000000000000000008388608",
                "numeric precision alias",
            ),
            ("/*a*/.25/*b*/", "CSS comment"),
            (".2/**/5", "CSS comment"),
            ("1e999", "admitted Web used-value range"),
            ("4em", "unit whose basis"),
            ("calc(25%)", "uses calc()"),
            ("min(25%,50%)", "uses min()"),
            ("max(25%,50%)", "uses max()"),
            ("clamp(0%,25%,50%)", "uses clamp()"),
            ("var(--v)", "uses var()"),
            ("env(safe-area-inset-left)", "uses env()"),
            ("initial", "CSS-wide value"),
            ("inherit", "CSS-wide value"),
            ("unset", "CSS-wide value"),
            ("revert", "CSS-wide value"),
            ("revert-layer", "CSS-wide value"),
        ] {
            let input = source(&format!(r#"{attr}="{value}""#));
            let viewport = InitialViewport::new(64.0, 32.0);
            let error = SvgFrameSource::from_standalone_svg(input.as_str(), viewport).unwrap_err();
            let CompileError::UnsupportedFill(reason) = error else {
                panic!("wrong refusal {error:?}")
            };
            assert!(
                reason.contains(&format!("gradient geometry {attr}")),
                "{reason}"
            );
            assert!(reason.contains(needle), "{reason}");
            let best =
                SvgFrameSource::from_standalone_svg_best_effort(input.as_str(), viewport).unwrap();
            let skipped: Vec<_> = best
                .degradations()
                .iter()
                .filter(|d| d.action() == DegradationAction::Skipped)
                .collect();
            assert_eq!(skipped.len(), 1);
            assert!(skipped[0].reason().to_string().contains(needle));
            assert!(
                skipped[0]
                    .reason()
                    .to_string()
                    .contains(&format!("gradient geometry {attr}"))
            );
            assert!(
                best.base_frame().nodes().is_empty(),
                "a refused target cannot leak concentric paint"
            );
        }
    }
}
