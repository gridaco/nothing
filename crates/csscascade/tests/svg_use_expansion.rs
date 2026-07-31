//! Structure and cascade laws for SVG `<use>` expansion.
//!
//! Expansion is mechanical (csscascade never refuses): the referenced
//! subtree is cloned under the `<use>` before the one cascade runs, so the
//! instance is styled like any other tree member — presentation attributes
//! and `style` clone with it, and inheritance flows from the use site,
//! exactly as the use/defs rung's Chromium probe matrix measured. The
//! compiler-facing refusal conditions travel as [`SvgUseRefusal`] flags.

use csscascade::adapter::{DocumentSession, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::{DemoDom, DemoNodeData};
use csscascade::svg_use::SvgUseRefusal;
use style::dom::TElement;
use style::properties::{ComputedValues, LonghandId};
use style::thread_state::{self, ThreadState};

fn styled(source: &str) -> DocumentSession {
    thread_state::initialize(ThreadState::LAYOUT);
    let dom = DemoDom::parse_xml_from_bytes(source.as_bytes()).expect("parse standalone SVG");
    assert_eq!(dom.errors, Vec::<String>::new(), "fixture is well-formed");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    session
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// The `<use>` element with the given id, panicking if absent.
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

fn property(element: HtmlElement<'_>, longhand: LonghandId) -> String {
    let data = element.borrow_data().expect("computed style");
    let style: &ComputedValues = data.styles.primary();
    let mut output = String::new();
    style
        .computed_or_resolved_value(longhand, None, &mut output)
        .expect("serialize computed property");
    output
}

fn element_children<'s>(element: HtmlElement<'s>) -> Vec<HtmlElement<'s>> {
    let mut out = Vec::new();
    let mut child = element.first_element_child();
    while let Some(next) = child {
        out.push(next);
        child = next.next_element_sibling();
    }
    out
}

fn refusal(element: HtmlElement<'_>) -> Option<SvgUseRefusal> {
    match &element.dom_node().data {
        DemoNodeData::Element(data) => data.svg_use_refusal,
        _ => None,
    }
}

