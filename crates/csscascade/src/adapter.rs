//! Stylo DOM adapter layer.
//!
//! Implements the [`TNode`], [`TElement`], [`TDocument`], and [`selectors::Element`]
//! traits for our arena DOM so that Stylo's cascade engine can match selectors
//! and resolve styles against it.
//!
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;
use std::sync::OnceLock;

use euclid::default::Size2D;
use markup5ever::{Attribute, Namespace as HtmlNamespace, ns};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::bloom::BloomFilter;
use selectors::matching::{ElementSelectorFlags, MatchingContext, VisitedHandlingMode};
use selectors::parser::SelectorImpl as SelectorsParser;
use selectors::{OpaqueElement, sink::Push};
use style::Namespace as StyleNamespace;
use style::applicable_declarations::ApplicableDeclarationBlock;
use style::context::SharedStyleContext;
use style::data::{ElementDataMut, ElementDataRef, ElementDataWrapper};
use style::dom::{LayoutIterator, OpaqueNode, TElement, TNode};
use style::properties::PropertyDeclarationBlock;
use style::selector_parser::{AttrValue as SelectorAttrValue, Lang, PseudoElement, SelectorImpl};
use style::servo_arc::{Arc, ArcBorrow};
use style::shared_lock::{Locked, SharedRwLock};
use style::stylist::CascadeData;
use style::values::AtomIdent;
use style::values::computed::Au;
use style::values::computed::Display;
use stylo_dom::ElementState;

use crate::dom::{DemoDom, DemoElementData, DemoNode, DemoNodeData, NodeId};

type Impl = SelectorImpl;

// ---------------------------------------------------------------------------
// Owned document session
// ---------------------------------------------------------------------------

/// Owns one frozen DOM and the Stylo data tied to that DOM.
///
/// Handles borrowed from a session cannot outlive it:
///
/// ```compile_fail
/// use csscascade::{adapter::DocumentSession, dom::DemoDom};
///
/// let document = {
///     let dom = DemoDom::parse_from_bytes(b"<html></html>").unwrap();
///     let session = DocumentSession::new(dom);
///     session.document()
/// };
/// let _ = document.root_element();
/// ```
pub struct DocumentSession {
    inner: Box<SessionInner>,
}

#[derive(Debug)]
struct SessionInner {
    dom: DemoDom,
    handles: Box<[SessionNode]>,
}

#[derive(Clone, Copy, Debug)]
struct SessionNode {
    owner: NonNull<SessionInner>,
    id: NodeId,
}

impl fmt::Debug for DocumentSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DocumentSession")
            .field("dom", &self.inner.dom)
            .finish_non_exhaustive()
    }
}

impl DocumentSession {
    pub fn new(dom: DemoDom) -> Self {
        let node_count = dom.node_count();
        let mut inner = Box::new(SessionInner {
            dom,
            handles: Box::new([]),
        });
        let owner = NonNull::from(inner.as_mut());
        inner.handles = (0..node_count)
            .map(|index| SessionNode {
                owner,
                id: NodeId(index),
            })
            .collect();
        Self { inner }
    }

    pub fn document(&self) -> HtmlDocument<'_> {
        HtmlDocument(
            self.handle(self.inner.dom.document_id())
                .expect("document identifier must belong to its session"),
        )
    }

    /// Read-only access to this session's frozen DOM.
    pub fn dom(&self) -> &DemoDom {
        &self.inner.dom
    }

    /// Resolve an arena-local node identifier inside this session.
    pub fn node(&self, id: NodeId) -> Option<&DemoNode> {
        self.inner.dom.get_node(id)
    }

    /// Resolve an arena-local element identifier into a session-bound handle.
    pub fn element(&self, id: NodeId) -> Option<HtmlElement<'_>> {
        matches!(
            self.inner.dom.get_node(id).map(|node| &node.data),
            Some(DemoNodeData::Element(_))
        )
        .then(|| {
            HtmlElement(
                self.handle(id)
                    .expect("element identifier must be in bounds"),
            )
        })
    }

    fn handle(&self, id: NodeId) -> Option<SessionHandle<'_>> {
        self.inner.handle(id)
    }
}

impl SessionInner {
    fn handle(&self, id: NodeId) -> Option<SessionHandle<'_>> {
        self.handles.get(id.idx()).map(SessionHandle::new)
    }
}

