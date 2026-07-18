//! HTML → Grida IR conversion.
//!
//! Parses HTML, resolves all CSS styles via Stylo (through [`csscascade`]),
//! and converts the styled DOM tree into a Grida [`SceneGraph`].
//!
//! This is the HTML counterpart to the SVG import pipeline in [`crate::svg`].
//!
//! **Property mapping tracker:** `docs/wg/format/css.md` and `docs/wg/format/html.md`
//! track which CSS properties and HTML elements are mapped, partially mapped, or blocked.

use crate::cg::prelude::*;
use crate::node::factory::NodeFactory;
use crate::node::scene_graph::{Parent, SceneGraph};
use crate::node::schema::*;

use crate::htmlcss::collect::styled_of;
use crate::htmlcss::style::StyledElement;
use crate::htmlcss::types::{Display as CssDisplay, Overflow as CssOverflow};

use csscascade::adapter::{self, HtmlElement};
use csscascade::dom::DemoNodeData;

mod from_styled;
use from_styled::{
    blend_mode_from, corner_radius_from, dimensions_from, effects_from, fills_from_background,
    flex_container_from, layout_child_from, margin_from, size_from, strokes_from_border,
    text_align_from, text_effects_from, text_style_from,
};

/// Parse an HTML string and convert it into a Grida [`SceneGraph`].
///
/// This is the main entry point, analogous to [`crate::import::svg::pack::from_svg_str`].
///
/// # Thread Safety
///
/// This function uses a process-global DOM slot ([`csscascade::adapter::DEMO_DOM`])
/// and is **not thread-safe**. Concurrent calls will race on the shared state.
/// Callers must serialize access externally (e.g. via a mutex).
pub fn from_html_str(html: &str) -> Result<SceneGraph, String> {
    // Parse + cascade via the shared front-end (htmlcss::frontend).
    let document = crate::htmlcss::frontend::parse_and_style(html)?;

    // Build scene graph from styled DOM
    let mut builder = SceneBuilder::new();
    if let Some(root) = document.root_element() {
        builder.build_element(root, Parent::Root);
    }

    Ok(builder.graph)
}

// ---------------------------------------------------------------------------
// Scene builder — walks styled DOM, emits IR nodes
// ---------------------------------------------------------------------------

struct SceneBuilder {
    factory: NodeFactory,
    graph: SceneGraph,
}

impl SceneBuilder {
    fn new() -> Self {
        Self {
            factory: NodeFactory::new(),
            graph: SceneGraph::new(),
        }
    }

