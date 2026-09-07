//! One ingress dispatch for the SVG and HTML named-refusal corpus.
use std::path::Path;
use websem::{CompileError, InitialViewport, SvgFrameSource};

pub(crate) fn compile(
    root: &Path,
    id: &str,
    best_effort: bool,
) -> Result<SvgFrameSource, CompileError> {
    let svg = root.join(format!("{id}.svg"));
    let html = root.join(format!("{id}.html"));
    assert_ne!(
        svg.exists(),
        html.exists(),
        "{id}: exactly one declared source ingress"
    );
    let source = std::fs::read_to_string(if svg.exists() { &svg } else { &html })
        .unwrap_or_else(|error| panic!("{id}: read: {error}"));
    if html.exists() {
        if best_effort {
            SvgFrameSource::from_html_inline_svg_best_effort(source.as_str())
        } else {
            SvgFrameSource::from_html_inline_svg(source.as_str())
        }
    } else if best_effort {
        SvgFrameSource::from_standalone_svg_best_effort_with_fonts(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
            crate::fixture_fonts::unsupported_environment(id),
        )
    } else {
        SvgFrameSource::from_standalone_svg_with_fonts(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
            crate::fixture_fonts::unsupported_environment(id),
        )
    }
}
