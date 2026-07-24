//! Shape laws for the conforming standalone SVG/XML grammar entry.
//!
//! `DemoDom::parse_xml_from_bytes` must produce the same semantic document
//! shape the HTML entry produces — one arena DOM, one adapter, one cascade —
//! while honoring XML grammar: authored namespaces, preserved case, tolerated
//! prolog, and recorded (never silently swallowed) recovery errors.

use csscascade::dom::{DemoDom, DemoNodeData};

const SVG_NS: &str = "http://www.w3.org/2000/svg";

fn first_element(dom: &DemoDom) -> (&DemoNodeData, Vec<csscascade::dom::NodeId>) {
    let root = dom
        .document_children()
        .iter()
        .copied()
        .find(|id| matches!(dom.node(*id).data, DemoNodeData::Element(_)))
        .expect("document has a root element");
    (&dom.node(root).data, dom.node(root).children.clone())
}

#[test]
fn namespaced_svg_parses_into_the_shared_document_shape_without_errors() {
    let dom = DemoDom::parse_xml_from_bytes(
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32" viewBox="0 0 64 32">
  <rect x="4" y="8" width="8" height="16" fill="#000000"/>
</svg>
"##,
    )
    .expect("read XML bytes");

    assert_eq!(
        dom.errors,
        Vec::<String>::new(),
        "well-formed XML recovers nothing"
    );
    let (root, children) = first_element(&dom);
    let DemoNodeData::Element(svg) = root else {
        panic!("root is an element");
    };
    assert_eq!(svg.name.local.as_ref(), "svg");
    assert_eq!(
        svg.name.ns.as_ref(),
        SVG_NS,
        "namespace comes from authored xmlns"
    );

    let rect = children
        .iter()
        .find_map(|id| match &dom.node(*id).data {
            DemoNodeData::Element(element) => Some(element),
            _ => None,
        })
        .expect("svg has a rect child");
    assert_eq!(rect.name.local.as_ref(), "rect");
    assert_eq!(
        rect.name.ns.as_ref(),
        SVG_NS,
        "children inherit the default namespace"
    );

    // Attribute case is preserved exactly — XML is case-sensitive.
    assert!(
        svg.attrs
            .iter()
            .any(|attribute| attribute.name.local.as_ref() == "viewBox"),
        "authored viewBox keeps its case"
    );
}

#[test]
fn xml_prolog_and_trailing_whitespace_are_tolerated() {
    let dom = DemoDom::parse_xml_from_bytes(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"/>
"#,
    )
    .expect("read XML bytes");
    assert_eq!(dom.errors, Vec::<String>::new());
    let (root, _) = first_element(&dom);
    let DemoNodeData::Element(svg) = root else {
        panic!("root is an element");
    };
    assert_eq!(svg.name.local.as_ref(), "svg");
    assert_eq!(svg.name.ns.as_ref(), SVG_NS);
}

#[test]
fn element_case_is_preserved_not_folded() {
    let dom = DemoDom::parse_xml_from_bytes(
        br#"<SVG xmlns="http://www.w3.org/2000/svg" width="16" height="16"/>"#,
    )
    .expect("read XML bytes");
    let (root, _) = first_element(&dom);
    let DemoNodeData::Element(element) = root else {
        panic!("root is an element");
    };
    assert_eq!(
        element.name.local.as_ref(),
        "SVG",
        "XML must not case-fold element names the way the HTML tokenizer does"
    );
}

#[test]
fn unnamespaced_root_is_not_given_the_svg_namespace() {
    let dom =
        DemoDom::parse_xml_from_bytes(br#"<svg width="16" height="16"/>"#).expect("read XML bytes");
    let (root, _) = first_element(&dom);
    let DemoNodeData::Element(element) = root else {
        panic!("root is an element");
    };
    assert_eq!(element.name.local.as_ref(), "svg");
    assert_ne!(
        element.name.ns.as_ref(),
        SVG_NS,
        "without an authored xmlns the element is not an SVG element"
    );
}

#[test]
fn structural_recoveries_are_recorded_not_swallowed() {
    for (label, source) in [
        (
            "mismatched close tag",
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="4" height="4"></svg>"#,
        ),
        (
            "unexpected end of input",
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="4""#,
        ),
        (
            "duplicate attribute",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" width="8"/>"#,
        ),
    ] {
        let dom = DemoDom::parse_xml_from_bytes(source.as_bytes()).expect("read XML bytes");
        assert!(
            !dom.errors.is_empty(),
            "{label}: the XML5 recovery must be recorded so a strict caller can refuse it"
        );
    }
}

/// The known XML5 leniency boundary, pinned as an executable law.
///
/// The XML5 grammar deliberately trades well-formedness for recovery, and its
/// tokenizer does not report every deviation from XML 1.0 — an unquoted
/// attribute value recovers silently. A strict caller refusing recorded
/// errors therefore still accepts this class. Full draconian XML 1.0
/// well-formedness checking is a named open obligation of the conforming
/// entry, not a claim this entry makes today. If a parser upgrade starts
/// recording this class, this test fails and the boundary doc moves.
#[test]
fn unquoted_attribute_values_recover_unrecorded_the_xml5_boundary() {
    let dom =
        DemoDom::parse_xml_from_bytes(br#"<svg xmlns="http://www.w3.org/2000/svg" width=16/>"#)
            .expect("read XML bytes");
    assert_eq!(
        dom.errors,
        Vec::<String>::new(),
        "XML5 recovers unquoted attribute values without recording an error"
    );
}
