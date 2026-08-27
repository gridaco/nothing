//! SVG pattern laws at the Web-semantic contract boundary.
//!
//! Chromium probes decide URL fallback, template ownership, coordinate
//! systems, cascade precedence, and the pinned picture-shader precision
//! envelope. These tests pin the resulting source-neutral program and every
//! stable refusal; committed Web-first cells separately grade the pixels.

#[allow(dead_code)]
mod support;

use cg::{CGColor, Paint};
use rframe::{Frame, FrameItem, PatternPaint, ScopeEffect};
use support::render_through_n0;
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64">
{body}
</svg>"##
    )
}

fn admit_both(source: &str) -> Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort admits");
    let static_degradations: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        static_degradations.is_empty(),
        "an admitted pattern declares nothing static: {static_degradations:?}"
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
        "the explicit backdrop survives"
    );
}

fn pattern_of(frame: &Frame, node: usize) -> &PatternPaint {
    frame.nodes()[node]
        .paints
        .pattern()
        .expect("resolved pattern paint")
}

fn solid_of(paint: &Paint) -> CGColor {
    match paint {
        Paint::Solid(solid) => solid.color,
        other => panic!("expected a solid, got {other:?}"),
    }
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 64 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

#[test]
fn a_same_document_pattern_becomes_one_repeating_local_program() {
    let frame = admit_both(&document(
        r##"  <defs>
    <pattern id="p" patternUnits="userSpaceOnUse" x="2" y="3" width="8" height="8">
      <rect width="4" height="8" fill="#ef4444"/>
      <rect x="4" width="4" height="8" fill="#22c55e"/>
    </pattern>
  </defs>
  <rect width="32" height="24" fill="url(#p)"/>"##,
    ));
    let pattern = pattern_of(&frame, 0);
    assert_eq!((pattern.width(), pattern.height()), (8.0, 8.0));
    assert_eq!(pattern.transform().matrix[0][2], 2.0);
    assert_eq!(pattern.transform().matrix[1][2], 3.0);
    assert_eq!(pattern.items().nodes().count(), 2);

    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 2, 4), [0xef, 0x44, 0x44, 0xff]);
    assert_eq!(at(&pixels, 6, 4), [0x22, 0xc5, 0x5e, 0xff]);
    assert_eq!(at(&pixels, 10, 4), [0xef, 0x44, 0x44, 0xff]);
    assert_eq!(
        pixels,
        render_through_n0(&frame, 64, 64),
        "fresh pattern replay is byte-identical"
    );
}

#[test]
fn invalid_and_valid_empty_patterns_select_different_fallback_outcomes() {
    let invalid = admit_both(&document(
        r##"  <defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"/></defs>
  <rect width="64" height="64" fill="url(#p) #ef4444"/>"##,
    ));
    let fallback = invalid.nodes()[0]
        .paints
        .iter()
        .next()
        .expect("invalid pattern selects the authored fallback");
    assert_eq!(solid_of(fallback), CGColor::from_rgb(0xef, 0x44, 0x44));

    let valid_empty = admit_both(&document(
        r##"  <defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"><title>selected local content</title></pattern></defs>
  <rect width="64" height="64" fill="url(#p) #ef4444"/>"##,
    ));
    let pattern = pattern_of(&valid_empty, 0);
    assert!(
        pattern.items().is_empty(),
        "the selected tile is transparent"
    );
    assert_eq!(valid_empty.nodes()[0].paints.iter().count(), 0);
}

#[test]
fn object_box_units_resolve_once_per_consuming_geometry() {
    let frame = admit_both(&document(
        r##"  <defs>
    <pattern id="p" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox"
             width=".5" height=".5">
      <rect width=".5" height=".5" fill="#16a34a"/>
    </pattern>
  </defs>
  <rect x="4" y="4" width="16" height="16" fill="url(#p)"/>
  <rect x="28" y="4" width="32" height="16" fill="url(#p)"/>"##,
    ));
    assert_eq!(frame.nodes().len(), 2);
    assert_eq!(
        (
            pattern_of(&frame, 0).width(),
            pattern_of(&frame, 0).height()
        ),
        (8.0, 8.0)
    );
    assert_eq!(
        (
            pattern_of(&frame, 1).width(),
            pattern_of(&frame, 1).height()
        ),
        (16.0, 8.0)
    );
}