// ---------------------------------------------------------------------------
// Wrapper types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct SessionHandle<'session> {
    node: &'session SessionNode,
}

impl<'session> SessionHandle<'session> {
    fn new(node: &'session SessionNode) -> Self {
        Self { node }
    }

    fn with_id(self, id: NodeId) -> Self {
        self.inner()
            .handle(id)
            .expect("related node identifier must belong to the same session")
    }

    fn id(self) -> NodeId {
        self.record().id
    }

    fn inner(self) -> &'session SessionInner {
        // SAFETY: DocumentSession allocates SessionInner before initializing
        // the stable boxed record slice. Every record's owner points to that
        // containing allocation, which is private and never moved or replaced.
        // `node` carries the borrow lifetime of the owning DocumentSession, so
        // the owner cannot be recovered after the session drops.
        unsafe { self.node.owner.as_ref() }
    }

    fn record(self) -> &'session SessionNode {
        self.node
    }

    fn dom(self) -> &'session DemoDom {
        &self.inner().dom
    }

    fn dom_node(self) -> &'session DemoNode {
        self.dom().node(self.id())
    }

    fn element(self, id: NodeId) -> Option<HtmlElement<'session>> {
        matches!(
            self.inner().dom.get_node(id).map(|node| &node.data),
            Some(DemoNodeData::Element(_))
        )
        .then(|| HtmlElement(self.with_id(id)))
    }
}

impl PartialEq for SessionHandle<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.node, other.node)
    }
}

impl Eq for SessionHandle<'_> {}

impl Hash for SessionHandle<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.node).hash(state);
    }
}

impl fmt::Debug for SessionHandle<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionHandle")
            .field("session", &self.record().owner)
            .field("id", &self.record().id)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HtmlNode<'session>(SessionHandle<'session>);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HtmlElement<'session>(SessionHandle<'session>);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HtmlDocument<'session>(SessionHandle<'session>);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HtmlShadowRoot<'session> {
    host: HtmlElement<'session>,
}

// ---------------------------------------------------------------------------
// HtmlDocument
// ---------------------------------------------------------------------------

impl<'session> HtmlDocument<'session> {
    pub fn dom(self) -> &'session DemoDom {
        self.0.dom()
    }

    pub fn node(self, id: NodeId) -> Option<&'session DemoNode> {
        self.0.dom().get_node(id)
    }

    pub fn element(self, id: NodeId) -> Option<HtmlElement<'session>> {
        self.0.element(id)
    }

    pub fn root_element(&self) -> Option<HtmlElement<'session>> {
        self.0
            .dom()
            .document_children()
            .iter()
            .find_map(|child| self.0.element(*child))
    }

    pub fn element_count(&self) -> usize {
        let mut count = 0;
        let mut stack = Vec::new();
        if let Some(root) = self.root_element() {
            stack.push(root);
        }
        while let Some(element) = stack.pop() {
            count += 1;
            let mut child = element.first_element_child();
            while let Some(next_child) = child {
                stack.push(next_child);
                child = next_child.next_element_sibling();
            }
        }
        count
    }
}

// ---------------------------------------------------------------------------
// HtmlElement helpers
// ---------------------------------------------------------------------------

