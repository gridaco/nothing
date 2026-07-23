use csscascade::adapter::{self, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::DemoDom;
use style::color::AbsoluteColor;
use style::dom::TElement;
use style::properties::{ComputedValues, LonghandId};
use style::thread_state::{self, ThreadState};
use style::values::computed::Color;
use style::values::generics::svg::{SVGPaintFallback, SVGPaintKind};

const DOCUMENT: &str = r##"<!doctype html>
<html><head><style>
  svg { color: #123456; fill: #010203; stroke: none; }
  #all {
    color: #2468ac;
    fill: currentColor;
    fill-opacity: 0.5;
    fill-rule: evenodd;
    stroke: #778899;
    stroke-width: 3px;
    stroke-linecap: round;
    stroke-linejoin: bevel;
    stroke-dasharray: 2px 3px;
    stroke-dashoffset: 1px;
    stroke-miterlimit: 5;
    stroke-opacity: 0.25;
  }
  #invalid { fill: #abcdef; }
  #computed-invalid { fill: #fedcba; }
  #disabled {
    background-clip: border-area;
    clip-path: circle(closest-corner);
  }
  #interaction { color: #010203; }
  @media (pointer: fine) and (hover: hover) and
         (any-pointer: fine) and (any-hover: hover) {
    #interaction { color: #2468ac; }
  }
  @media (pointer: coarse), (any-pointer: coarse) {
    #interaction { color: #ff0000; }
  }
</style></head><body>
  <svg xmlns="http://www.w3.org/2000/svg">
    <g><rect id="inherited"/></g>
    <rect id="all"/>
    <rect id="invalid" style="fill: not-a-paint"/>
    <rect id="computed-invalid" style="fill: var(--missing)"/>
    <rect id="server" style="fill: url(#gradient) red"/>
    <rect id="disabled"/>
  </svg>
  <div id="interaction"/>
</body></html>"##;

#[test]
fn official_servo_pin_exposes_svg_paint_and_declared_environment() {
    thread_state::initialize(ThreadState::LAYOUT);

    let dom = DemoDom::parse_from_bytes(DOCUMENT.as_bytes()).expect("parse HTML document");
    let mut driver = CascadeDriver::new(&dom);
    let document = adapter::bootstrap_dom(dom);
    driver.flush(document);
    driver.style_document(document);

    let root = document.root_element().expect("document root");
    let inherited = computed(find_by_id(root, "inherited"));
    let all = computed(find_by_id(root, "all"));
    let invalid = computed(find_by_id(root, "invalid"));
    let computed_invalid = computed(find_by_id(root, "computed-invalid"));
    let server = computed(find_by_id(root, "server"));
    let disabled = computed(find_by_id(root, "disabled"));
    let interaction = computed(find_by_id(root, "interaction"));

    assert_eq!(property(&inherited, LonghandId::Fill), "rgb(1, 2, 3)");

    let fill = all.clone_fill();
    let SVGPaintKind::Color(color) = &fill.kind else {
        panic!("expected typed SVG color paint")
    };
    assert!(matches!(color, Color::CurrentColor));
    assert_eq!(
        all.resolve_color(color),
        AbsoluteColor::srgb_legacy(0x24, 0x68, 0xac, 1.0),
    );

    assert_eq!(property(&all, LonghandId::FillOpacity), "0.5");
    assert_eq!(property(&all, LonghandId::FillRule), "evenodd");
    assert_eq!(property(&all, LonghandId::Stroke), "rgb(119, 136, 153)");
    assert_eq!(property(&all, LonghandId::StrokeWidth), "3px");
    assert_eq!(property(&all, LonghandId::StrokeLinecap), "round");
    assert_eq!(property(&all, LonghandId::StrokeLinejoin), "bevel");
    assert_eq!(property(&all, LonghandId::StrokeDasharray), "2px, 3px");
    assert_eq!(property(&all, LonghandId::StrokeDashoffset), "1px");
    assert_eq!(property(&all, LonghandId::StrokeMiterlimit), "5");
    assert_eq!(property(&all, LonghandId::StrokeOpacity), "0.25");

    assert_eq!(property(&invalid, LonghandId::Fill), "rgb(171, 205, 239)");
    assert_eq!(
        property(&computed_invalid, LonghandId::Fill),
        "rgb(1, 2, 3)"
    );
    let server_fill = server.clone_fill();
    let SVGPaintKind::PaintServer(url) = &server_fill.kind else {
        panic!("expected typed SVG paint server")
    };
    assert_eq!(
        url.url().map(|url| url.as_str()),
        Some("about:blank#gradient"),
    );
    let SVGPaintFallback::Color(fallback) = &server_fill.fallback else {
        panic!("expected typed SVG paint fallback color")
    };
    assert_eq!(
        server.resolve_color(fallback),
        AbsoluteColor::srgb_legacy(0xff, 0x00, 0x00, 1.0),
    );

    assert_eq!(
        property(&disabled, LonghandId::BackgroundClip),
        "border-box"
    );
    assert_eq!(
        property(&disabled, LonghandId::ClipPath),
        "circle(closest-corner)"
    );

    // The current cascade environment is a declared static-desktop profile,
    // not Stylo's target-dependent PointerCapabilities::default().
    assert_eq!(
        property(&interaction, LonghandId::Color),
        "rgb(36, 104, 172)"
    );
}

fn find_by_id(root: HtmlElement, wanted: &str) -> HtmlElement {
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

fn computed(element: HtmlElement) -> style::servo_arc::Arc<ComputedValues> {
    element
        .borrow_data()
        .expect("computed style")
        .styles
        .primary()
        .clone()
}

fn property(style: &ComputedValues, id: LonghandId) -> String {
    let mut output = String::new();
    style
        .computed_or_resolved_value(id, None, &mut output)
        .expect("serialize computed property");
    output
}