    fn build_element(&mut self, element: HtmlElement, parent: Parent) {
        let dom = adapter::dom();
        // Shared per-element style record (htmlcss seam, gridaco/nothing#30).
        let Some(styled) = styled_of(element) else {
            return;
        };

        // Skip display:none elements entirely
        if styled.display == CssDisplay::None {
            return;
        }

        let tag = element.local_name_string();

        // Decide what IR node type to emit
        let has_element_children = element.first_element_child().is_some();
        let has_text_children = {
            let node = dom.node(element.node_id());
            node.children.iter().any(
                |cid| matches!(&dom.node(*cid).data, DemoNodeData::Text(t) if !t.trim().is_empty()),
            )
        };

        let is_structural = matches!(tag.as_str(), "html" | "body");

        // Check if all element children are inline (outer display type
        // `inline` — includes inline-block, inline-flex, …). When true
        // and we have text, we can merge everything into AttributedText.
        let all_children_inline = has_element_children && {
            let mut all_inline = true;
            let mut child = element.first_element_child();
            while let Some(c) = child {
                if let Some(child_styled) = styled_of(c) {
                    if !child_styled.inline_outside {
                        all_inline = false;
                        break;
                    }
                }
                child = c.next_element_sibling();
            }
            all_inline
        };

        if has_text_children && all_children_inline && !is_structural {
            // All children are text or inline elements → emit as a single
            // Container (for box model) with an AttributedText child (for text).
            let container_id = self.emit_container(&styled, parent);
            self.emit_attributed_text(element, &styled, Parent::NodeId(container_id));
        } else if has_element_children || has_text_children || is_structural {
            // Mixed content or structural element → Container with separate children.
            let container_id = self.emit_container(&styled, parent);
            let container_parent = Parent::NodeId(container_id);

            // Emit inline text children
            let node = dom.node(element.node_id());
            for child_id in &node.children {
                let child_node = dom.node(*child_id);
                if let DemoNodeData::Text(text) = &child_node.data {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        self.emit_text_span(trimmed, &styled, container_parent.clone());
                    }
                }
            }

            // Recurse into child elements
            let mut child = element.first_element_child();
            while let Some(c) = child {
                self.build_element(c, container_parent.clone());
                child = c.next_element_sibling();
            }
        } else {
            // Empty visual element → Rectangle
            self.emit_rectangle(&styled, parent);
        }
    }

    /// Wrap a node in a transparent container whose padding equals the CSS margin.
    /// The wrapper inherits the `layout_child` role (grow, positioning) from the
    /// original node so it occupies the correct slot in the parent's flex layout.
    /// Returns the wrapper's NodeId — the caller should append content into it.
    fn wrap_with_margin_padding(
        &mut self,
        margin: &CSSMargin,
        layout_child: Option<LayoutChildStyle>,
        parent: Parent,
    ) -> NodeId {
        let mut wrapper = self.factory.create_container_node();
        wrapper.fills = Paints::default(); // transparent — no visual
        wrapper.strokes = Default::default();
        wrapper.stroke_width = StrokeWidth::default();
        wrapper.layout_container.layout_mode = LayoutMode::Flex;
        wrapper.layout_container.layout_direction = Axis::Vertical;
        // Clear factory default 100×100 — wrapper should auto-size to content.
        wrapper.layout_dimensions.layout_target_width = None;
        wrapper.layout_dimensions.layout_target_height = None;
        wrapper.layout_container.layout_padding = Some(EdgeInsets {
            top: margin.top,
            right: margin.right,
            bottom: margin.bottom,
            left: margin.left,
        });
        wrapper.layout_child = layout_child;
        self.graph.append_child(Node::Container(wrapper), parent)
    }

    fn emit_container(&mut self, styled: &StyledElement, parent: Parent) -> NodeId {
        let mut node = self.factory.create_container_node();

        // Display / layout mode, flex direction/wrap/alignment, gap
        flex_container_from(styled, &mut node.layout_container);

        // Opacity
        node.opacity = styled.opacity;

        // Background → fills (solid color + gradients)
        node.fills = fills_from_background(styled);

        // Border radius
        node.corner_radius = corner_radius_from(styled);

        // Padding
        let padding = styled.padding;
        if padding.top != 0.0
            || padding.right != 0.0
            || padding.bottom != 0.0
            || padding.left != 0.0
        {
            node.layout_container.layout_padding = Some(padding);
        }

        // Overflow → clip
        node.clip =
            styled.overflow_x != CssOverflow::Visible || styled.overflow_y != CssOverflow::Visible;

        // Borders → strokes + stroke_width
        let (border_strokes, border_stroke_width, border_stroke_style) =
            strokes_from_border(styled);
        node.strokes = border_strokes;
        node.stroke_width = border_stroke_width;
        node.stroke_style = border_stroke_style;

        // Effects (box-shadow, filter, backdrop-filter)
        node.effects = effects_from(styled);

        // Blend mode (mix-blend-mode)
        node.blend_mode = blend_mode_from(styled);

        // Width / height / min / max dimensions
        dimensions_from(styled, &mut node.layout_dimensions);

        // Flex child properties (for nested containers inside flex parents)
        node.layout_child = layout_child_from(styled);

        // Margin → tree surgery
        // Fixed positive margins are absorbed into the container's own padding when
        // the container has no visual properties (fills, borders) that would bleed
        // into the margin zone. Otherwise, a separate wrapper is created.
        let margin = margin_from(styled);
        if !margin.is_zero() && !margin.has_any_auto() && !margin.has_any_negative() {
            let has_visuals = !node.fills.is_empty() || !node.strokes.is_empty();
            if has_visuals {
                // Container has background/border — margin must stay outside.
                // Wrap in a transparent container whose padding = margin.
                let layout_child = node.layout_child.take();
                let wrapper_id = self.wrap_with_margin_padding(&margin, layout_child, parent);
                self.graph
                    .append_child(Node::Container(node), Parent::NodeId(wrapper_id))
            } else {
                // No visual properties — safe to merge margin into padding.
                // This avoids an extra wrapper node in the tree.
                let existing = node.layout_container.layout_padding.unwrap_or_default();
                node.layout_container.layout_padding = Some(EdgeInsets {
                    top: existing.top + margin.top,
                    right: existing.right + margin.right,
                    bottom: existing.bottom + margin.bottom,
                    left: existing.left + margin.left,
                });
                self.graph.append_child(Node::Container(node), parent)
            }
        } else {
            // TODO(margin): auto margins require SpacerNode siblings (not yet implemented).
            // TODO(margin): negative margins require negative offset support (not planned).
            self.graph.append_child(Node::Container(node), parent)
        }
    }

    fn emit_text_span(&mut self, text: &str, styled: &StyledElement, parent: Parent) {
        let mut node = self.factory.create_text_span_node();
        node.text = text.to_string();

        let (text_style, fills) = text_style_from(styled);
        node.text_style = text_style;
        node.fills = fills;
        node.text_align = text_align_from(styled);
        node.opacity = styled.opacity;
        node.effects = text_effects_from(styled);
        node.blend_mode = blend_mode_from(styled);

        if styled.flex_grow > 0.0 {
            node.layout_child = Some(LayoutChildStyle {
                layout_grow: styled.flex_grow,
                layout_positioning: LayoutPositioning::Auto,
            });
        }

        // NOTE: No margin surgery for text spans. Text spans are emitted using
        // the parent element's style (see build_element), which may carry
        // the parent's margin. Margin is handled at the container/rectangle level.
        self.graph.append_child(Node::TextSpan(node), parent);
    }

    /// Emit an `AttributedTextNodeRec` by merging all inline children (text nodes
    /// and inline elements like `<strong>`, `<em>`, `<code>`) into a single rich
    /// text node with per-run styling.
    fn emit_attributed_text(
        &mut self,
        element: HtmlElement,
        styled: &StyledElement,
        parent: Parent,
    ) {
        let dom = adapter::dom();
        let (default_style, default_fills) = text_style_from(styled);
        let default_color = Some(styled.color);

        let mut builder = AttributedStringBuilder::new();
        let node_data = dom.node(element.node_id());

        // Walk children in DOM order — interleaved text nodes and inline elements.
        // Use CSS white-space collapsing: newlines/tabs → space, collapse runs of spaces.
        for child_id in &node_data.children {
            let child_node = dom.node(*child_id);
            match &child_node.data {
                DemoNodeData::Text(text) => {
                    let collapsed = collapse_whitespace(text);
                    if !collapsed.is_empty() {
                        builder = builder.push(&collapsed, &default_style, default_color);
                    }
                }
                DemoNodeData::Element(_) => {
                    // Inline element — get its own style record and collect text.
                    let child_el = HtmlElement::from_node_id(*child_id);
                    if let Some(child_styled) = styled_of(child_el) {
                        Self::collect_inline_text(&mut builder, child_el, &child_styled);
                    }
                }
                _ => {} // comments, doctypes, etc. — skip
            }
        }

        // If nothing was collected (whitespace-only source), skip.
        if builder.is_empty() {
            return;
        }
        let mut attr_string = builder.build();
        attr_string.merge_adjacent_runs();

        let node = AttributedTextNodeRec {
            active: true,
            transform: Default::default(),
            width: None,
            height: None,
            layout_child: layout_child_from(styled),
            attributed_string: attr_string,
            default_style,
            text_align: text_align_from(styled),
            text_align_vertical: TextAlignVertical::Top,
            max_lines: None,
            ellipsis: None,
            fills: default_fills,
            strokes: Default::default(),
            stroke_width: 0.0,
            stroke_align: StrokeAlign::default(),
            opacity: styled.opacity,
            blend_mode: blend_mode_from(styled),
            mask: None,
            effects: text_effects_from(styled),
        };

        self.graph.append_child(Node::AttributedText(node), parent);
    }

    /// Recursively collect text from an inline element and its children into the builder.
    fn collect_inline_text(
        builder: &mut AttributedStringBuilder,
        element: HtmlElement,
        styled: &StyledElement,
    ) {
        let dom = adapter::dom();
        let (run_style, _) = text_style_from(styled);
        let run_color = Some(styled.color);
        let node_data = dom.node(element.node_id());

        for child_id in &node_data.children {
            let child_node = dom.node(*child_id);
            match &child_node.data {
                DemoNodeData::Text(text) => {
                    let collapsed = collapse_whitespace(text);
                    if !collapsed.is_empty() {
                        *builder = std::mem::take(builder).push(&collapsed, &run_style, run_color);
                    }
                }
                DemoNodeData::Element(_) => {
                    let child_el = HtmlElement::from_node_id(*child_id);
                    if let Some(child_styled) = styled_of(child_el) {
                        Self::collect_inline_text(builder, child_el, &child_styled);
                    }
                }
                _ => {}
            }
        }
    }

    fn emit_rectangle(&mut self, styled: &StyledElement, parent: Parent) {
        let mut node = self.factory.create_rectangle_node();

        node.fills = fills_from_background(styled);
        node.corner_radius = corner_radius_from(styled);
        node.opacity = styled.opacity;

        // CSS dimensions → node size
        node.size = size_from(styled);

        // Flex child properties (grow, positioning)
        node.layout_child = layout_child_from(styled);

        // Borders
        let (border_strokes, border_stroke_width, border_stroke_style) =
            strokes_from_border(styled);
        node.strokes = border_strokes;
        node.stroke_width = border_stroke_width;
        node.stroke_style = border_stroke_style;

        // Effects (box-shadow, filter, backdrop-filter)
        node.effects = effects_from(styled);

        // Blend mode (mix-blend-mode)
        node.blend_mode = blend_mode_from(styled);

        // Margin → tree surgery (same pattern as emit_container)
        let margin = margin_from(styled);
        if !margin.is_zero() && !margin.has_any_auto() && !margin.has_any_negative() {
            let layout_child = node.layout_child.take();
            let wrapper_id = self.wrap_with_margin_padding(&margin, layout_child, parent);
            self.graph
                .append_child(Node::Rectangle(node), Parent::NodeId(wrapper_id));
        } else {
            // TODO(margin): auto/negative margins not supported for rectangles.
            self.graph.append_child(Node::Rectangle(node), parent);
        }
    }
}

