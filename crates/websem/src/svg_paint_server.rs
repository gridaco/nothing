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
use style::color::{AbsoluteColor, ColorFlags, ColorSpace};
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

use cg::{CGColor, CGColor32F};
use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::svg::{
    WEB_USED_LENGTH_MAX, WEB_USED_LENGTH_MIN, admitted_srgb, dots_carry_digits,
    geometry_number_source_loses_provenance, get_attr, resolve_geometry_percentage,
    trim_svg_whitespace,
};
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
    /// A pattern is retained with its element because it resolves per
    /// consuming geometry into a checked repeating program. Keeping it in the
    /// same first-id table as gradients is load-bearing: otherwise a pattern
    /// in `<defs>` looks exactly like a missing id and silently becomes
    /// fallback/no-paint.
    Pattern {
        element: HtmlElement<'d>,
        inside_compiled_svg: bool,
    },
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
                "pattern" => Server::Pattern {
                    element: el,
                    inside_compiled_svg: is_inside(el, compiled_svg),
                },
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

/// Whole-document classification of one same-document paint-server id.
#[derive(Clone, Copy)]
pub(crate) enum ClassifiedServer<'d> {
    Invalid,
    Gradient,
    Pattern(HtmlElement<'d>),
}

/// Classification that must happen before context-box rebasing. It preserves
/// each construct's own outcome when a context relation selects it: an
/// external URL stays external, a pattern stays a pattern refusal, and a
/// missing/non-server id remains invalid so the authored fallback can fire.
pub(crate) fn classify<'d>(
    servers: &PaintServers<'d>,
    fragment: &str,
) -> Result<ClassifiedServer<'d>, String> {
    match servers.by_fragment.get(fragment) {
        None | Some(Server::Other) => Ok(ClassifiedServer::Invalid),
        Some(Server::Pattern {
            inside_compiled_svg: false,
            ..
        }) => Err(format!(
            "url(#{fragment}) resolves outside the compiled SVG subtree, which contributes nothing"
        )),
        Some(Server::Pattern { element, .. }) => Ok(ClassifiedServer::Pattern(*element)),
        Some(Server::Gradient {
            inside_compiled_svg: false,
            ..
        }) => Err(format!(
            "url(#{fragment}) resolves outside the compiled SVG subtree, which contributes nothing"
        )),
        Some(Server::Gradient { .. }) => Ok(ClassifiedServer::Gradient),
    }
}

/// Resolve a template-chain edge only when its first-id target is another
/// in-subtree pattern. A wrong-type or missing edge dies; crossing outside the
/// compiled SVG is a named boundary rather than a partial template.
pub(crate) fn pattern_template<'d>(
    servers: &PaintServers<'d>,
    fragment: &str,
) -> Result<Option<HtmlElement<'d>>, String> {
    match servers.by_fragment.get(fragment) {
        Some(Server::Pattern {
            element,
            inside_compiled_svg: true,
        }) => Ok(Some(*element)),
        Some(Server::Pattern {
            inside_compiled_svg: false,
            ..
        }) => Err(format!(
            "pattern template #{fragment} resolves outside the compiled SVG subtree"
        )),
        _ => Ok(None),
    }
}

