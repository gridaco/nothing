//! Compatibility façade for the extracted [`htmlcss`] renderer.
//!
//! New Web hosts depend on `htmlcss` directly. This module retains the public
//! renderer surface that existing `grida` consumers used before extraction;
//! importer-only styled-DOM operations remain crate-private.

pub use ::htmlcss::{
    collect_image_urls, markdown_to_styled_html, measure_content_height, render, render_any,
    render_svg, style, svg, types, with_extra_stylesheets, ImageProvider, NoImages,
    PreloadedImages,
};

pub(crate) mod styled_dom {
    pub(crate) use ::htmlcss::styled_dom::{parse_and_style, styled_of};
}