// ---------------------------------------------------------------------------
// CSS → CG type conversion helpers
// ---------------------------------------------------------------------------

/// Parsed CSS margin with per-edge auto tracking.
struct CSSMargin {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
    top_auto: bool,
    right_auto: bool,
    bottom_auto: bool,
    left_auto: bool,
}

impl CSSMargin {
    fn is_zero(&self) -> bool {
        !self.top_auto
            && !self.right_auto
            && !self.bottom_auto
            && !self.left_auto
            && self.top == 0.0
            && self.right == 0.0
            && self.bottom == 0.0
            && self.left == 0.0
    }

    fn has_any_auto(&self) -> bool {
        self.top_auto || self.right_auto || self.bottom_auto || self.left_auto
    }

    fn has_any_negative(&self) -> bool {
        self.top < 0.0 || self.right < 0.0 || self.bottom < 0.0 || self.left < 0.0
    }
}

/// Collapse whitespace per CSS `white-space: normal` rules.
/// Newlines and tabs become spaces; consecutive spaces collapse to one.
/// Leading/trailing whitespace is preserved as a single space (important for
/// inline text runs where `"Hello "` + `"world"` must keep the space).
fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(ch);
            prev_was_space = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::engine::LayoutEngine;
    use crate::layout::ComputedLayout;
    use crate::node::schema::Scene;
    use std::collections::HashMap;

    /// Parse HTML and run the layout engine, returning the scene and a
    /// map of every node's computed layout.
    fn html_layout(html: &str, vw: f32, vh: f32) -> (Scene, HashMap<NodeId, ComputedLayout>) {
        let graph = from_html_str(html).expect("HTML parse failed");
        let scene = Scene {
            name: "html-test".to_string(),
            graph,
            background_color: None,
        };
        let mut engine = LayoutEngine::new();
        let result = engine.compute(
            &scene,
            Size {
                width: vw,
                height: vh,
            },
            None,
        );
        let map: HashMap<NodeId, ComputedLayout> = result.iter().map(|(k, v)| (k, *v)).collect();
        (scene, map)
    }

    /// Collect all NodeIds in DFS order (pre-order).
    fn dfs_nodes(graph: &SceneGraph) -> Vec<NodeId> {
        let mut out = Vec::new();
        fn walk(graph: &SceneGraph, id: &NodeId, out: &mut Vec<NodeId>) {
            out.push(*id);
            if let Some(children) = graph.get_children(id) {
                for child_id in children {
                    walk(graph, child_id, out);
                }
            }
        }
        for root in graph.roots() {
            walk(graph, root, &mut out);
        }
        out
    }

    // -----------------------------------------------------------------------
    // Smoke / parsing tests (existing)
    // -----------------------------------------------------------------------

    #[test]
    fn smoke_test_basic_html() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html>
  <head>
    <style>
      body { background: #f5f5f5; color: #222; font-family: sans-serif; }
      h1 { font-size: 32px; color: hotpink; }
      .card { display: flex; gap: 12px; padding: 16px; background: white; border-radius: 8px; }
    </style>
  </head>
  <body>
    <h1>Hello</h1>
    <div class="card">
      <p>Paragraph text</p>
    </div>
  </body>
</html>"#;

        let graph = from_html_str(html).expect("should parse and convert HTML");
        assert!(
            graph.node_count() > 3,
            "expected at least 4 nodes, got {}",
            graph.node_count()
        );
    }

    #[test]
    fn test_inline_style_attribute() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html>
  <body>
    <div style="font-size: 20px; color: red;">Styled inline</div>
  </body>
