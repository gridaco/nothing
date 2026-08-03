//! Arena-based DOM representation for csscascade.
//!
//! Provides [`DemoDom`] — a flat, arena-allocated DOM tree built through the
//! shared markup5ever [`TreeSink`] trait. Two grammar entries drive the same
//! sink into the same semantic document shape: html5ever for HTML documents
//! and xml5ever for conforming standalone SVG/XML documents (namespace-aware,
//! case-preserving). Every node lives in a `Vec<DemoNode>` and is addressed
//! by a lightweight [`NodeId`] index.  After parsing, the DOM is frozen and
//! handed off to the Stylo adapter layer ([`crate::adapter`]).

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    io::{self, Cursor},
};

use html5ever::tendril::TendrilSink;
use html5ever::{driver::ParseOpts, parse_document};
use markup5ever::interface::tree_builder::{
    ElemName as ElemNameTrait, ElementFlags, NodeOrText, QuirksMode, TreeSink,
};
use markup5ever::{Attribute, LocalName, Namespace, QualName};
use std::sync::OnceLock;
use style::context::QuirksMode as StyleQuirksMode;
use style::data::ElementDataWrapper;
use style::properties::{
    Importance, LonghandId, PropertyId, SourcePropertyDeclaration, parse_one_declaration_into,
    parse_style_attribute,
};
use style::servo_arc::Arc;
use style::stylesheets::{CssRuleType, Origin, UrlExtraData};
use style::{
    LocalName as StyleLocalName, Namespace as StyleNamespace,
    properties::PropertyDeclarationBlock,
    shared_lock::{Locked, SharedRwLock},
    values::AtomIdent,
};
use style_traits::ParsingMode;
use stylo_atoms::Atom as WeakAtom;
use tendril::StrTendril;
use url::Url;
use xml5ever::driver::{XmlParseOpts, parse_document as parse_xml_document};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Index into the DOM arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NodeId(pub(crate) usize);

impl NodeId {
    pub(crate) fn idx(self) -> usize {
        self.0
    }
}

/// A single DOM node.
#[derive(Debug)]
pub struct DemoNode {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub data: DemoNodeData,
}

/// Payload carried by a [`DemoNode`].
#[derive(Debug)]
pub enum DemoNodeData {
    Document,
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    Text(StrTendril),
    Comment(StrTendril),
    Element(DemoElementData),
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

/// Extra metadata kept for element nodes.
#[derive(Debug)]
pub struct DemoElementData {
    pub name: QualName,
    pub attrs: Vec<Attribute>,
    pub template_contents: Option<NodeId>,
    pub mathml_annotation_xml_integration_point: bool,
    /// Pre-extracted `id` attribute value (if any).
    pub id_attr: Option<WeakAtom>,
    /// Pre-split class tokens.
    pub class_list: Vec<AtomIdent>,
    /// Style-compatible local names for each attribute.
    pub attr_local_names: Vec<StyleLocalName>,
    pub style_local_name: StyleLocalName,
    pub style_namespace: StyleNamespace,
    /// Parsed inline `style` attribute, if present.
    pub style_attribute: Option<Arc<Locked<PropertyDeclarationBlock>>>,
    /// Admitted SVG presentation attributes, pre-parsed as the SVG2
    /// presentation-hint declaration block — author origin, below every
    /// author rule (`CascadeLevel::PresHints`).
    pub presentation_hints: Option<Arc<Locked<PropertyDeclarationBlock>>>,
    /// Set on an SVG `<use>` element whose expansion the compiler must
    /// refuse by name instead of walking (see [`crate::svg_use`]).
    /// `None` for every other element and for every cleanly expanded,
    /// empty, or reference-less use.
    pub svg_use_refusal: Option<crate::svg_use::SvgUseRefusal>,
}

/// The frozen, arena-allocated DOM tree.
#[derive(Debug)]
pub struct DemoDom {
    nodes: Vec<DemoNode>,
    document: NodeId,
    quirks_mode: QuirksMode,
    shared_lock: SharedRwLock,
    pub errors: Vec<String>,
    /// Per-node slot for Stylo [`ElementDataWrapper`] (only meaningful for
    /// elements). Populated lazily the first time Stylo's traversal calls
    /// `ensure_data` on the element.
    pub(crate) element_data: Vec<OnceLock<ElementDataWrapper>>,
}

impl DemoDom {
    /// Parse a complete HTML document from raw bytes.
    pub fn parse_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut reader = Cursor::new(bytes);
        let dom = parse_document(DemoDomBuilder::new(), ParseOpts::default())
            .from_utf8()
            .read_from(&mut reader)?;
        Ok(dom)
    }

