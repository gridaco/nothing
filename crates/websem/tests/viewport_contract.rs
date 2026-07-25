//! The root viewport contract: what the one SVG compiler admits and maps,
//! and what refuses by name — in both admissions, because the outer
//! viewport is document-level (best-effort never invents the canvas).
//!
//! These laws pin the resolved contract — frame bounds and viewport
//! transforms — and every refusal shape. Of the admitted mappings, the
//! uniform equal-aspect cell is Chromium-baked today
//! (`reftest_oracle.rs`, `svg-viewbox-uniform-offset-rect`); the
//! unequal-aspect, `preserveAspectRatio`, and auto-sized cells gain their
//! committed bakes with the corpus step of this rung.

// This binary consumes only the n0 render half of the shared plumbing.
#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use support::render_through_n0;
use websem::{CompileError, InitialViewport, SvgFrameSource, compile_standalone_svg};

const INVALID_TOKEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/unsupported/svg-viewbox-invalid-token.svg"
));
const REPEATED_COMMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/unsupported/svg-viewbox-repeated-comma.svg"
));
const TRAILING_COMMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/unsupported/svg-viewbox-trailing-comma.svg"
));
const UNEQUAL_DEFAULT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/unsupported/svg-viewbox-unequal-default.svg"
));
const EXPLICIT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/unsupported/svg-preserve-aspect-ratio-explicit.svg"
));

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

/// Both admissions on one source; the outer viewport contract must treat
/// them identically, so every law here goes through this pair.
fn both_admissions(
    source: &str,
    viewport: InitialViewport,
) -> [(&'static str, Result<SvgFrameSource, CompileError>); 2] {
    [
        (
            "strict",
            SvgFrameSource::from_standalone_svg(source, viewport),
        ),
        (
            "best-effort",
            SvgFrameSource::from_standalone_svg_best_effort(source, viewport),
        ),
    ]
}

#[test]
fn malformed_viewbox_token_is_not_silently_discarded() {
    assert!(matches!(
        compile_standalone_svg(INVALID_TOKEN, viewport(64.0, 64.0)),
        Err(CompileError::BadViewBox(_))
    ));
}

#[test]
fn repeated_viewbox_comma_is_not_silently_filtered() {
    assert!(matches!(
        compile_standalone_svg(REPEATED_COMMA, viewport(64.0, 64.0)),
        Err(CompileError::BadViewBox(_))
    ));
}

#[test]
fn trailing_viewbox_comma_is_not_silently_filtered() {
    assert!(matches!(
        compile_standalone_svg(TRAILING_COMMA, viewport(64.0, 64.0)),
        Err(CompileError::BadViewBox(_))
    ));
}

/// An unequal-aspect viewBox maps under the default `xMidYMid meet`:
/// uniform scale, centered — a letterbox, never a silent stretch. (The
/// pre-rung refusal law this replaces lived here as
/// `unequal_aspect_viewbox_is_rejected_until_default_mapping_exists`.)
#[test]
fn unequal_aspect_viewbox_maps_under_the_default_meet() {
    let frame = compile_standalone_svg(UNEQUAL_DEFAULT, viewport(100.0, 50.0))
        .expect("the default meet mapping is admitted");
    assert_eq!(frame.bounds, Rectangle::from_xywh(0.0, 0.0, 100.0, 50.0));
    // viewBox 0 0 100 100 into 100x50: s = min(1.0, 0.5) = 0.5,
    // dx = (100 - 100*0.5) / 2 = 25, dy = (50 - 100*0.5) / 2 = 0.
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(0.5, 0.0, 25.0, 0.0, 0.5, 0.0)
    );
}

/// An explicit `preserveAspectRatio` parses and admits. This fixture's
/// equal-aspect `none` maps identically to the default — the admission is
/// the law here; the distinct mappings are pinned in the table below.
#[test]
fn explicit_preserve_aspect_ratio_is_admitted() {
    let frame = compile_standalone_svg(EXPLICIT, viewport(64.0, 64.0))
        .expect("preserveAspectRatio=\"none\" is admitted");
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
    );
}

