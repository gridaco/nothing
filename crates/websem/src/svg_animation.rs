//! Retained SVG animation semantics for the first Web-to-frame proving slice.
//!
//! This is deliberately source-specific. It associates one admitted SVG
//! `<animate>` with one already-materialized rectangle, then delegates only
//! checked time and scalar interpolation to `animation-sampling`. It does not
//! mutate the DOM, own playback, or introduce an animation representation into
//! `rframe`.

use std::collections::HashMap;

use animation_sampling::{FillMode, SampleTime, ScalarCurve, Timing};
use csscascade::adapter::HtmlElement;
use csscascade::dom::{DemoNodeData, NodeId};
use math2::rect_transform;
use rframe::{Frame, Geometry};
use style::dom::TElement;

use crate::svg::ShapeBinding;

/// A deterministic rejection from the deliberately closed rect-x animation
/// slice.
///
/// The current html5ever scaffold does not expose source spans. `path` is
/// therefore a stable structural location; it must not be presented as an XML
/// line or column.
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

#[derive(Debug)]
pub(crate) struct AnimationInventory {
    has_animation_elements: bool,
    result: Result<Option<RectXAnimation>, AnimationError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventoryScope {
    BareSvgScaffold,
    InlineHtml,
}

impl AnimationInventory {
    pub(crate) fn inspect(
        svg: HtmlElement<'_>,
        bindings: &[ShapeBinding],
        scope: InventoryScope,
    ) -> Self {
        let targets = bindings
            .iter()
            .map(|binding| (binding.source_node, binding.frame_index))
            .collect();
        let mut inspector = Inspector {
            targets,
            animation_count: 0,
            plan: None,
            error: None,
        };
        inspector.inspect_dynamic_surface(svg, "svg");
        inspector.walk_children(svg, "svg");
        if scope == InventoryScope::InlineHtml {
            inspector.record_error(AnimationError::new(
                "document",
                "inline HTML sampling is not admitted until document-wide CSS and script inventory is closed",
            ));
        }
        Self {
            has_animation_elements: inspector.animation_count != 0,
            result: match inspector.error {
                Some(error) => Err(error),
                None => Ok(inspector.plan),
            },
        }
    }

    pub(crate) const fn has_animation_elements(&self) -> bool {
        self.has_animation_elements
    }

    pub(crate) fn sample(&self, base: &Frame, time: SampleTime) -> Result<Frame, AnimationError> {
        let Some(plan) = self.result.as_ref().map_err(Clone::clone)? else {
            return Ok(base.clone());
        };
        let mut frame = base.clone();
        let Some(contribution) = plan.timing.contribution(time, FillMode::Freeze) else {
            return Ok(frame);
        };
        let effective_x = plan.curve.sample(contribution).value();
        let node = frame
            .nodes
            .get_mut(plan.frame_index)
            .expect("animation target was bound to this immutable base frame");
        match &mut node.geometry {
            Geometry::Rect(rect) => {
                rect.x = effective_x;
                node.bounds = rect_transform(*rect, &node.transform);
            }
        }
        Ok(frame)
    }
}

#[derive(Debug)]
struct RectXAnimation {
    frame_index: usize,
    timing: Timing,
    curve: ScalarCurve,
}

struct Inspector {
    targets: HashMap<NodeId, usize>,
    animation_count: usize,
    plan: Option<RectXAnimation>,
    error: Option<AnimationError>,
}

impl Inspector {
    fn walk_children(&mut self, parent: HtmlElement<'_>, parent_path: &str) {
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
                    self.record_error(AnimationError::new(
                        &path,
                        "the proving slice admits at most one animation element",
                    ));
                } else if tag == "animate" {
                    match self.compile_animate(element, parent, &path) {
                        Ok(plan) => self.plan = Some(plan),
                        Err(error) => self.record_error(error),
                    }
                } else {
                    self.record_error(AnimationError::new(
                        &path,
                        format!("animation element <{tag}> is outside the rect-x proving slice"),
                    ));
                }
            } else {
                self.walk_children(element, &path);
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
                        attribute.name.local
                    ),
                ));
            }
            if name.starts_with("animation") || name.starts_with("transition") {
                self.record_error(AnimationError::new(
                    path,
                    format!(
                        "CSS animation-affecting attribute {:?} is outside the rect-x proving slice",
                        attribute.name.local
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
        let Some(&frame_index) = self.targets.get(&parent.node_id()) else {
            return Err(AnimationError::new(
                path,
                "<animate> must be a direct child of a materialized top-level <rect>",
            ));
        };
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
            frame_index,
            timing,
            curve,
        })
    }

    fn record_error(&mut self, error: AnimationError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
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
