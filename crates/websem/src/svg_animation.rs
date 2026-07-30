//! Retained SVG animation semantics for the first Web-to-frame proving slice.
//!
//! This is deliberately source-specific. It associates one admitted SVG
//! `<animate>` with one materialized rectangle's source node, delegates only
//! checked time and scalar interpolation to `animation-sampling`, and
//! resolves each sample request into the Web-owned
//! [`EffectiveValues`] view that the one SVG compiler consumes. It never
//! mutates the DOM, never patches an already-compiled frame, and does not
//! own playback or introduce an animation representation into `rframe`.

use std::collections::{HashMap, HashSet};

use animation_sampling::{FillMode, SampleTime, ScalarCurve, Timing};
use csscascade::adapter::HtmlElement;
use csscascade::dom::{DemoNodeData, NodeId};
use style::dom::TElement;

use crate::effective_values::EffectiveValues;
use crate::svg::SourceEntry;

/// A deterministic rejection from the deliberately closed rect-x animation
/// slice.
///
/// The retained document sink does not expose source spans (for either
/// grammar entry). `path` is therefore a stable structural location; it must
/// not be presented as an XML line or column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationError {
    path: String,
    reason: String,
}

impl AnimationError {
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "SVG animation at {} is unsupported: {}",
            self.path, self.reason
        )
    }
}

impl std::error::Error for AnimationError {}

/// One beyond-inventory animation element, classified by what it distorts.
///
/// SMIL timing defaults `begin` to offset `0s`, so an animation element is
/// active the moment Chromium loads the document: the authored state of its
/// target is overridden *at load*, before any sample is requested. A Base
/// render of the authored state would therefore be a silent wrong pixel —
/// not a sampling gap — so the finding lands against the target element,
/// which the compiler leaves out of every view and declares by name.
#[derive(Debug)]
pub(crate) struct AuthoredOverride {
    error: AnimationError,
    /// The element whose authored state the animation overrides — its
    /// parent, the SMIL default target. The compiled views skip it.
    target: NodeId,
    /// The override cannot be attributed to one skippable element: it
    /// carries `href` (retargeting needs id resolution this slice does not
    /// own) or it targets the root `<svg>` (the override reaches the whole
    /// canvas). Document-level, like `<script>`: both admissions refuse.
    document_level: bool,
}

impl AuthoredOverride {
    pub(crate) fn error(&self) -> &AnimationError {
        &self.error
    }

    pub(crate) const fn target(&self) -> NodeId {
        self.target
    }

    pub(crate) const fn document_level(&self) -> bool {
        self.document_level
    }
}

#[derive(Debug)]
pub(crate) struct AnimationInventory {
    has_animation_elements: bool,
    plan: Option<RectXAnimation>,
    /// Every recorded reason the closed dynamic inventory rejects this
    /// source's *sampling*, in inspection order — dynamic surfaces (event
    /// handlers, CSS animation carriers) that leave the Base view honest.
    /// Strict sampling refuses with the first; best-effort declares each
    /// one and resolves samples to Base.
    blockers: Vec<AnimationError>,
    /// Beyond-inventory animation elements, active at load in Chromium:
    /// these distort Base itself, not just sampling. Strict refuses at
    /// construction; best-effort skips each target and declares it.
    overrides: Vec<AuthoredOverride>,
}

impl AnimationInventory {
    pub(crate) fn inspect(
        svg: HtmlElement<'_>,
        materialized: &[NodeId],
        entry: SourceEntry,
    ) -> Self {
        let mut inspector = Inspector {
            materialized: materialized.iter().copied().collect(),
            animation_count: 0,
            plan: None,
            errors: Vec::new(),
            overrides: Vec::new(),
        };
        inspector.inspect_dynamic_surface(svg, "svg");
        inspector.walk_children(svg, "svg", 0);
        if entry == SourceEntry::InlineHtml {
            inspector.record_error(AnimationError::new(
                "document",
                "inline HTML sampling is not admitted until document-wide CSS and script inventory is closed",
            ));
        }
        Self {
            has_animation_elements: inspector.animation_count != 0,
            plan: inspector.plan,
            blockers: inspector.errors,
            overrides: inspector.overrides,
        }
    }

    pub(crate) const fn has_animation_elements(&self) -> bool {
        self.has_animation_elements
    }