/// A viewBox-only document (no width/height) sizes to the host's initial
/// viewport — `auto` resolves to 100% of it (SVG2 §8.2) — identically in
/// both admissions, with nothing degraded: sizing is the document contract,
/// not a capability edge.
#[test]
fn viewbox_only_documents_size_to_the_initial_viewport() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 16"><rect width="32" height="16" fill="#16a34a"/></svg>"##;
    let [(_, strict), (_, best)] = both_admissions(source, viewport(64.0, 32.0));
    let strict = strict.expect("strict admits viewBox-only sizing");
    let best = best.expect("best-effort admits viewBox-only sizing");
    assert!(
        best.degradations().is_empty(),
        "sizing is not a degradation"
    );
    assert_eq!(strict.base_frame(), best.base_frame());

    let frame = strict.base_frame();
    assert_eq!(frame.bounds, Rectangle::from_xywh(0.0, 0.0, 64.0, 32.0));
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 2.0, 0.0)
    );

    // A different window, a different mapping — the same document.
    let resized = compile_standalone_svg(source, viewport(128.0, 64.0))
        .expect("the initial viewport scales the mapping");
    assert_eq!(resized.bounds, Rectangle::from_xywh(0.0, 0.0, 128.0, 64.0));
    assert_eq!(
        resized.nodes[0].transform,
        AffineTransform::from_acebdf(4.0, 0.0, 0.0, 0.0, 4.0, 0.0)
    );
}

/// An authored `width="auto"` is literally the absent-attribute value —
/// SVG2 makes these geometry properties, and `auto` (ASCII
/// case-insensitive) is their initial keyword — so the two spell one
/// document.
#[test]
fn authored_auto_dimension_equals_the_absent_dimension() {
    let absent = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 16"><rect width="32" height="16" fill="#16a34a"/></svg>"##;
    let authored = r##"<svg xmlns="http://www.w3.org/2000/svg" width="auto" height="AUTO" viewBox="0 0 32 16"><rect width="32" height="16" fill="#16a34a"/></svg>"##;
    assert_eq!(
        compile_standalone_svg(authored, viewport(64.0, 32.0)).expect("authored auto"),
        compile_standalone_svg(absent, viewport(64.0, 32.0)).expect("absent twin")
    );
}

/// Authored dimensions always win over the initial viewport, and each
/// dimension resolves independently: an authored width with an auto height
/// takes only the height from the host.
#[test]
fn authored_dimensions_win_and_resolve_independently() {
    let explicit = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16"><rect width="8" height="8" fill="#16a34a"/></svg>"##;
    let frame =
        compile_standalone_svg(explicit, viewport(640.0, 480.0)).expect("explicit dimensions");
    assert_eq!(frame.bounds, Rectangle::from_xywh(0.0, 0.0, 32.0, 16.0));

    let mixed = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32"><rect width="8" height="8" fill="#16a34a"/></svg>"##;
    let frame = compile_standalone_svg(mixed, viewport(64.0, 48.0)).expect("mixed sizing");
    assert_eq!(frame.bounds, Rectangle::from_xywh(0.0, 0.0, 32.0, 48.0));
}

/// A valid `preserveAspectRatio` without a viewBox is inert, as in
/// Chromium: there is no viewBox mapping for it to shape.
#[test]
fn valid_preserve_aspect_ratio_without_viewbox_is_inert() {
    let plain = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16"><rect width="8" height="8" fill="#16a34a"/></svg>"##;
    let with_par = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16" preserveAspectRatio="xMaxYMax slice"><rect width="8" height="8" fill="#16a34a"/></svg>"##;
    assert_eq!(
        compile_standalone_svg(with_par, viewport(32.0, 16.0)).expect("inert preserveAspectRatio"),
        compile_standalone_svg(plain, viewport(32.0, 16.0))
            .expect("the preserveAspectRatio-less twin")
    );
}

