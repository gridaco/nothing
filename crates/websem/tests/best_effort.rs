//! Laws of the best-effort admission mode.
//!
//! Best-effort is the product default at the host: it compiles the admitted
//! subset and declares every beyond-slice construct as a [`Degradation`] —
//! skipped by name, or resolved to the Base view for a blocked dynamic
//! surface. Strict is the dev harness: the same compiler, refusing on the
//! first beyond-slice construct. These laws pin the boundary between the
//! two: where degradations are empty the modes are frame-identical, the
//! degradation set is a stable property of the retained source, and
//! document-level contracts refuse identically in both modes.

use std::path::PathBuf;

use animation_sampling::SampleTime;
use serde::Deserialize;
use websem::{DegradationAction, SvgFrameSource};

const ADMITTED_STATIC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000"/>
</svg>"##;

const MIXED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <rect width="64" height="64" fill="#ffffff"/>
  <circle cx="32" cy="32" r="16" fill="#16a34a"/>
  <rect x="4" y="4" width="8" height="8" fill="#000000"/>
  <path d="M0 0 h8 v8 z" fill="#000000"/>
</svg>"##;

const ADMITTED_ANIMATION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;

const UNSUPPORTED_ANIMATION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="y" from="8" to="16" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;

#[derive(Debug, Deserialize)]
struct PrimitiveSuite {
    fixtures: Vec<Primitive>,
}

#[derive(Debug, Deserialize)]
struct Primitive {
    id: String,
    source: String,
    entry: String,
}

/// The FULL oracle corpus is admission-invariant: every enumerated
/// primitive compiles with zero degradations, and the strict and
/// best-effort frames are identical. This is the corpus-wide gate behind
/// the claim that where nothing degrades the two admissions agree.
#[test]
fn the_full_oracle_corpus_is_admission_invariant() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first");
    let suite: PrimitiveSuite = serde_json::from_str(
        &std::fs::read_to_string(root.join("primitives.json")).expect("read primitives.json"),
    )
    .expect("parse primitives.json");
    assert!(!suite.fixtures.is_empty());
    for primitive in &suite.fixtures {
        let source =
            std::fs::read_to_string(root.join(&primitive.source)).expect("read primitive source");
        let (strict, best) = match primitive.entry.as_str() {
            "standalone-svg" => (
                SvgFrameSource::from_standalone_svg(source.as_str()),
                SvgFrameSource::from_standalone_svg_best_effort(source.as_str()),
            ),
            "html-inline-svg" => (
                SvgFrameSource::from_html_inline_svg(source.as_str()),
                SvgFrameSource::from_html_inline_svg_best_effort(source.as_str()),
            ),
            other => panic!("{} has unknown entry {other:?}", primitive.id),
        };
        let strict = strict.unwrap_or_else(|error| panic!("{}: strict: {error}", primitive.id));
        let best = best.unwrap_or_else(|error| panic!("{}: best-effort: {error}", primitive.id));
        let static_degradations: Vec<_> = best
            .degradations()
            .iter()
            .filter(|d| d.action() == DegradationAction::Skipped)
            .collect();
        assert!(
            static_degradations.is_empty(),
            "{}: the oracle corpus is fully admitted; got {static_degradations:?}",
            primitive.id
        );
        assert_eq!(
            strict.base_frame(),
            best.base_frame(),
            "{}: admissions agree frame-for-frame",
            primitive.id
        );
    }
}

/// Where nothing degrades, best-effort IS strict: same Base frame, same
/// sampled frames, no degradations. The mode changes failure handling only.
#[test]
fn zero_degradation_best_effort_is_frame_identical_to_strict() {
    for source in [ADMITTED_STATIC, ADMITTED_ANIMATION] {
        let strict = SvgFrameSource::from_standalone_svg(source).expect("strict compile");
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source).expect("best-effort compile");
        assert!(best.degradations().is_empty(), "nothing to degrade");
        assert_eq!(strict.base_frame(), best.base_frame(), "Base frames equal");
        for nanoseconds in [0, 1_000_000_000, 3_000_000_000] {
            let time = SampleTime::from_nanoseconds(nanoseconds);
            assert_eq!(
                strict.sample_frame(time).expect("strict sample"),
                best.sample_frame(time).expect("best-effort sample"),
                "Sample({nanoseconds}ns) frames equal"
            );
        }
    }
}

