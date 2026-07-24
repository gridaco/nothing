//! Precedence and intake laws for SVG presentation hints and SVG-namespace
//! stylesheets.
//!
//! SVG2: presentation attributes are author-origin declarations that lose to
//! every author rule (`CascadeLevel::PresHints`); a value that fails its
//! property grammar drops exactly like an invalid CSS declaration; and SVG's
//! own `<style>` element feeds the same one cascade, so a standalone SVG/XML
//! document styles itself with no HTML wrapper. Only the admitted hint set
//! (`fill` today) enters — every widening lands with its own law here.

use csscascade::adapter::{DocumentSession, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::DemoDom;
use style::dom::TElement;
use style::properties::{ComputedValues, LonghandId};
use style::thread_state::{self, ThreadState};

const STANDALONE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
  <style>
    #rule-beats-hint { fill: #2563eb; }
    #style-attr-beats-rule { fill: #2563eb; }
  </style>
  <rect id="hint-only" fill="#16a34a" width="8" height="8"/>
  <rect id="named" fill="rebeccapurple" width="8" height="8"/>
  <rect id="rule-beats-hint" fill="#16a34a" width="8" height="8"/>
  <rect id="style-attr-beats-rule" fill="#16a34a" style="fill: #ef4444" width="8" height="8"/>
  <rect id="invalid-hint" fill="not-a-paint" width="8" height="8"/>
  <rect id="unadmitted" stroke="#16a34a" width="8" height="8"/>
</svg>"##;

#[test]
fn standalone_svg_presentation_hints_enter_below_author_rules() {
    thread_state::initialize(ThreadState::LAYOUT);
    let dom = DemoDom::parse_xml_from_bytes(STANDALONE.as_bytes()).expect("parse standalone SVG");
    assert_eq!(dom.errors, Vec::<String>::new(), "fixture is well-formed");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    let document = session.document();
    let root = document.root_element().expect("svg root");

    // The admitted hint alone computes as typed SVG paint.
    assert_eq!(fill(root, "hint-only"), "rgb(22, 163, 74)");
    // Hints get the full CSS value grammar, not a bespoke matcher.
    assert_eq!(fill(root, "named"), "rgb(102, 51, 153)");
    // An author rule from SVG's own <style> beats the presentation hint —
    // this asserts both the SVG-namespace stylesheet intake and the
    // PresHints precedence in one law.
    assert_eq!(fill(root, "rule-beats-hint"), "rgb(37, 99, 235)");
    // The inline style attribute beats the author rule.
    assert_eq!(fill(root, "style-attr-beats-rule"), "rgb(239, 68, 68)");
    // A hint value failing the property grammar drops like an invalid CSS
    // declaration: fill falls back to its initial black.
    assert_eq!(fill(root, "invalid-hint"), "rgb(0, 0, 0)");
    // An unadmitted presentation attribute contributes nothing yet: stroke
    // keeps its initial `none` until its own capability step admits it.
    assert_eq!(
        property(root, "unadmitted", LonghandId::Stroke),
        "none",
        "unadmitted presentation attributes must not leak into the cascade"
    );
}

#[test]
fn inline_html_svg_presentation_hints_behave_identically() {
    thread_state::initialize(ThreadState::LAYOUT);
    let html = r##"<!doctype html>
<html><head><style>#ruled { fill: #2563eb; }</style></head><body>
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
  <rect id="plain" fill="#16a34a" width="8" height="8"/>
  <rect id="ruled" fill="#16a34a" width="8" height="8"/>
</svg></body></html>"##;
    let dom = DemoDom::parse_from_bytes(html.as_bytes()).expect("parse HTML");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    let document = session.document();
    let root = document.root_element().expect("html root");

    assert_eq!(fill(root, "plain"), "rgb(22, 163, 74)");
    assert_eq!(
        fill(root, "ruled"),
        "rgb(37, 99, 235)",
        "the surrounding HTML document's author rules beat the hint"
    );
}

#[test]
fn svg_only_stylesheets_feed_the_one_cascade_without_invented_css() {
    // A document whose only <style> is SVG-namespace feeds its authored
    // styles to the one cascade, and no fallback author sheet exists in any
    // trigger condition — the engine invents no CSS (the demo-era fallback
    // that injected `body { color: #111 }` into wholly unstyled documents
    // was removed for silently diverging from Chromium's initial `color`).
    thread_state::initialize(ThreadState::LAYOUT);
    let html = r##"<html><body id="host">
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
  <style>svg { color: #ef4444 }</style>
  <rect id="probe" fill="currentColor" width="8" height="8"/>
</svg></body></html>"##;
    let dom = DemoDom::parse_from_bytes(html.as_bytes()).expect("parse HTML");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    let document = session.document();
    let root = document.root_element().expect("html root");

    assert_eq!(
        property(root, "probe", LonghandId::Color),
        "rgb(239, 68, 68)",
        "the SVG stylesheet must be the collected author CSS"
    );
    assert_eq!(
        property(root, "host", LonghandId::Color),
        "rgb(0, 0, 0)",
        "no author sheet is invented alongside real styles"
    );
}

#[test]
fn wholly_unstyled_documents_cascade_from_initial_values_alone() {
    // Chromium ground truth: a page with zero author CSS resolves `color`
    // to the initial value (#000), so `fill="currentColor"` paints black.
    // The cascade must not invent an author sheet for this case.
    thread_state::initialize(ThreadState::LAYOUT);
    let html = r##"<html><body id="host">
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
  <rect id="probe" fill="currentColor" width="8" height="8"/>
</svg></body></html>"##;
    let dom = DemoDom::parse_from_bytes(html.as_bytes()).expect("parse HTML");
    let mut session = DocumentSession::new(dom);
    CascadeDriver::new(&mut session).style_document();
    let document = session.document();
    let root = document.root_element().expect("html root");

    assert_eq!(
        property(root, "host", LonghandId::Color),
        "rgb(0, 0, 0)",
        "zero author CSS must resolve color to the initial value, as Chromium does"
    );
    assert_eq!(
        property(root, "probe", LonghandId::Color),
        "rgb(0, 0, 0)",
        "currentColor inside the inline SVG inherits the same uninvented initial"
    );
}

fn fill(root: HtmlElement<'_>, id: &str) -> String {
    property(root, id, LonghandId::Fill)
}

fn property(root: HtmlElement<'_>, id: &str, longhand: LonghandId) -> String {
    let element = find_by_id(root, id);
    let data = element.borrow_data().expect("computed style");
    let style: &ComputedValues = data.styles.primary();
    let mut output = String::new();
    style
        .computed_or_resolved_value(longhand, None, &mut output)
        .expect("serialize computed property");
    output
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
