//! Paint servers: the document's gradient elements, resolved to `cg` paints.
//!
//! This module owns the id-resolution table for `<linearGradient>` and
//! `<radialGradient>`, the href template chain, the stop list, and the fold
//! of every SVG gradient coordinate system into the contract's unit-box
//! facts. It owns no painter and no document walk policy: the compiler hands
//! it a fragment, the consuming geometry's local box, and the viewport's
//! percentage bases, and gets back a resolved paint — or a refusal that
//! names itself.
//!
//! Every rule here is Chromium-measured (the gradient rung's probe matrix):
//!
//! - The id table is whole-document, document-ordered, first-id-wins, and
//!   skips any element inside `<use>` shadow content — `url(#id)` resolves
//!   against the document, never against an expanded clone.
//! - A reference to a missing id or a non-gradient element is **invalid**:
//!   the authored fallback fires. A *valid* reference that resolves to
//!   nothing painted — zero stops, a cycle that composes to zero stops, a
//!   non-invertible gradient transform, a zero-area object-bounding-box —
//!   paints nothing, and the fallback does **not** fire.
//! - Stops clamp to `[0, 1]` against a running maximum, are never sorted,
//!   and an invalid `offset` is `0`. `stop-color` and `stop-opacity` are
//!   attribute reads (the pinned cascade cannot represent the longhands);
//!   `currentColor` resolves against the stop's own computed `color`, and
//!   an unparseable `stop-color` (including the `inherit` keyword) is the
//!   initial black.
//! - The degenerate rules are the backend's own. A one-stop ramp is spatially
//!   constant but retains gradient rasterization; zero/negative radial radius
//!   and linear endpoints closer than the backend threshold resolve to a
//!   solid — the last stop under `pad`, the ramp's integral average under
//!   `reflect`/`repeat`. Resolving those cases here keeps downstream preflight
//!   inside its checked gradient domain.
//! - `gradientTransform` and an author `transform` declaration are one
//!   computed value (csscascade hints the attribute), applied about the raw
//!   origin of the gradient's own space; percentages in it are refused by
//!   name (Chromium resolves them against the viewport and then applies the
//!   number in fraction space — an incoherence this slice will not repeat).
//!
//! What refuses by name: a focal radial (resolved `fx`/`fy` off the center,
//! or `fr > 0` — the shared radial leaf is concentric), font-relative or
//! viewport-relative units in gradient geometry, `color-interpolation:
//! linearRGB`, author CSS on stops (`stop-color`/`stop-opacity` in a style
//! attribute), resolved stop alpha or degenerate alpha staging that the RGBA8
//! paint contract cannot preserve, an external reference, and a user-space
//! gradient on zero-area geometry.

use std::collections::HashMap;
use std::collections::HashSet;

use csscascade::adapter::HtmlElement;
use csscascade::dom::{DemoNodeData, NodeId};
use style::color::AbsoluteColor;
use style::context::QuirksMode as StyleQuirksMode;
use style::dom::TElement;
use style::properties::{
    ComputedValues, Importance, LonghandId, PropertyDeclaration, PropertyDeclarationBlock,
    PropertyId, SourcePropertyDeclaration, parse_one_declaration_into,
};
use style::stylesheets::{CssRuleType, Origin, UrlExtraData};
use style::values::specified::color::Color as SpecifiedColor;
use style_traits::ParsingMode;
use url::Url;

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::svg::{admitted_srgb, dots_carry_digits, get_attr, trim_svg_whitespace};
use crate::svg_transform::{TransformRefusal, computed_transform_to_affine};

/// The two same-document URL bases the cascade resolves references against:
/// presentation attributes and inline style resolve against the document
/// base, stylesheet-authored values against the synthetic sheet URL. A
/// resolved reference whose non-fragment part is neither is external.
/// (csscascade owns the bases; a change there fails the gradient cells
/// loudly rather than silently resolving an external reference.)
const SAME_DOCUMENT_URL_BASES: &[&str] = &["about:blank", "https://grida.local/inline.css"];

/// Mirror of the engine's degenerate-linear renderability threshold: at or
/// below this distance in the gradient's own coordinate space the pinned
/// backend substitutes its degenerate behavior, so the producer resolves
/// the measured meaning first. websem deliberately does not depend on the
/// engine crate; the gradient cells gate the two constants against each
/// other through pixels.
const DEGENERATE_LINEAR_THRESHOLD: f32 = 1.0 / (1 << 15) as f32;

/// The viewport's user-unit percentage bases (SVG2 §7.10), copied from the
/// compiler's one viewport.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GradientBases {
    pub width: f32,
    pub height: f32,
}

impl GradientBases {
    fn diagonal(self) -> f32 {
        ((f64::from(self.width) * f64::from(self.width)
            + f64::from(self.height) * f64::from(self.height))
            / 2.0)
            .sqrt() as f32
    }

    /// Percentage basis per gradient geometry attribute: x-axis lengths
    /// against the width, y-axis against the height, radii against the
    /// normalized diagonal.
    fn axis(self, attr: &str) -> f32 {
        match attr {
            "x1" | "x2" | "cx" | "fx" => self.width,
            "y1" | "y2" | "cy" | "fy" => self.height,
            _ => self.diagonal(),
        }
    }
}

