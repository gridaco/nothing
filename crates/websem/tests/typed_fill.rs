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

/// Chromium's paint-fallback semantics, honored since the gradient rung (the
/// obligation this law carried from the day the refusal was pinned): an
/// unresolvable paint server WITH a declared fallback renders the fallback
/// color (`fill="url(#missing) red"` paints red — Chromium-baked as the
/// `svg-gradient-fallback` cell), and one without a fallback paints nothing
/// at all.
#[test]
fn paint_server_fill_honors_the_authored_fallback() {
    let without = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="url(#missing)"/></svg>"##,
        host_viewport(),
    )
    .expect("an invalid reference without a fallback is admitted");
    assert!(
        without.nodes[0].paints.is_empty(),
        "no fallback paints nothing"
    );

    let with = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="url(#missing) red"/></svg>"##,
        host_viewport(),
    )
    .expect("an invalid reference with a fallback is admitted");
    let cg::Paint::Solid(solid) = with.nodes[0]
        .paints
        .iter()
        .next()
        .expect("the fallback paint")
    else {
        panic!("expected the fallback to resolve solid");
    };
    assert_eq!(solid.color, cg::CGColor::from_rgba(255, 0, 0, 255));
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
    let cg::Paint::Solid(solid) = frame.nodes[0].paints.iter().next().expect("one paint") else {
        panic!("expected a solid paint");
    };
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
    let cg::Paint::Solid(solid) = frame.nodes[0].paints.iter().next().expect("one paint") else {
        panic!("expected a solid paint");
    };
    assert_eq!(solid.color.a(), 128);
}
