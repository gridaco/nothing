//! The visibility contract: what `display: none` and `visibility` mean.
//!
//! The rung converts two over-refusals into the correct nothing. Both
//! properties are admitted presentation hints (csscascade) read as computed
//! values, so the attribute and every CSS spelling resolve identically and
//! an author rule beats the attribute (measured — the un-hide cell is
//! Chromium-baked). The split is semantic and measured: `display: none`
//! generates no box, so the subtree is pruned and a `visibility: visible`
//! descendant stays gone; `visibility: hidden | collapse` turns off one
//! element's own paint, inherits, and a descendant whose computed value is
//! `visible` un-hides itself. Neither is a hole: nothing is declared,
//! because Chromium also paints nothing — the admitted nothing of `r="0"`,
//! restated for the visibility pair. `display: contents` stays a refusal:
//! it paints children in the parent's place, which the flattened walk
//! cannot express without lying about a transform.

// This binary consumes only the compiler half of the shared plumbing.
#[allow(dead_code)]
mod support;

use websem::{InitialViewport, SvgFrameSource};

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

/// Strict and best-effort agree and declare no *static* degradation — the
/// visibility pair is admitted, not degraded. (A `style` attribute or
/// `<style>` sheet still declares its sampling-only blocker, which leaves
/// Base honest; that surface is the sampling inventory's, not this rung's.)
fn admit_both(source: &str) -> rframe::Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best-effort admits");
    let static_degradations: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() != websem::DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        static_degradations.is_empty(),
        "an admitted nothing declares nothing static: {static_degradations:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

/// `display: none` on a shape removes exactly that shape; siblings render.
/// Every spelling — attribute, style attribute, stylesheet — resolves
/// through the one cascade to the same frame.
#[test]
fn display_none_removes_the_shape_in_every_spelling() {
    let attribute = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" display="none"/>
  <rect x="40" y="8" width="16" height="16" fill="#2563eb"/>"##,
    ));
    assert_eq!(attribute.nodes().len(), 1, "the sibling alone materializes");

    let style_attribute = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" style="display: none"/>
  <rect x="40" y="8" width="16" height="16" fill="#2563eb"/>"##,
    ));
    assert_eq!(attribute, style_attribute, "one cascade, one meaning");

    // The stylesheet spelling declares its sampling blocker (the CSS
    // animation inventory) but the Base frame is identical.
    let sheet = SvgFrameSource::from_standalone_svg(
        document(
            r##"  <style>.gone { display: none }</style>
  <rect class="gone" x="8" y="8" width="24" height="24" fill="#16a34a"/>
  <rect x="40" y="8" width="16" height="16" fill="#2563eb"/>"##,
        ),
        viewport(),
    )
    .expect("strict admits the sheet at Base")
    .base_frame();
    assert_eq!(
        attribute.nodes(),
        sheet.nodes(),
        "the sheet spelling agrees"
    );
}