/// The `preserveAspectRatio` mapping table: fit and alignment produce the
/// documented scales and offsets, pinned at the contract level. Chromium
/// pixel bakes for representative cells land with the corpus step of this
/// rung; until then the transform assertions here are the gate.
#[test]
fn preserve_aspect_ratio_maps_fit_and_alignment() {
    // viewBox "0 0 32 32" into a 64x32 viewport: scale_x = 2, scale_y = 1.
    for (par, expected) in [
        // none: non-uniform, each axis fills exactly.
        ("none", (2.0, 0.0, 1.0, 0.0)),
        // meet: s = min = 1; the x axis has 32 spare units to align.
        ("xMidYMid meet", (1.0, 16.0, 1.0, 0.0)),
        ("xMinYMin meet", (1.0, 0.0, 1.0, 0.0)),
        ("xMaxYMax meet", (1.0, 32.0, 1.0, 0.0)),
        // meet with no fit token: `meet` is the grammar default.
        ("xMaxYMid", (1.0, 32.0, 1.0, 0.0)),
        // slice: s = max = 2; the y axis overhangs 32 units to align.
        ("xMidYMid slice", (2.0, 0.0, 2.0, -16.0)),
        ("xMinYMin slice", (2.0, 0.0, 2.0, 0.0)),
        ("xMidYMax slice", (2.0, 0.0, 2.0, -32.0)),
        // none with an explicit fit token: validated, then ignored per spec.
        ("none slice", (2.0, 0.0, 1.0, 0.0)),
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 32 32" preserveAspectRatio="{par}"><rect width="32" height="32" fill="#16a34a"/></svg>"##
        );
        let frame = compile_standalone_svg(&source, viewport(64.0, 32.0))
            .unwrap_or_else(|error| panic!("{par}: {error}"));
        let (sx, dx, sy, dy) = expected;
        assert_eq!(
            frame.nodes[0].transform,
            AffineTransform::from_acebdf(sx, 0.0, dx, 0.0, sy, dy),
            "{par}"
        );
    }
}

/// Malformed `preserveAspectRatio` grammar refuses by name in both
/// admissions — where Chromium silently renders the default `xMidYMid
/// meet` mapping, this slice refuses instead of silently defaulting. That
/// includes every `defer`-carrying value: SVG2 dropped the 1.1 prefix and
/// Chromium treats the whole value as unparseable, so honoring the
/// remainder would paint pixels the oracle does not.
#[test]
fn malformed_preserve_aspect_ratio_refuses_in_both_admissions() {
    for value in [
        "",
        "xMidYMiddle meet",
        "xmidymid meet",
        "XMidYMid meet",
        "xMidYMid fit",
        "xMidYMid meet extra",
        "meet",
        "defer xMidYMid meet",
        "defer",
        "defer junk",
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 32 32" preserveAspectRatio="{value}"><rect width="32" height="32" fill="#16a34a"/></svg>"##
        );
        for (mode, result) in both_admissions(&source, viewport(64.0, 32.0)) {
            let error = result
                .err()
                .unwrap_or_else(|| panic!("{value:?} ({mode}): malformed grammar refuses"));
            assert!(
                matches!(&error, CompileError::BadPreserveAspectRatio(v) if v == value),
                "{value:?} ({mode}): expected BadPreserveAspectRatio, got {error}"
            );
        }
    }

    // The refusal does not depend on a viewBox being present: the grammar
    // is patrolled before (and regardless of) the mapping it would shape.
    let no_viewbox = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="16" preserveAspectRatio="xMidYMiddle meet"><rect width="8" height="8" fill="#16a34a"/></svg>"##;
    for (mode, result) in both_admissions(no_viewbox, viewport(32.0, 16.0)) {
        assert!(
            matches!(
                result,
                Err(CompileError::BadPreserveAspectRatio(ref v)) if v == "xMidYMiddle meet"
            ),
            "{mode}: malformed grammar refuses even without a viewBox"
        );
    }
}