    /// Every recorded reason the closed dynamic inventory rejects this
    /// source's sampling — the facts the best-effort mode declares when it
    /// resolves every sample request to the Base view instead. The Base
    /// view stays honest under each of these; contrast [`Self::overrides`].
    pub(crate) fn blockers(&self) -> &[AnimationError] {
        &self.blockers
    }

    /// Every beyond-inventory animation element, active at load in
    /// Chromium, whose target's authored state therefore cannot render as
    /// the Base view.
    pub(crate) fn overrides(&self) -> &[AuthoredOverride] {
        &self.overrides
    }

    /// Resolve one frame request's animated contribution into the
    /// [`EffectiveValues`] view the compiler consumes. Base semantics apply
    /// whenever the admitted animation contributes no value at `time`. A
    /// blocked inventory refuses with its first recorded reason.
    pub(crate) fn effective_values(
        &self,
        time: SampleTime,
    ) -> Result<EffectiveValues, AnimationError> {
        if let Some(first) = self.blockers.first() {
            return Err(first.clone());
        }
        let Some(plan) = self.plan.as_ref() else {
            return Ok(EffectiveValues::base());
        };
        let Some(contribution) = plan.timing.contribution(time, FillMode::Freeze) else {
            return Ok(EffectiveValues::base());
        };
        Ok(EffectiveValues::with_scalar(
            plan.target,
            "x",
            plan.curve.sample(contribution).value(),
        ))
    }
}

#[derive(Debug)]
struct RectXAnimation {
    target: NodeId,
    timing: Timing,
    curve: ScalarCurve,
}

/// How deep the inventory inspects before declaring the rest uninspected.
///
/// Derived from the compiler's container bound rather than chosen, so the
/// two cannot drift: the deepest admitted container sits at nesting level
/// `MAX_CONTAINER_DEPTH`, the shapes it materializes one level below that,
/// and an `<animate>` under such a shape is found while inspecting the
/// shape itself — so inspection must be permitted two levels past the
/// container bound. Anything deeper cannot materialize, so declining to
/// inspect it withholds nothing the compiler would have admitted.
const MAX_INSPECTION_DEPTH: usize = crate::svg::MAX_CONTAINER_DEPTH + 2;

struct Inspector {
    materialized: HashSet<NodeId>,
    animation_count: usize,
    plan: Option<RectXAnimation>,
    errors: Vec<AnimationError>,
    overrides: Vec<AuthoredOverride>,
}

impl Inspector {
    fn walk_children(&mut self, parent: HtmlElement<'_>, parent_path: &str, depth: usize) {
        // The inventory descends the same tree the compiler does and runs
        // whether or not the compiler admitted every container, so it
        // carries its own bound: without one a deeply nested document would
        // exhaust the stack here instead of reaching a named refusal.
        if depth >= MAX_INSPECTION_DEPTH {
            self.record_error(AnimationError::new(
                parent_path,
                format!("nesting deeper than {MAX_INSPECTION_DEPTH} is not inspected"),
            ));
            return;
        }
        let mut ordinals = HashMap::<String, usize>::new();
        let mut child = parent.first_element_child();
        while let Some(element) = child {
            let tag = element.local_name_string();
            let ordinal = ordinals.entry(tag.clone()).or_default();
            *ordinal += 1;
            let path = format!("{parent_path}/{tag}[{ordinal}]");
            self.inspect_dynamic_surface(element, &path);

            if is_animation_element(&tag) {
                self.animation_count += 1;
                if self.animation_count > 1 {
                    self.record_animation_finding(
                        AnimationError::new(
                            &path,
                            "the proving slice admits at most one animation element",
                        ),
                        element,
                        parent,
                        parent_path,
                    );
                } else if tag == "animate" {
                    match self.compile_animate(element, parent, &path) {
                        Ok(plan) => self.plan = Some(plan),
                        Err(error) => {
                            self.record_animation_finding(error, element, parent, parent_path);
                        }
                    }
                } else {
                    self.record_animation_finding(
                        AnimationError::new(
                            &path,
                            format!(
                                "animation element <{tag}> is outside the rect-x proving slice"
                            ),
                        ),
                        element,
                        parent,
                        parent_path,
                    );
                }
            } else {
                self.walk_children(element, &path, depth + 1);
            }

            child = element.next_element_sibling();
        }
    }