impl<'session> HtmlElement<'session> {
    /// Returns the underlying DOM [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.0.id()
    }

    /// Returns this element's owning frozen DOM.
    pub fn dom(self) -> &'session DemoDom {
        self.0.dom()
    }

    /// Returns this element's arena node from its owning session.
    pub fn dom_node(self) -> &'session DemoNode {
        self.0.dom_node()
    }

    /// Resolve an arena-local element identifier in this handle's session.
    pub fn element(self, id: NodeId) -> Option<HtmlElement<'session>> {
        self.0.element(id)
    }

    pub fn local_name_string(&self) -> String {
        self.element_data().name.local.to_string()
    }

    pub fn first_element_child(self) -> Option<HtmlElement<'session>> {
        self.node().first_element_child()
    }

    pub fn next_element_sibling(self) -> Option<HtmlElement<'session>> {
        self.node().next_element_sibling()
    }

    fn element_data(&self) -> &DemoElementData {
        match &self.dom_node().data {
            DemoNodeData::Element(data) => data,
            _ => panic!("HtmlElement must wrap an element node"),
        }
    }

    fn node(self) -> HtmlNode<'session> {
        HtmlNode(self.0)
    }

    fn data_slot(&self) -> &OnceLock<ElementDataWrapper> {
        self.0.dom().element_data_slot(self.0.id())
    }

    fn attr_iter(&self) -> impl Iterator<Item = (&Attribute, &style::LocalName)> + '_ {
        let data = self.element_data();
        data.attrs.iter().zip(data.attr_local_names.iter())
    }

    fn attr_matches_impl(
        &self,
        ns: &NamespaceConstraint<&StyleNamespace>,
        local_name: &style::LocalName,
        operation: &AttrSelectorOperation<&SelectorAttrValue>,
    ) -> bool {
        self.attr_iter()
            .filter(|(attr, _)| namespace_matches(ns, &attr.name.ns))
            .find(|(_, stored)| *stored == local_name)
            .is_some_and(|(attr, _)| operation.eval_str(attr.value.as_ref()))
    }

    fn lang_attribute_value(&self) -> Option<&str> {
        self.element_data().attrs.iter().find_map(|attr| {
            if !attr.name.local.as_ref().eq_ignore_ascii_case("lang") {
                return None;
            }
            let ns = &attr.name.ns;
            if *ns == markup5ever::ns!() || *ns == markup5ever::ns!(xml) {
                Some(attr.value.as_ref())
            } else {
                None
            }
        })
    }

    fn has_class_token(&self, name: &AtomIdent, case_sensitivity: CaseSensitivity) -> bool {
        let needle = atom_ident_str(name);
        self.element_data()
            .class_list
            .iter()
            .any(|class| case_sensitivity.eq(atom_ident_str(class).as_bytes(), needle.as_bytes()))
    }

    fn id_string(&self) -> Option<&str> {
        self.element_data()
            .id_attr
            .as_ref()
            .map(|atom| atom.as_ref())
    }
}

// ---------------------------------------------------------------------------
// HtmlNode helpers
// ---------------------------------------------------------------------------

impl<'session> HtmlNode<'session> {
    fn node(self) -> &'session DemoNode {
        self.0.dom_node()
    }

    fn parent(self) -> Option<HtmlNode<'session>> {
        self.node().parent.map(|id| HtmlNode(self.0.with_id(id)))
    }

    fn to_element(self) -> Option<HtmlElement<'session>> {
        matches!(self.node().data, DemoNodeData::Element(_)).then_some(HtmlElement(self.0))
    }

    fn first_element_child(self) -> Option<HtmlElement<'session>> {
        let mut child = self.first_child();
        while let Some(node) = child {
            if let Some(element) = node.to_element() {
                return Some(element);
            }
            child = node.next_sibling();
        }
        None
    }

    fn prev_element_sibling(self) -> Option<HtmlElement<'session>> {
        let mut prev = self.prev_sibling();
        while let Some(node) = prev {
            if let Some(element) = node.to_element() {
                return Some(element);
            }
            prev = node.prev_sibling();
        }
        None
    }

    fn next_element_sibling(self) -> Option<HtmlElement<'session>> {
        let mut next = self.next_sibling();
        while let Some(node) = next {
            if let Some(element) = node.to_element() {
                return Some(element);
            }
            next = node.next_sibling();
        }
        None
    }

    fn first_child(self) -> Option<HtmlNode<'session>> {
        self.node()
            .children
            .first()
            .copied()
            .map(|id| HtmlNode(self.0.with_id(id)))
    }

    fn prev_sibling(self) -> Option<HtmlNode<'session>> {
        sibling_pair(self.0).0
    }

    fn next_sibling(self) -> Option<HtmlNode<'session>> {
        sibling_pair(self.0).1
    }
}

// ---------------------------------------------------------------------------
// style::dom::NodeInfo
// ---------------------------------------------------------------------------

impl<'session> ::style::dom::NodeInfo for HtmlNode<'session> {
    fn is_element(&self) -> bool {
        matches!(self.node().data, DemoNodeData::Element(_))
    }

    fn is_text_node(&self) -> bool {
        matches!(self.node().data, DemoNodeData::Text(_))
    }
}

// ---------------------------------------------------------------------------
// style::dom::TNode
// ---------------------------------------------------------------------------

impl<'session> ::style::dom::TNode for HtmlNode<'session> {
    type ConcreteElement = HtmlElement<'session>;
    type ConcreteDocument = HtmlDocument<'session>;
    type ConcreteShadowRoot = HtmlShadowRoot<'session>;