enum Server<'d> {
    Gradient {
        element: HtmlElement<'d>,
        inside_compiled_svg: bool,
    },
    /// A pattern is a valid SVG paint server, but it is deliberately outside
    /// rframe's resolved paint vocabulary. Keeping it in the same first-id
    /// table as gradients is load-bearing: otherwise a pattern in `<defs>`
    /// looks exactly like a missing id and silently becomes fallback/no-paint.
    Pattern,
    /// An id on any other element makes the reference invalid as a paint
    /// server. It still occupies the first-id slot, exactly as DOM id lookup
    /// does, so a later gradient with the same id cannot incorrectly win.
    Other,
}

/// The document's paint-resource id table: document-ordered, first-id-wins,
/// shadow-content excluded. Non-gradients are retained for exact URL
/// classification, not lowered as paints.
pub(crate) struct PaintServers<'d> {
    by_fragment: HashMap<String, Server<'d>>,
}

fn has_use_ancestor(el: HtmlElement<'_>) -> bool {
    let mut current = el.traversal_parent();
    while let Some(parent) = current {
        if parent.local_name_string() == "use" {
            return true;
        }
        current = parent.traversal_parent();
    }
    false
}

fn is_inside(el: HtmlElement<'_>, ancestor: HtmlElement<'_>) -> bool {
    let mut current = Some(el);
    while let Some(node) = current {
        if node.node_id() == ancestor.node_id() {
            return true;
        }
        current = node.traversal_parent();
    }
    false
}

impl<'d> PaintServers<'d> {
    /// Walk the whole document in order and classify every element with an
    /// `id`, first id wins. Elements inside `<use>` shadow content
    /// are skipped: the expansion physically nests clones under the `<use>`,
    /// and Chromium resolves `url(#id)` against the document, never a clone
    /// (measured — a clone earlier in expanded order does not shadow the
    /// original). Authored element children of `<use>` refuse before paint,
    /// so a use-descendant here is always a clone.
    pub(crate) fn collect(document_root: HtmlElement<'d>, compiled_svg: HtmlElement<'d>) -> Self {
        let mut by_fragment = HashMap::new();
        let mut stack = vec![document_root];
        while let Some(el) = stack.pop() {
            // Children push in reverse so the pop order is document order.
            let mut children = Vec::new();
            let mut child = el.first_element_child();
            while let Some(c) = child {
                children.push(c);
                child = c.next_element_sibling();
            }
            for c in children.into_iter().rev() {
                stack.push(c);
            }
            if has_use_ancestor(el) {
                continue;
            }
            let Some(id) = element_id(el) else { continue };
            let server = match el.local_name_string().as_str() {
                "linearGradient" | "radialGradient" => Server::Gradient {
                    element: el,
                    inside_compiled_svg: is_inside(el, compiled_svg),
                },
                "pattern" => Server::Pattern,
                _ => Server::Other,
            };
            by_fragment.entry(id).or_insert(server);
        }
        Self { by_fragment }
    }
}

fn element_id(el: HtmlElement<'_>) -> Option<String> {
    if let DemoNodeData::Element(e) = &el.dom_node().data {
        return e.id_attr.as_ref().map(|atom| atom.to_string());
    }
    None
}

/// One resolved paint-server reference.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolvedPaintServer {
    /// The reference is invalid — no gradient carries the id. The authored
    /// fallback decides what paints.
    Invalid,
    /// A valid reference that paints nothing (measured correct nothings).
    Nothing,
    /// A valid reference that losslessly resolves to one RGBA8 solid color.
    Solid(CGColor),
    /// A valid gradient paint in the contract's unit-box facts.
    Gradient(cg::Paint),
}

/// Classification that must happen before context-box rebasing. It preserves
/// each construct's own outcome when a context relation selects it: an
/// external URL stays external, a pattern stays a pattern refusal, and a
/// missing/non-server id remains invalid so the authored fallback can fire.
pub(crate) fn classify(servers: &PaintServers<'_>, fragment: &str) -> Result<bool, String> {
    match servers.by_fragment.get(fragment) {
        None | Some(Server::Other) => Ok(false),
        Some(Server::Pattern) => Err(format!(
            "url(#{fragment}) resolves to a <pattern> paint server, which the resolved frame cannot express"
        )),
        Some(Server::Gradient {
            inside_compiled_svg: false,
            ..
        }) => Err(format!(
            "url(#{fragment}) resolves outside the compiled SVG subtree, which contributes nothing"
        )),
        Some(Server::Gradient { .. }) => Ok(true),
    }
}

/// The `<stop>` list, resolved: offset clamped against the running maximum;
/// color and effective alpha are admitted only when the RGBA8 leaf preserves
/// them exactly.
struct ResolvedStop {
    offset: f32,
    color: CGColor,
}

enum GradientKind {
    Linear,
    Radial,
}