    fn inspect_dynamic_surface(&mut self, element: HtmlElement<'_>, path: &str) {
        let tag = element.local_name_string();
        if tag.eq_ignore_ascii_case("script") {
            self.record_error(AnimationError::new(
                path,
                "<script> is outside the closed static sampling inventory",
            ));
        }
        if tag.eq_ignore_ascii_case("style") {
            self.record_error(AnimationError::new(
                path,
                "<style> requires a CSS animation inventory that the proving slice does not yet own",
            ));
        }

        let DemoNodeData::Element(data) = &element.dom_node().data else {
            unreachable!("HtmlElement always wraps element data");
        };
        for attribute in &data.attrs {
            let name = attribute.name.local.as_ref().to_ascii_lowercase();
            let value = attribute.value.as_ref();
            if name.starts_with("on") {
                self.record_error(AnimationError::new(
                    path,
                    format!(
                        "event-handler attribute {:?} is outside the closed static sampling inventory",
                        attribute.name.local.as_ref()
                    ),
                ));
            }
            if name.starts_with("animation") || name.starts_with("transition") {
                self.record_error(AnimationError::new(
                    path,
                    format!(
                        "CSS animation-affecting attribute {:?} is outside the rect-x proving slice",
                        attribute.name.local.as_ref()
                    ),
                ));
            }
            if name == "style" {
                self.record_error(AnimationError::new(
                    path,
                    format!(
                        "style attribute {value:?} requires a CSS animation inventory that the proving slice does not yet own"
                    ),
                ));
            }
        }
    }

    fn compile_animate(
        &self,
        animate: HtmlElement<'_>,
        parent: HtmlElement<'_>,
        path: &str,
    ) -> Result<RectXAnimation, AnimationError> {
        if !animate.is_svg_element() {
            return Err(AnimationError::new(
                path,
                "<animate> is not in the SVG namespace",
            ));
        }
        let target = parent.node_id();
        // The candidate set is the root's own materialized children, and
        // the tag check narrows it to rects: an <animate> under a circle,
        // an ellipse, or a shape nested in a <g> stays a declared blocker
        // rather than silently admitting an override the sampling corpus
        // does not bake.
        if !self.materialized.contains(&target) || parent.local_name_string() != "rect" {
            return Err(AnimationError::new(
                path,
                "<animate> must be a direct child of a materialized top-level <rect>",
            ));
        }
        validate_whitespace_only(animate, path)?;

        let mut values = HashMap::<String, String>::new();
        let DemoNodeData::Element(data) = &animate.dom_node().data else {
            unreachable!("HtmlElement always wraps element data");
        };
        for attribute in &data.attrs {
            let name = attribute.name.local.to_string();
            let qualified = if attribute.name.ns.as_ref().is_empty() {
                name.clone()
            } else {
                format!("{{{}}}{name}", attribute.name.ns)
            };
            if !matches!(
                name.as_str(),
                "id" | "attributeName" | "from" | "to" | "dur" | "fill" | "calcMode"
            ) || !attribute.name.ns.as_ref().is_empty()
            {
                return Err(AnimationError::new(
                    path,
                    format!("attribute {qualified:?} is outside the accepted attribute set"),
                ));
            }
            values.insert(name, attribute.value.to_string());
        }

        required(&values, "attributeName", path).and_then(|value| {
            (value == "x")
                .then_some(())
                .ok_or_else(|| AnimationError::new(path, "only attributeName=\"x\" is admitted"))
        })?;
        required(&values, "fill", path).and_then(|value| {
            (value == "freeze").then_some(()).ok_or_else(|| {
                AnimationError::new(path, "the proving slice requires fill=\"freeze\"")
            })
        })?;
        if let Some(mode) = values.get("calcMode")
            && mode != "linear"
        {
            return Err(AnimationError::new(
                path,
                "calcMode must be absent or \"linear\"",
            ));
        }

        let from = parse_finite_number(required(&values, "from", path)?, "from", path)?;
        let to = parse_finite_number(required(&values, "to", path)?, "to", path)?;
        let duration_ns = parse_clock(required(&values, "dur", path)?, path)?;
        let timing = Timing::new(0, duration_ns, 1)
            .map_err(|error| AnimationError::new(path, error.to_string()))?;
        let curve = ScalarCurve::linear(from, to)
            .map_err(|error| AnimationError::new(path, error.to_string()))?;

        Ok(RectXAnimation {
            target,
            timing,
            curve,
        })
    }