/// A cascaded CSS `width`/`height` — a `<style>` rule or a `style`
/// attribute — beats both the authored attribute and the auto default in
/// Chromium, and the compiler reads geometry from attributes only, so it
/// refuses by name in both admissions rather than paint at the wrong size.
#[test]
fn cascaded_css_root_sizing_refuses_in_both_admissions() {
    for (label, source) in [
        (
            "style element over auto sizing",
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><style>svg { width: 40px; }</style><rect width="10" height="10" fill="#16a34a"/></svg>"##,
        ),
        (
            "style attribute over auto sizing",
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" style="width: 40px; height: 20px"><rect width="10" height="10" fill="#16a34a"/></svg>"##,
        ),
        (
            "style element over explicit attributes",
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><style>svg { width: 32px; height: 32px }</style><rect width="64" height="64" fill="#16a34a"/></svg>"##,
        ),
    ] {
        for (mode, result) in both_admissions(source, viewport(64.0, 64.0)) {
            let error = result
                .err()
                .unwrap_or_else(|| panic!("{label} ({mode}): cascaded CSS sizing refuses"));
            assert!(
                matches!(&error, CompileError::UnsupportedStyle(reason) if reason.contains("width")),
                "{label} ({mode}): expected a named CSS-sizing refusal, got {error}"
            );
        }
    }
}

/// Rust's float grammar is a superset of the SVG number grammar: a
/// trailing-dot dimension (`32.`) parses as f32 but Chromium drops the
/// attribute (resolving `auto` instead), so admitting it would paint a
/// different geometry than the oracle. It refuses as a bad number in both
/// admissions.
#[test]
fn rust_float_superset_dimensions_refuse_as_bad_numbers() {
    for value in ["32.", "3.e2"] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{value}" height="16"><rect width="8" height="8" fill="#16a34a"/></svg>"##
        );
        for (mode, result) in both_admissions(&source, viewport(64.0, 48.0)) {
            assert!(
                matches!(result, Err(CompileError::BadNumber { ref attr, .. }) if attr == "width"),
                "{value:?} ({mode}): a non-SVG number token refuses as a bad number"
            );
        }
    }
}

/// Percentage root sizing refuses by name in both admissions and both
/// entries: `N%` is valid SVG length grammar the slice does not yet
/// resolve, and misreporting it as a bad number would be dishonest.
#[test]
fn percentage_root_sizing_refuses_by_name() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="50%" height="50%" viewBox="0 0 32 32"><rect width="32" height="32" fill="#16a34a"/></svg>"##;
    for (mode, result) in both_admissions(source, viewport(64.0, 64.0)) {
        let error = result
            .err()
            .unwrap_or_else(|| panic!("{mode}: percentage sizing refuses"));
        assert!(
            matches!(&error, CompileError::UnsupportedSizing(reason) if reason.contains("percentage width=\"50%\"")),
            "{mode}: expected a named percentage refusal, got {error}"
        );
    }

    // The inline HTML entry patrols the same grammar.
    let html = r##"<html><body><svg xmlns="http://www.w3.org/2000/svg" width="100%" height="32"><rect width="8" height="8" fill="#16a34a"/></svg></body></html>"##;
    for (mode, result) in [
        ("strict", SvgFrameSource::from_html_inline_svg(html)),
        (
            "best-effort",
            SvgFrameSource::from_html_inline_svg_best_effort(html),
        ),
    ] {
        let error = result
            .err()
            .unwrap_or_else(|| panic!("{mode}: inline-HTML percentage sizing refuses"));
        assert!(
            matches!(&error, CompileError::UnsupportedSizing(reason) if reason.contains("percentage")),
            "{mode}: expected a named percentage refusal, got {error}"
        );
    }
}

/// `width="0"` disables rendering (SVG2 §8.2): admitted, a zero-extent
/// viewport clip, every output pixel transparent — an honest nothing, not
/// a refusal. (No Chromium bake: a zero-size element cannot be
/// screenshotted; the downstream clip law carries it.)
#[test]
fn zero_root_extent_renders_nothing() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="16" viewBox="0 0 32 32"><rect width="32" height="32" fill="#16a34a"/></svg>"##;
    let [(_, strict), (_, best)] = both_admissions(source, viewport(16.0, 16.0));
    let frame = strict.expect("zero extent admits").base_frame();
    assert_eq!(
        frame,
        best.expect("zero extent admits").base_frame(),
        "identical in both admissions"
    );
    assert_eq!(frame.bounds, Rectangle::from_xywh(0.0, 0.0, 0.0, 16.0));
    let pixels = render_through_n0(&frame, 16, 16);
    assert!(
        pixels.chunks_exact(4).all(|pixel| pixel == [0, 0, 0, 0]),
        "every pixel stays transparent"
    );
}