    fn parent_node(&self) -> Option<Self> {
        self.parent()
    }

    fn first_child(&self) -> Option<Self> {
        self.node()
            .children
            .first()
            .copied()
            .map(|id| HtmlNode(self.0.with_id(id)))
    }

    fn last_child(&self) -> Option<Self> {
        self.node()
            .children
            .last()
            .copied()
            .map(|id| HtmlNode(self.0.with_id(id)))
    }

    fn prev_sibling(&self) -> Option<Self> {
        sibling_pair(self.0).0
    }

    fn next_sibling(&self) -> Option<Self> {
        sibling_pair(self.0).1
    }

    fn owner_doc(&self) -> Self::ConcreteDocument {
        HtmlDocument(self.0.with_id(self.0.dom().document_id()))
    }

    fn is_in_document(&self) -> bool {
        true
    }

    fn traversal_parent(&self) -> Option<Self::ConcreteElement> {
        self.parent()?.to_element()
    }

    fn opaque(&self) -> OpaqueNode {
        OpaqueNode(std::ptr::from_ref(self.node()) as usize)
    }

    fn debug_id(self) -> usize {
        self.0.id().idx()
    }

    fn as_element(&self) -> Option<Self::ConcreteElement> {
        self.to_element()
    }

    fn as_document(&self) -> Option<Self::ConcreteDocument> {
        matches!(self.node().data, DemoNodeData::Document).then_some(HtmlDocument(self.0))
    }

    fn as_shadow_root(&self) -> Option<Self::ConcreteShadowRoot> {
        None
    }
}

// ---------------------------------------------------------------------------
// style::dom::TDocument
// ---------------------------------------------------------------------------

impl<'session> ::style::dom::TDocument for HtmlDocument<'session> {
    type ConcreteNode = HtmlNode<'session>;

    fn as_node(&self) -> Self::ConcreteNode {
        HtmlNode(self.0)
    }

    fn is_html_document(&self) -> bool {
        true
    }

    fn quirks_mode(&self) -> style::context::QuirksMode {
        style::context::QuirksMode::NoQuirks
    }

    fn shared_lock(&self) -> &SharedRwLock {
        self.0.inner().dom.shared_lock()
    }
}

// ---------------------------------------------------------------------------
// style::dom::TShadowRoot
// ---------------------------------------------------------------------------

impl<'session> ::style::dom::TShadowRoot for HtmlShadowRoot<'session> {
    type ConcreteNode = HtmlNode<'session>;

    fn as_node(&self) -> Self::ConcreteNode {
        self.host.as_node()
    }

    fn host(&self) -> <Self::ConcreteNode as TNode>::ConcreteElement {
        self.host
    }

    fn style_data<'a>(&self) -> Option<&'a CascadeData>
    where
        Self: 'a,
    {
        None
    }
}

// ---------------------------------------------------------------------------
// style::dom::TElement
// ---------------------------------------------------------------------------

impl<'session> ::style::dom::TElement for HtmlElement<'session> {
    type ConcreteNode = HtmlNode<'session>;
    type TraversalChildrenIterator = std::vec::IntoIter<Self::ConcreteNode>;

    fn as_node(&self) -> Self::ConcreteNode {
        HtmlNode(self.0)
    }

    fn traversal_children(&self) -> LayoutIterator<Self::TraversalChildrenIterator> {
        let nodes: Vec<_> = self
            .node()
            .node()
            .children
            .iter()
            .map(|child| HtmlNode(self.0.with_id(*child)))
            .collect();
        LayoutIterator(nodes.into_iter())
    }

    fn is_html_element(&self) -> bool {
        self.element_data().name.ns == ns!(html)
    }

    fn is_mathml_element(&self) -> bool {
        self.element_data().name.ns == ns!(mathml)
    }

    fn is_svg_element(&self) -> bool {
        self.element_data().name.ns == ns!(svg)
    }

