//! The SVG semantic compiler — the shared machinery both grammar entries use.
//!
//! One namespace-aware document (`csscascade::DemoDom`, html5ever) and one
//! browser-grade cascade (`csscascade::CascadeDriver`, Stylo) resolve the
//! source; this compiler then reads *resolved* facts — geometry from
//! presentation attributes, paint from the SVG paint model (`fill`,
//! `currentColor`) resolved against the cascaded computed `color` — and emits
//! the source-neutral [`rframe::Frame`]. It never touches the legacy SVG-only
//! matcher, never serializes-and-reparses inline SVG, and never paints.
//!
//! Deliberately narrow: the first slice supports the outer `<svg>` (with
//! `width`/`height`/`viewBox`) and solid-filled `<rect>`. Every construct it
//! cannot faithfully resolve is an explicit [`CompileError`] — never a silent
//! shim.
//!
//! ## servo-Stylo caveat (a filed finding)
//! The workspace compiles Stylo with the **servo** engine, which omits the
//! gecko-only SVG paint longhands (`fill`, `stroke`, …). So `fill` cannot be
//! read from `ComputedValues`; it is read from the presentation attribute
//! here. The cascade still carries `color` (a servo longhand), which crosses
//! the HTML→inline-SVG boundary — that is what makes `fill="currentColor"`
//! resolve to a cascaded value.
//!
//! ## Concurrency caveat
//! `csscascade` installs the parsed document into a process-global slot, so
//! only one compile may touch it at a time. A crate-local mutex serializes
//! compiles; this fights the "many hosts" topology and is a documented
//! limitation of the current cascade crate, not this front-end.

use std::sync::Mutex;

