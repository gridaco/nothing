//! Conforming-entry laws for standalone SVG/XML sources.
//!
//! The standalone grammar entry is namespace-aware and case-preserving XML —
//! not HTML foreign-content recovery. The enumerated refusal classes —
//! recorded XML recoveries, a missing SVG namespace, and case-folded names —
//! track Chromium's treatment of standalone SVG documents; the XML5 recovery
//! classes left unrecorded remain a pinned leniency boundary (csscascade's
//! entry laws), not a universal Chromium-alignment claim. Case-insensitive
//! leniency remains the HTML entry's own (parser-applied) behavior, not the
//! compiler's.

use websem::{CompileError, compile_standalone_svg};

/// The host-established initial viewport for this file's laws — inert:
/// every source here authors explicit root dimensions or refuses before
/// sizing resolves.
fn host_viewport() -> websem::InitialViewport {
    websem::InitialViewport::new(64.0, 64.0)
}

#[test]
fn recorded_xml_recovery_is_refused_not_rendered() {
    for (label, source) in [
        (
            "mismatched close tag",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="4" height="4"></svg>"#,
        ),
        (
            "unexpected end of input",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="4""#,
        ),
        (
            "duplicate attribute",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" width="8" height="16"/>"#,
        ),
    ] {
        assert!(
            matches!(
                compile_standalone_svg(source, host_viewport()),
                Err(CompileError::MalformedXml(_))
            ),
            "{label} must be refused as malformed XML"
        );
    }
}

#[test]
fn missing_svg_namespace_is_not_svg() {
    // Chromium renders a namespace-less `<svg>` document as an XML tree, not
    // as SVG; the conforming entry refuses it the same way.
    assert!(matches!(
        compile_standalone_svg(
            r#"<svg width="16" height="16"><rect width="4" height="4"/></svg>"#,
            host_viewport()
        ),
        Err(CompileError::NoSvgRoot)
    ));
}

#[test]
fn case_folded_root_is_not_svg() {
    // XML is case-sensitive: `SVG` is not the SVG `svg` element even inside
    // the SVG namespace.
    assert!(matches!(
        compile_standalone_svg(
            r#"<SVG xmlns="http://www.w3.org/2000/svg" width="16" height="16"/>"#,
            host_viewport()
        ),
        Err(CompileError::NoSvgRoot)
    ));
}

#[test]
fn case_folded_attributes_are_not_their_canonical_selves() {
    // An authored `viewbox` is not `viewBox` in XML. Chromium ignores the
    // unknown attribute and maps the viewport identically; so does the entry.
    let with_junk_case = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewbox="0 0 4 4"><rect width="8" height="8" fill="#16a34a"/></svg>"##,
        host_viewport(),
    )
    .expect("unknown attribute is ignored, not honored");
    let without = compile_standalone_svg(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="#16a34a"/></svg>"##,
        host_viewport(),
    )
    .expect("baseline");
    assert_eq!(
        with_junk_case, without,
        "a case-folded viewBox must not select a viewport mapping"
    );
}

#[test]
fn xml_prolog_is_tolerated() {
    let frame = compile_standalone_svg(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="8" height="8" fill="#16a34a"/></svg>
"##,
        host_viewport(),
    )
    .expect("prolog and trailing whitespace are conforming XML");
    assert_eq!(frame.nodes.len(), 1);
}

#[test]
fn html_document_bytes_are_not_standalone_svg() {
    // An HTML page fed to the standalone entry must not silently reroute
    // through HTML recovery: without an SVG-namespace root it is refused.
    assert!(matches!(
        compile_standalone_svg(
            "<html><body><svg width=\"16\" height=\"16\"/></body></html>",
            host_viewport()
        ),
        Err(CompileError::NoSvgRoot) | Err(CompileError::MalformedXml(_))
    ));
}
