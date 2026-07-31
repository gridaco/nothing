//! SVG `<use>` expansion: the use-element shadow tree, flattened into the
//! one document tree before the one cascade runs.
//!
//! SVG 2 (struct.html §5.6) renders a `<use>` "as if the host element ...
//! was a container and the shadow content was its descendents", with the
//! instance inheriting computed values from the `<use>` element
//! (css-scoping §3.3.2: "the top-level elements of a shadow tree inherit
//! from their host element"). Physically cloning the referenced subtree
//! under the `<use>` before styling reproduces exactly that: the clone is
//! styled by the same single cascade pass, its presentation attributes and
//! `style` attribute clone with it (§5.6.3), and inheritance flows through
//! the use site — all Chromium-measured (the use/defs rung's probe matrix:
//! `fill` on the use colors a clone that sets none of its own; a target's
//! own attribute beats the use's; `currentColor` resolves against the use
//! site's `color`; `display: none` cloned onto the instance prunes it).
//!
//! What expansion deliberately does NOT reproduce is shadow-scoped
//! *selector matching*: the measured boundary is total — no ancestor
//! outside the cloned subtree participates, not even through descendant
//! combinators — while a clone parented under the `<use>` in the one tree
//! would let every ancestor combinator through. The websem compiler
//! therefore refuses a `<use>` in any document that carries author CSS;
//! this module is mechanical and never refuses. It communicates the three
//! conditions the compiler must refuse by name through
//! [`SvgUseRefusal`] on the use element itself.
//!
//! Resolution mechanics, each Chromium-measured: the id table covers the
//! whole document (forward references resolve) and the first id in tree
//! order wins; a plain `href` beats `xlink:href` when both are present; a
//! missing reference expands to nothing and renders nothing; a reference
//! to a (shadow-including) ancestor is an invalid circular reference and
//! expands to nothing; a mutual `use` cycle expands to skeleton `<use>`
//! clones that paint nothing. `<style>` and `<script>` are never cloned —
//! a stylesheet applies to the document once, not once per instance.

use std::collections::HashMap;

use markup5ever::ns;

use crate::dom::{DemoElementData, DemoNode, DemoNodeData, NodeId};

/// Why the websem compiler must refuse this `<use>` by name instead of
/// walking its (possibly partial) expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgUseRefusal {
    /// The reference is not a same-document fragment. The engine is
    /// declared resource-free; Chromium with a network would render the
    /// external content, so silence would be a wrong pixel.
    ExternalReference,
    /// The use has authored element children beyond `title`/`desc`/
    /// `metadata`. The shadow content replaces authored children in
    /// Chromium; painting them would be a wrong pixel, and this slice
    /// refuses rather than models that replacement.
    AuthoredChildren,
    /// Expansion hit the depth or node budget — an indirect reference
    /// cycle beyond the measured shapes, or a pathological fan-out.
    /// The partial expansion must not paint.
    ExpansionOverflow,
}

/// The chain-depth bound. Chains this deep are indistinguishable from the
/// indirect-cycle shapes SVG 2's own issue list calls defective
/// (svgwg#770); past it the use refuses loudly rather than recursing.
const MAX_USE_CHAIN_DEPTH: usize = 40;

/// Total cloned-node budget per document — bounds the billion-laughs
/// fan-out a chain of group-referencing uses can author.
const MAX_CLONED_NODES: usize = 65_536;

/// Expand every SVG `<use>` in the frozen tree, in document order.
/// Runs after parsing and before `element_data` is sized, so clones are
/// ordinary tree members by the time the cascade walks the document.
pub(crate) fn expand_svg_use_references(nodes: &mut Vec<DemoNode>, document: NodeId) {
    let id_table = build_id_table(nodes, document);
    let uses = collect_uses(nodes, document);
    let mut budget = MAX_CLONED_NODES;
    for use_id in uses {
        // Authored element children (beyond the descriptive set) would
        // paint beside the shadow content; refuse before expanding.
        if has_authored_element_children(nodes, use_id) {
            set_refusal(nodes, use_id, SvgUseRefusal::AuthoredChildren);
            continue;
        }
        let mut chain = Vec::new();
        expand_use(nodes, use_id, use_id, &id_table, &mut chain, &mut budget);
    }
}

