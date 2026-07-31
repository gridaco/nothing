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
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

const ADMITTED_STATIC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000"/>
</svg>"##;

const MIXED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <rect width="64" height="64" fill="#ffffff"/>
  <circle cx="32" cy="32" r="16" fill="#16a34a"/>
  <rect x="4" y="4" width="8" height="8" fill="#000000"/>
  <text x="4" y="60" fill="#000000">hi</text>
</svg>"##;

const ADMITTED_ANIMATION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;

const LOAD_ACTIVE_ANIMATION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="y" from="8" to="16" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;

const LOAD_ACTIVE_SET: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#0000ff">
    <set attributeName="fill" to="#ff0000"/>
  </rect>
</svg>"##;

const DYNAMIC_SIDE_CHANNEL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect width="64" height="32" fill="#ffffff"/>
  <rect x="4" y="8" width="8" height="16" fill="#000000" onclick="window.a = 1"/>
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
    width: i32,
    height: i32,
}

/// The host-established initial viewport for this file's inline sources —
/// inert: every inline source here authors explicit root dimensions or
/// refuses before sizing resolves. The oracle-corpus law builds per-fixture
/// viewports from the declared dimensions instead.
fn host_viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
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
        let viewport = InitialViewport::new(primitive.width as f32, primitive.height as f32);
        let (strict, best) = match primitive.entry.as_str() {
            "standalone-svg" => (
                SvgFrameSource::from_standalone_svg(source.as_str(), viewport),
                SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport),
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
        let strict =
            SvgFrameSource::from_standalone_svg(source, host_viewport()).expect("strict compile");
        let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
            .expect("best-effort compile");
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
    SvgFrameSource::from_standalone_svg(MIXED, host_viewport())
        .expect_err("strict refuses the text");

    let best = SvgFrameSource::from_standalone_svg_best_effort(MIXED, host_viewport())
        .expect("best-effort");
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
        vec![("svg/text[1]", "unsupported element <text>")],
        "each skip names its construct at its stable path"
    );
    assert_eq!(
        best.base_frame().nodes.len(),
        3,
        "the admitted rects and the circle materialize; the skip leaves a hole, not a guess"
    );
}

/// The degradation set and the compiled frames are stable across repeated
/// requests: the skips are a property of the retained source.
#[test]
fn degradations_and_frames_are_deterministic() {
    let best = SvgFrameSource::from_standalone_svg_best_effort(MIXED, host_viewport())
        .expect("best-effort");
    let first = best.base_frame();
    assert_eq!(first, best.base_frame(), "repeat Base compile");
    let time = SampleTime::from_nanoseconds(500_000_000);
    assert_eq!(
        best.sample_frame(time).expect("sample"),
        best.sample_frame(time).expect("repeat sample"),
        "repeat Sample compile (same skips, discarded sink)"
    );
    assert_eq!(best.degradations().len(), 1, "the set does not grow");
}

/// A dynamic surface outside the closed sampling inventory — one that
/// leaves the Base view honest, here an event handler — resolves every
/// sample request to the Base view, declared once as `SamplesAsBase`.
/// Strict compiles the same Base and refuses the same sample request.
#[test]
fn blocked_dynamic_surface_samples_as_base_and_declares_it() {
    let strict = SvgFrameSource::from_standalone_svg(DYNAMIC_SIDE_CHANNEL, host_viewport())
        .expect("strict Base: an event handler paints nothing at load");
    strict
        .sample_frame(SampleTime::ZERO)
        .expect_err("strict sampling refuses the event handler");

    let best =
        SvgFrameSource::from_standalone_svg_best_effort(DYNAMIC_SIDE_CHANNEL, host_viewport())
            .expect("best-effort");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::SamplesAsBase)
        .collect();
    assert_eq!(declared.len(), 1, "declared exactly once");
    assert_eq!(declared[0].path(), "svg/rect[2]");
    assert!(
        declared[0].reason().contains("event-handler"),
        "the reason names the surface: {}",
        declared[0].reason()
    );
    let base = best.base_frame();
    assert_eq!(base.nodes.len(), 2, "Base stays honest: both rects render");
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