    fn style_attribute(&self) -> Option<ArcBorrow<'_, Locked<PropertyDeclarationBlock>>> {
        self.element_data()
            .style_attribute
            .as_ref()
            .map(|arc| arc.borrow_arc())
    }

    fn animation_rule(
        &self,
        _context: &SharedStyleContext,
    ) -> Option<Arc<Locked<PropertyDeclarationBlock>>> {
        None
    }

    fn transition_rule(
        &self,
        _context: &SharedStyleContext,
    ) -> Option<Arc<Locked<PropertyDeclarationBlock>>> {
        None
    }

    fn state(&self) -> ElementState {
        ElementState::empty()
    }

    fn has_part_attr(&self) -> bool {
        false
    }

    fn exports_any_part(&self) -> bool {
        false
    }

    fn id(&self) -> Option<&stylo_atoms::Atom> {
        self.element_data().id_attr.as_ref()
    }

    fn each_class<F>(&self, mut callback: F)
    where
        F: FnMut(&AtomIdent),
    {
        for class_atom in &self.element_data().class_list {
            callback(class_atom);
        }
    }

    fn each_custom_state<F>(&self, _callback: F)
    where
        F: FnMut(&AtomIdent),
    {
    }

    fn each_attr_name<F>(&self, mut callback: F)
    where
        F: FnMut(&style::LocalName),
    {
        for attr_name in &self.element_data().attr_local_names {
            callback(attr_name);
        }
    }

    fn has_dirty_descendants(&self) -> bool {
        false
    }

    fn has_snapshot(&self) -> bool {
        false
    }

    fn handled_snapshot(&self) -> bool {
        false
    }

    unsafe fn set_handled_snapshot(&self) {}
    unsafe fn set_dirty_descendants(&self) {}
    unsafe fn unset_dirty_descendants(&self) {}

    fn store_children_to_process(&self, _n: isize) {}

    fn did_process_child(&self) -> isize {
        0
    }

    unsafe fn ensure_data(&self) -> ElementDataMut<'_> {
        let slot = self.data_slot();
        slot.get_or_init(ElementDataWrapper::default).borrow_mut()
    }

    unsafe fn clear_data(&self) {
        // OnceLock-backed storage: we cannot reset the slot safely, and
        // callers in the cascade driver never rely on clearing data between
        // passes. Leaving the entry in place matches Stylo's Gecko backend,
        // which also reuses the allocation.
    }

    fn has_data(&self) -> bool {
        self.data_slot().get().is_some()
    }

    fn borrow_data(&self) -> Option<ElementDataRef<'_>> {
        self.data_slot().get().map(|w| w.borrow())
    }

    fn mutate_data(&self) -> Option<ElementDataMut<'_>> {
        self.data_slot().get().map(|w| w.borrow_mut())
    }

    fn skip_item_display_fixup(&self) -> bool {
        false
    }

    fn may_have_animations(&self) -> bool {
        false
    }

    fn has_animations(&self, _context: &SharedStyleContext) -> bool {
        false
    }

    fn has_css_animations(
        &self,
        _context: &SharedStyleContext,
        _pseudo_element: Option<PseudoElement>,
    ) -> bool {
        false
    }

    fn has_css_transitions(
        &self,
        _context: &SharedStyleContext,
        _pseudo_element: Option<PseudoElement>,
    ) -> bool {
        false
    }

    fn shadow_root(&self) -> Option<<Self::ConcreteNode as TNode>::ConcreteShadowRoot> {
        None
    }

    fn containing_shadow(&self) -> Option<<Self::ConcreteNode as TNode>::ConcreteShadowRoot> {
        None
    }

    fn lang_attr(&self) -> Option<SelectorAttrValue> {
        self.lang_attribute_value().map(SelectorAttrValue::from)
    }

    fn match_element_lang(
        &self,
        _override_lang: Option<Option<SelectorAttrValue>>,
        _value: &Lang,
    ) -> bool {
        false
    }

    fn is_html_document_body_element(&self) -> bool {
        false
    }

    fn synthesize_presentational_hints_for_legacy_attributes<V>(
        &self,
        _visited_handling: VisitedHandlingMode,
        _hints: &mut V,
    ) where
        V: Push<ApplicableDeclarationBlock>,
    {
    }

    fn synthesize_view_transition_dynamic_rules<V>(&self, _rules: &mut V)
    where
        V: Push<ApplicableDeclarationBlock>,
    {
    }

    fn local_name(&self) -> &<Impl as SelectorsParser>::BorrowedLocalName {
        self.element_data().style_local_name.borrow()
    }

    fn namespace(&self) -> &<Impl as SelectorsParser>::BorrowedNamespaceUrl {
        self.element_data().style_namespace.borrow()
    }

    fn query_container_size(&self, _display: &Display) -> Size2D<Option<Au>> {
        Size2D::new(None, None)
    }

    fn has_selector_flags(&self, _flags: ElementSelectorFlags) -> bool {
        false
    }

    fn relative_selector_search_direction(&self) -> ElementSelectorFlags {
        ElementSelectorFlags::empty()
    }

    fn get_attr(&self, attr: &style::LocalName, namespace: &StyleNamespace) -> Option<String> {
        self.attr_iter()
            .filter(|(a, _)| {
                let dom_ns: &str = &a.name.ns;
                let sel_ns: &str = namespace.as_ref();
                dom_ns == sel_ns
            })
            .find(|(_, stored)| *stored == attr)
            .map(|(a, _)| a.value.to_string())
    }
}