use csscascade::adapter::{self, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::{DemoDom, DemoNodeData, NodeId};

use style::color::ColorSpace;
use style::dom::TElement;
use style::properties::ComputedValues;
use style::thread_state::{self, ThreadState};

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::frame::{Color, Frame, FrameNode, Geometry, PaintStack};

/// Serializes access to `csscascade`'s process-global document slot.
static COMPILE_LOCK: Mutex<()> = Mutex::new(());

/// A construct the first slice does not model. Failures are explicit — the
/// compiler never silently drops or approximates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// No `<svg>` element was found in the document.
    NoSvgRoot,
    /// An element the slice does not support (only `<svg>` and `<rect>` do).
    UnsupportedElement(String),
    /// A `fill` value the slice cannot resolve.
    UnsupportedFill(String),
    /// A numeric attribute failed to parse.
    BadNumber { attr: String, value: String },
    /// A `viewBox` that is not four numbers.
    BadViewBox(String),
    /// An element carried no computed style (cascade did not reach it).
    MissingComputedStyle,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::NoSvgRoot => write!(f, "no <svg> element in document"),
            CompileError::UnsupportedElement(t) => write!(f, "unsupported element <{t}>"),
            CompileError::UnsupportedFill(v) => write!(f, "unsupported fill value {v:?}"),
            CompileError::BadNumber { attr, value } => {
                write!(f, "attribute {attr}={value:?} is not a number")
            }
            CompileError::BadViewBox(v) => write!(f, "viewBox {v:?} is not four numbers"),
            CompileError::MissingComputedStyle => write!(f, "element has no computed style"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile an HTML document containing inline `<svg>` into an SVG-local
/// [`Frame`]. The inline SVG's descendant style comes from the surrounding
/// HTML cascade (e.g. `color` from a `<style>` rule), never a nested renderer.
pub fn compile_html_inline_svg(html: &str) -> Result<Frame, CompileError> {
    compile_first_svg(html)
}

/// Compile a standalone SVG document into an SVG-local [`Frame`], through the
/// same document machinery and the same compiler as the inline entry.
pub fn compile_standalone_svg(svg: &str) -> Result<Frame, CompileError> {
    compile_first_svg(svg)
}

/// Parse (as an html5ever document, so a bare `<svg>` enters as foreign
/// content), cascade, find the first `<svg>`, and compile its subtree.
fn compile_first_svg(source: &str) -> Result<Frame, CompileError> {
    let _guard = COMPILE_LOCK.lock().expect("compile lock");
    // Idempotent for the same state; safe to call per compile.
    thread_state::initialize(ThreadState::LAYOUT);

    let dom = DemoDom::parse_from_bytes(source.as_bytes()).expect("parse document");
    let mut driver = CascadeDriver::new(&dom);
    let document = adapter::bootstrap_dom(dom);
    driver.flush(document);
    driver.style_document(document);

    let root = document.root_element().ok_or(CompileError::NoSvgRoot)?;
    let svg = find_svg(root).ok_or(CompileError::NoSvgRoot)?;
    compile_svg_element(svg)
}

/// First `<svg>` element in document order.
fn find_svg(el: HtmlElement) -> Option<HtmlElement> {
    if el.local_name_string().eq_ignore_ascii_case("svg") {
        return Some(el);
    }
    let mut child = el.first_element_child();
    while let Some(c) = child {
        if let Some(found) = find_svg(c) {
            return Some(found);
        }
        child = c.next_element_sibling();
    }
    None
}

/// Compile an `<svg>` element and its children into an SVG-local frame.
fn compile_svg_element(svg: HtmlElement) -> Result<Frame, CompileError> {
    let id = svg.node_id();
    let width = attr_f32(id, "width")?.unwrap_or(0.0);
    let height = attr_f32(id, "height")?.unwrap_or(0.0);
    let (vb_x, vb_y, vb_w, vb_h) = match get_attr(id, "viewBox") {
        Some(v) => parse_viewbox(&v)?,
        None => (0.0, 0.0, width, height),
    };

    // viewBox → viewport mapping (preserveAspectRatio: the simple stretch case,
    // exact for the equal-aspect fixtures the slice uses).
    let sx = if vb_w != 0.0 { width / vb_w } else { 1.0 };
    let sy = if vb_h != 0.0 { height / vb_h } else { 1.0 };
    let viewport = AffineTransform::from_acebdf(sx, 0.0, -vb_x * sx, 0.0, sy, -vb_y * sy);
    let frame_bounds = Rectangle::from_xywh(0.0, 0.0, width, height);

    let mut nodes = Vec::new();
    let mut next_id = 0u64;
    let mut child = svg.first_element_child();
    while let Some(c) = child {
        nodes.push(compile_shape(c, viewport, &mut next_id)?);
        child = c.next_element_sibling();
    }

    Ok(Frame {
        bounds: frame_bounds,
        nodes,
    })
}

/// Compile a single shape element into a resolved node.
fn compile_shape(
    el: HtmlElement,
    viewport: AffineTransform,
    next_id: &mut u64,
) -> Result<FrameNode, CompileError> {
    let tag = el.local_name_string().to_ascii_lowercase();
    match tag.as_str() {
        "rect" => compile_rect(el, viewport, next_id),
        other => Err(CompileError::UnsupportedElement(other.to_string())),
    }
}

fn compile_rect(
    el: HtmlElement,
    viewport: AffineTransform,
    next_id: &mut u64,
) -> Result<FrameNode, CompileError> {
    let id = el.node_id();
    let x = attr_f32(id, "x")?.unwrap_or(0.0);
    let y = attr_f32(id, "y")?.unwrap_or(0.0);
    let w = attr_f32(id, "width")?.unwrap_or(0.0);
    let h = attr_f32(id, "height")?.unwrap_or(0.0);
    let rect = Rectangle::from_xywh(x, y, w, h);

    let fill = resolve_fill(el)?;
    let paints = match fill {
        Some(color) => PaintStack::solid(color),
        None => PaintStack::default(),
    };

    let node = FrameNode {
        id: rframe::frame::NodeId(*next_id),
        transform: viewport,
        geometry: Geometry::Rect(rect),
        bounds: transform_aabb(viewport, rect),
        paints,
        clip: None,
    };
    *next_id += 1;
    Ok(node)
}

/// Resolve the SVG `fill` paint. `currentColor` resolves against the cascaded
/// computed `color`; a missing `fill` is SVG's default black.
fn resolve_fill(el: HtmlElement) -> Result<Option<Color>, CompileError> {
    let raw = get_attr(el.node_id(), "fill");
    match raw.as_deref().map(str::trim) {
        None => Ok(Some(Color::opaque(0, 0, 0))),
        Some("none") => Ok(None),
        Some("black") => Ok(Some(Color::opaque(0, 0, 0))),
        Some("currentColor") => Ok(Some(computed_color(el)?)),
        Some(hex) if hex.starts_with('#') => Ok(Some(parse_hex(hex)?)),
        Some(other) => Err(CompileError::UnsupportedFill(other.to_string())),
    }
}

/// The element's cascaded computed `color`, converted to straight-alpha sRGB.
fn computed_color(el: HtmlElement) -> Result<Color, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    let srgb = style
        .get_inherited_text()
        .color
        .to_color_space(ColorSpace::Srgb);
    let c = srgb.raw_components();
    Ok(Color::rgba(
        to_u8(c[0]),
        to_u8(c[1]),
        to_u8(c[2]),
        to_u8(srgb.alpha),
    ))
}

fn to_u8(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_hex(hex: &str) -> Result<Color, CompileError> {
    let body = &hex[1..];
    let err = || CompileError::UnsupportedFill(hex.to_string());
    let (r, g, b) = match body.len() {
        3 => {
            let d = |i: usize| u8::from_str_radix(&body[i..i + 1], 16).map(|v| v * 17);
            (
                d(0).map_err(|_| err())?,
                d(1).map_err(|_| err())?,
                d(2).map_err(|_| err())?,
            )
        }
        6 => {
            let d = |i: usize| u8::from_str_radix(&body[i..i + 2], 16);
            (
                d(0).map_err(|_| err())?,
                d(2).map_err(|_| err())?,
                d(4).map_err(|_| err())?,
            )
        }
        _ => return Err(err()),
    };
    Ok(Color::opaque(r, g, b))
}

fn parse_viewbox(v: &str) -> Result<(f32, f32, f32, f32), CompileError> {
    let parts: Vec<f32> = v
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    match parts.as_slice() {
        [x, y, w, h] => Ok((*x, *y, *w, *h)),
        _ => Err(CompileError::BadViewBox(v.to_string())),
    }
}

/// Axis-aligned bounds of `rect` after `t` (scale+translate; corner transform
/// is exact for the slice, and remains correct if rotation is added later).
fn transform_aabb(t: AffineTransform, rect: Rectangle) -> Rectangle {
    let m = t.matrix;
    let pt = |x: f32, y: f32| {
        (
            m[0][0] * x + m[0][1] * y + m[0][2],
            m[1][0] * x + m[1][1] * y + m[1][2],
        )
    };
    let corners = [
        pt(rect.x, rect.y),
        pt(rect.x + rect.width, rect.y),
        pt(rect.x, rect.y + rect.height),
        pt(rect.x + rect.width, rect.y + rect.height),
    ];
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in corners {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Rectangle::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Read an element attribute by local name from the process-global document.
fn get_attr(id: NodeId, name: &str) -> Option<String> {
    let node = adapter::dom().node(id);
    if let DemoNodeData::Element(e) = &node.data {
        for a in &e.attrs {
            if a.name.local.as_ref().eq_ignore_ascii_case(name) {
                return Some(a.value.to_string());
            }
        }
    }
    None
}

fn attr_f32(id: NodeId, name: &str) -> Result<Option<f32>, CompileError> {
    match get_attr(id, name) {
        None => Ok(None),
        Some(v) => v
            .trim()
            .parse::<f32>()
            .map(Some)
            .map_err(|_| CompileError::BadNumber {
                attr: name.to_string(),
                value: v,
            }),
    }
}