/// `chain` carries every use id and target id on the current expansion
/// path (push/pop discipline), so sibling expansions never see each
/// other's history — only a true ancestor closes a cycle.
fn expand_use(
    nodes: &mut Vec<DemoNode>,
    use_id: NodeId,
    root_use: NodeId,
    id_table: &HashMap<String, NodeId>,
    chain: &mut Vec<NodeId>,
    budget: &mut usize,
) {
    let Some(fragment) = resolve_href(nodes, use_id) else {
        return; // no href: renders nothing, like a missing reference
    };
    let fragment = match fragment {
        HrefValue::Fragment(f) => f,
        HrefValue::External => {
            set_refusal(nodes, root_use, SvgUseRefusal::ExternalReference);
            return;
        }
    };
    let Some(&target) = id_table.get(&fragment) else {
        return; // unresolved reference: renders nothing (measured)
    };
    // A reference to a shadow-including ancestor is an invalid circular
    // reference: the use is in error and renders nothing (measured: the
    // enclosing content paints once, the self-instance never).
    if is_ancestor_or_self(nodes, target, use_id) {
        return;
    }
    // A target already on this chain closes a reference cycle: either it
    // is itself mid-expansion (the mutual-use pair) or it was this chain's
    // own earlier target reached again through expanded content. Both
    // render nothing (measured: the cyclic pair paints nothing while the
    // document renders), so the instance stays childless, silently.
    if chain.contains(&target) || chain.contains(&use_id) {
        return;
    }
    if chain.len() >= 2 * MAX_USE_CHAIN_DEPTH {
        set_refusal(nodes, root_use, SvgUseRefusal::ExpansionOverflow);
        return;
    }
    chain.push(use_id);
    chain.push(target);
    let mut cloned = Vec::new();
    if !clone_subtree(nodes, target, use_id, budget, &mut cloned) {
        set_refusal(nodes, root_use, SvgUseRefusal::ExpansionOverflow);
        chain.truncate(chain.len() - 2);
        return;
    }
    // The clone of a use arrives childless when its original was not yet
    // (or never) expanded; chase each one with the chain carried. A clone
    // of an already-expanded use carries its expansion and needs nothing.
    for clone_id in cloned {
        if is_svg_use(nodes, clone_id) && element_children(nodes, clone_id).is_empty() {
            expand_use(nodes, clone_id, root_use, id_table, chain, budget);
        }
    }
    chain.truncate(chain.len() - 2);
}

/// Deep-clone `src` as the last child of `parent`. Element and text nodes
/// clone; `<style>`/`<script>` (and non-element metadata nodes) do not —
/// a stylesheet applies to the document once, not once per instance, and
/// script-bearing documents are refused upstream. Every cloned node id is
/// appended to `out`. Returns false when the node budget runs out.
fn clone_subtree(
    nodes: &mut Vec<DemoNode>,
    src: NodeId,
    parent: NodeId,
    budget: &mut usize,
    out: &mut Vec<NodeId>,
) -> bool {
    let data = match &nodes[src.idx()].data {
        DemoNodeData::Element(element) => {
            let local = element.name.local.as_ref();
            if local.eq_ignore_ascii_case("style") || local.eq_ignore_ascii_case("script") {
                return true;
            }
            DemoNodeData::Element(DemoElementData {
                name: element.name.clone(),
                attrs: element.attrs.clone(),
                template_contents: None,
                mathml_annotation_xml_integration_point: element
                    .mathml_annotation_xml_integration_point,
                id_attr: element.id_attr.clone(),
                class_list: element.class_list.clone(),
                attr_local_names: element.attr_local_names.clone(),
                style_local_name: element.style_local_name.clone(),
                style_namespace: element.style_namespace.clone(),
                style_attribute: element.style_attribute.clone(),
                presentation_hints: element.presentation_hints.clone(),
                svg_use_refusal: element.svg_use_refusal,
            })
        }
        DemoNodeData::Text(text) => DemoNodeData::Text(text.clone()),
        _ => return true, // comments, PIs, doctypes: inert, not cloned
    };
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    let clone_id = NodeId(nodes.len());
    nodes.push(DemoNode {
        parent: Some(parent),
        children: Vec::new(),
        data,
    });
    nodes[parent.idx()].children.push(clone_id);
    out.push(clone_id);
    let children = nodes[src.idx()].children.clone();
    for child in children {
        if !clone_subtree(nodes, child, clone_id, budget, out) {
            return false;
        }
    }
    true
}