/// Resolve one same-document fragment against the table for one consuming
/// geometry. `paint_opacity` is the consumer's `fill-opacity` /
/// `stroke-opacity`; it folds into the gradient's float opacity, or into a
/// solid's alpha with one quantize. `post_paint_opacity` is the later
/// one-draw element factor. A live gradient can carry it separately, while a
/// degenerate gradient may have to refuse before collapsing those stages to
/// one RGBA8 solid.
pub(crate) fn resolve(
    servers: &PaintServers<'_>,
    fragment: &str,
    destination_box: Rectangle,
    reference_space: impl FnOnce() -> Result<Option<(Rectangle, AffineTransform)>, String>,
    bases: GradientBases,
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<ResolvedPaintServer, String> {
    let Some(server) = servers.by_fragment.get(fragment) else {
        return Ok(ResolvedPaintServer::Invalid);
    };
    let (element, inside_compiled_svg) = match server {
        Server::Gradient {
            element,
            inside_compiled_svg,
        } => (*element, *inside_compiled_svg),
        Server::Pattern => {
            return Err(format!(
                "url(#{fragment}) resolves to a <pattern> paint server, which the resolved frame cannot express"
            ));
        }
        Server::Other => return Ok(ResolvedPaintServer::Invalid),
    };
    if !inside_compiled_svg {
        return Err(format!(
            "url(#{fragment}) resolves outside the compiled SVG subtree, which contributes nothing"
        ));
    }
    let chain = template_chain(servers, element);
    let kind = match element.local_name_string().as_str() {
        "linearGradient" => GradientKind::Linear,
        _ => GradientKind::Radial,
    };

    for el in &chain {
        patrol_gradient_element(*el)?;
    }

    let stops = match stops_owner(&chain) {
        None => Vec::new(),
        Some(owner) => resolve_stops(owner)?,
    };
    if stops.is_empty() {
        return Ok(ResolvedPaintServer::Nothing);
    }

    let units = resolve_units(&chain);
    let tile_mode = resolve_spread(&chain);
    let transform = match resolve_gradient_transform(&chain)? {
        GradientTransform::NonInvertible => return Ok(ResolvedPaintServer::Nothing),
        GradientTransform::Affine(affine) => affine,
    };

    if stops.len() == 1 {
        // A one-stop ramp is spatially constant but retains the backend's
        // gradient material route (including dithering and paint-alpha
        // staging). Duplicate the sole resolved stop in a source-neutral
        // constant gradient. Geometry and reference-box mappings are inert for
        // a constant shader; the transform outcome above still decides the
        // measured non-invertible nothing before this branch.
        return Ok(constant_gradient(kind, stops[0].color, paint_opacity));
    }

    match kind {
        GradientKind::Linear => resolve_linear(
            &chain,
            units,
            tile_mode,
            transform,
            stops,
            destination_box,
            reference_space,
            bases,
            paint_opacity,
            post_paint_opacity,
        ),
        GradientKind::Radial => resolve_radial(
            &chain,
            units,
            tile_mode,
            transform,
            stops,
            destination_box,
            reference_space,
            bases,
            paint_opacity,
            post_paint_opacity,
        ),
    }
}

/// One spatially constant gradient that retains gradient rasterization.
fn constant_gradient(kind: GradientKind, color: CGColor, opacity: f32) -> ResolvedPaintServer {
    let stops = vec![
        cg::GradientStop {
            offset: 0.0,
            color: color.into(),
        },
        cg::GradientStop {
            offset: 1.0,
            color: color.into(),
        },
    ];
    let paint = match kind {
        GradientKind::Linear => cg::Paint::LinearGradient(cg::LinearGradientPaint {
            stops,
            opacity,
            ..cg::LinearGradientPaint::default()
        }),
        GradientKind::Radial => cg::Paint::RadialGradient(cg::RadialGradientPaint {
            stops,
            opacity,
            ..cg::RadialGradientPaint::default()
        }),
    };
    ResolvedPaintServer::Gradient(paint)
}

/// The href template chain: the referenced gradient first, then each
/// template in order. A cycle, a missing target, or a non-gradient target
/// ends the chain — the edge dies, the attributes already collected live
/// (measured: a cyclic pair still paints the referenced element's own
/// stops). An external (non-fragment) href refuses.
fn template_chain<'d>(servers: &PaintServers<'d>, first: HtmlElement<'d>) -> Vec<HtmlElement<'d>> {
    let mut chain = vec![first];
    let mut visited: HashSet<NodeId> = HashSet::from([first.node_id()]);
    let mut current = first;
    loop {
        let Some(reference) = gradient_href(current) else {
            break;
        };
        let Some(fragment) = reference.strip_prefix('#') else {
            // An external href template: the edge is not followed. The
            // element's own attributes still resolve; if its meaning
            // depended on the external template, the missing pieces take
            // their initial values exactly as for any dead edge.
            break;
        };
        let Some(Server::Gradient {
            element,
            inside_compiled_svg: _,
        }) = servers.by_fragment.get(fragment)
        else {
            break;
        };
        if !visited.insert(element.node_id()) {
            break;
        }
        chain.push(*element);
        current = *element;
    }
    chain
}

/// `href` beats `xlink:href` when both are present (measured).
fn gradient_href(el: HtmlElement<'_>) -> Option<String> {
    if let DemoNodeData::Element(e) = &el.dom_node().data {
        let mut xlink = None;
        for attr in &e.attrs {
            let local = attr.name.local.as_ref();
            if local != "href" {
                continue;
            }
            if attr.name.ns.as_ref().is_empty() {
                return Some(attr.value.to_string());
            }
            if attr.name.ns.as_ref() == "http://www.w3.org/1999/xlink" && xlink.is_none() {
                xlink = Some(attr.value.to_string());
            }
        }
        return xlink;
    }
    None
}

/// The stops owner: the first chain element with at least one `<stop>`
/// element child. Any own stop suppresses every template stop (measured —
/// there is no merging).
fn stops_owner<'d>(chain: &[HtmlElement<'d>]) -> Option<HtmlElement<'d>> {
    chain.iter().copied().find(|el| {
        let mut child = el.first_element_child();
        while let Some(c) = child {
            if c.local_name_string() == "stop" {
                return true;
            }
            child = c.next_element_sibling();
        }
        false
    })
}

