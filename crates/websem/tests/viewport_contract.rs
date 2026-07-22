//! Unsupported SVG viewport semantics fail explicitly instead of rendering a
//! plausible-but-wrong stretch.

use websem::{CompileError, compile_standalone_svg};

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

#[test]
fn malformed_viewbox_token_is_not_silently_discarded() {
    assert!(matches!(
        compile_standalone_svg(INVALID_TOKEN),
        Err(CompileError::BadViewBox(_))
    ));
}

#[test]
fn repeated_viewbox_comma_is_not_silently_filtered() {
    assert!(matches!(
        compile_standalone_svg(REPEATED_COMMA),
        Err(CompileError::BadViewBox(_))
    ));
}

#[test]
fn trailing_viewbox_comma_is_not_silently_filtered() {
    assert!(matches!(
        compile_standalone_svg(TRAILING_COMMA),
        Err(CompileError::BadViewBox(_))
    ));
}

#[test]
fn unequal_aspect_viewbox_is_rejected_until_default_mapping_exists() {
    assert!(matches!(
        compile_standalone_svg(UNEQUAL_DEFAULT),
        Err(CompileError::UnsupportedViewport(_))
    ));
}

#[test]
fn explicit_preserve_aspect_ratio_is_rejected_until_its_grammar_exists() {
    assert!(matches!(
        compile_standalone_svg(EXPLICIT),
        Err(CompileError::UnsupportedViewport(_))
    ));
}