    /// Parse a standalone SVG/XML document from raw bytes into the same
    /// semantic DOM shape [`Self::parse_from_bytes`] produces for HTML.
    ///
    /// The XML grammar is namespace-aware and case-preserving: element and
    /// attribute names keep their authored case, and namespaces come from
    /// authored `xmlns` declarations rather than HTML foreign-content rules.
    /// xml5ever implements the error-recovering XML5 grammar; recoveries the
    /// grammar records are surfaced in [`DemoDom::errors`] so a strict caller
    /// can refuse recovered-from input, while the recovery classes XML5
    /// deliberately leaves unrecorded are pinned as executable boundary laws
    /// in `tests/xml_document_entry.rs`.
    pub fn parse_xml_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut reader = Cursor::new(bytes);
        let dom = parse_xml_document(DemoDomBuilder::new(), XmlParseOpts::default())
            .from_utf8()
            .read_from(&mut reader)?;
        Ok(dom)
    }

    pub fn document_id(&self) -> NodeId {
        self.document
    }

    pub fn document_children(&self) -> &[NodeId] {
        &self.nodes[self.document.idx()].children
    }

    pub fn quirks_mode(&self) -> QuirksMode {
        self.quirks_mode
    }

    pub fn node(&self, id: NodeId) -> &DemoNode {
        &self.nodes[id.idx()]
    }

    pub fn get_node(&self, id: NodeId) -> Option<&DemoNode> {
        self.nodes.get(id.idx())
    }

    pub(crate) fn shared_lock(&self) -> &SharedRwLock {
        &self.shared_lock
    }

    pub(crate) fn element_data_slot(&self, id: NodeId) -> &OnceLock<ElementDataWrapper> {
        &self.element_data[id.idx()]
    }

    pub fn all_node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        (0..self.nodes.len()).map(NodeId)
    }

    pub(crate) fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ---------------------------------------------------------------------------
// TreeSink builder (internal)
// ---------------------------------------------------------------------------

struct DemoDomBuilder {
    nodes: RefCell<Vec<NodeTemp>>,
    document: NodeId,
    errors: RefCell<Vec<Cow<'static, str>>>,
    quirks_mode: Cell<QuirksMode>,
    shared_lock: SharedRwLock,
}

#[derive(Debug)]
struct NodeTemp {
    parent: Cell<Option<NodeId>>,
    children: RefCell<Vec<NodeId>>,
    data: NodeDataTemp,
}

impl NodeTemp {
    fn new(data: NodeDataTemp) -> Self {
        Self {
            parent: Cell::new(None),
            children: RefCell::new(Vec::new()),
            data,
        }
    }
}