/// Beyond-slice children are skipped by name with a stable structural path;
/// the admitted children still compile. Strict refuses the same source.
#[test]
fn beyond_slice_children_skip_by_name_and_admitted_children_render() {
    SvgFrameSource::from_standalone_svg(MIXED).expect_err("strict refuses the circle");

    let best = SvgFrameSource::from_standalone_svg_best_effort(MIXED).expect("best-effort");
    let skipped: Vec<(&str, &str)> = best
        .degradations()
        .iter()
        .map(|d| {
            assert_eq!(d.action(), DegradationAction::Skipped);
            (d.path(), d.reason())
        })
        .collect();
    assert_eq!(
        skipped,
        vec![
            ("svg/circle[1]", "unsupported element <circle>"),
            ("svg/path[1]", "unsupported element <path>"),
        ],
        "each skip names its construct at its stable path"
    );
    assert_eq!(
        best.base_frame().nodes.len(),
        2,
        "both admitted rects materialize; the skips leave holes, not guesses"
    );
}

/// The degradation set and the compiled frames are stable across repeated
/// requests: the skips are a property of the retained source.
#[test]
fn degradations_and_frames_are_deterministic() {
    let best = SvgFrameSource::from_standalone_svg_best_effort(MIXED).expect("best-effort");
    let first = best.base_frame();
    assert_eq!(first, best.base_frame(), "repeat Base compile");
    let time = SampleTime::from_nanoseconds(500_000_000);
    assert_eq!(
        best.sample_frame(time).expect("sample"),
        best.sample_frame(time).expect("repeat sample"),
        "repeat Sample compile (same skips, discarded sink)"
    );
    assert_eq!(best.degradations().len(), 2, "the set does not grow");
}

/// A dynamic surface outside the closed sampling inventory resolves every
/// sample request to the Base view, declared once as `SamplesAsBase`.
/// Strict refuses the same sample request.
#[test]
fn blocked_dynamic_surface_samples_as_base_and_declares_it() {
    let strict = SvgFrameSource::from_standalone_svg(UNSUPPORTED_ANIMATION).expect("strict Base");
    strict
        .sample_frame(SampleTime::ZERO)
        .expect_err("strict sampling refuses the y animation");

    let best = SvgFrameSource::from_standalone_svg_best_effort(UNSUPPORTED_ANIMATION)
        .expect("best-effort");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::SamplesAsBase)
        .collect();
    assert_eq!(declared.len(), 1, "declared exactly once");
    assert_eq!(declared[0].path(), "svg/rect[2]/animate[1]");
    assert!(
        declared[0].reason().contains("attributeName=\"x\""),
        "the reason names the admitted subset: {}",
        declared[0].reason()
    );
    let base = best.base_frame();
    for nanoseconds in [0, 1_000_000_000, 2_500_000_000] {
        let sampled = best
            .sample_frame(SampleTime::from_nanoseconds(nanoseconds))
            .expect("best-effort sampling never refuses a retained source");
        assert_eq!(
            sampled, base,
            "Sample({nanoseconds}ns) equals the Base view"
        );
    }
}

/// The inline-HTML entry's closed dynamic inventory is a blocked surface
/// like any other: best-effort sampling resolves to Base and declares it.
#[test]
fn inline_html_sampling_falls_back_to_base_under_best_effort() {
    let html = r##"<!doctype html><html><body>
      <svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
        <rect width="64" height="32" fill="#ffffff"/>
      </svg></body></html>"##;
    let strict = SvgFrameSource::from_html_inline_svg(html).expect("strict Base");
    strict
        .sample_frame(SampleTime::ZERO)
        .expect_err("strict inline-HTML sampling refuses");

    let best = SvgFrameSource::from_html_inline_svg_best_effort(html).expect("best-effort");
    assert_eq!(best.degradations().len(), 1);
    let declared = &best.degradations()[0];
    assert_eq!(declared.action(), DegradationAction::SamplesAsBase);
    assert!(
        declared.reason().contains("inline HTML"),
        "the reason names the entry: {}",
        declared.reason()
    );
    assert_eq!(
        best.sample_frame(SampleTime::ZERO).expect("falls back"),
        best.base_frame()
    );
}