/// `display: none` on a container prunes the subtree: a
/// `visibility: visible` descendant stays gone (measured — its cell is
/// Chromium-baked), because there is no box to un-hide into.
#[test]
fn display_none_prunes_the_subtree_past_visible_descendants() {
    let frame = admit_both(&document(
        r##"  <g display="none"><rect x="8" y="8" width="24" height="24" fill="#16a34a" visibility="visible"/></g>
  <rect x="40" y="40" width="16" height="16" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        frame.nodes().len(),
        1,
        "the pruned subtree contributes nothing"
    );
}

/// `visibility: hidden` and `collapse` (identical for shapes — measured)
/// turn off one element's own paint; `visibility` inherits, and a
/// descendant whose computed value is `visible` un-hides itself while its
/// sibling stays inherited-hidden.
#[test]
fn visibility_hides_per_element_and_a_visible_descendant_unhides() {
    for value in ["hidden", "collapse"] {
        let frame = admit_both(&document(&format!(
            r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" visibility="{value}"/>
  <rect x="40" y="8" width="16" height="16" fill="#2563eb"/>"##
        )));
        assert_eq!(
            frame.nodes().len(),
            1,
            "visibility {value}: sibling renders"
        );
    }

    let unhide = admit_both(&document(
        r##"  <g visibility="hidden">
    <rect x="8" y="8" width="24" height="24" fill="#16a34a" visibility="visible"/>
    <rect x="40" y="8" width="16" height="16" fill="#2563eb"/>
  </g>"##,
    ));
    assert_eq!(
        unhide.nodes().len(),
        1,
        "the visible descendant un-hides; its sibling inherits hidden"
    );
    assert_eq!(
        unhide.nodes()[0].bounds,
        math2::Rectangle::from_xywh(8.0, 8.0, 24.0, 24.0),
        "and it is the un-hidden rect that materialized"
    );
}

/// An author rule beats the attribute spelling: a stylesheet
/// `visibility: visible` un-hides `visibility="hidden"` (measured — the
/// presentation hint enters below every author rule).
#[test]
fn an_author_rule_beats_the_visibility_attribute() {
    let source = document(
        r##"  <style>.show { visibility: visible }</style>
  <rect class="show" x="8" y="8" width="24" height="24" fill="#16a34a" visibility="hidden"/>"##,
    );
    let frame = SvgFrameSource::from_standalone_svg(source, viewport())
        .expect("strict admits")
        .base_frame();
    assert_eq!(frame.nodes().len(), 1, "the rule un-hides the rect");
}

/// A root `display: none` splits by entry, and both halves are measured:
/// a *standalone* document's outermost `<svg>` ignores it and paints
/// normally (the Chromium-baked `svg-display-none-root` cell — the oracle
/// corrected an embedded-context probe here), while an *embedded*
/// inline-HTML root generates no box, so its compiled subtree is the
/// empty canvas.
#[test]
fn a_root_display_none_splits_by_entry() {
    let standalone = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" display="none"><rect width="64" height="64" fill="#16a34a"/></svg>"##;
    let frame = admit_both(standalone);
    assert_eq!(
        frame.nodes().len(),
        1,
        "the standalone outermost svg ignores display: none (measured)"
    );

    let html = r##"<html><body><svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" display="none"><rect width="64" height="64" fill="#16a34a"/></svg></body></html>"##;
    let inline = websem::compile_html_inline_svg(html).expect("the inline entry admits");
    assert_eq!(
        inline.nodes().len(),
        0,
        "an embedded root generates no box (measured)"
    );
    assert_eq!(
        inline.bounds,
        math2::Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        "the canvas contract is sizing's, not visibility's"
    );
}

/// A hidden element's *other* unconsumed cascaded properties stay silent:
/// Chromium paints nothing for it regardless, so a refusal there would
/// turn a correct nothing into a false alarm. The same property on a
/// *rendering* element still refuses.
#[test]
fn hidden_elements_do_not_false_alarm_on_unconsumed_properties() {
    let hidden = admit_both(&document(
        r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" stroke="#000000" visibility="hidden" style="stroke-dasharray: 4 4"/>"##,
    ));
    assert_eq!(
        hidden.nodes().len(),
        0,
        "hidden: the property reaches nothing"
    );

    SvgFrameSource::from_standalone_svg(
        document(
            r##"  <rect x="8" y="8" width="24" height="24" fill="#16a34a" stroke="#000000" style="stroke-dasharray: 4 4"/>"##,
        ),
        viewport(),
    )
    .expect_err("rendering: the same property still refuses");
}

/// `display: contents` stays a refusal by name: Chromium paints the
/// children in the parent's place, which the flattened walk cannot express
/// without dropping the element's transform silently.
#[test]
fn display_contents_stays_a_named_refusal() {
    let source = document(
        r##"  <g transform="translate(8,8)" display="contents"><rect width="24" height="24" fill="#16a34a"/></g>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("strict refuses");
    assert!(
        strict.to_string().contains("display: contents"),
        "named; got {strict}"
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best-effort declares");
    assert_eq!(best.base_frame().nodes().len(), 0, "a declared hole");
    assert!(
        best.degradations()[0]
            .reason()
            .contains("display: contents"),
        "named; got {}",
        best.degradations()[0].reason()
    );
}