enum HrefValue {
    Fragment(String),
    External,
}

/// The use's reference: a plain `href` wins over `xlink:href` when both
/// are present (SVG 2 linking; measured — the deprecated spelling is
/// ignored, not merged).
fn resolve_href(nodes: &[DemoNode], use_id: NodeId) -> Option<HrefValue> {
    let element = as_element(nodes, use_id)?;
    let plain = element
        .attrs
        .iter()
        .find(|attr| attr.name.ns.as_ref().is_empty() && attr.name.local.as_ref() == "href");
    let xlink = element
        .attrs
        .iter()
        .find(|attr| attr.name.ns == ns!(xlink) && attr.name.local.as_ref() == "href");
    let value = plain.or(xlink)?.value.trim().to_string();
    match value.strip_prefix('#') {
        Some(fragment) => Some(HrefValue::Fragment(fragment.to_string())),
        None => Some(HrefValue::External),
    }
}

/// The document-order id table, first id wins (DOM `getElementById`
/// semantics; measured — a duplicate id resolves to the first in tree
/// order). Built once over the pre-expansion tree: clones are never
/// referenceable, so an id inside an instance resolves to its original.
fn build_id_table(nodes: &[DemoNode], document: NodeId) -> HashMap<String, NodeId> {
    let mut table = HashMap::new();
    for node in tree_order(nodes, document) {
        if let Some(element) = as_element(nodes, node)
            && let Some(id) = element
                .attrs
                .iter()
                .find(|attr| attr.name.ns.as_ref().is_empty() && attr.name.local.as_ref() == "id")
        {
            table.entry(id.value.to_string()).or_insert(node);
        }
    }
    table
}

fn collect_uses(nodes: &[DemoNode], document: NodeId) -> Vec<NodeId> {
    tree_order(nodes, document)
        .into_iter()
        .filter(|&node| is_svg_use(nodes, node))
        .collect()
}

fn tree_order(nodes: &[DemoNode], document: NodeId) -> Vec<NodeId> {
    let mut order = Vec::new();
    let mut stack = vec![document];
    while let Some(node) = stack.pop() {
        order.push(node);
        for &child in nodes[node.idx()].children.iter().rev() {
            stack.push(child);
        }
    }
    order
}

fn as_element(nodes: &[DemoNode], id: NodeId) -> Option<&DemoElementData> {
    match &nodes[id.idx()].data {
        DemoNodeData::Element(element) => Some(element),
        _ => None,
    }
}

fn is_svg_use(nodes: &[DemoNode], id: NodeId) -> bool {
    as_element(nodes, id)
        .is_some_and(|element| element.name.ns == ns!(svg) && element.name.local.as_ref() == "use")
}

fn is_ancestor_or_self(nodes: &[DemoNode], candidate: NodeId, of: NodeId) -> bool {
    let mut cursor = Some(of);
    while let Some(node) = cursor {
        if node == candidate {
            return true;
        }
        cursor = nodes[node.idx()].parent;
    }
    false
}

/// Authored element children beyond the descriptive set (`title`/`desc`/
/// `metadata`, which SVG permits on `use` and which never paint).
fn has_authored_element_children(nodes: &[DemoNode], use_id: NodeId) -> bool {
    element_children(nodes, use_id).iter().any(|&child| {
        as_element(nodes, child).is_some_and(|element| {
            !matches!(element.name.local.as_ref(), "title" | "desc" | "metadata")
        })
    })
}

fn element_children(nodes: &[DemoNode], id: NodeId) -> Vec<NodeId> {
    nodes[id.idx()]
        .children
        .iter()
        .copied()
        .filter(|&child| as_element(nodes, child).is_some())
        .collect()
}

fn set_refusal(nodes: &mut [DemoNode], id: NodeId, refusal: SvgUseRefusal) {
    if let DemoNodeData::Element(element) = &mut nodes[id.idx()].data {
        element.svg_use_refusal = Some(refusal);
    }
}
