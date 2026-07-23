use csscascade::adapter::{DocumentSession, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::{DemoDom, DemoNodeData};
use std::collections::HashSet;
use std::mem;
use style::dom::{TElement, TNode};
use style::properties::LonghandId;
use style::thread_state::{self, ThreadState};

const FIRST_DOCUMENT: &str = r#"<!doctype html>
<html>
  <head><style>#target { color: #112233; }</style></head>
  <body><div id="target" data-owner="first" style="font-size: 11px">first text</div></body>
</html>"#;

const SECOND_DOCUMENT: &str = r#"<!doctype html>
<html>
  <head><style>#target { color: #aabbcc; }</style></head>
  <body><div id="target" data-owner="second" style="font-size: 13px">second text</div></body>
</html>"#;

#[test]
fn live_sessions_keep_their_own_tree_attributes_and_computed_styles() {
    thread_state::initialize(ThreadState::LAYOUT);

    let first = styled_session(FIRST_DOCUMENT);
    let second = styled_session(SECOND_DOCUMENT);
    let first_target = find_by_id(first.document().root_element().unwrap(), "target");
    let second_target = find_by_id(second.document().root_element().unwrap(), "target");

    // Identical source shape deliberately gives these nodes the same
    // arena-local identifier. The session carried by each Copy handle is the
    // document identity that prevents cross-resolution.
    assert_eq!(first_target.node_id(), second_target.node_id());
    assert_ne!(first_target, second_target);
    assert_eq!(HashSet::from([first_target, second_target]).len(), 2);
    assert_eq!(
        first_target.element(first_target.node_id()),
        Some(first_target)
    );
    assert_eq!(
        second_target.element(second_target.node_id()),
        Some(second_target)
    );
    assert_ne!(
        first_target.as_node().opaque(),
        second_target.as_node().opaque()
    );
    assert_ne!(
        selectors::Element::opaque(&first_target),
        selectors::Element::opaque(&second_target)
    );

    for _ in 0..3 {
        assert_eq!(attribute(first_target, "data-owner"), "first");
        assert_eq!(text(first_target), "first text");
        assert_eq!(color(first_target), "rgb(17, 34, 51)");
        assert_eq!(property(first_target, LonghandId::FontSize), "11px");

        assert_eq!(attribute(second_target, "data-owner"), "second");
        assert_eq!(text(second_target), "second text");
        assert_eq!(color(second_target), "rgb(170, 187, 204)");
        assert_eq!(property(second_target, LonghandId::FontSize), "13px");
    }
}

#[test]
fn dropping_one_session_does_not_invalidate_another() {
    thread_state::initialize(ThreadState::LAYOUT);

    let survivor = styled_session(SECOND_DOCUMENT);
    let surviving_target = find_by_id(survivor.document().root_element().unwrap(), "target");

    {
        let temporary = styled_session(FIRST_DOCUMENT);
        let temporary_target = find_by_id(temporary.document().root_element().unwrap(), "target");
        assert_eq!(attribute(temporary_target, "data-owner"), "first");
        assert_eq!(color(temporary_target), "rgb(17, 34, 51)");
    }

    assert_eq!(attribute(surviving_target, "data-owner"), "second");
    assert_eq!(text(surviving_target), "second text");
    assert_eq!(color(surviving_target), "rgb(170, 187, 204)");
}

#[test]
fn stylo_handles_remain_copy_values_borrowed_from_the_session() {
    fn assert_copy<T: Copy>() {}

    assert_copy::<csscascade::adapter::HtmlDocument<'_>>();
    assert_copy::<csscascade::adapter::HtmlNode<'_>>();
    assert_copy::<csscascade::adapter::HtmlElement<'_>>();

    assert_eq!(
        mem::size_of::<csscascade::adapter::HtmlDocument<'_>>(),
        mem::size_of::<usize>()
    );
    assert_eq!(
        mem::size_of::<csscascade::adapter::HtmlNode<'_>>(),
        mem::size_of::<usize>()
    );
    assert_eq!(
        mem::size_of::<csscascade::adapter::HtmlElement<'_>>(),
        mem::size_of::<usize>()
    );
}

fn styled_session(source: &str) -> DocumentSession {
    let dom = DemoDom::parse_from_bytes(source.as_bytes()).expect("parse document");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    session
}

fn find_by_id<'session>(root: HtmlElement<'session>, wanted: &str) -> HtmlElement<'session> {
    let mut stack = vec![root];
    while let Some(element) = stack.pop() {
        if element.id().is_some_and(|id| id.as_ref() == wanted) {
            return element;
        }
        let mut child = element.first_element_child();
        while let Some(next) = child {
            stack.push(next);
            child = next.next_element_sibling();
        }
    }
    panic!("missing element #{wanted}")
}

fn attribute<'session>(element: HtmlElement<'session>, wanted: &str) -> &'session str {
    let DemoNodeData::Element(data) = &element.dom_node().data else {
        unreachable!("HtmlElement always resolves to an element node")
    };
    data.attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == wanted)
        .map(|attr| attr.value.as_ref())
        .unwrap_or_else(|| panic!("missing attribute {wanted}"))
}

fn text<'session>(element: HtmlElement<'session>) -> &'session str {
    let dom = element.dom();
    element
        .dom_node()
        .children
        .iter()
        .find_map(|child| match &dom.get_node(*child)?.data {
            DemoNodeData::Text(text) => Some(text.as_ref()),
            _ => None,
        })
        .expect("element text")
}

fn color(element: HtmlElement<'_>) -> String {
    property(element, LonghandId::Color)
}

fn property(element: HtmlElement<'_>, property: LonghandId) -> String {
    let style = element
        .borrow_data()
        .expect("computed style")
        .styles
        .primary()
        .clone();
    let mut output = String::new();
    style
        .computed_or_resolved_value(property, None, &mut output)
        .expect("serialize property");
    output
}