/// Rendering-relevant attributes the slice does not consume never paint
/// wrong pixels: an admitted-element candidate carrying one refuses under
/// strict and skips-and-declares under best-effort. Attributes outside the
/// SVG rendering vocabulary stay ignored, exactly as Chromium ignores them.
#[test]
fn unconsumed_rendering_attributes_never_paint_wrong_pixels() {
    for (label, rect_attrs, named) in [
        ("element opacity", r#"opacity="0.5""#, "opacity"),
        ("transform", r#"transform="translate(20,0)""#, "transform"),
        ("rounded corners", r#"rx="8""#, "rx"),
        (
            "stroke paint",
            r##"stroke="#0000ff" stroke-width="8""##,
            "stroke",
        ),
        (
            "conditional processing",
            r#"systemLanguage="fr""#,
            "systemLanguage",
        ),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="16" fill="#ff0000" {rect_attrs}/></svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(source.as_str())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains(named),
            "{label}: strict names the attribute; got {strict}"
        );
        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        assert_eq!(
            best.base_frame().nodes.len(),
            0,
            "{label}: the rect is a declared hole, not a wrong paint"
        );
        assert_eq!(best.degradations().len(), 1, "{label}");
        assert_eq!(best.degradations()[0].path(), "svg/rect[1]", "{label}");
        assert!(
            best.degradations()[0].reason().contains(named),
            "{label}: the skip names the attribute; got {}",
            best.degradations()[0].reason()
        );
    }

    // Unknown attributes are not rendering-relevant: ignored in both modes
    // (the standalone entry law pins the case-folded `viewbox` form; this
    // pins the same shape for an arbitrary foreign name).
    let unknown = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="16" fill="#ff0000" data-name="hero"/></svg>"##;
    let strict = SvgFrameSource::from_standalone_svg(unknown).expect("strict ignores unknown");
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(unknown).expect("best-effort ignores");
    assert!(best.degradations().is_empty());
    assert_eq!(strict.base_frame(), best.base_frame());
    assert_eq!(strict.base_frame().nodes.len(), 1, "the rect paints");
}

/// Values a stylesheet or `style` attribute could smuggle past the
/// attribute patrol are patrolled at the computed level: `opacity`,
/// `display: none`, `visibility`, and shape `stroke` refuse or skip by
/// name instead of painting wrong pixels.
#[test]
fn stylesheet_smuggled_values_are_patrolled_at_the_computed_level() {
    for (label, css, named) in [
        ("stylesheet stroke", "rect { stroke: #0000ff }", "stroke"),
        ("stylesheet opacity", "rect { opacity: 0.5 }", "opacity"),
        ("stylesheet display", "rect { display: none }", "display"),
        (
            "stylesheet visibility",
            "rect { visibility: hidden }",
            "visibility",
        ),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><style>{css}</style><rect width="16" height="16" fill="#ff0000"/></svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(source.as_str())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains(named),
            "{label}: strict names the property; got {strict}"
        );
        let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        assert_eq!(best.base_frame().nodes.len(), 0, "{label}: declared hole");
        assert!(
            best.degradations()[0].reason().contains(named),
            "{label}: named; got {}",
            best.degradations()[0].reason()
        );
    }

    // A style-attribute smuggle is the same surface.
    let styled = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="16" fill="#ff0000" style="opacity: 0.25"/></svg>"##;
    SvgFrameSource::from_standalone_svg(styled).expect_err("strict refuses style opacity");
    let best = SvgFrameSource::from_standalone_svg_best_effort(styled).expect("best-effort");
    assert_eq!(best.base_frame().nodes.len(), 0);
    assert!(best.degradations()[0].reason().contains("opacity"));
}

/// Every beyond-inventory dynamic construct is declared — not just the
/// first: multiple blockers each get their own `SamplesAsBase` entry, and
/// they follow the skips (which stay in document order).
#[test]
fn every_dynamic_blocker_is_declared_and_ordering_holds() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32">
  <circle cx="8" cy="8" r="4" fill="#16a34a"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000" onclick="window.a = 1"/>
  <rect x="20" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="y" from="8" to="16" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;
    let best = SvgFrameSource::from_standalone_svg_best_effort(source).expect("best-effort");
    let entries: Vec<(DegradationAction, &str)> = best
        .degradations()
        .iter()
        .map(|d| (d.action(), d.path()))
        .collect();
    assert_eq!(
        entries,
        vec![
            (DegradationAction::Skipped, "svg/circle[1]"),
            (DegradationAction::SamplesAsBase, "svg/rect[1]"),
            (DegradationAction::SamplesAsBase, "svg/rect[2]/animate[1]"),
        ],
        "skips first in document order, then every dynamic blocker"
    );
    assert_eq!(
        best.sample_frame(SampleTime::from_nanoseconds(1_000_000_000))
            .expect("samples as base"),
        best.base_frame()
    );
}

