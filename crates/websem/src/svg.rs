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
//! Deliberately narrow: the proving shell supports only the enumerated
//! viewport/fill cases around an outer `<svg>` and solid-filled `<rect>`.
//! [`CompileError`] makes those patrolled rejection cases explicit. This is
//! not yet an exhaustive SVG-surface validator or an SVG capability claim.
//!
//! ## SVG paint boundary
//! The workspace's official Stylo revision exposes the typed basic SVG paint
//! longhands under the Servo engine. This proving shell has not yet wired SVG
//! presentation hints or SVG stylesheets into that cascade, nor switched its
//! compiler to the typed paint values. [`resolve_fill`] therefore still reads
//! the direct `fill` attribute and uses computed `color` only to resolve
//! `currentColor`. The dependency provenance is solved; production ingress
//! and semantic consumption are not.
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

/// An explicit failure in the proving shell's enumerated grammar checks.
///
/// This list is not yet exhaustive over SVG attributes or computed style; the
/// closed primitive suite defines the shell's positive coverage.
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
    /// Viewport sizing needs a default/CSS sizing path this slice lacks.
    UnsupportedSizing(String),
    /// A viewport dimension is syntactically numeric but invalid.
    InvalidDimension { attr: String, value: String },
    /// A `viewBox` whose four-number grammar or positive extent is invalid.
    BadViewBox(String),
    /// A valid SVG viewport mode the current slice does not implement.
    UnsupportedViewport(String),
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
            CompileError::UnsupportedSizing(reason) => {
                write!(f, "unsupported SVG viewport sizing: {reason}")
            }
            CompileError::InvalidDimension { attr, value } => {
                write!(f, "invalid SVG viewport dimension {attr}={value:?}")
            }
            CompileError::BadViewBox(v) => write!(f, "viewBox {v:?} is invalid"),
            CompileError::UnsupportedViewport(v) => {
                write!(f, "unsupported SVG viewport mapping: {v}")
            }
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

/// Compile a bare `<svg>` scaffold into an SVG-local [`Frame`], through
/// html5ever's foreign-content handling and the same compiler as the inline
/// entry.
///
/// This is deliberately not advertised as the conforming standalone SVG/XML
/// grammar entry required by the Web-First Amendment.
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
    let width = attr_f32(id, "width")?.ok_or_else(|| {
        CompileError::UnsupportedSizing(
            "missing width; CSS/default intrinsic sizing is not implemented".to_string(),
        )
    })?;
    let height = attr_f32(id, "height")?.ok_or_else(|| {
        CompileError::UnsupportedSizing(
            "missing height; CSS/default intrinsic sizing is not implemented".to_string(),
        )
    })?;
    reject_negative_dimension("width", width)?;
    reject_negative_dimension("height", height)?;
    if let Some(value) = get_attr(id, "preserveAspectRatio") {
        return Err(CompileError::UnsupportedViewport(format!(
            "preserveAspectRatio={value:?}"
        )));
    }
    let viewbox = match get_attr(id, "viewBox") {
        Some(v) => Some(parse_viewbox(&v)?),
        None => None,
    };

    // The first slice proves only the equal-aspect default mapping. Reject
    // every other valid preserveAspectRatio case rather than silently stretch.
    let viewport = match viewbox {
        Some((vb_x, vb_y, vb_w, vb_h)) => {
            let sx = width / vb_w;
            let sy = height / vb_h;
            let tolerance = f32::EPSILON * sx.abs().max(sy.abs()).max(1.0) * 8.0;
            if (sx - sy).abs() > tolerance {
                return Err(CompileError::UnsupportedViewport(
                    "non-uniform viewBox mapping requires preserveAspectRatio semantics"
                        .to_string(),
                ));
            }
            AffineTransform::from_acebdf(sx, 0.0, -vb_x * sx, 0.0, sy, -vb_y * sy)
        }
        None => AffineTransform::identity(),
    };
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
    // Support the explicit proving-shell grammar: four finite numbers
    // separated by ASCII whitespace and/or one comma. Empty comma groups are
    // malformed; reject them instead of filtering repeated/trailing commas.
    // More compact SVG number-list forms remain unsupported rather than
    // guessed.
    let comma_groups: Vec<&str> = v.split(',').collect();
    if comma_groups.iter().any(|group| group.trim().is_empty()) {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    let tokens: Vec<&str> = comma_groups
        .iter()
        .flat_map(|group| group.split_ascii_whitespace())
        .collect();
    if tokens.len() != 4 {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    let mut parts = [0.0f32; 4];
    for (index, token) in tokens.iter().enumerate() {
        let value = token
            .parse::<f32>()
            .map_err(|_| CompileError::BadViewBox(v.to_string()))?;
        if !value.is_finite() {
            return Err(CompileError::BadViewBox(v.to_string()));
        }
        parts[index] = value;
    }
    if parts[2] <= 0.0 || parts[3] <= 0.0 {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    Ok((parts[0], parts[1], parts[2], parts[3]))
}

fn reject_negative_dimension(attr: &str, value: f32) -> Result<(), CompileError> {
    if value < 0.0 {
        return Err(CompileError::InvalidDimension {
            attr: attr.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
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
        Some(v) => {
            let parsed = v
                .trim()
                .parse::<f32>()
                .map_err(|_| CompileError::BadNumber {
                    attr: name.to_string(),
                    value: v.clone(),
                })?;
            if !parsed.is_finite() {
                return Err(CompileError::BadNumber {
                    attr: name.to_string(),
                    value: v,
                });
            }
            Ok(Some(parsed))
        }
    }
}
