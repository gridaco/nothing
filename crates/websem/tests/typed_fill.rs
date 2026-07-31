//! Refusal laws for the typed fill consumption's enumerated boundaries.
//!
//! Every paint kind the slice does not model refuses explicitly — pinned
//! here so a future capability step inherits the recorded semantics instead
//! of silently forgetting them.

use websem::{CompileError, compile_standalone_svg};

/// The host-established initial viewport for this file's laws — inert:
/// every source here authors explicit root dimensions or refuses before
/// sizing resolves.
fn host_viewport() -> websem::InitialViewport {
    websem::InitialViewport::new(64.0, 64.0)
}

/// Chromium's paint-fallback semantics, recorded for the future step: an
/// unresolvable paint server WITH a declared fallback renders the fallback
/// color (`fill="url(#missing) red"` paints red). The slice models no paint
/// servers, so both forms refuse today; whichever step admits paint servers
/// must honor the fallback, and this law is where that obligation lives.
#[test]
fn paint_server_fill_refuses_with_and_without_fallback() {
    for source in [
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="url(#missing)"/></svg>"##,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="url(#missing) red"/></svg>"##,
    ] {
        let error = compile_standalone_svg(source, host_viewport())
            .expect_err("paint servers are outside the slice");
        let CompileError::UnsupportedFill(value) = error else {
            panic!("expected an explicit fill refusal, got {error:?}");
        };
        assert!(
            value.contains("url("),
            "the refusal names the server: {value}"
        );
    }
}

/// The admitted color surface is opaque sRGB — exactly what the
/// Chromium-baked primitive suite gates. A wide-gamut value would pass
/// through an unverified color-space conversion and per-channel clamp, so it
/// refuses with its color space named until its capability step bakes
/// fixtures.
#[test]
fn wide_gamut_fill_refuses_until_baked() {
    for value in [
        "oklch(70% 0.3 340)",
        "lab(50% 40 59.5)",
        "color(display-p3 1 0 0)",
    ] {
        let source = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="{value}"/></svg>"##
        );
        let error = compile_standalone_svg(&source, host_viewport())
            .expect_err("non-sRGB color spaces are outside the gated surface");
        let CompileError::UnsupportedFill(reason) = error else {
            panic!("expected an explicit fill refusal for {value}");
        };
        assert!(
            reason.contains("color space"),
            "the refusal names the space for {value}: {reason}"
        );
    }
}

/// The translucency rung folds a translucent sRGB fill into the paint's
/// alpha — the typed read now admits it, frame-identically across both
/// admissions, and the compositing itself is Chromium-baked in the corpus
/// (`svg-translucent-fill-rgba`).
#[test]
fn translucent_fill_folds_into_the_paint_alpha() {
    let frame = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="rgb(239 68 68 / 0.5)"/></svg>"##,
        host_viewport(),
    )
    .expect("a translucent fill is admitted");
    let solid = frame.nodes[0].paints.iter().next().expect("one paint");
    assert_eq!(solid.color.a(), 128, "alpha 0.5 quantizes to 128, once");
}

/// `fill-opacity` multiplies into the colour's own alpha in float and
/// quantizes once — the multiplied cell (`svg-fill-opacity-times-alpha`)
/// pins the rounding against Chromium.
#[test]
fn css_fill_opacity_multiplies_into_the_paint_alpha() {
    let frame = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" style="fill: #ef4444; fill-opacity: 0.5"/></svg>"##,
        host_viewport(),
    )
    .expect("fill-opacity is consumed");
    let solid = frame.nodes[0].paints.iter().next().expect("one paint");
    assert_eq!(solid.color.a(), 128);
}