/// A beyond-inventory animation element is active at document load (SMIL
/// defaults `begin` to offset 0s): Chromium paints the overridden value, so
/// the target's authored state never renders. Strict refuses at
/// construction — not at sample time — and best-effort skips the target in
/// every view, declared at the target's stable path with the animation
/// element named. This is the law the recorded SMIL hole lacked: a Base
/// render used to paint the authored state with exit 0 and no declaration.
#[test]
fn load_active_animation_never_renders_its_targets_authored_state() {
    for (label, source, named) in [
        (
            "animate beyond the admitted attribute",
            LOAD_ACTIVE_ANIMATION,
            "attributeName=\"x\"",
        ),
        ("set on a consumed attribute", LOAD_ACTIVE_SET, "<set>"),
    ] {
        let strict = SvgFrameSource::from_standalone_svg(source, host_viewport())
            .expect_err(&format!("{label}: strict refuses at construction"));
        assert!(
            matches!(strict, websem::CompileError::UnsupportedAnimation(_)),
            "{label}: expected UnsupportedAnimation, got {strict}"
        );
        assert!(
            strict.to_string().contains(named) && strict.to_string().contains("document load"),
            "{label}: the refusal names the construct and the load-time law; got {strict}"
        );

        let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        let base = best.base_frame();
        assert_eq!(
            base.nodes.len(),
            1,
            "{label}: the target is a declared hole; the backdrop still renders"
        );
        assert_eq!(best.degradations().len(), 1, "{label}");
        assert_eq!(best.degradations()[0].action(), DegradationAction::Skipped);
        assert_eq!(
            best.degradations()[0].path(),
            "svg/rect[2]",
            "{label}: declared at the target's path"
        );
        assert!(
            best.degradations()[0].reason().contains("document load")
                && best.degradations()[0].reason().contains("svg/rect[2]/"),
            "{label}: the reason names the law and the animation element; got {}",
            best.degradations()[0].reason()
        );
        let sampled = best
            .sample_frame(SampleTime::from_nanoseconds(1_000_000_000))
            .expect("best-effort sampling never refuses a retained source");
        assert_eq!(
            sampled, base,
            "{label}: the skip is a property of the source — every view shares it"
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
        ("rounded corners", r#"rx="8""#, "rx"),
        (
            "transform-origin",
            r#"transform-origin="center""#,
            "transform-origin",
        ),
        (
            // The strokes rung consumed the stroke paint and its geometry; what
            // remains unconsumed is the compositing and dashing half.
            "stroke opacity",
            r##"stroke="#0000ff" stroke-width="8" stroke-opacity="0.5""##,
            "stroke-opacity",
        ),
        (
            "stroke dashing",
            r##"stroke="#0000ff" stroke-width="8" stroke-dasharray="4 4""##,
            "stroke-dasharray",
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
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), host_viewport())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains(named),
            "{label}: strict names the attribute; got {strict}"
        );
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), host_viewport())
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
    let strict = SvgFrameSource::from_standalone_svg(unknown, host_viewport())
        .expect("strict ignores unknown");
    let best = SvgFrameSource::from_standalone_svg_best_effort(unknown, host_viewport())
        .expect("best-effort ignores");
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
        (
            "stylesheet stroke opacity",
            "rect { stroke: #0000ff; stroke-opacity: 0.5 }",
            "stroke-opacity",
        ),
        ("stylesheet opacity", "rect { opacity: 0.5 }", "opacity"),
        (
            "stylesheet display: contents",
            "rect { display: contents }",
            "display: contents",
        ),
        // SVG2 geometry properties: a CSS width/height beats the authored
        // attribute in Chromium, and the compiler reads attributes only.
        ("stylesheet width", "rect { width: 10px }", "width"),
        ("stylesheet height", "rect { height: 10px }", "height"),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><style>{css}</style><rect width="16" height="16" fill="#ff0000"/></svg>"##
        );
        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), host_viewport())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains(named),
            "{label}: strict names the property; got {strict}"
        );
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), host_viewport())
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
    SvgFrameSource::from_standalone_svg(styled, host_viewport())
        .expect_err("strict refuses style opacity");
    let best = SvgFrameSource::from_standalone_svg_best_effort(styled, host_viewport())
        .expect("best-effort");
    assert_eq!(best.base_frame().nodes.len(), 0);
    assert!(best.degradations()[0].reason().contains("opacity"));
}