#[derive(Debug)]
enum NodeDataTemp {
    Document,
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    Text {
        contents: RefCell<StrTendril>,
    },
    Comment {
        contents: StrTendril,
    },
    Element {
        name: QualName,
        attrs: RefCell<Vec<Attribute>>,
        template_contents: RefCell<Option<NodeId>>,
        mathml_annotation_xml_integration_point: bool,
    },
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

#[derive(Debug, Clone)]
struct OwnedElemName(QualName);

impl ElemNameTrait for OwnedElemName {
    fn ns(&self) -> &Namespace {
        &self.0.ns
    }
    fn local_name(&self) -> &LocalName {
        &self.0.local
    }
}

impl DemoDomBuilder {
    fn new() -> Self {
        let nodes = vec![NodeTemp::new(NodeDataTemp::Document)];
        Self {
            nodes: RefCell::new(nodes),
            document: NodeId(0),
            errors: RefCell::new(Vec::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
            shared_lock: SharedRwLock::new(),
        }
    }

    fn new_node(&self, data: NodeDataTemp) -> NodeId {
        let mut nodes = self.nodes.borrow_mut();
        let id = NodeId(nodes.len());
        nodes.push(NodeTemp::new(data));
        id
    }

    fn node_parent(&self, id: NodeId) -> Option<NodeId> {
        let nodes = self.nodes.borrow();
        nodes[id.idx()].parent.get()
    }

    fn set_parent(&self, id: NodeId, parent: Option<NodeId>) {
        let nodes = self.nodes.borrow();
        nodes[id.idx()].parent.set(parent);
    }

    fn append_child(&self, parent: NodeId, child: NodeId) {
        self.set_parent(child, Some(parent));
        let nodes = self.nodes.borrow();
        nodes[parent.idx()].children.borrow_mut().push(child);
    }

    fn last_child(&self, parent: NodeId) -> Option<NodeId> {
        let nodes = self.nodes.borrow();
        nodes[parent.idx()].children.borrow().last().copied()
    }

    fn append_to_existing_text(&self, node: NodeId, text: &str) -> bool {
        let nodes = self.nodes.borrow();
        if let NodeDataTemp::Text { contents } = &nodes[node.idx()].data {
            contents.borrow_mut().push_slice(text);
            return true;
        }
        false
    }

    fn remove_from_parent(&self, target: NodeId) {
        if let Some((parent, index)) = self.get_parent_and_index(target) {
            self.set_parent(target, None);
            let nodes = self.nodes.borrow();
            nodes[parent.idx()].children.borrow_mut().remove(index);
        }
    }

    fn get_parent_and_index(&self, target: NodeId) -> Option<(NodeId, usize)> {
        let nodes = self.nodes.borrow();
        let parent = nodes[target.idx()].parent.get()?;
        let idx = nodes[parent.idx()]
            .children
            .borrow()
            .iter()
            .position(|&child| child == target)
            .expect("parent missing child");
        Some((parent, idx))
    }

    fn insert_child_at(&self, parent: NodeId, index: usize, child: NodeId) {
        self.remove_from_parent(child);
        self.set_parent(child, Some(parent));
        let nodes = self.nodes.borrow();
        nodes[parent.idx()]
            .children
            .borrow_mut()
            .insert(index, child);
    }

    fn create_text_node(&self, text: StrTendril) -> NodeId {
        self.new_node(NodeDataTemp::Text {
            contents: RefCell::new(text),
        })
    }

    fn node_used_for_template(&self, handle: NodeId) -> NodeId {
        let nodes = self.nodes.borrow();
        if let NodeDataTemp::Element {
            template_contents, ..
        } = &nodes[handle.idx()].data
        {
            template_contents
                .borrow()
                .as_ref()
                .copied()
                .expect("missing template contents")
        } else {
            panic!("not a template element");
        }
    }

    fn add_attrs_if_missing_impl(&self, target: NodeId, attrs: Vec<Attribute>) {
        let nodes = self.nodes.borrow();
        let NodeDataTemp::Element {
            attrs: existing, ..
        } = &nodes[target.idx()].data
        else {
            panic!("not an element");
        };
        let mut existing = existing.borrow_mut();
        let existing_names: Vec<_> = existing.iter().map(|attr| attr.name.clone()).collect();
        for attr in attrs {
            if existing_names.contains(&attr.name) {
                continue;
            }
            existing.push(attr);
        }
    }
}

impl TreeSink for DemoDomBuilder {
    type Handle = NodeId;
    type Output = DemoDom;