// ---------------------------------------------------------------------------
// selectors::Element
// ---------------------------------------------------------------------------

impl<'session> ::selectors::Element for HtmlElement<'session> {
    type Impl = Impl;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.dom_node())
    }

    fn parent_element(&self) -> Option<Self> {
        self.as_node().parent_node()?.to_element()
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn pseudo_element_originating_element(&self) -> Option<Self> {
        None
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.as_node().prev_element_sibling()
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.as_node().next_element_sibling()
    }

    fn first_element_child(&self) -> Option<Self> {
        self.as_node().first_element_child()
    }

    fn has_local_name(&self, name: &<Impl as SelectorsParser>::BorrowedLocalName) -> bool {
        self.element_data().name.local.as_ref() == name.as_ref()
    }

    fn has_namespace(&self, ns: &<Impl as SelectorsParser>::BorrowedNamespaceUrl) -> bool {
        self.element_data().name.ns.as_ref() == ns.as_ref()
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.element_data().name == other.element_data().name
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&<Impl as SelectorsParser>::NamespaceUrl>,
        local_name: &<Impl as SelectorsParser>::LocalName,
        operation: &AttrSelectorOperation<&<Impl as SelectorsParser>::AttrValue>,
    ) -> bool {
        self.attr_matches_impl(ns, local_name, operation)
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &<Impl as SelectorsParser>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Impl as SelectorsParser>::PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn is_link(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Impl as SelectorsParser>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        let Some(current) = self.id_string() else {
            return false;
        };
        case_sensitivity.eq(current.as_bytes(), atom_ident_str(id).as_bytes())
    }

    fn is_part(&self, _name: &AtomIdent) -> bool {
        false
    }

    fn imported_part(
        &self,
        _name: &<Impl as SelectorsParser>::Identifier,
    ) -> Option<<Impl as SelectorsParser>::Identifier> {
        None
    }

    fn has_class(
        &self,
        name: &<Impl as SelectorsParser>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.has_class_token(name, case_sensitivity)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.is_html_element()
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.as_node().first_child().is_none()
    }

    fn is_root(&self) -> bool {
        self.as_node().parent_node().is_none()
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }

    fn has_custom_state(&self, _name: &<Impl as SelectorsParser>::Identifier) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn namespace_matches(
    constraint: &NamespaceConstraint<&StyleNamespace>,
    attr_ns: &HtmlNamespace,
) -> bool {
    match constraint {
        NamespaceConstraint::Any => true,
        NamespaceConstraint::Specific(ns) => {
            let selector_ns_atom = ns.as_ref();
            let selector_ns: &str = selector_ns_atom;
            let dom_ns: &str = attr_ns;
            selector_ns == dom_ns
        }
    }
}

fn atom_ident_str(atom: &AtomIdent) -> &str {
    atom.as_ref()
}

fn sibling_pair<'session>(
    handle: SessionHandle<'session>,
) -> (Option<HtmlNode<'session>>, Option<HtmlNode<'session>>) {
    let node = handle.dom_node();
    let Some(parent) = node.parent else {
        return (None, None);
    };

    let siblings = &handle
        .dom()
        .get_node(parent)
        .expect("parent identifier must resolve in the same session")
        .children;
    let idx = siblings
        .iter()
        .position(|child| *child == handle.id())
        .expect("parent missing child");

    let prev = idx
        .checked_sub(1)
        .map(|i| HtmlNode(handle.with_id(siblings[i])));
    let next = siblings
        .get(idx + 1)
        .copied()
        .map(|id| HtmlNode(handle.with_id(id)));

    (prev, next)
}