</html>"#;
        let graph = from_html_str(html).expect("should parse inline styles");
        assert!(graph.node_count() >= 3);
    }

    #[test]
    fn test_borders_and_shadows() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html>
  <head>
    <style>
      .box { border: 2px solid #333; box-shadow: 4px 4px 8px rgba(0,0,0,0.3); }
    </style>
  </head>
  <body>
    <div class="box">bordered</div>
  </body>
</html>"#;
        let graph = from_html_str(html).expect("should parse borders and shadows");
        assert!(graph.node_count() >= 3);
    }

    #[test]
    fn test_flex_alignment() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html>
  <head>
    <style>
      .flex { display: flex; align-items: center; justify-content: space-between; }
    </style>
  </head>
  <body>
    <div class="flex">
      <span>A</span>
      <span>B</span>
    </div>
  </body>
</html>"#;
        let graph = from_html_str(html).expect("should parse flex alignment");
        assert!(graph.node_count() >= 4);
    }

    #[test]
    fn test_gradient_backgrounds() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html>
  <head>
    <style>
      .linear { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
      .radial { background: radial-gradient(circle, #ff0000, #0000ff); }
    </style>
  </head>
  <body>
    <div class="linear">linear</div>
    <div class="radial">radial</div>
  </body>
</html>"#;
        let graph = from_html_str(html).expect("should parse gradient backgrounds");
        assert!(graph.node_count() >= 4);
    }

    // -----------------------------------------------------------------------
    // Deterministic flex layout tests (divs only, no text)
    // -----------------------------------------------------------------------

    /// 3 fixed-size divs in a flex row with gap.
    #[test]
    fn test_flex_row_positions() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; flex-direction:row; gap:10px; width:300px; height:100px;">
    <div style="width:50px; height:50px;"></div>
    <div style="width:50px; height:50px;"></div>
    <div style="width:50px; height:50px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        // Find the three leaf rectangles (last 3 in DFS of the flex container subtree)
        // Tree: html > body > flex-container > [child0, child1, child2]
        // DFS order should have the 3 children at the end
        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                // Children are 50×50
                if (l.width - 50.0).abs() < 1.0 && (l.height - 50.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 3, "expected 3 leaf children");

        assert_eq!(leaf_layouts[0].x, 0.0, "child0 x");
        assert_eq!(leaf_layouts[1].x, 60.0, "child1 x = 50 + 10 gap");
        assert_eq!(leaf_layouts[2].x, 120.0, "child2 x = 50+10+50+10");
    }

    /// 3 fixed-size divs in a flex column with gap.
    #[test]
    fn test_flex_column_positions() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; flex-direction:column; gap:10px; width:100px; height:300px;">
    <div style="width:50px; height:50px;"></div>
    <div style="width:50px; height:50px;"></div>
    <div style="width:50px; height:50px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 50.0).abs() < 1.0 && (l.height - 50.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 3, "expected 3 leaf children");

        assert_eq!(leaf_layouts[0].y, 0.0, "child0 y");
        assert_eq!(leaf_layouts[1].y, 60.0, "child1 y = 50 + 10 gap");
        assert_eq!(leaf_layouts[2].y, 120.0, "child2 y = 50+10+50+10");
    }

    /// justify-content: center with 2 fixed children.
    #[test]
    fn test_flex_justify_center() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; justify-content:center; width:200px; height:100px;">
    <div style="width:40px; height:40px;"></div>
    <div style="width:40px; height:40px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 40.0).abs() < 1.0 && (l.height - 40.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 2, "expected 2 leaf children");

        // Total child width = 80, remaining = 120, offset = 60
        assert_eq!(leaf_layouts[0].x, 60.0, "child0 x centered");
        assert_eq!(leaf_layouts[1].x, 100.0, "child1 x centered");
    }

    /// justify-content: space-between with 3 fixed children.
    #[test]
    fn test_flex_justify_space_between() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; justify-content:space-between; width:200px; height:100px;">
    <div style="width:40px; height:40px;"></div>
    <div style="width:40px; height:40px;"></div>
    <div style="width:40px; height:40px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 40.0).abs() < 1.0 && (l.height - 40.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 3, "expected 3 leaf children");

        assert_eq!(leaf_layouts[0].x, 0.0, "first child at start");
        assert_eq!(leaf_layouts[2].x, 160.0, "last child at end (200-40)");
    }

    /// align-items: center with a single child shorter than container.
    #[test]
    fn test_flex_align_center() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; align-items:center; width:200px; height:100px;">
    <div style="width:40px; height:40px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf = nodes
            .iter()
            .find_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 40.0).abs() < 1.0 && (l.height - 40.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .expect("should find 40×40 child");

        assert_eq!(leaf.y, 30.0, "child centered: (100-40)/2 = 30");
    }

    /// flex-grow: second child fills remaining space.
    #[test]
    fn test_flex_grow() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; width:300px; height:100px;">
    <div style="width:100px; height:50px;"></div>
    <div style="flex-grow:1; height:50px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let child_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.height - 50.0).abs() < 1.0 && l.width > 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(child_layouts.len(), 2, "expected 2 children");

        assert_eq!(child_layouts[0].width, 100.0, "first child fixed 100px");
        assert_eq!(child_layouts[0].x, 0.0, "first child at x=0");
        assert_eq!(
            child_layouts[1].width, 200.0,
            "second child grows to fill 300-100=200"
        );
        assert_eq!(child_layouts[1].x, 100.0, "second child starts after first");
    }

    /// Container padding offsets children.
    #[test]
    fn test_flex_padding() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; padding:10px; width:200px; height:100px;">
    <div style="width:30px; height:30px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf = nodes
            .iter()
            .find_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 30.0).abs() < 1.0 && (l.height - 30.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .expect("should find 30×30 child");

        assert_eq!(leaf.x, 10.0, "child offset by left padding");
        assert_eq!(leaf.y, 10.0, "child offset by top padding");
    }

    /// Flex column gap direction is correct (gap applies vertically).
    #[test]
    fn test_flex_gap_column_direction() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; flex-direction:column; gap:15px; width:100px; height:300px;">
    <div style="width:40px; height:40px;"></div>
    <div style="width:40px; height:40px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 40.0).abs() < 1.0 && (l.height - 40.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 2, "expected 2 leaf children");

        assert_eq!(leaf_layouts[0].y, 0.0, "child0 at y=0");
        assert_eq!(leaf_layouts[1].y, 55.0, "child1 at y=40+15 gap");
    }

    /// Nested flex: outer row, inner column with children.
    #[test]
    fn test_nested_flex() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; flex-direction:row; width:400px; height:200px;">
    <div style="display:flex; flex-direction:column; width:100px; height:200px;">
      <div style="width:80px; height:60px;"></div>
      <div style="width:80px; height:60px;"></div>
    </div>
    <div style="width:100px; height:100px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        // Find the 80×60 leaves (inner column children)
        let inner_leaves: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 80.0).abs() < 1.0 && (l.height - 60.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(inner_leaves.len(), 2, "expected 2 inner column children");
        assert_eq!(inner_leaves[0].y, 0.0, "first inner child at y=0");
        assert_eq!(inner_leaves[1].y, 60.0, "second inner child at y=60");

        // Find the 100×100 sibling in the outer row
        let sibling = nodes
            .iter()
            .find_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 100.0).abs() < 1.0 && (l.height - 100.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .expect("should find 100×100 sibling");
        assert_eq!(
            sibling.x, 100.0,
            "sibling starts after inner column (width=100)"
        );
    }

    /// Explicit width/height dimensions are preserved.
    #[test]
    fn test_explicit_dimensions() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:200px; height:100px;"></div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf = nodes
            .iter()
            .find_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 200.0).abs() < 1.0 && (l.height - 100.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .expect("should find 200×100 div");

        assert_eq!(leaf.width, 200.0);
        assert_eq!(leaf.height, 100.0);
    }

    /// flex-wrap: children that overflow wrap to the next line.
    #[test]
    fn test_flex_wrap() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; flex-wrap:wrap; width:150px; height:200px;">
    <div style="width:60px; height:60px;"></div>
    <div style="width:60px; height:60px;"></div>
    <div style="width:60px; height:60px;"></div>
  </div>