    type ElemName<'a>
        = OwnedElemName
    where
        Self: 'a;

    fn finish(self) -> Self::Output {
        let quirks = self.quirks_mode.get();
        let document = self.document;
        let shared_lock = self.shared_lock;
        // (SVG use expansion runs below, after the node vector is built and
        // before element_data is sized to it.)
        let errors = self
            .errors
            .into_inner()
            .into_iter()
            .map(|e| e.into_owned())
            .collect();
        let nodes: Vec<DemoNode> = self
            .nodes
            .into_inner()
            .into_iter()
            .map(|node| DemoNode {
                parent: node.parent.get(),
                children: node.children.into_inner(),
                data: match node.data {
                    NodeDataTemp::Document => DemoNodeData::Document,
                    NodeDataTemp::Doctype {
                        name,
                        public_id,
                        system_id,
                    } => DemoNodeData::Doctype {
                        name,
                        public_id,
                        system_id,
                    },
                    NodeDataTemp::Text { contents } => DemoNodeData::Text(contents.into_inner()),
                    NodeDataTemp::Comment { contents } => DemoNodeData::Comment(contents),
                    NodeDataTemp::Element {
                        name,
                        attrs,
                        template_contents,
                        mathml_annotation_xml_integration_point,
                    } => {
                        let attrs_vec = attrs.into_inner();
                        let (id_attr, class_list, attr_local_names, style_value) =
                            derive_attr_metadata(&attrs_vec);
                        let style_local_name = style_local_name_from(&name.local);
                        let style_namespace = style_namespace_from(&name.ns);

                        let style_attribute = style_value.map(|css_text| {
                            let url = Url::parse("about:blank").unwrap();
                            let url_data = UrlExtraData::from(url);
                            let block = parse_style_attribute(
                                &css_text,
                                &url_data,
                                None,
                                StyleQuirksMode::NoQuirks,
                                CssRuleType::Style,
                            );
                            let locked = shared_lock.wrap(block);
                            Arc::new(locked)
                        });
                        let presentation_hints =
                            svg_presentation_hints(&name, &attrs_vec, &shared_lock);

                        DemoNodeData::Element(DemoElementData {
                            name,
                            attrs: attrs_vec,
                            template_contents: template_contents.into_inner(),
                            mathml_annotation_xml_integration_point,
                            id_attr,
                            class_list,
                            attr_local_names,
                            style_local_name,
                            style_namespace,
                            style_attribute,
                            presentation_hints,
                            svg_use_refusal: None,
                        })
                    }
                    NodeDataTemp::ProcessingInstruction { target, contents } => {
                        DemoNodeData::ProcessingInstruction { target, contents }
                    }
                },
            })
            .collect();

        let mut nodes = nodes;
        crate::svg_use::expand_svg_use_references(&mut nodes, document);

        let element_data = nodes.iter().map(|_| OnceLock::new()).collect();

        DemoDom {
            nodes,
            document,
            quirks_mode: quirks,
            shared_lock,
            errors,
            element_data,
        }
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.errors.borrow_mut().push(msg);
    }