/// A `stroke-width` whose basis this build lacks departs by name from every
/// ingress, and the sheet is the ingress that used to paint quietly.
///
/// The presentation attribute and the `style` attribute are attributable, so
/// they skip the element. A sheet is not — attributing a rule needs selector
/// matching — so it is document-level: strict refuses, best-effort declares
/// once against the sheet and renders. What best-effort renders is the wrong
/// width (`2ex` resolves to 16 units against placeholder font metrics), which
/// is precisely why silence was not an option.
#[test]
fn a_basis_less_stroke_width_departs_by_name_from_every_ingress() {
    let sheet = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><style>rect { stroke-width: 2ex }</style><rect width="16" height="16" fill="none" stroke="#0000ff"/></svg>"##;
    let attribute = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="16" fill="none" stroke="#0000ff" stroke-width="2ex"/></svg>"##;
    let style_attribute = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="16" fill="none" stroke="#0000ff" style="stroke-width: 2ex"/></svg>"##;

    for (label, source) in [
        ("sheet", sheet),
        ("attribute", attribute),
        ("style attribute", style_attribute),
    ] {
        let strict = SvgFrameSource::from_standalone_svg(source, host_viewport())
            .expect_err(&format!("{label}: strict refuses"));
        assert!(
            strict.to_string().contains("basis"),
            "{label}: strict names the missing basis; got {strict}"
        );
        let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
            .unwrap_or_else(|error| panic!("{label}: best-effort compiles: {error}"));
        let declared: Vec<&websem::Degradation> = best
            .degradations()
            .iter()
            .filter(|d| d.action() != DegradationAction::SamplesAsBase)
            .collect();
        assert_eq!(declared.len(), 1, "{label}: exactly one departure");
        assert!(
            declared[0].reason().contains("basis"),
            "{label}: named; got {}",
            declared[0].reason()
        );
    }

    // The two admissions differ in what they do, not in whether they notice.
    // A sheet leaves the shape in the frame and says so against the sheet; an
    // attribute takes the shape out and says so against the shape.
    let from_sheet =
        SvgFrameSource::from_standalone_svg_best_effort(sheet, host_viewport()).expect("sheet");
    assert_eq!(
        from_sheet.base_frame().nodes.len(),
        1,
        "the rect still paints"
    );
    assert_eq!(
        from_sheet.degradations()[0].action(),
        DegradationAction::DeclarationIgnored
    );
    assert_eq!(from_sheet.degradations()[0].path(), "svg/style[1]");

    let from_attribute =
        SvgFrameSource::from_standalone_svg_best_effort(attribute, host_viewport())
            .expect("attribute");
    assert_eq!(from_attribute.base_frame().nodes.len(), 0, "declared hole");
    assert_eq!(
        from_attribute.degradations()[0].action(),
        DegradationAction::Skipped
    );
    assert_eq!(from_attribute.degradations()[0].path(), "svg/rect[1]");
}