/// Refusals carried by a gradient element itself, independent of geometry:
/// author CSS the cascade cannot represent on its style attribute, and a
/// color interpolation space the backend cannot execute in one ramp.
fn patrol_gradient_element(el: HtmlElement<'_>) -> Result<(), String> {
    patrol_style_attribute_text(el, "a gradient element")?;
    if let Some(value) = get_attr(el, "color-interpolation") {
        let value = trim_svg_whitespace(&value).to_string();
        if value == "linearRGB" {
            return Err(
                "color-interpolation: linearRGB interpolates stops in linear-light sRGB, \
                 which this slice does not execute (sRGB interpolation only)"
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// The properties the pinned cascade cannot represent that change what a
/// gradient paints. A style attribute declaring one refuses the paint by
/// name — Chromium honors the declaration (measured), and silence would be
/// a wrong pixel.
const GRADIENT_STYLE_PROPERTIES_NOT_REPRESENTED: &[&str] =
    &["stop-color", "stop-opacity", "color-interpolation"];

fn patrol_style_attribute_text(el: HtmlElement<'_>, place: &str) -> Result<(), String> {
    let Some(style) = style_attribute_text(el) else {
        return Ok(());
    };
    for declaration in style.split(';') {
        let Some((name, _)) = declaration.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        if GRADIENT_STYLE_PROPERTIES_NOT_REPRESENTED.contains(&name.as_str()) {
            return Err(format!(
                "{place} declares {name} in a style attribute, which this cascade does not \
                 represent"
            ));
        }
    }
    Ok(())
}

fn style_attribute_text(el: HtmlElement<'_>) -> Option<String> {
    get_attr(el, "style")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GradientUnits {
    ObjectBoundingBox,
    UserSpaceOnUse,
}

fn resolve_units(chain: &[HtmlElement<'_>]) -> GradientUnits {
    match chain_attr(chain, "gradientUnits").as_deref() {
        Some("userSpaceOnUse") => GradientUnits::UserSpaceOnUse,
        // Absent and invalid values are the initial objectBoundingBox
        // (SVG2's invalid-attribute rule).
        _ => GradientUnits::ObjectBoundingBox,
    }
}

fn resolve_spread(chain: &[HtmlElement<'_>]) -> cg::TileMode {
    match chain_attr(chain, "spreadMethod").as_deref() {
        Some("reflect") => cg::TileMode::Mirror,
        Some("repeat") => cg::TileMode::Repeated,
        _ => cg::TileMode::Clamp,
    }
}

/// First-present attribute along the chain, whitespace-trimmed.
fn chain_attr(chain: &[HtmlElement<'_>], name: &str) -> Option<String> {
    chain
        .iter()
        .find_map(|el| get_attr(*el, name))
        .map(|value| trim_svg_whitespace(&value).to_string())
}

/// First-present attribute along the chain, counting only elements of the
/// referenced gradient's own type — geometry never crosses gradient types
/// (a radial templated on a linear keeps radial defaults; measured).
fn chain_attr_same_type(chain: &[HtmlElement<'_>], tag: &str, name: &str) -> Option<String> {
    chain
        .iter()
        .filter(|el| el.local_name_string() == tag)
        .find_map(|el| get_attr(*el, name))
        .map(|value| trim_svg_whitespace(&value).to_string())
}

enum GradientTransform {
    Affine(AffineTransform),
    NonInvertible,
}

/// The gradient's computed transform: `gradientTransform` enters the one
/// cascade as the transform property's presentation hint on gradient
/// elements, so the attribute and an author declaration are one computed
/// value with cascade precedence (measured byte-identical). The first chain
/// element whose computed transform is non-empty supplies it (href
/// inheritance of the attribute travels through the hint). Percentages
/// refuse by name; a non-invertible transform paints nothing (measured).
fn resolve_gradient_transform(chain: &[HtmlElement<'_>]) -> Result<GradientTransform, String> {
    for el in chain {
        let Some(data) = el.borrow_data() else {
            continue;
        };
        let style: &ComputedValues = data.styles.primary();
        let transform = style.clone_transform();
        if transform.0.is_empty() {
            continue;
        }
        let affine = computed_transform_to_affine(&transform, None).map_err(|refusal| {
            match refusal {
                TransformRefusal::Function(name) => format!(
                    "the gradient transform uses {name}(), which is outside the 2D affine \
                     function set this slice consumes"
                ),
                TransformRefusal::Calc => {
                    "the gradient transform uses calc(), which this slice does not resolve"
                        .to_string()
                }
                TransformRefusal::Percentage => {
                    // Measured: Chromium resolves the percentage against the
                    // viewport and applies the raw number in the gradient's
                    // fraction space — an incoherence this slice refuses.
                    "the gradient transform uses a percentage translation, which Chromium \
                     resolves against mismatched spaces; this slice refuses it"
                        .to_string()
                }
            }
        })?;
        if !affine.matrix.into_iter().flatten().all(f32::is_finite) {
            return Err("the gradient transform is not finite".to_string());
        }
        let [[a, c, _], [b, d, _]] = affine.matrix;
        let determinant = f64::from(a) * f64::from(d) - f64::from(b) * f64::from(c);
        if determinant == 0.0 || !determinant.is_finite() {
            return Ok(GradientTransform::NonInvertible);
        }
        return Ok(GradientTransform::Affine(affine));
    }
    Ok(GradientTransform::Affine(AffineTransform::identity()))
}

/// One gradient geometry length: a plain number, a `px` length (its
/// number), or a percentage. Other units refuse — font-relative and
/// viewport-relative bases are outside this slice, and in
/// objectBoundingBox units no spec defines them at all.
fn gradient_length(
    chain: &[HtmlElement<'_>],
    tag: &str,
    name: &str,
    units: GradientUnits,
    bases: GradientBases,
) -> Result<Option<f32>, String> {
    let Some(text) = chain_attr_same_type(chain, tag, name) else {
        return Ok(None);
    };
    let trimmed = text.as_str();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let (number_text, percent) = match trimmed.strip_suffix('%') {
        Some(number) => (number, true),
        None => (trimmed.strip_suffix("px").unwrap_or(trimmed), false),
    };
    if !dots_carry_digits(number_text) {
        // Invalid number: the attribute is in error and takes its initial
        // value (as-if-absent).
        return Ok(None);
    }
    let Ok(value) = number_text.parse::<f32>() else {
        if number_text
            .bytes()
            .any(|byte| byte.is_ascii_alphabetic() && byte != b'e' && byte != b'E')
        {
            return Err(format!(
                "gradient geometry {name}=\"{text}\" uses a unit whose basis this slice does \
                 not consume (numbers, px, and percentages only)"
            ));
        }
        return Ok(None);
    };
    if !value.is_finite() {
        return Ok(None);
    }
    let resolved = if percent {
        match units {
            GradientUnits::ObjectBoundingBox => value / 100.0,
            GradientUnits::UserSpaceOnUse => value / 100.0 * bases.axis(name),
        }
    } else {
        value
    };
    Ok(Some(resolved))
}

/// Resolve the `<stop>` children: offset against the running maximum, with
/// only stop alpha whose effective value survives the resolved RGBA8 contract
/// exactly.
fn resolve_stops(owner: HtmlElement<'_>) -> Result<Vec<ResolvedStop>, String> {
    let mut stops = Vec::new();
    let mut running_max = 0.0f32;
    let mut child = owner.first_element_child();
    while let Some(stop) = child {
        child = stop.next_element_sibling();
        if stop.local_name_string() != "stop" {
            continue;
        }
        patrol_style_attribute_text(stop, "a gradient <stop>")?;
        let offset = stop_offset(stop);
        let offset = offset.clamp(0.0, 1.0).max(running_max);
        running_max = offset;
        let opacity = stop_opacity(stop);
        let color = stop_color(stop)?;
        // Chromium resolves a stop color's own alpha to its byte-equivalent,
        // then multiplies `stop-opacity` in float. The present cg/rframe stop
        // leaf can carry the result only when it lands exactly on a byte.
        let base_alpha = (color.alpha.clamp(0.0, 1.0) * 255.0).round() / 255.0;
        let effective_alpha = base_alpha * opacity;
        if !rgba8_exact(effective_alpha) {
            return Err(
                "resolved gradient stop alpha loses float precision at the RGBA8 paint contract"
                    .to_string(),
            );
        }
        let color = admitted_srgb(color, opacity)
            .map_err(|reason| format!("a gradient <stop> is outside the slice: {reason}"))?;
        stops.push(ResolvedStop { offset, color });
    }
    Ok(stops)
}

/// Whether one clamped alpha round-trips through the frame's RGBA8 color leaf
/// without changing its value. Chromium preserves `stop-opacity` as a float
/// into the gradient shader; accepting a value that fails this check would
/// silently substitute a neighboring alpha byte.
fn rgba8_exact(component: f32) -> bool {
    let component = component.clamp(0.0, 1.0);
    ((component * 255.0).round() / 255.0) == component
}

/// `offset`: a number or percentage; an invalid value is 0 (measured).
fn stop_offset(stop: HtmlElement<'_>) -> f32 {
    let Some(text) = get_attr(stop, "offset") else {
        return 0.0;
    };
    let trimmed = trim_svg_whitespace(&text);
    let (number_text, percent) = match trimmed.strip_suffix('%') {
        Some(number) => (number, true),
        None => (trimmed, false),
    };
    if !dots_carry_digits(number_text) {
        return 0.0;
    }
    let Ok(value) = number_text.parse::<f32>() else {
        return 0.0;
    };
    if !value.is_finite() {
        return 0.0;
    }
    if percent { value / 100.0 } else { value }
}

/// `stop-opacity`: an alpha value (number or percentage), clamped; an
/// invalid or absent value is the initial 1.
fn stop_opacity(stop: HtmlElement<'_>) -> f32 {
    let Some(text) = get_attr(stop, "stop-opacity") else {
        return 1.0;
    };
    let trimmed = trim_svg_whitespace(&text);
    let (number_text, percent) = match trimmed.strip_suffix('%') {
        Some(number) => (number, true),
        None => (trimmed, false),
    };
    if !dots_carry_digits(number_text) {
        return 1.0;
    }
    let Ok(value) = number_text.parse::<f32>() else {
        return 1.0;
    };
    if !value.is_finite() {
        return 1.0;
    }
    let value = if percent { value / 100.0 } else { value };
    value.clamp(0.0, 1.0)
}

/// `stop-color`: the attribute's CSS `<color>` value. `currentColor`
/// resolves against the stop's own computed `color` (inheritance through
/// the gradient's ancestor chain — measured, the referencing element's
/// `color` is irrelevant). An unparseable value — the `inherit` keyword
/// included — is the initial black, which is also what Chromium paints for
/// `inherit` (the parent gradient's `stop-color` is initial black).
fn stop_color(stop: HtmlElement<'_>) -> Result<AbsoluteColor, String> {
    let Some(text) = get_attr(stop, "stop-color") else {
        return Ok(AbsoluteColor::BLACK);
    };
    match parse_color_attribute(&text) {
        Some(ParsedStopColor::Absolute(color)) => Ok(color),
        Some(ParsedStopColor::CurrentColor) => {
            let Some(data) = stop.borrow_data() else {
                return Ok(AbsoluteColor::BLACK);
            };
            let style: &ComputedValues = data.styles.primary();
            Ok(style.clone_color())
        }
        Some(ParsedStopColor::BeyondSlice(reason)) => Err(format!(
            "a gradient <stop> color is outside the slice: {reason}"
        )),
        None => Ok(AbsoluteColor::BLACK),
    }
}

/// What a parsed `stop-color` attribute value can be.
enum ParsedStopColor {
    Absolute(AbsoluteColor),
    CurrentColor,
    /// Parses as CSS but needs resolution machinery outside the slice
    /// (color-mix, light-dark, contrast-color …).
    BeyondSlice(&'static str),
}

/// Parse one `stop-color` attribute value with the same CSS `<color>`
/// parser the cascade uses for the `color` property. `None` is an
/// unparseable value — including the CSS-wide keywords, whose wide-keyword
/// declaration this attribute read deliberately does not honor (the
/// measured `inherit` meaning coincides with the initial black).
fn parse_color_attribute(text: &str) -> Option<ParsedStopColor> {
    let mut source = SourcePropertyDeclaration::default();
    let url_data = UrlExtraData::from(Url::parse("about:blank").unwrap());
    parse_one_declaration_into(
        &mut source,
        PropertyId::NonCustom(LonghandId::Color.into()),
        text,
        Origin::Author,
        &url_data,
        None,
        ParsingMode::DEFAULT,
        StyleQuirksMode::NoQuirks,
        CssRuleType::Style,
    )
    .ok()?;
    let mut block = PropertyDeclarationBlock::new();
    block.extend(source.drain(), Importance::Normal);
    let declaration = block.declarations().first()?;
    let PropertyDeclaration::Color(value) = declaration else {
        return None;
    };
    Some(match &value.0 {
        SpecifiedColor::CurrentColor => ParsedStopColor::CurrentColor,
        SpecifiedColor::Absolute(absolute) => ParsedStopColor::Absolute(absolute.color),
        SpecifiedColor::ColorFunction(_) => {
            ParsedStopColor::BeyondSlice("an unresolved color function")
        }
        SpecifiedColor::ColorMix(_) => ParsedStopColor::BeyondSlice("color-mix()"),
        SpecifiedColor::LightDark(_) => ParsedStopColor::BeyondSlice("light-dark()"),
        SpecifiedColor::ContrastColor(_) => ParsedStopColor::BeyondSlice("contrast-color()"),
        SpecifiedColor::System(_) => ParsedStopColor::BeyondSlice("a system color"),
        SpecifiedColor::InheritFromBodyQuirk => ParsedStopColor::BeyondSlice("a quirks-mode color"),
    })
}

fn solid_paint(
    color: CGColor,
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<ResolvedPaintServer, String> {
    // A live gradient keeps stop alpha in its shader and the consumer's
    // fill/stroke opacity in the paint. Collapsing a degenerate gradient to
    // one RGBA8 solid is lossless only when at least one alpha stage is an
    // endpoint; otherwise the frame would silently flatten two raster stages.
    let paint_alpha = (paint_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    if post_paint_opacity != 1.0 && !matches!(color.a, 0 | 255) {
        return Err(
            "resolved gradient stop alpha loses staged precision when a degenerate paint server collapses before post-paint opacity"
                .to_string(),
        );
    }
    let alpha = match (color.a, paint_alpha) {
        (0, _) | (_, 0) => 0,
        (255, paint) => paint,
        (stop, 255) => stop,
        _ => {
            return Err(
                "resolved gradient stop alpha loses staged precision when a degenerate paint server collapses to RGBA8"
                    .to_string(),
            );
        }
    };
    let color = CGColor { a: alpha, ..color };
    if color.a == 0 {
        return Ok(ResolvedPaintServer::Nothing);
    }
    Ok(ResolvedPaintServer::Solid(color))
}

/// The integral average of the ramp — the backend's degenerate color for
/// `reflect`/`repeat` (measured through Chromium, which shares the
/// backend's rule): the piecewise-linear ramp integrated over `[0, 1]`
/// with edge plateaus, per channel in unpremultiplied float, quantized
/// once.
fn ramp_average(
    stops: &[ResolvedStop],
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<CGColor, String> {
    let channels = |stop: &ResolvedStop| {
        [
            f32::from(stop.color.r),
            f32::from(stop.color.g),
            f32::from(stop.color.b),
            f32::from(stop.color.a),
        ]
    };
    let first = &stops[0];
    let last = &stops[stops.len() - 1];
    let mut sum = [0.0f32; 4];
    let first_channels = channels(first);
    let last_channels = channels(last);
    for (index, value) in first_channels.iter().enumerate() {
        sum[index] += value * first.offset;
    }
    for pair in stops.windows(2) {
        let width = pair[1].offset - pair[0].offset;
        let a = channels(&pair[0]);
        let b = channels(&pair[1]);
        for index in 0..4 {
            sum[index] += (a[index] + b[index]) * 0.5 * width;
        }
    }
    for (index, value) in last_channels.iter().enumerate() {
        sum[index] += value * (1.0 - last.offset);
    }
    if sum[3].round() != sum[3] {
        return Err(
            "resolved gradient stop alpha loses float precision when a degenerate ramp average collapses to RGBA8"
                .to_string(),
        );
    }
    let average_visible = sum[3] > 0.0 && paint_opacity > 0.0 && post_paint_opacity > 0.0;
    if average_visible
        && (sum[3] != 255.0 || paint_opacity != 1.0 || post_paint_opacity != 1.0)
        && sum[..3]
            .iter()
            .any(|component| component.round() != *component)
    {
        return Err(
            "resolved gradient stop color loses float precision when a degenerate ramp average collapses to RGBA8"
                .to_string(),
        );
    }
    Ok(CGColor {
        r: sum[0].round() as u8,
        g: sum[1].round() as u8,
        b: sum[2].round() as u8,
        a: sum[3].round() as u8,
    })
}

fn cg_stops(stops: &[ResolvedStop]) -> Vec<cg::GradientStop> {
    stops
        .iter()
        .map(|stop| cg::GradientStop {
            offset: stop.offset,
            color: stop.color.into(),
        })
        .collect()
}

/// The inverse of the consumer geometry's box transform
/// `translate(x, y) · scale(w, h)`. It exists because zero-area boxes
/// resolved earlier.
fn box_inverse(rect: Rectangle) -> AffineTransform {
    AffineTransform::from_acebdf(
        1.0 / rect.width,
        0.0,
        -rect.x / rect.width,
        0.0,
        1.0 / rect.height,
        -rect.y / rect.height,
    )
}

/// The affine from the unit square into a non-degenerate geometry box.
fn box_transform(rect: Rectangle) -> AffineTransform {
    AffineTransform::from_acebdf(rect.width, 0.0, rect.x, 0.0, rect.height, rect.y)
}

/// `first × second` as one mapping: apply `second` to the point, then
/// `first` (`compose` is the plain matrix product `self × other`).
fn concat(first: &AffineTransform, second: &AffineTransform) -> AffineTransform {
    first.compose(second)
}

#[allow(clippy::too_many_arguments)]
fn resolve_linear(
    chain: &[HtmlElement<'_>],
    units: GradientUnits,
    tile_mode: cg::TileMode,
    transform: AffineTransform,
    stops: Vec<ResolvedStop>,
    destination_box: Rectangle,
    reference_space: impl FnOnce() -> Result<Option<(Rectangle, AffineTransform)>, String>,
    bases: GradientBases,
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<ResolvedPaintServer, String> {
    let read = |name: &str| gradient_length(chain, "linearGradient", name, units, bases);
    let default_fraction = |value: f32| match units {
        GradientUnits::ObjectBoundingBox => value,
        GradientUnits::UserSpaceOnUse => 0.0,
    };
    let x1 = read("x1")?.unwrap_or(0.0);
    let y1 = read("y1")?.unwrap_or(0.0);
    let x2 = read("x2")?.unwrap_or(match units {
        GradientUnits::ObjectBoundingBox => 1.0,
        GradientUnits::UserSpaceOnUse => bases.width,
    });
    let y2 = read("y2")?.unwrap_or(default_fraction(0.0));

    // The backend substitutes degenerate behavior at or below its
    // threshold, tested in the gradient's own coordinate space — resolve
    // the measured meaning here instead (pad: the last stop; reflect and
    // repeat: the ramp's integral average).
    let (dx, dy) = (x2 - x1, y2 - y1);
    let distance = (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)).sqrt() as f32;
    if !distance.is_finite() || distance <= DEGENERATE_LINEAR_THRESHOLD {
        let color = match tile_mode {
            cg::TileMode::Clamp => stops[stops.len() - 1].color,
            _ => ramp_average(&stops, paint_opacity, post_paint_opacity)?,
        };
        return solid_paint(color, paint_opacity, post_paint_opacity);
    }

    // Only an actual ramp needs owner geometry and a mapping into the leaf.
    // A singular context consumer paints nothing; no gradient fact crosses
    // the resolved contract.
    let Some((reference_box, reference_to_destination)) = reference_space()? else {
        return Ok(ResolvedPaintServer::Nothing);
    };
    if matches!(units, GradientUnits::ObjectBoundingBox)
        && (reference_box.width <= 0.0 || reference_box.height <= 0.0)
    {
        return Ok(ResolvedPaintServer::Nothing);
    }

    let direct_reference =
        destination_box == reference_box && reference_to_destination == AffineTransform::identity();
    let paint = match units {
        GradientUnits::ObjectBoundingBox if direct_reference => cg::LinearGradientPaint {
            active: true,
            xy1: cg::Alignment(x1 * 2.0 - 1.0, y1 * 2.0 - 1.0),
            xy2: cg::Alignment(x2 * 2.0 - 1.0, y2 * 2.0 - 1.0),
            tile_mode,
            transform,
            stops: cg_stops(&stops),
            opacity: paint_opacity,
            blend_mode: cg::BlendMode::Normal,
        },
        GradientUnits::ObjectBoundingBox => {
            // Context paint makes the reference box and the destination
            // paint box different. Build the ramp in the eventual owner's
            // box, move it into the destination element's local space, then
            // return it to that element's unit box. For an ordinary direct
            // paint all three extra matrices cancel to identity, preserving
            // the original objectBoundingBox lowering exactly.
            let transform = concat(
                &box_inverse(destination_box),
                &concat(
                    &reference_to_destination,
                    &concat(&box_transform(reference_box), &transform),
                ),
            );
            cg::LinearGradientPaint {
                active: true,
                // Endpoints are bbox fractions; the box maps them, and level
                // lines skew with a non-square box exactly as measured.
                xy1: cg::Alignment(x1 * 2.0 - 1.0, y1 * 2.0 - 1.0),
                xy2: cg::Alignment(x2 * 2.0 - 1.0, y2 * 2.0 - 1.0),
                tile_mode,
                transform,
                stops: cg_stops(&stops),
                opacity: paint_opacity,
                blend_mode: cg::BlendMode::Normal,
            }
        }
        GradientUnits::UserSpaceOnUse => {
            // User space: the ramp's level lines are perpendicular to the
            // gradient vector in *user* units, so the unit segment maps to
            // the user segment through a similarity, and the box inverse
            // returns the whole mapping to the contract's unit square.
            let similarity = AffineTransform::from_acebdf(dx, -dy, x1, dy, dx, y1);
            let transform = concat(
                &box_inverse(destination_box),
                &concat(&reference_to_destination, &concat(&transform, &similarity)),
            );
            cg::LinearGradientPaint {
                active: true,
                xy1: cg::Alignment(-1.0, -1.0),
                xy2: cg::Alignment(1.0, -1.0),
                tile_mode,
                transform,
                stops: cg_stops(&stops),
                opacity: paint_opacity,
                blend_mode: cg::BlendMode::Normal,
            }
        }
    };
    Ok(ResolvedPaintServer::Gradient(cg::Paint::LinearGradient(
        paint,
    )))
}

#[allow(clippy::too_many_arguments)]
fn resolve_radial(
    chain: &[HtmlElement<'_>],
    units: GradientUnits,
    tile_mode: cg::TileMode,
    transform: AffineTransform,
    stops: Vec<ResolvedStop>,
    destination_box: Rectangle,
    reference_space: impl FnOnce() -> Result<Option<(Rectangle, AffineTransform)>, String>,
    bases: GradientBases,
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<ResolvedPaintServer, String> {
    let read = |name: &str| gradient_length(chain, "radialGradient", name, units, bases);
    let half = |basis: f32| match units {
        GradientUnits::ObjectBoundingBox => 0.5,
        GradientUnits::UserSpaceOnUse => basis * 0.5,
    };
    let cx = read("cx")?.unwrap_or(half(bases.width));
    let cy = read("cy")?.unwrap_or(half(bases.height));
    let r = read("r")?.unwrap_or(match units {
        GradientUnits::ObjectBoundingBox => 0.5,
        GradientUnits::UserSpaceOnUse => bases.diagonal() * 0.5,
    });
    let fx = read("fx")?.unwrap_or(cx);
    let fy = read("fy")?.unwrap_or(cy);
    let fr = read("fr")?.unwrap_or(0.0);

    if fx != cx || fy != cy || fr > 0.0 {
        return Err(
            "the radial gradient has a focal point or focal radius, which the shared \
             radial paint leaf cannot state (concentric radials only)"
                .to_string(),
        );
    }

    if r <= 0.0 {
        // A non-positive radius reaches the same tile-specific backend
        // degeneracy as a collapsed linear ramp. Measured at zero: clamp is
        // the last stop, while repeat/reflect are the ramp's integral average.
        // A negative authored radius likewise does not fall back to the
        // default positive radius.
        let color = match tile_mode {
            cg::TileMode::Clamp => stops[stops.len() - 1].color,
            _ => ramp_average(&stops, paint_opacity, post_paint_opacity)?,
        };
        return solid_paint(color, paint_opacity, post_paint_opacity);
    }

    // A non-positive radius is already a source-neutral solid. Only a live
    // radial needs the context box/space.
    let Some((reference_box, reference_to_destination)) = reference_space()? else {
        return Ok(ResolvedPaintServer::Nothing);
    };
    if matches!(units, GradientUnits::ObjectBoundingBox)
        && (reference_box.width <= 0.0 || reference_box.height <= 0.0)
    {
        return Ok(ResolvedPaintServer::Nothing);
    }

    // The unit circle (center ½,½, radius ½) maps to the resolved circle
    // through a similarity; objectBoundingBox composes in fraction space,
    // user space returns through the box inverse.
    let scale = 2.0 * r;
    let similarity = AffineTransform::from_acebdf(scale, 0.0, cx - r, 0.0, scale, cy - r);
    let direct_reference =
        destination_box == reference_box && reference_to_destination == AffineTransform::identity();
    let transform = match units {
        GradientUnits::ObjectBoundingBox if direct_reference => concat(&transform, &similarity),
        GradientUnits::ObjectBoundingBox => concat(
            &box_inverse(destination_box),
            &concat(
                &reference_to_destination,
                &concat(
                    &box_transform(reference_box),
                    &concat(&transform, &similarity),
                ),
            ),
        ),
        GradientUnits::UserSpaceOnUse => concat(
            &box_inverse(destination_box),
            &concat(&reference_to_destination, &concat(&transform, &similarity)),
        ),
    };
    Ok(ResolvedPaintServer::Gradient(cg::Paint::RadialGradient(
        cg::RadialGradientPaint {
            active: true,
            transform,
            stops: cg_stops(&stops),
            opacity: paint_opacity,
            blend_mode: cg::BlendMode::Normal,
            tile_mode,
        },
    )))
}

/// Whether a resolved paint-server URL is a same-document fragment, and
/// its fragment. `None` means external.
pub(crate) fn same_document_fragment(url: &::url::Url) -> Option<&str> {
    let fragment = url.fragment()?;
    let mut without_fragment = url.clone();
    without_fragment.set_fragment(None);
    let serialized = without_fragment.as_str();
    SAME_DOCUMENT_URL_BASES
        .contains(&serialized)
        .then_some(fragment)
}