    fn record_error(&mut self, error: AnimationError) {
        self.errors.push(error);
    }

    /// Classify one beyond-inventory animation element.
    ///
    /// SMIL's default `begin` is offset `0s`, so the element is active when
    /// Chromium loads the document — it distorts the Base view, not just
    /// sampling. The finding lands as an [`AuthoredOverride`] against its
    /// SMIL default target (the parent), unless nothing renderable is
    /// targeted: an animation element under a non-rendering parent can
    /// distort no pixel the compiler paints, so sampling stays the only
    /// surface it blocks.
    fn record_animation_finding(
        &mut self,
        error: AnimationError,
        element: HtmlElement<'_>,
        parent: HtmlElement<'_>,
        parent_path: &str,
    ) {
        if crate::svg::is_non_rendering_element(&parent.local_name_string()) {
            self.errors.push(error);
            return;
        }
        let carries_href = has_href(element);
        let targets_root = parent_path == "svg";
        let error = if carries_href {
            AnimationError::new(
                error.path(),
                format!(
                    "{}; it carries href, so its target cannot be attributed to one \
                     element without id resolution",
                    error.reason()
                ),
            )
        } else if targets_root {
            AnimationError::new(
                error.path(),
                format!(
                    "{}; it targets the root <svg>, so the override reaches the whole canvas",
                    error.reason()
                ),
            )
        } else {
            error
        };
        self.overrides.push(AuthoredOverride {
            error,
            target: parent.node_id(),
            document_level: carries_href || targets_root,
        });
    }
}

/// Whether the animation element carries an `href` (or `xlink:href`)
/// retargeting attribute, in any namespace: SMIL resolves it to an
/// arbitrary element by id, which this slice cannot follow.
fn has_href(element: HtmlElement<'_>) -> bool {
    let DemoNodeData::Element(data) = &element.dom_node().data else {
        unreachable!("HtmlElement always wraps element data");
    };
    data.attrs
        .iter()
        .any(|attribute| attribute.name.local.as_ref() == "href")
}

pub(crate) fn is_animation_element(tag: &str) -> bool {
    tag.starts_with("animate") || matches!(tag, "set" | "discard")
}

fn required<'a>(
    values: &'a HashMap<String, String>,
    name: &str,
    path: &str,
) -> Result<&'a str, AnimationError> {
    values
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| AnimationError::new(path, format!("required attribute {name:?} is missing")))
}

fn parse_finite_number(value: &str, name: &str, path: &str) -> Result<f32, AnimationError> {
    let parsed = value.trim().parse::<f32>().map_err(|_| {
        AnimationError::new(
            path,
            format!("{name}={value:?} is not a unitless SVG number"),
        )
    })?;
    if !parsed.is_finite() {
        return Err(AnimationError::new(
            path,
            format!("{name}={value:?} is not finite"),
        ));
    }
    Ok(parsed)
}

fn parse_clock(value: &str, path: &str) -> Result<u64, AnimationError> {
    let value = value.trim();
    let (digits, scale) = value
        .strip_suffix("ms")
        .map(|digits| (digits, 1_000_000_u64))
        .or_else(|| {
            value
                .strip_suffix('s')
                .map(|digits| (digits, 1_000_000_000_u64))
        })
        .ok_or_else(|| {
            AnimationError::new(
                path,
                format!("dur={value:?} must be a positive integer in ms or s"),
            )
        })?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AnimationError::new(
            path,
            format!("dur={value:?} must be a positive integer in ms or s"),
        ));
    }
    let magnitude = digits.parse::<u64>().map_err(|_| {
        AnimationError::new(path, format!("dur={value:?} exceeds the admitted range"))
    })?;
    magnitude
        .checked_mul(scale)
        .filter(|duration| *duration != 0)
        .ok_or_else(|| {
            AnimationError::new(
                path,
                format!("dur={value:?} must convert to positive nanoseconds without overflow"),
            )
        })
}

fn validate_whitespace_only(animate: HtmlElement<'_>, path: &str) -> Result<(), AnimationError> {
    for child_id in &animate.dom_node().children {
        match &animate.dom().node(*child_id).data {
            DemoNodeData::Text(text) if text.trim().is_empty() => {}
            _ => {
                return Err(AnimationError::new(
                    path,
                    "<animate> content must contain whitespace only",
                ));
            }
        }
    }
    Ok(())
}