/// Skips and an ADMITTED animation compose: the sampled frame moves while
/// the skipped element stays a declared hole.
#[test]
fn admitted_animation_samples_through_declared_skips() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32">
  <circle cx="8" cy="8" r="4" fill="#16a34a"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;
    let best = SvgFrameSource::from_standalone_svg_best_effort(source).expect("best-effort");
    assert_eq!(best.degradations().len(), 1, "only the circle degrades");
    assert_eq!(best.degradations()[0].action(), DegradationAction::Skipped);
    let base = best.base_frame();
    let sampled = best
        .sample_frame(SampleTime::from_nanoseconds(1_000_000_000))
        .expect("the admitted x animation samples");
    assert_ne!(sampled, base, "time moves the admitted rect");
    assert_eq!(sampled.nodes.len(), base.nodes.len(), "same declared holes");
}

/// `<script>` inside the compiled inline SVG refuses in both admissions at
/// any nesting depth — a load-time script can rewrite the authored state
/// the Base view renders. Scripts elsewhere on the page stay under the
/// pinned first-SVG-only entry contract: Base renders.
#[test]
fn script_inside_the_compiled_inline_svg_refuses_in_both_admissions() {
    for (label, html) in [
        (
            "direct child",
            r##"<html><body><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><script>1</script><rect width="8" height="8" fill="#ff0000"/></svg></body></html>"##,
        ),
        (
            "nested inside an admitted rect",
            r##"<html><body><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="#ff0000"><script>document.querySelector('rect').setAttribute('fill','#0000ff')</script></rect></svg></body></html>"##,
        ),
    ] {
        for (mode, result) in [
            ("strict", SvgFrameSource::from_html_inline_svg(html)),
            (
                "best-effort",
                SvgFrameSource::from_html_inline_svg_best_effort(html),
            ),
        ] {
            let error = result
                .err()
                .unwrap_or_else(|| panic!("{label} ({mode}): script in the compiled SVG refuses"));
            assert!(
                error.to_string().contains("<script>"),
                "{label} ({mode}): names the construct; got {error}"
            );
        }
    }

    // A page script outside the first SVG is the pinned entry contract's
    // territory: Base still renders.
    let page_script = r##"<html><head><script>1</script></head><body><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="#ff0000"/></svg></body></html>"##;
    let best = SvgFrameSource::from_html_inline_svg_best_effort(page_script).expect(
        "page script is
outside the compiled subtree",
    );
    assert_eq!(best.base_frame().nodes.len(), 1, "the rect renders");
}

/// Document-level contracts refuse identically in both modes: best-effort
/// degrades subtree content, it never invents the canvas.
#[test]
fn document_level_contracts_refuse_in_both_modes() {
    for (label, source) in [
        (
            "missing width (viewBox-only sizing)",
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 32"><rect width="64" height="32" fill="#ffffff"/></svg>"##,
        ),
        ("no svg root", r##"<x xmlns="urn:none"/>"##),
        (
            "malformed XML (recorded recovery: mismatched close tag)",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="4" height="4"></svg>"##,
        ),
    ] {
        let strict = SvgFrameSource::from_standalone_svg(source)
            .expect_err(&format!("strict refuses: {label}"));
        let best = SvgFrameSource::from_standalone_svg_best_effort(source)
            .expect_err(&format!("best-effort refuses: {label}"));
        assert_eq!(
            strict, best,
            "{label}: one identical document-level refusal"
        );
    }
}