/// Every beyond-inventory dynamic construct is declared — not just the
/// first. Skips stay in document order, with a load-active animation's
/// target among them; the sampling-only blockers follow as `SamplesAsBase`
/// entries.
#[test]
fn every_dynamic_blocker_is_declared_and_ordering_holds() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32">
  <text x="4" y="60" fill="#16a34a">hi</text>
  <rect x="4" y="8" width="8" height="16" fill="#000000" onclick="window.a = 1"/>
  <rect x="20" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="y" from="8" to="16" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
        .expect("best-effort");
    let entries: Vec<(DegradationAction, &str)> = best
        .degradations()
        .iter()
        .map(|d| (d.action(), d.path()))
        .collect();
    assert_eq!(
        entries,
        vec![
            (DegradationAction::Skipped, "svg/text[1]"),
            (DegradationAction::Skipped, "svg/rect[2]"),
            (DegradationAction::SamplesAsBase, "svg/rect[1]"),
        ],
        "skips first in document order — the load-active animation's target \
         among them — then every sampling-only blocker"
    );
    assert_eq!(
        best.base_frame().nodes.len(),
        1,
        "the onclick rect renders (Base-honest); the text and the \
         overridden rect are declared holes"
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
  <text x="4" y="60" fill="#16a34a">hi</text>
  <rect x="4" y="8" width="8" height="16" fill="#000000">
    <animate attributeName="x" from="20" to="44" dur="2s" fill="freeze"/>
  </rect>
</svg>"##;
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
        .expect("best-effort");
    assert_eq!(best.degradations().len(), 1, "only the text degrades");
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
/// degrades subtree content, it never invents the canvas. (A missing
/// standalone root width/height is no longer on this list — it is `auto`
/// and resolves against the host's initial viewport; the viewport laws pin
/// that admission.)
#[test]
fn document_level_contracts_refuse_in_both_modes() {
    for (label, source) in [
        (
            "percentage root sizing",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="50%" height="32" viewBox="0 0 64 32"><rect width="64" height="32" fill="#ffffff"/></svg>"##,
        ),
        (
            "malformed preserveAspectRatio",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32" preserveAspectRatio="xMidYMiddle meet"><rect width="64" height="32" fill="#ffffff"/></svg>"##,
        ),
        ("no svg root", r##"<x xmlns="urn:none"/>"##),
        (
            "malformed XML (recorded recovery: mismatched close tag)",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="4" height="4"></svg>"##,
        ),
        (
            // The override reaches the whole canvas: no per-element hole
            // can express it, so it refuses like <script> does.
            "load-active animation targeting the root <svg>",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><set attributeName="fill" to="#ff0000"/><rect width="4" height="4" fill="#0000ff"/></svg>"##,
        ),
        (
            // href retargets by id, which this slice cannot resolve, so the
            // override cannot be attributed to one skippable element.
            "load-active animation retargeting through href",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect id="hero" width="4" height="4" fill="#0000ff"/><rect x="4" width="4" height="4" fill="#000000"><set href="#hero" attributeName="fill" to="#ff0000"/></rect></svg>"##,
        ),
    ] {
        let strict = SvgFrameSource::from_standalone_svg(source, host_viewport())
            .expect_err(&format!("strict refuses: {label}"));
        let best = SvgFrameSource::from_standalone_svg_best_effort(source, host_viewport())
            .expect_err(&format!("best-effort refuses: {label}"));
        assert_eq!(
            strict, best,
            "{label}: one identical document-level refusal"
        );
    }

    // The inline HTML entry has no initial-viewport semantics until CSS
    // replaced-element sizing lands, so a missing dimension there stays a
    // named document-level refusal in both modes.
    let html = r##"<html><body><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 32"><rect width="64" height="32" fill="#ffffff"/></svg></body></html>"##;
    let strict = SvgFrameSource::from_html_inline_svg(html)
        .expect_err("strict refuses inline-HTML auto sizing");
    let best = SvgFrameSource::from_html_inline_svg_best_effort(html)
        .expect_err("best-effort refuses inline-HTML auto sizing");
    assert_eq!(strict, best, "one identical inline-HTML sizing refusal");
    assert!(
        strict.to_string().contains("missing width"),
        "the refusal names the missing dimension; got {strict}"
    );
}
