//! Refusal laws for the typed fill consumption's enumerated boundaries.
//!
//! Every paint kind the slice does not model refuses explicitly — pinned
//! here so a future capability step inherits the recorded semantics instead
//! of silently forgetting them.

use websem::{CompileError, compile_standalone_svg};

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
        let error =
            compile_standalone_svg(source).expect_err("paint servers are outside the slice");
        let CompileError::UnsupportedFill(value) = error else {
            panic!("expected an explicit fill refusal, got {error:?}");
        };
        assert!(
            value.contains("url("),
            "the refusal names the server: {value}"
        );
    }
}

/// fill-opacity is deliberately not yet consumed. A non-initial cascaded
/// value would silently render opaque where Chromium renders translucent, so
/// Base compilation refuses it explicitly until its capability step lands.
#[test]
fn css_fill_opacity_refuses_until_admitted() {
    let error = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" style="fill: #ef4444; fill-opacity: 0.5"/></svg>"##,
    )
    .expect_err("non-initial fill-opacity must refuse, not render opaque");
    let CompileError::UnsupportedFill(value) = error else {
        panic!("expected an explicit fill refusal");
    };
    assert!(
        value.contains("fill-opacity"),
        "the refusal names the property: {value}"
    );
}