#[test]
fn a_use_clones_its_target_and_the_clone_cascades_at_the_use_site() {
    let session = styled(&document(
        r##"  <defs><rect id="r" width="8" height="8"/></defs>
  <use id="u" href="#r" fill="#d0342c"/>
  <use id="v" href="#r"/>
  <use id="w" href="#r" fill="#2563eb"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");

    // The instance is a real child of the use, styled by the one cascade.
    let clones = element_children(find_by_id(root, "u"));
    assert_eq!(clones.len(), 1, "one instance root");
    assert_eq!(clones[0].local_name_string(), "rect");
    // Inheritance flows from the use site (measured): the target authors
    // no fill, so the use's presentation hint colors the clone.
    assert_eq!(property(clones[0], LonghandId::Fill), "rgb(208, 52, 44)");
    // With nothing at the use site either, the initial black stands.
    let bare = element_children(find_by_id(root, "v"));
    assert_eq!(property(bare[0], LonghandId::Fill), "rgb(0, 0, 0)");
    // Two instances of one original are styled independently.
    let third = element_children(find_by_id(root, "w"));
    assert_eq!(property(third[0], LonghandId::Fill), "rgb(37, 99, 235)");
}

#[test]
fn a_targets_own_attribute_beats_the_use_site() {
    let session = styled(&document(
        r##"  <defs><rect id="r" width="8" height="8" fill="#d0342c"/></defs>
  <use id="u" href="#r" fill="#2563eb"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    let clones = element_children(find_by_id(root, "u"));
    // The clone carries the target's presentation attributes (SVG2
    // §5.6.3), and an element's own hint beats inherited values.
    assert_eq!(property(clones[0], LonghandId::Fill), "rgb(208, 52, 44)");
}

#[test]
fn current_color_resolves_against_the_use_site() {
    let session = styled(&document(
        r##"  <defs><rect id="r" width="8" height="8" fill="currentColor"/></defs>
  <use id="u" href="#r" color="#d0342c"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    let clones = element_children(find_by_id(root, "u"));
    assert_eq!(property(clones[0], LonghandId::Color), "rgb(208, 52, 44)");
}

#[test]
fn forward_references_resolve_and_the_first_id_wins() {
    let session = styled(&document(
        r##"  <use id="u" href="#fwd"/>
  <defs>
    <rect id="fwd" width="8" height="8"/>
    <circle id="fwd" r="4"/>
  </defs>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    let clones = element_children(find_by_id(root, "u"));
    assert_eq!(clones.len(), 1);
    assert_eq!(
        clones[0].local_name_string(),
        "rect",
        "the first element in tree order with the id is the target"
    );
}

#[test]
fn unresolved_references_expand_to_nothing_without_a_refusal() {
    let session = styled(&document(
        r##"  <use id="missing" href="#nope"/>
  <use id="bare"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    for id in ["missing", "bare"] {
        let use_el = find_by_id(root, id);
        assert!(element_children(use_el).is_empty(), "#{id} stays childless");
        assert_eq!(
            refusal(use_el),
            None,
            "#{id}: nothing to refuse — it renders nothing"
        );
    }
}

#[test]
fn chained_uses_expand_through_and_cycles_terminate_as_skeletons() {
    let session = styled(&document(
        r##"  <defs>
    <rect id="leaf" width="8" height="8"/>
    <use id="mid" href="#leaf"/>
    <use id="a" href="#b"/>
    <use id="b" href="#a"/>
  </defs>
  <use id="chain" href="#mid"/>
  <use id="cycle" href="#a"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");

    // The chain reaches the leaf: use -> clone(use#mid) -> clone(rect).
    let chain = element_children(find_by_id(root, "chain"));
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].local_name_string(), "use");
    let inner = element_children(chain[0]);
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0].local_name_string(), "rect");

    // The mutual cycle expands to use skeletons only — nothing paintable,
    // no refusal: Chromium renders nothing for the pair, silently.
    let mut stack = element_children(find_by_id(root, "cycle"));
    assert!(!stack.is_empty(), "the first hop clones");
    while let Some(element) = stack.pop() {
        assert_eq!(
            element.local_name_string(),
            "use",
            "a cycle yields only use skeletons"
        );
        assert_eq!(refusal(element), None);
        stack.extend(element_children(element));
    }
}

#[test]
fn an_ancestor_reference_is_an_invalid_circle_and_expands_to_nothing() {
    let session = styled(&document(
        r##"  <g id="cy"><rect width="8" height="8"/><use id="u" href="#cy"/></g>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    let use_el = find_by_id(root, "u");
    assert!(element_children(use_el).is_empty());
    assert_eq!(
        refusal(use_el),
        None,
        "renders nothing, exactly as Chromium"
    );
}

#[test]
fn refusal_flags_name_the_conditions_the_compiler_must_refuse() {
    let session = styled(&document(
        r##"  <defs><rect id="r" width="8" height="8"/></defs>
  <use id="external" href="other.svg#r"/>
  <use id="authored" href="#r"><rect width="4" height="4"/></use>
  <use id="described" href="#r"><title>fine</title></use>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    assert_eq!(
        refusal(find_by_id(root, "external")),
        Some(SvgUseRefusal::ExternalReference)
    );
    assert_eq!(
        refusal(find_by_id(root, "authored")),
        Some(SvgUseRefusal::AuthoredChildren)
    );
    // Descriptive children are content-model-valid and never paint; the
    // use still expands (the clone lands after the title).
    let described = find_by_id(root, "described");
    assert_eq!(refusal(described), None);
    let children = element_children(described);
    assert_eq!(children.len(), 2);
    assert_eq!(children[1].local_name_string(), "rect");
}

#[test]
fn style_and_script_are_never_cloned() {
    let session = styled(&document(
        r##"  <defs><g id="g"><style>/* sheet */</style><rect width="8" height="8"/></g></defs>
  <use id="u" href="#g"/>"##,
    ));
    let root = session.document().root_element().expect("svg root");
    let clones = element_children(find_by_id(root, "u"));
    assert_eq!(clones.len(), 1);
    let inner = element_children(clones[0]);
    assert_eq!(inner.len(), 1, "the style did not clone");
    assert_eq!(inner[0].local_name_string(), "rect");
}

#[test]
fn xlink_href_resolves_and_the_plain_spelling_beats_it() {
    let session = styled(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64">
  <defs><rect id="r" width="8" height="8"/><circle id="c" r="4"/></defs>
  <use id="legacy" xlink:href="#r"/>
  <use id="both" href="#r" xlink:href="#c"/>
</svg>"##
    ));
    let root = session.document().root_element().expect("svg root");
    let legacy = element_children(find_by_id(root, "legacy"));
    assert_eq!(legacy[0].local_name_string(), "rect");
    let both = element_children(find_by_id(root, "both"));
    assert_eq!(
        both[0].local_name_string(),
        "rect",
        "the plain href wins; the deprecated spelling is ignored (measured)"
    );
}
