//! Precedence and intake laws for SVG presentation hints and SVG-namespace
//! stylesheets.
//!
//! SVG2: presentation attributes are author-origin declarations that lose to
//! every author rule (`CascadeLevel::PresHints`); a value that fails its
//! property grammar drops exactly like an invalid CSS declaration; and SVG's
//! own `<style>` element feeds the same one cascade, so a standalone SVG/XML
//! document styles itself with no HTML wrapper. Only the admitted hint set
//! enters — every widening lands with its own law here.

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
    #stroke-rule-beats-hint { stroke: #2563eb; }
    #dash-rule-beats-hint { stroke-dasharray: 6px 2px; }
    #dash-style-attr-beats-rule { stroke-dasharray: 6px 2px; }
    #visibility-rule-beats-hint { visibility: visible; }
    #transform-rule-beats-hint { transform: translate(30px, 0px); }
    #transform-none-beats-hint { transform: none; }
    #transform-style-attr-beats-rule { transform: translate(30px, 0px); }
    #transform-webkit-alias { -webkit-transform: translate(30px, 0px); }
    #family-rule-beats-hint { font-family: monospace; }
  </style>
  <rect id="hint-only" fill="#16a34a" width="8" height="8"/>
  <rect id="named" fill="rebeccapurple" width="8" height="8"/>
  <rect id="rule-beats-hint" fill="#16a34a" width="8" height="8"/>
  <rect id="style-attr-beats-rule" fill="#16a34a" style="fill: #ef4444" width="8" height="8"/>
  <rect id="invalid-hint" fill="not-a-paint" width="8" height="8"/>
  <rect id="stroked" stroke="#16a34a" stroke-width="4px" stroke-linecap="round"
        stroke-linejoin="bevel" stroke-miterlimit="7" width="8" height="8"/>
  <rect id="stroke-rule-beats-hint" stroke="#16a34a" width="8" height="8"/>
  <rect id="dash-hint" stroke-dasharray="8 4" width="8" height="8"/>
  <rect id="dash-rule-beats-hint" stroke-dasharray="8 4" width="8" height="8"/>
  <rect id="dash-style-attr-beats-rule" stroke-dasharray="8 4"
        style="stroke-dasharray: 10px 5px" width="8" height="8"/>
  <rect id="dash-invalid-css-falls-back" stroke-dasharray="8 4"
        style="stroke-dasharray: 8px -4px" width="8" height="8"/>
  <rect id="dash-invalid-hint" stroke-dasharray="8 -4" width="8" height="8"/>
  <g stroke-dasharray="8 4"><rect id="dash-inherited" width="8" height="8"/></g>
  <rect id="unadmitted" pathLength="100" width="8" height="8"/>
  <g id="sized" font-size="32"><rect id="em-basis" stroke-width="0.5em" width="8" height="8"/></g>
  <rect id="hidden-hint" visibility="hidden" width="8" height="8"/>
  <rect id="display-none-hint" display="none" width="8" height="8"/>
  <rect id="invalid-visibility-hint" visibility="bogus" width="8" height="8"/>
  <rect id="visibility-rule-beats-hint" visibility="hidden" width="8" height="8"/>
  <rect id="translucent" fill-opacity="0.5" stroke-opacity="25%" width="8" height="8"/>
  <rect id="transform-hint" transform="translate(10 10)" width="8" height="8"/>
  <rect id="transform-rule-beats-hint" transform="translate(10 10)" width="8" height="8"/>
  <rect id="transform-none-beats-hint" transform="translate(10 10)" width="8" height="8"/>
  <rect id="transform-style-attr-beats-rule" transform="translate(10 10)"
        style="transform: translate(50px, 0px)" width="8" height="8"/>
  <rect id="transform-invalid-css-falls-back" transform="translate(10 10)"
        style="transform: translate(30, 0)" width="8" height="8"/>
  <rect id="transform-malformed-attr" transform="translate(10 10)," width="8" height="8"/>
  <rect id="transform-three-arg" transform="rotate(45 32 16)" width="8" height="8"/>
  <rect id="transform-run-together" transform="translate(10-10)" width="8" height="8"/>
  <rect id="transform-webkit-alias" width="8" height="8"/>
  <g color="#d0342c"><rect id="color-basis" fill="currentColor" width="8" height="8"/></g>
  <text id="family-hint" font-family="Ahem">X</text>
  <text id="family-rule-beats-hint" font-family="Ahem">X</text>
  <text id="family-invalid-hint" font-family="">X</text>
  <g font-family="Ahem"><text id="family-inherited">X</text></g>
  <linearGradient id="gradient-transform-hint" gradientTransform="translate(10 10)"/>
  <linearGradient id="gradient-plain-transform-inert" transform="translate(10 10)"/>
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
    // The strokes rung admitted the stroke geometry hints, so each computes
    // as its typed value — a unit-bearing length included, since the hint gets
    // the full CSS value grammar.
    assert_eq!(
        property(root, "stroked", LonghandId::Stroke),
        "rgb(22, 163, 74)"
    );
    assert_eq!(property(root, "stroked", LonghandId::StrokeWidth), "4px");
    assert_eq!(
        property(root, "stroked", LonghandId::StrokeLinecap),
        "round"
    );
    assert_eq!(
        property(root, "stroked", LonghandId::StrokeLinejoin),
        "bevel"
    );
    assert_eq!(property(root, "stroked", LonghandId::StrokeMiterlimit), "7");
    // Dasharray joins the same hint intake only with the dashing capability:
    // typed grammar, author-origin precedence, invalid-declaration fallback,
    // and inheritance are all the one cascade's work.
    assert_eq!(
        property(root, "dash-hint", LonghandId::StrokeDasharray),
        "8px, 4px"
    );
    assert_eq!(
        property(root, "dash-rule-beats-hint", LonghandId::StrokeDasharray),
        "6px, 2px"
    );
    assert_eq!(
        property(
            root,
            "dash-style-attr-beats-rule",
            LonghandId::StrokeDasharray
        ),
        "10px, 5px"
    );
    assert_eq!(
        property(
            root,
            "dash-invalid-css-falls-back",
            LonghandId::StrokeDasharray
        ),
        "8px, 4px"
    );
    assert_eq!(
        property(root, "dash-invalid-hint", LonghandId::StrokeDasharray),
        "none"
    );
    assert_eq!(
        property(root, "dash-inherited", LonghandId::StrokeDasharray),
        "8px, 4px"
    );
    // And they lose to an author rule exactly as `fill` does.
    assert_eq!(
        property(root, "stroke-rule-beats-hint", LonghandId::Stroke),
        "rgb(37, 99, 235)"
    );
    // An unadmitted presentation attribute still contributes nothing —
    // `pathLength` has no CSS longhand at all, and the block synthesizer
    // must skip it rather than invent one. (The websem compiler refuses
    // rendering-relevant unadmitted attributes by name, so a document
    // carrying one is a declared hole, not a silent one.)
    assert_eq!(
        property(root, "unadmitted", LonghandId::StrokeOpacity),
        "1",
        "unadmitted presentation attributes must not leak into the cascade"
    );
    // The text rung's hint: `font-family` is what a run resolves against.
    // Measured in Chromium: the attribute alone selects the face, an author
    // rule beats it, `font-family=""` drops to the default family, and the
    // property inherits.
    assert_eq!(
        property(root, "family-hint", LonghandId::FontFamily),
        "Ahem"
    );
    assert_eq!(
        property(root, "family-rule-beats-hint", LonghandId::FontFamily),
        "monospace"
    );
    assert_ne!(
        property(root, "family-invalid-hint", LonghandId::FontFamily),
        "Ahem",
        "an invalid family hint drops like an invalid CSS declaration"
    );
    assert_eq!(
        property(root, "family-inherited", LonghandId::FontFamily),
        "Ahem",
        "font-family inherits from the group carrying the hint"
    );
    // `font-size` is admitted for its *basis*, not because anything paints it:
    // it is what an `em` length resolves against, and Chromium treats it as a
    // presentation attribute. Inherited, so the shape reads the group's.
    assert_eq!(property(root, "sized", LonghandId::FontSize), "32px");
    assert_eq!(property(root, "em-basis", LonghandId::FontSize), "32px");
    assert_eq!(
        property(root, "em-basis", LonghandId::StrokeWidth),
        "16px",
        "an em stroke-width resolves against the presentation-attribute font size"
    );
    // The visibility rung's pair: both hints compute as typed values, an
    // author rule beats the hint (measured: a stylesheet
    // `visibility: visible` un-hides `visibility="hidden"` in Chromium),
    // and an invalid value drops to the initial `visible` exactly as
    // `display="bogus"` renders in Chromium.
    assert_eq!(
        property(root, "hidden-hint", LonghandId::Visibility),
        "hidden"
    );
    assert_eq!(
        property(root, "display-none-hint", LonghandId::Display),
        "none"
    );
    assert_eq!(
        property(root, "invalid-visibility-hint", LonghandId::Visibility),
        "visible"
    );
    assert_eq!(
        property(root, "visibility-rule-beats-hint", LonghandId::Visibility),
        "visible"
    );
    // The translucency pair: number and percentage spellings both compute
    // through the CSS <alpha-value> grammar.
    assert_eq!(
        property(root, "translucent", LonghandId::FillOpacity),
        "0.5"
    );
    assert_eq!(
        property(root, "translucent", LonghandId::StrokeOpacity),
        "0.25"
    );
    // The transform rung: the attribute is a presentation attribute of the
    // one CSS `transform` property (CSS Transforms L1 §7), entering through
    // the measured rewrite. Precedence is the cascade's, not reimplemented:
    // any author rule beats the attribute — `transform: none` included —
    // the style attribute beats the rule, and an *invalid* CSS declaration
    // never enters, so the attribute hint stands (all Chromium-measured).
    assert_eq!(
        property(root, "transform-hint", LonghandId::Transform),
        "translate(10px, 10px)"
    );
    assert_eq!(
        property(root, "transform-rule-beats-hint", LonghandId::Transform),
        "translate(30px)"
    );
    assert_eq!(
        property(root, "transform-none-beats-hint", LonghandId::Transform),
        "none"
    );
    assert_eq!(
        property(
            root,
            "transform-style-attr-beats-rule",
            LonghandId::Transform
        ),
        "translate(50px)"
    );
    assert_eq!(
        property(
            root,
            "transform-invalid-css-falls-back",
            LonghandId::Transform
        ),
        "translate(10px, 10px)"
    );
    // A malformed attribute list contributes no hint at all — the element
    // computes `none`, rendering untransformed exactly as Chromium drops
    // the whole attribute (measured across every refusal-boundary probe).
    assert_eq!(
        property(root, "transform-malformed-attr", LonghandId::Transform),
        "none"
    );
    // The attribute-only 3-argument rotate enters as its §7.3 defining
    // expansion; the computed list carries the sandwich.
    assert_eq!(
        property(root, "transform-three-arg", LonghandId::Transform),
        "translate(32px, 16px) rotate(45deg) translate(-32px, -16px)"
    );
    // The run-together leniency (csswg-drafts#2623) is part of the grammar.
    assert_eq!(
        property(root, "transform-run-together", LonghandId::Transform),
        "translate(10px, -10px)"
    );
    // The pinned Stylo implements the `-webkit-transform` alias, so that
    // spelling reaches the same longhand (Chromium applies it on SVG).
    assert_eq!(
        property(root, "transform-webkit-alias", LonghandId::Transform),
        "translate(30px)"
    );
    // The use/defs rung's addition: `color` is an admitted hint — the
    // inherited currentColor basis, measured through a `<use>` instance.
    assert_eq!(
        property(root, "color-basis", LonghandId::Color),
        "rgb(208, 52, 44)"
    );
    // The gradient rung: on a gradient element the transform property's
    // presentation attribute is `gradientTransform` (measured byte-identical
    // to an author `transform` declaration through non-quarter rotations
    // and scales), and the plain `transform` attribute is inert there
    // (measured: it changes no pixel in Chromium).
    assert_eq!(
        property(root, "gradient-transform-hint", LonghandId::Transform),
        "translate(10px, 10px)"
    );
    assert_eq!(
        property(
            root,
            "gradient-plain-transform-inert",
            LonghandId::Transform
        ),
        "none"
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