/// The `<stop>` list, resolved: offset clamped against the running maximum,
/// and one colour whose RGB is the admitted sRGB byte triple and whose alpha
/// is the float product Chromium hands the ramp.
struct ResolvedStop {
    offset: f32,
    color: CGColor32F,
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
        Server::Pattern { .. } => {
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

    // An object-bounding-box paint server has no coordinate system when
    // either consumer-box extent is zero. Chromium paints nothing before
    // considering whether the ramp itself is constant or degenerate. User
    // space has no such dependency and reaches the source-neutral branches
    // below before its live-ramp contract boundary.
    if matches!(units, GradientUnits::ObjectBoundingBox)
        && (destination_box.width == 0.0 || destination_box.height == 0.0)
    {
        return Ok(ResolvedPaintServer::Nothing);
    }

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
fn constant_gradient(kind: GradientKind, color: CGColor32F, opacity: f32) -> ResolvedPaintServer {
    let stops = vec![
        cg::GradientStop { offset: 0.0, color },
        cg::GradientStop { offset: 1.0, color },
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
        let Some(reference) = paint_server_href(current) else {
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
pub(crate) fn paint_server_href(el: HtmlElement<'_>) -> Option<String> {
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

fn strip_ascii_case_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    let split = text.len().checked_sub(suffix.len())?;
    text.get(split..)?
        .eq_ignore_ascii_case(suffix)
        .then(|| &text[..split])
}

/// One gradient geometry length: a plain number, a `px` length (its
/// number), or a percentage. Other units refuse — font-relative and
/// viewport-relative bases are outside this slice, and in
/// objectBoundingBox units no spec defines them at all.
///
/// This remains a direct attribute decoder because the pinned Stylo build
/// carries no gradient-coordinate longhands. It therefore patrols every
/// source class whose raw `f32` route cannot prove Blink's CSS-number
/// provenance, and every value that would cross the checked frame/backend
/// range, before a paint fact is emitted. The shadow number routes classify
/// only; neither supplies a value.
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
    if trimmed.contains("/*") || trimmed.contains("*/") {
        return Err(format!(
            "gradient geometry {name} contains a CSS comment this direct length parser cannot tokenize"
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "initial" | "inherit" | "unset" | "revert" | "revert-layer"
    ) {
        return Err(format!(
            "gradient geometry {name} uses the CSS-wide value {trimmed}, whose resource-side cascade is not represented at this Stylo pin"
        ));
    }
    if let Some((function, _)) = lower.split_once('(') {
        return Err(format!(
            "gradient geometry {name} uses {}(), whose computed length is not represented by the direct resource decoder",
            function.trim()
        ));
    }
    let (number_text, percent) = match trimmed.strip_suffix('%') {
        Some(number) => (number, true),
        None => (
            strip_ascii_case_suffix(trimmed, "px").unwrap_or(trimmed),
            false,
        ),
    };
    if !dots_carry_digits(number_text) {
        // Invalid number: the attribute is in error and takes its initial
        // value (as-if-absent).
        return Ok(None);
    }
    let Ok(value) = number_text.parse::<f32>() else {
        if number_text.parse::<f64>().is_ok() {
            return Err(format!(
                "gradient geometry {name} exceeds the admitted Web used-value range"
            ));
        }
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
        return Err(format!(
            "gradient geometry {name} exceeds the admitted Web used-value range"
        ));
    }
    if geometry_number_source_loses_provenance(number_text, percent) {
        return Err(format!(
            "gradient geometry {name} numeric precision alias loses Chromium used-value provenance"
        ));
    }
    let resolved = if percent {
        match units {
            GradientUnits::ObjectBoundingBox => resolve_geometry_percentage(value, 1.0),
            GradientUnits::UserSpaceOnUse => resolve_geometry_percentage(value, bases.axis(name)),
        }
    } else {
        value
    };
    if !resolved.is_finite() || !(WEB_USED_LENGTH_MIN..=WEB_USED_LENGTH_MAX).contains(&resolved) {
        return Err(format!(
            "gradient geometry {name} exceeds the admitted Web used-value range"
        ));
    }
    Ok(Some(resolved))
}

/// Resolve the `<stop>` children: offset against the running maximum, and one
/// colour per stop.
///
/// Chromium resolves a stop colour's own alpha to its byte equivalent
/// (measured: `rgb(22 163 74 / 0.5)` is byte-identical to `#16a34a80`), then
/// multiplies `stop-opacity` in float and hands the product to the ramp
/// unquantized (measured: `stop-opacity="0.5"` differs from both 127/255 and
/// 128/255). The resolved stop carries exactly that: byte RGB, float alpha.
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
        let opacity = stop_opacity(stop)?;
        let color = stop_color(stop)?;
        let base_alpha = (color.alpha.clamp(0.0, 1.0) * 255.0).round() / 255.0;
        let rgb = admitted_srgb(color, 1.0)
            .map_err(|reason| format!("a gradient <stop> is outside the slice: {reason}"))?;
        let color = CGColor32F::from_rgb8_alpha(rgb, (base_alpha * opacity).clamp(0.0, 1.0))
            .map_err(|error| format!("a gradient <stop> colour is unusable: {error}"))?;
        stops.push(ResolvedStop { offset, color });
    }
    Ok(stops)
}

/// The spellings a `<stop>` presentation attribute can hide behind, each
/// measured painting a silently wrong pixel before this patrol existed.
///
/// The pinned cascade has no `stop-color`/`stop-opacity` longhand, so these
/// two attributes are read directly rather than cascaded — which means no
/// resolver runs over them, and any value that needs one has to refuse
/// instead of falling back to the initial. Each refusal names a construct
/// that carries its own checklist row, so the two attribute rows tick over
/// them (the gridaco/nothing#75/#80 own-row precedent):
///
/// - **`inherit`**: measured to take the ancestor gradient's own computed
///   value (`<linearGradient stop-color="red"><stop stop-color="inherit">`
///   paints red; the engine painted the initial black, 4096 px at Δ253).
///   `initial`, `unset` and `revert` all coincide with the initial here and
///   are admitted (measured identical).
/// - **`var()`**: measured substituted in a presentation attribute
///   (`--o: 0.25` reached the stop; the engine painted 1, 4096 px at Δ190).
///   Which declaration feeds the substitution is a resolver question.
/// - **CSS escapes**: a spelling this scan cannot read at all.
fn patrol_stop_attribute(text: &str, attribute: &str) -> Result<(), String> {
    if text.contains('\\') {
        return Err(format!(
            "a <stop> {attribute} carries a CSS escape this patrol cannot read"
        ));
    }
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("var(") {
        return Err(format!(
            "a <stop> {attribute} resolves through var(), an indirection this patrol cannot \
             follow"
        ));
    }
    if trim_svg_whitespace(&lowered) == "inherit" {
        return Err(format!(
            "a <stop> {attribute} is inherit, which needs a cascaded longhand this build does \
             not have"
        ));
    }
    Ok(())
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

/// `stop-opacity`: SVG 2 gives it the `opacity` property's own grammar
/// (`<number> | <percentage>`), clamped to `[0, 1]`; an invalid or absent
/// value is the initial 1 (measured: `initial`, `unset` and `revert` are all
/// byte-identical to `1`).
///
/// A CSS math function is valid there and Chromium evaluates it (measured:
/// `calc(1 / 3)` is byte-identical to the literal third, while the engine
/// painted 1 — 4096 px at Δ169). Evaluating one here would need a computation
/// context this build does not construct, and re-implementing the fold would
/// be a second matcher, so any function spelling refuses by name on the
/// `calc()` family's own checklist rows. The patrol is one-way: it reads the
/// plain grammar and refuses everything else it cannot prove invalid.
fn stop_opacity(stop: HtmlElement<'_>) -> Result<f32, String> {
    let Some(text) = get_attr(stop, "stop-opacity") else {
        return Ok(1.0);
    };
    patrol_stop_attribute(&text, "stop-opacity")?;
    let trimmed = trim_svg_whitespace(&text);
    let (number_text, percent) = match trimmed.strip_suffix('%') {
        Some(number) => (number, true),
        None => (trimmed, false),
    };
    let plain = dots_carry_digits(number_text)
        .then(|| number_text.parse::<f32>().ok())
        .flatten()
        .filter(|value| value.is_finite());
    let Some(value) = plain else {
        if trimmed.contains('(') {
            return Err(
                "a <stop> stop-opacity is a function this build cannot evaluate without a \
                 computation context"
                    .to_string(),
            );
        }
        return Ok(1.0);
    };
    let value = if percent { value / 100.0 } else { value };
    Ok(value.clamp(0.0, 1.0))
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
    patrol_stop_attribute(&text, "stop-color")?;
    match parse_color_attribute(&text) {
        Some(ParsedColorAttribute::Absolute(color)) => admitted_legacy_srgb(color),
        Some(ParsedColorAttribute::CurrentColor) => {
            let Some(data) = stop.borrow_data() else {
                return Ok(AbsoluteColor::BLACK);
            };
            let style: &ComputedValues = data.styles.primary();
            admitted_legacy_srgb(style.clone_color())
        }
        Some(ParsedColorAttribute::BeyondSlice(reason)) => Err(format!(
            "a gradient <stop> color is outside the slice: {reason}"
        )),
        None => Ok(AbsoluteColor::BLACK),
    }
}

/// A stop colour must be a *legacy* sRGB colour — the kind hex, a named
/// colour, `transparent` and `rgb()`/`rgba()` produce.
///
/// A non-legacy sRGB colour parses to the same colour space and would look
/// admissible, but it changes what Chromium does with the whole ramp, not
/// just this endpoint: a ramp from `color(srgb 0.00196078431372549 0 0)`
/// measured 4080 px away from the same ramp started at either neighbouring
/// byte, at Δ26 — far more than an endpoint's rounding. The same value as a
/// *solid* is byte-identical to `#010000`, so this is a stop-only rule and
/// the ordinary paint path keeps its own admission.
///
/// `color()` and the other non-legacy colour functions carry their own
/// checklist rows, so this refusal does not block the `stop-color` row.
fn admitted_legacy_srgb(color: AbsoluteColor) -> Result<AbsoluteColor, String> {
    if color.color_space == ColorSpace::Srgb && !color.flags.contains(ColorFlags::IS_LEGACY_SRGB) {
        return Err(
            "a gradient <stop> colour is a non-legacy sRGB colour, which changes how Chromium \
             interpolates the whole ramp"
                .to_string(),
        );
    }
    Ok(color)
}

/// What a parsed `stop-color` attribute value can be.
pub(crate) enum ParsedColorAttribute {
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
pub(crate) fn parse_color_attribute(text: &str) -> Option<ParsedColorAttribute> {
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
        SpecifiedColor::CurrentColor => ParsedColorAttribute::CurrentColor,
        SpecifiedColor::Absolute(absolute) => ParsedColorAttribute::Absolute(absolute.color),
        SpecifiedColor::ColorFunction(_) => {
            ParsedColorAttribute::BeyondSlice("an unresolved color function")
        }
        SpecifiedColor::ColorMix(_) => ParsedColorAttribute::BeyondSlice("color-mix()"),
        SpecifiedColor::LightDark(_) => ParsedColorAttribute::BeyondSlice("light-dark()"),
        SpecifiedColor::ContrastColor(_) => ParsedColorAttribute::BeyondSlice("contrast-color()"),
        SpecifiedColor::System(_) => ParsedColorAttribute::BeyondSlice("a system color"),
        SpecifiedColor::InheritFromBodyQuirk => {
            ParsedColorAttribute::BeyondSlice("a quirks-mode color")
        }
    })
}

/// The colour a degenerate paint server substitutes, folded with the
/// consumer's opacity stages into one solid.
///
/// **This is the rung's open gap, and it is a gradient-geometry gap rather
/// than a stop-grammar one.** Chromium does not paint a degenerate paint
/// server as a flat colour: it keeps a shader, and that shader dithers. When
/// every stage lands on a byte there is nothing to dither and the flat solid
/// is byte-identical (measured: a ramp average of 0.5/255 is exactly
/// `#010000`; one of 1.5/255 is exactly `#020000`; an averaged alpha of
/// 102.5/255 is exactly the 103/255 constant, and under `fill-opacity=".5"`
/// it is exactly the float product). When a stage does *not*, Chromium's
/// output matched **no** flat solid and **no** constant ramp this probe could
/// construct — a degenerate `pad` at stop alpha `0.5` under
/// `fill-opacity=".7"` sat 2560–4096 px away from every candidate at Δ1, the
/// signature of a dither pattern tied to the degenerate shader's own
/// geometry. Rather than guess that rule, this refuses by name.
///
/// The refusal fires on the collapsed value, not on how it arose: two
/// byte-exact `stop-color`s whose ramp average lands between codes trip it
/// with no `stop-opacity` present at all. It therefore belongs to
/// `<linearGradient>`/`<radialGradient>`, whose rows already carry it, and
/// not to either `stop-*` row.
fn solid_paint(
    color: CGColor32F,
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<ResolvedPaintServer, String> {
    // Only the *alpha* has to land on a byte here. A substituted colour whose
    // RGB sits between codes is reproducible while it is fully opaque at
    // identity (measured: a red→blue average is exactly `#800080`, and a
    // 1.5/255 average exactly `#020000`); [`ramp_average`] owns the rule for
    // when translucency makes that rounding visible.
    if !CGColor32F::new(0.0, 0.0, 0.0, color.a())
        .is_ok_and(|alpha_only| alpha_only.is_rgba8_exact())
    {
        return Err(
            "a degenerate paint server substitutes a colour this build cannot reproduce: the \
             collapsed alpha is not exactly representable in eight bits, and Chromium dithers it"
                .to_string(),
        );
    }
    let color = color.to_rgba8();
    // Beyond the substituted colour itself, each further alpha stage must be
    // an endpoint, for the same reason: two multiplied stages would land off
    // a byte again.
    let paint_alpha = (paint_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    if post_paint_opacity != 1.0 && !matches!(color.a, 0 | 255) {
        return Err(
            "a degenerate paint server collapses before post-paint opacity, and the staged \
             product is not exactly representable in eight bits"
                .to_string(),
        );
    }
    let alpha = match (color.a, paint_alpha) {
        (0, _) | (_, 0) => 0,
        (255, paint) => paint,
        (stop, 255) => stop,
        _ => {
            return Err(
                "a degenerate paint server collapses two alpha stages, and their product is \
                 not exactly representable in eight bits"
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
/// with edge plateaus, per channel in unpremultiplied float.
///
/// The averaged **RGB** is quantized here, because the solid the backend
/// substitutes stores it in eight bits and rounds half away from zero
/// (measured: an average of 0.5/255 is byte-identical to `#010000`, and one
/// of 1.5/255 to `#020000`). The averaged **alpha** is not: it still has the
/// consumer's opacity stages to meet, and those fold in float — see
/// [`solid_paint`].
fn ramp_average(
    stops: &[ResolvedStop],
    paint_opacity: f32,
    post_paint_opacity: f32,
) -> Result<CGColor32F, String> {
    // The average is taken over byte channels, which is what it means when
    // every stop already lands on one. A stop that does not cannot produce a
    // reproducible average — two float alphas can average onto a byte, but
    // whether the backend averages the float or the quantized stop is exactly
    // the rule this build refuses to guess — so it refuses here first. This
    // also keeps the integral out of a unit-space round trip, where
    // `77/255 * 255` is not `77`.
    for stop in stops {
        if !stop.color.is_rgba8_exact() {
            return Err(
                "a degenerate paint server averages a ramp whose stop is not exactly \
                 representable in eight bits, and Chromium dithers the result"
                    .to_string(),
            );
        }
    }
    let channels = |stop: &ResolvedStop| {
        let byte = stop.color.to_rgba8();
        [
            f32::from(byte.r),
            f32::from(byte.g),
            f32::from(byte.b),
            f32::from(byte.a),
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
    // An averaged colour channel between two codes is reproducible only while
    // the substituted colour is fully opaque at identity — measured: an
    // average of 0.5/255 is exactly `#010000` opaque, while the same average
    // at an exact-byte `0x80` alpha, or under any later opacity stage, moves
    // 2,304 Chromium pixels by one code value.
    let average_visible = sum[3] > 0.0 && paint_opacity > 0.0 && post_paint_opacity > 0.0;
    if average_visible
        && (sum[3] != 255.0 || paint_opacity != 1.0 || post_paint_opacity != 1.0)
        && sum[..3]
            .iter()
            .any(|component| component.round() != *component)
    {
        return Err(
            "a degenerate paint server's ramp average has a colour channel between codes, \
             which its translucency makes visible"
                .to_string(),
        );
    }
    // Narrow by the same multiply the byte→unit widening uses, so an integral
    // average lands on exactly the bits a byte colour would have produced —
    // `103.0 / 255.0` and `103.0 * (1.0 / 255.0)` are one ulp apart, and the
    // byte-exactness test downstream can tell them apart.
    const BYTE_TO_UNIT: f32 = 1.0 / 255.0;
    let unit = |value: f32| (value * BYTE_TO_UNIT).clamp(0.0, 1.0);
    CGColor32F::new(unit(sum[0]), unit(sum[1]), unit(sum[2]), unit(sum[3]))
        .map_err(|error| format!("a degenerate ramp average is unusable: {error}"))
}

fn cg_stops(stops: &[ResolvedStop]) -> Vec<cg::GradientStop> {
    stops
        .iter()
        .map(|stop| cg::GradientStop {
            offset: stop.offset,
            color: stop.color,
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

    // Every live gradient fact in `cg` is stated in the consumer geometry's
    // unit box. A user-space ramp on a line-like zero-area geometry cannot be
    // mapped into that box: either inverse extent would be non-finite. The
    // object-box case resolved before constant/degenerate classification;
    // user-space degenerate ramps resolved above and never reach this boundary.
    if destination_box.width == 0.0 || destination_box.height == 0.0 {
        return match units {
            GradientUnits::ObjectBoundingBox => Ok(ResolvedPaintServer::Nothing),
            GradientUnits::UserSpaceOnUse => Err(
                "a live user-space gradient on zero-area geometry cannot be mapped into the resolved unit-box paint contract"
                    .to_string(),
            ),
        };
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

    // As for a linear ramp, only live user-space radial geometry needs the
    // inverse destination-box mapping. Object-box zero area resolved before
    // constant/degenerate classification; preserve the source-neutral
    // non-positive-radius result above before enforcing this boundary.
    if destination_box.width == 0.0 || destination_box.height == 0.0 {
        return match units {
            GradientUnits::ObjectBoundingBox => Ok(ResolvedPaintServer::Nothing),
            GradientUnits::UserSpaceOnUse => Err(
                "a live user-space gradient on zero-area geometry cannot be mapped into the resolved unit-box paint contract"
                    .to_string(),
            ),
        };
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