#[test]
fn the_pattern_transform_hint_uses_the_one_transform_cascade() {
    let frame = admit_both(&document(
        r##"  <defs>
    <pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"
             patternTransform="translate(4 0)" transform="translate(8 0)"
             style="transform: translate(12px, 0px)">
      <rect width="8" height="8" fill="#16a34a"/>
    </pattern>
  </defs>
  <rect width="64" height="64" fill="url(#p)"/>"##,
    ));
    assert_eq!(
        pattern_of(&frame, 0).transform().matrix,
        [[1.0, 0.0, 12.0], [0.0, 1.0, 0.0]],
        "the author declaration beats patternTransform; plain transform is inert"
    );
}

#[test]
fn percentage_pattern_transform_spellings_split_before_the_frame() {
    let invalid_attribute = admit_both(&document(
        r##"  <defs>
    <pattern id="p" patternUnits="userSpaceOnUse" width="14" height="14"
             patternTransform="translate(50% 0)">
      <rect width="4" height="14" fill="red"/>
      <rect x="4" width="10" height="14" fill="green"/>
    </pattern>
  </defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##,
    ));
    assert_eq!(
        pattern_of(&invalid_attribute, 0).transform().matrix,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        "the invalid presentation-attribute percentage drops to identity"
    );

    assert_target_skip(
        &document(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs>
    <pattern id="p" patternUnits="userSpaceOnUse" width="14" height="14"
             style="transform:translate(50%, 0px)">
      <rect width="4" height="14" fill="red"/>
      <rect x="4" width="10" height="14" fill="green"/>
    </pattern>
  </defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##,
        ),
        "pattern transform percentage has no proved reference-box basis",
    );
}

#[test]
fn a_ninth_distinct_pattern_refuses_before_its_source_starts_compiling() {
    let mut defs = String::new();
    for index in 0..9 {
        let child = if index == 8 {
            // If the ninth source starts compiling, the circle reaches the
            // independent source-coverage patrol before contract construction.
            // The program-depth refusal must win first.
            r##"<circle cx="7" cy="7" r="5" fill="red"/>"##.to_string()
        } else {
            format!(
                r##"<rect width="14" height="14" fill="url(#p{})"/>"##,
                index + 1
            )
        };
        defs.push_str(&format!(
            r##"<pattern id="p{index}" patternUnits="userSpaceOnUse" width="14" height="14">{child}</pattern>"##
        ));
    }

    assert_target_skip(
        &document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs>{defs}</defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p0)"/>"##
        )),
        "nested pattern paint chain exceeds the resolved 8-program limit",
    );
}

#[test]
fn href_beats_xlink_and_the_first_local_content_owner_wins() {
    let frame = admit_both(&document(
        r##"  <defs>
    <pattern id="red" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="8" height="8" fill="red"/></pattern>
    <pattern id="blue" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="8" height="8" fill="blue"/></pattern>
    <pattern id="p" href="#red" xlink:href="#blue"/>
  </defs>
  <rect width="64" height="64" fill="url(#p)"/>"##,
    ));
    let source = pattern_of(&frame, 0);
    let paint = source
        .items()
        .nodes()
        .next()
        .unwrap()
        .paints
        .iter()
        .next()
        .unwrap();
    assert_eq!(solid_of(paint), CGColor::from_rgb(255, 0, 0));
}