    fn get_document(&self) -> Self::Handle {
        self.document
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        let nodes = self.nodes.borrow();
        match &nodes[target.idx()].data {
            NodeDataTemp::Element { name, .. } => OwnedElemName(name.clone()),
            _ => panic!("not an element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        let template_contents = if flags.template {
            Some(self.new_node(NodeDataTemp::Document))
        } else {
            None
        };
        self.new_node(NodeDataTemp::Element {
            name,
            attrs: RefCell::new(attrs),
            template_contents: RefCell::new(template_contents),
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        self.new_node(NodeDataTemp::Comment { contents: text })
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        self.new_node(NodeDataTemp::ProcessingInstruction {
            target,
            contents: data,
        })
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        if let NodeOrText::AppendText(ref text) = child
            && let Some(last) = self.last_child(*parent)
            && self.append_to_existing_text(last, text)
        {
            return;
        }

        let new_child = match child {
            NodeOrText::AppendText(text) => self.create_text_node(text),
            NodeOrText::AppendNode(node) => {
                self.remove_from_parent(node);
                node
            }
        };

        self.append_child(*parent, new_child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if self.node_parent(*element).is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let node = self.new_node(NodeDataTemp::Doctype {
            name,
            public_id,
            system_id,
        });
        self.append_child(self.document, node);
    }

    fn mark_script_already_started(&self, _node: &Self::Handle) {}

    fn pop(&self, _node: &Self::Handle) {}

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        self.node_used_for_template(*target)
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let (parent, index) = self
            .get_parent_and_index(*sibling)
            .expect("sibling missing parent");

        let new_child = match (child, index) {
            (NodeOrText::AppendText(text), 0) => self.create_text_node(text),
            (NodeOrText::AppendText(text), i) => {
                let nodes = self.nodes.borrow();
                let prev = nodes[parent.idx()].children.borrow()[i - 1];
                drop(nodes);
                if self.append_to_existing_text(prev, &text) {
                    return;
                }
                self.create_text_node(text)
            }
            (NodeOrText::AppendNode(node), _) => {
                self.remove_from_parent(node);
                node
            }
        };

        self.insert_child_at(parent, index, new_child);
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        self.add_attrs_if_missing_impl(*target, attrs);
    }

    fn associate_with_form(
        &self,
        _target: &Self::Handle,
        _form: &Self::Handle,
        _nodes: (&Self::Handle, Option<&Self::Handle>),
    ) {
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.remove_from_parent(*target);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        loop {
            let next_child = {
                let nodes = self.nodes.borrow();
                nodes[node.idx()].children.borrow().first().copied()
            };
            let Some(child) = next_child else {
                break;
            };
            self.remove_from_parent(child);
            self.append_child(*new_parent, child);
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Self::Handle) -> bool {
        let nodes = self.nodes.borrow();
        if let NodeDataTemp::Element {
            mathml_annotation_xml_integration_point,
            ..
        } = &nodes[handle.idx()].data
        {
            *mathml_annotation_xml_integration_point
        } else {
            false
        }
    }

    fn set_current_line(&self, _line_number: u64) {}

    fn allow_declarative_shadow_roots(&self, _intended_parent: &Self::Handle) -> bool {
        true
    }

    fn attach_declarative_shadow(
        &self,
        _location: &Self::Handle,
        _template: &Self::Handle,
        _attrs: &[Attribute],
    ) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn derive_attr_metadata(
    attrs: &[Attribute],
) -> (
    Option<WeakAtom>,
    Vec<AtomIdent>,
    Vec<StyleLocalName>,
    Option<String>,
) {
    let mut id_attr = None;
    let mut class_list = Vec::new();
    let mut attr_local_names = Vec::with_capacity(attrs.len());
    let mut style_value = None;

    for attr in attrs {
        attr_local_names.push(StyleLocalName::from(attr.name.local.as_ref()));
        if !is_htmlish_namespace(&attr.name.ns) {
            continue;
        }

        let local = attr.name.local.as_ref();
        if id_attr.is_none() && local.eq_ignore_ascii_case("id") {
            id_attr = Some(WeakAtom::from(attr.value.as_ref()));
        } else if local.eq_ignore_ascii_case("class") {
            class_list = parse_class_list(attr.value.as_ref());
        } else if local.eq_ignore_ascii_case("style") {
            style_value = Some(attr.value.to_string());
        }
    }

    (id_attr, class_list, attr_local_names, style_value)
}

/// SVG presentation attributes admitted into the cascade as presentation
/// hints. Each entry is a semantic claim gated by the precedence laws in
/// `tests/svg_presentation_hints.rs`; the set grows one capability step at a
/// time — never speculatively.
fn admitted_svg_presentation_property(local: &str) -> Option<LonghandId> {
    match local {
        "fill" => Some(LonghandId::Fill),
        "fill-rule" => Some(LonghandId::FillRule),
        "stroke" => Some(LonghandId::Stroke),
        "stroke-width" => Some(LonghandId::StrokeWidth),
        "stroke-linecap" => Some(LonghandId::StrokeLinecap),
        "stroke-linejoin" => Some(LonghandId::StrokeLinejoin),
        "stroke-miterlimit" => Some(LonghandId::StrokeMiterlimit),
        // Not painted by any consumer yet, and admitted anyway: `font-size` is
        // the basis for an `em`/`rem` length, and `stroke-width` is now a
        // consumed length. Chromium treats it as a presentation attribute
        // (measured: `<g font-size="32">` makes a `0.5em` stroke 16px), so
        // dropping it here computed the wrong width from the right document.
        "font-size" => Some(LonghandId::FontSize),
        // The visibility rung's pair. Both are SVG2 presentation attributes
        // whose author-rule precedence is measured (a stylesheet
        // `visibility: visible` overrides `visibility="hidden"` in
        // Chromium), and an invalid value drops exactly as an invalid CSS
        // declaration — `display="bogus"` renders (measured).
        "display" => Some(LonghandId::Display),
        "visibility" => Some(LonghandId::Visibility),
        // The translucency rung's pair: both fold into paint alpha at the
        // consumer, and both take the CSS <alpha-value> grammar (number or
        // percentage, clamped) exactly as the SVG2 presentation attribute.
        "fill-opacity" => Some(LonghandId::FillOpacity),
        "stroke-opacity" => Some(LonghandId::StrokeOpacity),
        // The group-scope rung's addition: element `opacity` is the same
        // <alpha-value> grammar as a presentation attribute (measured: the
        // percentage spelling paints, an author rule or style attribute
        // beats it, and an invalid value drops so the element renders
        // opaque), consumed by websem as a fold or a compositing scope.
        "opacity" => Some(LonghandId::Opacity),
        // The use/defs rung's addition: `color` is the `currentColor`
        // basis, inherited, and Chromium honors the attribute spelling
        // (measured: `color` on a `<use>` colors a `currentColor` fill
        // inside the instance).
        "color" => Some(LonghandId::Color),
        _ => None,
    }
}

/// Build the SVG2 presentation-hint declaration block for one SVG-namespace
/// element, if any admitted presentation attribute parses. Hints enter the
/// author origin below every author rule; a value that fails its property
/// grammar is dropped exactly as an invalid CSS declaration would be.
fn svg_presentation_hints(
    name: &QualName,
    attrs: &[Attribute],
    shared_lock: &SharedRwLock,
) -> Option<Arc<Locked<PropertyDeclarationBlock>>> {
    if name.ns != markup5ever::ns!(svg) {
        return None;
    }
    // On a gradient element the transform property's presentation attribute
    // is `gradientTransform`, and the plain `transform` attribute is inert
    // (measured: it changes no pixel in Chromium). Both spellings share one
    // grammar and one measured rewrite; the computed value is applied about
    // the raw origin in gradient space, identically for the attribute and an
    // author `transform` declaration (measured with non-quarter rotations and
    // scales — byte-identical).
    let transform_attribute = match name.local.as_ref() {
        "linearGradient" | "radialGradient" => "gradientTransform",
        _ => "transform",
    };
    let mut block = PropertyDeclarationBlock::new();
    let mut source = SourcePropertyDeclaration::default();
    let mut parsed_any = false;
    for attr in attrs {
        if !attr.name.ns.as_ref().is_empty() {
            continue;
        }
        if attr.name.local.as_ref() == "transform" && transform_attribute != "transform" {
            continue;
        }
        let url_data = UrlExtraData::from(Url::parse("about:blank").unwrap());
        // The `transform` attribute is a presentation attribute of the CSS
        // `transform` property (CSS Transforms L1 §7) whose grammar the CSS
        // parser cannot read — unitless numbers, comma-wsp, the 3-argument
        // rotate. It enters through its own measured rewrite: a valid list
        // becomes equivalent CSS text, a malformed one becomes no hint at
        // all, which renders untransformed exactly as Chromium drops it.
        if attr.name.local.as_ref() == transform_attribute {
            if let Some(css) = crate::svg_transform::transform_attribute_to_css(&attr.value) {
                let mut throwaway = SourcePropertyDeclaration::default();
                if parse_one_declaration_into(
                    &mut throwaway,
                    PropertyId::NonCustom(LonghandId::Transform.into()),
                    &css,
                    Origin::Author,
                    &url_data,
                    None,
                    ParsingMode::DEFAULT,
                    StyleQuirksMode::NoQuirks,
                    CssRuleType::Style,
                )
                .is_ok()
                {
                    block.extend(throwaway.drain(), Importance::Normal);
                    parsed_any = true;
                } else {
                    // The rewrite's output is CSS by construction; a parse
                    // failure here is a rewrite bug, never author input.
                    debug_assert!(false, "rewritten transform must parse: {css}");
                }
            }
            continue;
        }
        let Some(longhand) = admitted_svg_presentation_property(attr.name.local.as_ref()) else {
            continue;
        };
        fn parse(
            source: &mut SourcePropertyDeclaration,
            longhand: LonghandId,
            url_data: &UrlExtraData,
            value: &str,
        ) -> bool {
            parse_one_declaration_into(
                source,
                PropertyId::NonCustom(longhand.into()),
                value,
                Origin::Author,
                url_data,
                None,
                ParsingMode::DEFAULT,
                StyleQuirksMode::NoQuirks,
                CssRuleType::Style,
            )
            .is_ok()
        }
        // SVG presentation attributes take a length in *user units*, so a bare
        // number is valid where the CSS property grammar requires a unit —
        // Blink parses these in a dedicated SVG attribute mode for exactly this
        // reason. Retrying a rejected bare number as `px` reproduces it, and
        // can only ever turn a dropped declaration into the one the browser
        // computed: a property whose grammar takes no length rejects the number
        // either way. (`stroke-width` needs no retry — SVG's own grammar for it
        // admits a number, and Stylo implements that.)
        let admitted = parse(&mut source, longhand, &url_data, &attr.value)
            || (is_bare_number(&attr.value) && {
                source.clear();
                parse(
                    &mut source,
                    longhand,
                    &url_data,
                    &format!("{}px", attr.value.trim()),
                )
            });
        if admitted {
            block.extend(source.drain(), Importance::Normal);
            parsed_any = true;
        } else {
            source.clear();
        }
    }
    parsed_any.then(|| Arc::new(shared_lock.wrap(block)))
}

/// Whether the whole value is one CSS number and nothing else — the shape an
/// SVG presentation attribute may use for a length in user units.
fn is_bare_number(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.parse::<f64>().is_ok_and(f64::is_finite)
}

fn parse_class_list(value: &str) -> Vec<AtomIdent> {
    value
        .split_ascii_whitespace()
        .filter(|token| !token.is_empty())
        .map(AtomIdent::from)
        .collect()
}

fn is_htmlish_namespace(ns: &Namespace) -> bool {
    *ns == markup5ever::ns!(html) || *ns == markup5ever::ns!()
}

fn style_local_name_from(local: &LocalName) -> StyleLocalName {
    StyleLocalName::from(local.as_ref())
}

fn style_namespace_from(ns: &Namespace) -> StyleNamespace {
    StyleNamespace::from(ns.as_ref())
}