</body></html>"#;
        let (scene, layouts) = html_layout(html, 800.0, 600.0);
        let nodes = dfs_nodes(&scene.graph);

        let leaf_layouts: Vec<_> = nodes
            .iter()
            .filter_map(|id| {
                let l = layouts.get(id)?;
                if (l.width - 60.0).abs() < 1.0 && (l.height - 60.0).abs() < 1.0 {
                    Some(*l)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(leaf_layouts.len(), 3, "expected 3 children");

        // First two fit on first row (60+60=120 <= 150)
        assert_eq!(leaf_layouts[0].y, 0.0, "child0 on first row");
        assert_eq!(leaf_layouts[1].y, 0.0, "child1 on first row");
        // Third wraps to second row
        assert!(
            leaf_layouts[2].y >= 60.0,
            "child2 should wrap to y >= 60, got {}",
            leaf_layouts[2].y
        );
    }

    // -----------------------------------------------------------------------
    // Effects / blend mode tests
    // -----------------------------------------------------------------------

    /// Parse HTML without running layout — returns just the SceneGraph.
    fn html_graph(html: &str) -> SceneGraph {
        from_html_str(html).expect("HTML parse failed")
    }

    /// text-shadow maps to drop-shadow effects on the TextSpan node.
    #[test]
    fn test_text_shadow() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div><span style="text-shadow: 2px 3px 4px rgba(0,0,0,0.6);">Hello</span></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        // Find the TextSpan node
        let text_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::TextSpan(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a TextSpan node");

        let effects = text_node.effects().expect("TextSpan should have effects");
        assert_eq!(effects.shadows.len(), 1, "one text-shadow");
        match &effects.shadows[0] {
            FilterShadowEffect::DropShadow(s) => {
                assert_eq!(s.dx, 2.0);
                assert_eq!(s.dy, 3.0);
                assert_eq!(s.blur, 4.0);
                assert_eq!(s.spread, 0.0, "text-shadow has no spread");
                assert!(s.active);
            }
            _ => panic!("expected DropShadow"),
        }
    }

    /// Multiple text-shadows produce multiple DropShadow effects.
    #[test]
    fn test_text_shadow_multiple() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div><span style="text-shadow: 1px 1px 2px black, 0 0 8px blue;">Multi</span></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let text_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::TextSpan(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a TextSpan node");

        let effects = text_node.effects().expect("TextSpan should have effects");
        assert_eq!(effects.shadows.len(), 2, "two text-shadows");
    }

    /// box-shadow (inset + outer) maps to InnerShadow + DropShadow.
    #[test]
    fn test_box_shadow() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:100px; height:100px; box-shadow: 4px 4px 8px black, inset 2px 2px 4px red;"></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        let effects = rect_node.effects().expect("Rectangle should have effects");
        assert_eq!(effects.shadows.len(), 2, "two box-shadows");
        assert!(
            matches!(&effects.shadows[0], FilterShadowEffect::DropShadow(_)),
            "first is DropShadow"
        );
        assert!(
            matches!(&effects.shadows[1], FilterShadowEffect::InnerShadow(_)),
            "second is InnerShadow"
        );
    }

    /// filter: blur() maps to FeLayerBlur on a rectangle.
    #[test]
    fn test_filter_blur() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:100px; height:100px; filter: blur(6px);"></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        let effects = rect_node.effects().expect("Rectangle should have effects");
        let blur = effects.blur.as_ref().expect("should have blur");
        match &blur.blur {
            FeBlur::Gaussian(g) => assert_eq!(g.radius, 6.0),
            _ => panic!("expected Gaussian blur"),
        }
    }

    /// filter: drop-shadow() maps to DropShadow effect on a rectangle.
    #[test]
    fn test_filter_drop_shadow() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:100px; height:100px; filter: drop-shadow(4px 4px 8px black);"></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        let effects = rect_node.effects().expect("Rectangle should have effects");
        assert_eq!(effects.shadows.len(), 1, "one drop-shadow from filter");
        match &effects.shadows[0] {
            FilterShadowEffect::DropShadow(s) => {
                assert_eq!(s.dx, 4.0);
                assert_eq!(s.dy, 4.0);
                assert_eq!(s.blur, 8.0);
            }
            _ => panic!("expected DropShadow"),
        }
    }

    /// backdrop-filter: blur() maps to FeBackdropBlur.
    ///
    /// NOTE: Stylo marks `backdrop-filter` as `servo_pref="layout.unimplemented"`,
    /// so in servo mode the property is parsed but treated as initial (none).
    /// The mapping code is correct but untestable until a gecko build or pref
    /// override is available. This test verifies the pipeline doesn't crash.
    #[test]
    fn test_backdrop_filter_blur() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:200px; height:200px;">
    <div style="width:100px; height:100px; backdrop-filter: blur(12px);"></div>
  </div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        // backdrop-filter is unimplemented in servo mode so effects may lack backdrop_blur.
        // Just verify the node exists and effects are accessible (no crash).
        let _effects = rect_node.effects().expect("Rectangle should have effects");
    }

    /// mix-blend-mode: multiply maps to LayerBlendMode::Blend(BlendMode::Multiply).
    #[test]
    fn test_mix_blend_mode() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:100px; height:100px; mix-blend-mode: multiply;"></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        assert_eq!(
            rect_node.blend_mode(),
            LayerBlendMode::Blend(BlendMode::Multiply)
        );
    }

    /// mix-blend-mode: normal stays as PassThrough (default).
    #[test]
    fn test_mix_blend_mode_normal() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="width:100px; height:100px; mix-blend-mode: normal;"></div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let rect_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Rectangle(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Rectangle node");

        assert_eq!(rect_node.blend_mode(), LayerBlendMode::PassThrough);
    }

    /// Effects on containers: filter + blend mode on a flex container.
    #[test]
    fn test_container_effects() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="display:flex; width:200px; height:100px; filter: blur(4px); mix-blend-mode: screen;">
    <div style="width:50px; height:50px;"></div>
  </div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        // Find the container with non-PassThrough blend mode (the one with mix-blend-mode: screen)
        let container_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::Container(_)) && n.blend_mode() != LayerBlendMode::PassThrough
                {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a Container with non-default blend mode");

        let effects = container_node
            .effects()
            .expect("Container should have effects");
        assert!(effects.blur.is_some(), "container should have blur");
        assert_eq!(
            container_node.blend_mode(),
            LayerBlendMode::Blend(BlendMode::Screen)
        );
    }

    // -----------------------------------------------------------------------
    // Text decoration (color, style) tests
    // -----------------------------------------------------------------------

    /// text-decoration-color maps to TextDecorationRec.text_decoration_color.
    #[test]
    fn test_text_decoration_color() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="text-decoration: underline; text-decoration-color: #ef4444;">Red underline</div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let text_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::TextSpan(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a TextSpan node");

        if let Node::TextSpan(ts) = text_node {
            let dec = ts
                .text_style
                .text_decoration
                .as_ref()
                .expect("should have decoration");
            assert_eq!(dec.text_decoration_line, TextDecorationLine::Underline);
            let color = dec
                .text_decoration_color
                .expect("should have decoration color");
            assert_eq!(color, CGColor::from_rgba(239, 68, 68, 255));
        } else {
            panic!("expected TextSpan");
        }
    }

    /// text-decoration-style maps to TextDecorationRec.text_decoration_style.
    #[test]
    fn test_text_decoration_style() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="text-decoration: underline; text-decoration-style: wavy;">Wavy</div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let text_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::TextSpan(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a TextSpan node");

        if let Node::TextSpan(ts) = text_node {
            let dec = ts
                .text_style
                .text_decoration
                .as_ref()
                .expect("should have decoration");
            assert_eq!(dec.text_decoration_style, Some(TextDecorationStyle::Wavy));
        } else {
            panic!("expected TextSpan");
        }
    }

    /// Combined: text-decoration with color + style + line.
    #[test]
    fn test_text_decoration_combined() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body>
  <div style="text-decoration: line-through; text-decoration-color: #3b82f6; text-decoration-style: dashed;">Combo</div>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let text_node = nodes
            .iter()
            .find_map(|id| {
                let n = graph.get_node(id).ok()?;
                if matches!(n, Node::TextSpan(_)) {
                    Some(n)
                } else {
                    None
                }
            })
            .expect("should find a TextSpan node");

        if let Node::TextSpan(ts) = text_node {
            let dec = ts
                .text_style
                .text_decoration
                .as_ref()
                .expect("should have decoration");
            assert_eq!(dec.text_decoration_line, TextDecorationLine::LineThrough);
            assert_eq!(
                dec.text_decoration_color,
                Some(CGColor::from_rgba(59, 130, 246, 255))
            );
            assert_eq!(dec.text_decoration_style, Some(TextDecorationStyle::Dashed));
            assert_eq!(
                dec.text_decoration_thickness, None,
                "thickness unavailable in servo mode"
            );
        } else {
            panic!("expected TextSpan");
        }
    }

    /// h1 margin should merge into padding (no extra wrapper container).
    #[test]
    fn test_h1_margin_no_double_wrap() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body style="margin:0; padding:0;">
  <h1>Hello</h1>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        // h1 has UA margin ~21px top/bottom. With body margin:0, the tree should be:
        //   ICB → html → h1(margin merged as padding) → TextSpan
        // Total containers: 3 (ICB + html + h1), no extra wrapper.
        let container_count = nodes
            .iter()
            .filter(|id| matches!(graph.get_node(id).ok(), Some(Node::Container(_))))
            .count();
        // ICB is InitialContainer, not Container
        let icb_count = nodes
            .iter()
            .filter(|id| matches!(graph.get_node(id).ok(), Some(Node::InitialContainer(_))))
            .count();
        assert_eq!(
            icb_count + container_count,
            3,
            "ICB + html + h1 = 3 frames, no margin wrapper"
        );

        // h1 container should have margin merged as padding
        let h1_node = nodes
            .iter()
            .rev()
            .find_map(|id| match graph.get_node(id).ok()? {
                Node::Container(c) if c.layout_container.layout_padding.is_some() => {
                    Some(c.layout_container.layout_padding.as_ref().unwrap().clone())
                }
                _ => None,
            })
            .expect("h1 should have padding from merged margin");
        assert!(
            h1_node.top > 10.0,
            "h1 should have top padding from UA margin"
        );
        assert!(
            h1_node.bottom > 10.0,
            "h1 should have bottom padding from UA margin"
        );
    }

    /// Inline elements (<strong>, <em>, <code>) should merge into a single AttributedText.
    #[test]
    fn test_inline_elements_merge_to_attributed_text() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body style="margin:0;">
  <p>Hello <strong>world</strong>!</p>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        // Find the AttributedText node
        let attr_node = nodes
            .iter()
            .find_map(|id| match graph.get_node(id).ok()? {
                Node::AttributedText(n) => Some(n),
                _ => None,
            })
            .expect("should produce an AttributedText node");

        // Full text should be the concatenation of all inline segments
        assert!(
            attr_node.attributed_string.text.contains("Hello"),
            "text should contain 'Hello'"
        );
        assert!(
            attr_node.attributed_string.text.contains("world"),
            "text should contain 'world'"
        );

        // Should have multiple runs (at least: normal + bold + normal)
        assert!(
            attr_node.attributed_string.runs.len() >= 2,
            "should have at least 2 styled runs, got {}",
            attr_node.attributed_string.runs.len()
        );

        // The "world" run should be bold (font-weight >= 700)
        let bold_run = attr_node
            .attributed_string
            .runs
            .iter()
            .find(|r| {
                let text = &attr_node.attributed_string.text[r.start as usize..r.end as usize];
                text.contains("world")
            })
            .expect("should find a run containing 'world'");
        assert!(
            bold_run.style.font_weight.0 >= 700,
            "bold run should have weight >= 700, got {}",
            bold_run.style.font_weight.0
        );

        // There should be NO separate TextSpan nodes (everything merged)
        let text_span_count = nodes
            .iter()
            .filter(|id| matches!(graph.get_node(id).ok(), Some(Node::TextSpan(_))))
            .count();
        assert_eq!(text_span_count, 0, "no separate TextSpan nodes expected");
    }

    /// Whitespace between inline elements must be preserved.
    #[test]
    fn test_inline_whitespace_preserved() {
        let _guard = crate::stylo_test::lock();
        let html = r#"<!doctype html>
<html><body style="margin:0;">
  <p>Default <span style="color: red;">red</span> and <span style="color: green;">green</span> text.</p>
</body></html>"#;
        let graph = html_graph(html);
        let nodes = dfs_nodes(&graph);

        let attr_node = nodes
            .iter()
            .find_map(|id| match graph.get_node(id).ok()? {
                Node::AttributedText(n) => Some(n),
                _ => None,
            })
            .expect("should produce an AttributedText node");

        let text = &attr_node.attributed_string.text;
        assert!(
            text.contains("Default "),
            "should preserve space after 'Default', got: {:?}",
            text
        );
        assert!(
            text.contains(" and "),
            "should preserve spaces around 'and', got: {:?}",
            text
        );
        assert!(
            text.contains(" text."),
            "should preserve space before 'text.', got: {:?}",
            text
        );
    }
}