#[test]
fn a_filter_inside_pattern_content_stays_in_the_resolved_source_program() {
    let frame = admit_both(&document(
        r##"  <defs>
    <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="16" height="16"
            color-interpolation-filters="sRGB"><feGaussianBlur stdDeviation="2"/></filter>
    <pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16">
      <rect x="4" y="4" width="8" height="8" fill="#16a34a" filter="url(#f)"/>
    </pattern>
  </defs>
  <rect width="64" height="64" fill="url(#p)"/>"##,
    ));
    let source = pattern_of(&frame, 0);
    assert!(source.items().iter().any(|item| {
        matches!(
            item,
            FrameItem::ScopeBegin(scope) if matches!(scope.effect, ScopeEffect::Filter(_))
        )
    }));

    let nested = admit_both(&document(
        r##"  <defs>
    <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="16" height="16"
            color-interpolation-filters="sRGB"><feGaussianBlur stdDeviation="2"/></filter>
    <pattern id="q" patternUnits="userSpaceOnUse" width="4" height="4"><rect width="2" height="4" fill="#e11d48"/></pattern>
    <pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16">
      <rect width="16" height="16" fill="url(#q)" filter="url(#f)"/>
    </pattern>
  </defs>
  <rect width="64" height="64" fill="url(#p)"/>"##,
    ));
    assert!(pattern_of(&nested, 0).items().iter().any(|item| {
        matches!(
            item,
            FrameItem::ScopeBegin(scope) if matches!(scope.effect, ScopeEffect::Filter(_))
        )
    }));
}

#[test]
fn every_measured_picture_shader_boundary_refuses_in_both_admissions() {
    let cases = [
        (
            r##"<pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16"><circle cx="8" cy="8" r="5" fill="red"/></pattern>"##,
            "source-coverage precision boundary",
        ),
        (
            r##"<pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16"><rect width="5" height="5" transform="matrix(.6 .8 -.8 .6 4 0)" fill="red"/></pattern>"##,
            "source-coverage precision boundary",
        ),
        (
            r##"<pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16"><g opacity=".5"><rect width="8" height="16" fill="red"/><rect x="8" width="8" height="16" fill="blue"/></g></pattern>"##,
            "source-effect precision boundary",
        ),
        (
            r##"<pattern id="q" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="8" height="8" fill="red"/></pattern><pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16"><rect width="16" height="16" fill="url(#q)"/><rect width="4" height="4" fill="white"/></pattern>"##,
            "composition precision boundary",
        ),
        (
            r##"<pattern id="q" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="8" height="8" fill="red"/></pattern><pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16"><rect width="16" height="16" fill="url(#q)" stroke="white"/></pattern>"##,
            "composition precision boundary",
        ),
        (
            r##"<pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16" patternTransform="rotate(17)"><rect width="16" height="16" fill="red"/></pattern>"##,
            "affine precision boundary",
        ),
        (
            r##"<pattern id="p" patternUnits="userSpaceOnUse" width="7.5" height="8"><rect width="4" height="8" fill="red"/></pattern>"##,
            "sampling precision boundary",
        ),
    ];

    for (defs, reason) in cases {
        assert_target_skip(
            &document(&format!(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs>{defs}</defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##
            )),
            reason,
        );
    }
}

#[test]
fn unresolved_length_contexts_refuse_by_the_exact_pattern_field() {
    for (value, reason) in [
        (
            ".2cm",
            "length unit whose basis this slice does not consume",
        ),
        ("calc(4px + 4px)", "pattern width uses a CSS function"),
        ("var(--w)", "pattern width resolves through var()"),
        ("inherit", "pattern width uses the CSS-wide value"),
        ("/**/8/**/", "pattern width contains a CSS comment"),
    ] {
        assert_target_skip(
            &document(&format!(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><pattern id="p" patternUnits="userSpaceOnUse" width="{value}" height="8" style="--w:8px"><rect width="8" height="8" fill="red"/></pattern></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##
            )),
            reason,
        );
    }
}

#[test]
fn external_template_dependency_and_raw_number_aliases_are_named() {
    assert_target_skip(
        &document(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8" href="external.svg#q"><rect width="8" height="8" fill="red"/></pattern></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##,
        ),
        "external template",
    );
    assert_target_skip(
        &document(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs><pattern id="p" patternUnits="userSpaceOnUse" x="57384.267578125007%" width="8" height="8"><rect width="8" height="8" fill="red"/></pattern></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##,
        ),
        "numeric precision alias",
    );
    for value in ["1000000000", "33554430", "-1000000000", "1e100"] {
        assert_target_skip(
            &document(&format!(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><pattern id="p" patternUnits="userSpaceOnUse" x="{value}" width="12" height="12"><rect width="4" height="12" fill="red"/></pattern></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#p)"/>"##
            )),
            "admitted Web used-value range",
        );
    }
}
