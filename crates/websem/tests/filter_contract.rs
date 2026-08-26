//! SVG image-filter laws at the Web-semantic contract boundary.
//!
//! Chromium probes decide the source grammar, graph fallback, color space,
//! region, and effect order. These tests pin the resolved result and the
//! stable refusals that keep every unrepresented graph branch from becoming
//! an unfiltered silent fallback.

#[allow(dead_code)]
mod support;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    Filter, FilterBlend, FilterColorSpace, FilterComposite, FilterDisplacementChannel, FilterInput,
    FilterMorphology, FilterPrimitive, FilterTurbulenceKind, Frame, FrameItem, ScopeEffect,
};
use support::render_through_n0;
use websem::{DegradationAction, InitialViewport, SvgFrameSource};

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

fn admit_both(source: &str) -> Frame {
    let strict = SvgFrameSource::from_standalone_svg(source, viewport()).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort admits");
    let declared: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        declared.is_empty(),
        "an admitted filter declares nothing: {declared:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn assert_target_skip(source: &str, reason: &str) {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport()).expect_err("strict must refuse");
    assert!(strict.to_string().contains(reason), "{strict}");

    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport())
        .expect("best effort declares the affected target");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|degradation| degradation.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1, "one affected target: {skipped:?}");
    assert!(
        skipped[0].reason().contains(reason),
        "{}",
        skipped[0].reason()
    );
    assert_eq!(
        best.base_frame().nodes().len(),
        1,
        "the white backdrop survives"
    );
}

fn at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 64 + x) * 4;
    pixels[offset..offset + 4].try_into().expect("RGBA pixel")
}

fn resolved_filter(frame: &Frame) -> &Filter {
    frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("one resolved filter")
}

#[test]
fn gaussian_blur_resolves_to_one_source_neutral_checked_graph() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" primitiveUnits="objectBoundingBox">
    <feGaussianBlur stdDeviation=".125" result="blurred"/>
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##,
    ));
    let tags: Vec<_> = frame
        .items
        .iter()
        .map(|item| match item {
            FrameItem::Node(_) => "node",
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Filter(_) => "filter-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
                ScopeEffect::Clip(_) => "clip-begin",
            },
            FrameItem::ScopeEnd => "scope-end",
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
        })
        .collect();
    assert_eq!(tags, ["node", "filter-begin", "node", "scope-end"]);

    let filter = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("resolved filter scope");
    assert_eq!(filter.transform(), AffineTransform::identity());
    assert_eq!(
        filter.region(),
        Rectangle::from_xywh(17.6, 17.6, 28.8, 28.8)
    );
    let node = filter.program().iter().next().expect("one blur node");
    assert_eq!(node.inputs(), [FilterInput::Source]);
    assert_eq!(node.color_space(), FilterColorSpace::LinearRgb);
    assert_eq!(
        node.primitive(),
        FilterPrimitive::GaussianBlur {
            sigma_x: 3.0,
            sigma_y: 3.0
        }
    );

    let pixels = render_through_n0(&frame, 64, 64);
    assert_ne!(at(&pixels, 17, 32), [255, 255, 255, 255]);
    assert_eq!(at(&pixels, 16, 32), [255, 255, 255, 255]);
}

#[test]
fn hard_shadow_graph_resolves_zero_one_two_and_n_input_operations() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feOffset in="SourceAlpha" dx="5" dy="4" result="o"/>
    <feFlood flood-color="#7c3aed" flood-opacity=".65" result="f"/>
    <feComposite in="f" in2="o" operator="in" result="s"/>
    <feMerge><feMergeNode in="s"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#0ea5e9" filter="url(#f)"/>"##,
    ));
    let filter = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("one resolved filter");
    let nodes: Vec<_> = filter.program().iter().collect();
    assert_eq!(nodes.len(), 4);
    assert_eq!(nodes[0].inputs(), [FilterInput::SourceAlpha]);
    assert_eq!(
        nodes[0].primitive(),
        FilterPrimitive::Offset { dx: 5.0, dy: 4.0 }
    );
    assert!(nodes[1].inputs().is_empty());
    let FilterPrimitive::SolidColor { color } = nodes[1].primitive() else {
        panic!("second node is the resolved solid source")
    };
    assert_eq!(color.to_rgba8(), cg::CGColor::from_rgba(124, 58, 237, 166));
    assert_eq!(
        nodes[2].inputs(),
        [FilterInput::Node(1), FilterInput::Node(0)]
    );
    assert_eq!(
        nodes[2].primitive(),
        FilterPrimitive::Composite {
            operator: FilterComposite::In
        }
    );
    assert_eq!(
        nodes[3].inputs(),
        [FilterInput::Node(2), FilterInput::Source]
    );
    assert_eq!(nodes[3].primitive(), FilterPrimitive::Merge);
    assert!(
        nodes
            .iter()
            .all(|node| node.color_space() == FilterColorSpace::Srgb)
    );

    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 24, 24), [14, 165, 233, 255]);
    assert_ne!(at(&pixels, 47, 32), [255, 255, 255, 255]);
}

#[test]
fn drop_shadow_resolves_to_one_native_checked_operation() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feDropShadow in="SourceGraphic" dx="5" dy="-4" stdDeviation="2 3"
                  flood-color="#7c3aed" flood-opacity=".65" result="shadow"/>
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#0ea5e9" filter="url(#f)"/>"##,
    ));
    let filter = frame
        .items
        .iter()
        .find_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Filter(filter) => Some(filter),
                ScopeEffect::Opacity(_) | ScopeEffect::Clip(_) => None,
            },
            _ => None,
        })
        .expect("resolved filter scope");
    let nodes: Vec<_> = filter.program().iter().collect();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].inputs(), [FilterInput::Source]);
    assert_eq!(nodes[0].color_space(), FilterColorSpace::Srgb);
    let FilterPrimitive::DropShadow {
        dx,
        dy,
        sigma_x,
        sigma_y,
        color,
    } = nodes[0].primitive()
    else {
        panic!("the graph carries one native drop shadow")
    };
    assert_eq!((dx, dy, sigma_x, sigma_y), (5.0, -4.0, 2.0, 3.0));
    assert_eq!(color.to_rgba8(), cg::CGColor::from_rgba(124, 58, 237, 166));

    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 24, 24), [14, 165, 233, 255]);
    assert_ne!(at(&pixels, 47, 32), [255, 255, 255, 255]);
}

#[test]
fn color_matrix_types_lower_to_one_finite_checked_matrix() {
    let source = |primitive: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{primitive}</filter>
  <circle cx="32" cy="32" r="16" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let matrix = |primitive: &str| {
        let frame = admit_both(&source(primitive));
        let filter = resolved_filter(&frame);
        assert_eq!(filter.program().iter().count(), 1, "one matrix node");
        let node = filter.program().iter().next().expect("one matrix node");
        assert_eq!(node.inputs(), [FilterInput::Source]);
        assert_eq!(node.color_space(), FilterColorSpace::Srgb);
        let FilterPrimitive::ColorMatrix { matrix } = node.primitive() else {
            panic!("the source syntax resolves away before the frame")
        };
        assert!(matrix.iter().all(|coefficient| coefficient.is_finite()));
        matrix
    };

    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    assert_eq!(
        matrix(r##"<feColorMatrix values="+1,0 0 0 0,0 +1 0 0 0,0 0 +1 0 0,0 0 0 +1 0,"/>"##),
        identity,
        "the measured SVG number-list grammar reaches one row-major matrix"
    );
    for primitive in [
        r##"<feColorMatrix/>"##,
        r##"<feColorMatrix values="1 0 0"/>"##,
        r##"<feColorMatrix values="1,,0"/>"##,
        r##"<feColorMatrix type="saturate"/>"##,
        r##"<feColorMatrix type="hueRotate" values="90 180"/>"##,
    ] {
        assert_eq!(matrix(primitive), identity, "{primitive}");
    }

    assert_eq!(
        matrix(r##"<feColorMatrix type="saturate" values="0"/>"##),
        [
            0.213, 0.715, 0.072, 0.0, 0.0, 0.213, 0.715, 0.072, 0.0, 0.0, 0.213, 0.715, 0.072, 0.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ]
    );
    assert_ne!(
        matrix(r##"<feColorMatrix type="saturate" values="-1"/>"##),
        matrix(r##"<feColorMatrix type="saturate" values="0"/>"##),
        "saturation is not clamped"
    );
    assert_ne!(
        matrix(r##"<feColorMatrix type="hueRotate" values="360000090"/>"##),
        matrix(r##"<feColorMatrix type="hueRotate" values="90"/>"##),
        "large angles are not reduced before Blink's f32 trig route"
    );
    let luminance = matrix(r##"<feColorMatrix type="luminanceToAlpha" values="bad"/>"##);
    assert_eq!(&luminance[..15], &[0.0; 15]);
    assert_eq!(&luminance[15..], &[0.2125, 0.7154, 0.0721, 0.0, 0.0]);
}

#[test]
fn component_transfer_children_lower_to_four_exact_byte_tables() {
    let source = |primitive: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{primitive}</filter>
  <rect x="8" y="8" width="48" height="48" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let tables = |primitive: &str| {
        let frame = admit_both(&source(primitive));
        let filter = resolved_filter(&frame);
        assert_eq!(filter.program().iter().count(), 1, "one transfer node");
        let node = filter.program().iter().next().expect("one transfer node");
        assert_eq!(node.inputs(), [FilterInput::Source]);
        assert_eq!(node.color_space(), FilterColorSpace::Srgb);
        let FilterPrimitive::ComponentTransfer { tables } = node.primitive() else {
            panic!("the authored function vocabulary resolves before the frame")
        };
        tables
    };

    let identity = std::array::from_fn(|index| index as u8);
    for primitive in [
        r##"<feComponentTransfer/>"##,
        r##"<feComponentTransfer><feFuncR/></feComponentTransfer>"##,
        r##"<feComponentTransfer><feFuncR type="Linear" slope="0"/></feComponentTransfer>"##,
        r##"<feComponentTransfer><feFuncR type="table" tableValues="0,,1"/></feComponentTransfer>"##,
        r##"<feComponentTransfer><g><feFuncR type="linear" slope="0"/></g></feComponentTransfer>"##,
    ] {
        let actual = tables(primitive);
        assert_eq!(actual.red(), &identity, "{primitive}");
        assert_eq!(actual.green(), &identity, "{primitive}");
        assert_eq!(actual.blue(), &identity, "{primitive}");
        assert_eq!(actual.alpha(), &identity, "{primitive}");
    }

    let actual = tables(
        r##"<feComponentTransfer>
          <feFuncR type="table" tableValues="-.2 .1 .9 1.2"/>
          <feFuncG type="discrete" tableValues="1 .5 0"/>
          <feFuncB type="linear" slope=".73" intercept=".11"/>
          <feFuncA type="gamma" amplitude=".83" exponent="1.3" offset=".07"/>
        </feComponentTransfer>"##,
    );
    assert_eq!(
        (actual.red()[175], actual.red()[185], actual.red()[195]),
        (234, 243, 252)
    );
    assert_eq!(
        (
            actual.green()[84],
            actual.green()[85],
            actual.green()[169],
            actual.green()[170]
        ),
        (255, 127, 127, 0)
    );
    assert_eq!(
        (actual.blue()[0], actual.blue()[127], actual.blue()[255]),
        (28, 120, 214)
    );
    assert_eq!(
        (actual.alpha()[0], actual.alpha()[127], actual.alpha()[255]),
        (17, 103, 229)
    );

    let ordered = tables(
        r##"<feComponentTransfer><feFuncR type="linear" slope="1.654435761" intercept=".18682"/></feComponentTransfer>"##,
    );
    assert_eq!(ordered.red()[25], 89, "Blink's ordered SVG-number f32 wins");
    let linear = tables(
        r##"<feComponentTransfer><feFuncG type="linear" slope="2" intercept="-.4"/></feComponentTransfer>"##,
    );
    assert_eq!(linear.green()[52], 2, "linear products and sum stay f32");
    let singleton = tables(
        r##"<feComponentTransfer><feFuncR type="table" tableValues=".5"/></feComponentTransfer>"##,
    );
    assert_eq!(
        singleton.red()[0],
        127,
        "LUT conversion truncates, not rounds"
    );

    let duplicate = tables(
        r##"<feComponentTransfer><feFuncR type="linear" slope="0"/><feFuncR type="identity"/></feComponentTransfer>"##,
    );
    assert_eq!(
        duplicate.red(),
        &identity,
        "the last same-channel child wins"
    );
}

#[test]
fn morphology_resolves_complete_parser_fallback_and_axis_semantics() {
    let source = |primitive: &str, primitive_units: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="{primitive_units}"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    {primitive}
  </filter>
  <rect x="20" y="20" width="20" height="10" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let resolved = |primitive: &str, primitive_units: &str| {
        let frame = admit_both(&source(primitive, primitive_units));
        let filter = resolved_filter(&frame);
        assert_eq!(filter.program().iter().count(), 1);
        let node = filter.program().iter().next().expect("one morphology node");
        assert_eq!(node.inputs(), [FilterInput::Source]);
        let FilterPrimitive::Morphology {
            operator,
            radius_x,
            radius_y,
        } = node.primitive()
        else {
            panic!("morphology syntax resolves before the frame")
        };
        (operator, radius_x, radius_y, frame)
    };

    let (operator, radius_x, radius_y, frame) = resolved(
        r##"<feMorphology operator="dilate" radius="+2,3,"/>"##,
        "userSpaceOnUse",
    );
    assert_eq!(operator, FilterMorphology::Dilate);
    assert_eq!((radius_x, radius_y), (2.0, 3.0));
    let pixels = render_through_n0(&frame, 64, 64);
    assert_eq!(at(&pixels, 18, 25), [14, 165, 233, 255]);
    assert_eq!(at(&pixels, 17, 25), [255, 255, 255, 255]);

    for (primitive, expected) in [
        (
            r##"<feMorphology operator=" dilate " radius="2px"/>"##,
            (FilterMorphology::Erode, 0.0, 0.0),
        ),
        (
            r##"<feMorphology operator="dilate" radius="-1 2"/>"##,
            (FilterMorphology::Dilate, 0.0, 2.0),
        ),
        (
            r##"<feMorphology operator="dilate" radius="2.0.2"/>"##,
            (FilterMorphology::Dilate, 2.0, 0.2),
        ),
        (
            r##"<feMorphology operator="dilate" radius="2 2 2"/>"##,
            (FilterMorphology::Dilate, 0.0, 0.0),
        ),
    ] {
        let (operator, radius_x, radius_y, _) = resolved(primitive, "userSpaceOnUse");
        assert_eq!((operator, radius_x, radius_y), expected, "{primitive}");
    }

    let (operator, radius_x, radius_y, _) = resolved(
        r##"<feMorphology operator="dilate" radius=".025 .05"/>"##,
        "objectBoundingBox",
    );
    assert_eq!(operator, FilterMorphology::Dilate);
    assert_eq!((radius_x, radius_y), (0.50000006, 0.50000006));
}

#[test]
fn morphology_precision_boundaries_refuse_before_a_wrong_pixel() {
    let filter = r##"<filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feMorphology operator="dilate" radius="2"/>
  </filter>"##;
    for (target, reason) in [
        (
            r##"<rect x="20" y="18" width="20" height="18" fill="url(#g)" filter="url(#f)"/>"##,
            "morphology paint-server precision boundary",
        ),
        (
            r##"<circle cx="30" cy="27" r="9" fill="#d946ef" filter="url(#f)"/>"##,
            "retained filled-ellipse coverage boundary",
        ),
        (
            r##"<use href="#circle-source" filter="url(#f)"/>"##,
            "retained filled-ellipse coverage boundary",
        ),
        (
            r##"<rect x="20" y="18" width="20" height="18" fill="#d946ef" transform="rotate(17 30 27)" filter="url(#f)"/>"##,
            "morphology transform precision boundary",
        ),
    ] {
        assert_target_skip(
            &document(&format!(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><linearGradient id="g"><stop stop-color="#dc2626"/><stop offset="1" stop-color="#2563eb"/></linearGradient><circle id="circle-source" cx="30" cy="27" r="9" fill="#d946ef"/></defs>
  {filter}
  {target}"##
            )),
            reason,
        );
    }

    admit_both(&document(&format!(
        r##"  {filter}
  <circle cx="30" cy="27" r="9" fill="none" stroke="#d946ef" stroke-width="3" filter="url(#f)"/>
  <rect x="20" y="18" width="20" height="18" rx="6" fill="#d946ef" filter="url(#f)"/>"##
    )));

    let generated_rotated = document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood x="20" y="18" width="20" height="18" flood-color="#d946ef" result="seed"/>
    <feMorphology in="seed" operator="dilate" radius="2" x="0" y="0" width="64" height="64"/>
  </filter>
  <rect width="64" height="64" fill="transparent" transform="rotate(17 30 27)" filter="url(#f)"/>"##,
    );
    assert_target_skip(
        &generated_rotated,
        "morphology transform precision boundary",
    );
}

#[test]
fn turbulence_resolves_the_measured_grammar_before_crossing_the_frame_seam() {
    let source = |primitive: &str| {
        document(&format!(
            r##"  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    {primitive}
  </filter>
  <rect x="12" y="14" width="40" height="36" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let primitive = |authored: &str| {
        let frame = admit_both(&source(authored));
        let filter = resolved_filter(&frame);
        assert_eq!(filter.program().iter().count(), 1);
        let node = filter.program().iter().next().expect("one turbulence node");
        assert!(node.inputs().is_empty(), "turbulence is a generated source");
        node.primitive()
    };

    assert_eq!(
        primitive(
            r##"<feTurbulence type="fractalNoise" baseFrequency="+.125,.25," numOctaves="12"
                seed="-3.5," stitchTiles="stitch"/>"##
        ),
        FilterPrimitive::Turbulence {
            kind: FilterTurbulenceKind::FractalNoise,
            base_frequency_x: 0.125,
            base_frequency_y: 0.25,
            num_octaves: 9,
            seed: -3.5,
            stitch_tiles: true,
        }
    );
    assert_eq!(
        primitive(r##"<feTurbulence baseFrequency=".125" numOctaves="+2"/>"##),
        FilterPrimitive::Turbulence {
            kind: FilterTurbulenceKind::Turbulence,
            base_frequency_x: 0.125,
            base_frequency_y: 0.125,
            num_octaves: 2,
            seed: 0.0,
            stitch_tiles: false,
        },
        "one frequency duplicates across both axes"
    );

    for authored in [
        r##"<feTurbulence baseFrequency="-1 2"/>"##,
        r##"<feTurbulence baseFrequency="2 -1"/>"##,
        r##"<feTurbulence baseFrequency="2 2 2"/>"##,
        r##"<feTurbulence baseFrequency="2px"/>"##,
        r##"<feTurbulence baseFrequency="calc(2)"/>"##,
    ] {
        let FilterPrimitive::Turbulence {
            base_frequency_x,
            base_frequency_y,
            ..
        } = primitive(authored)
        else {
            panic!("invalid frequency retains the initial turbulence member")
        };
        assert_eq!(
            (base_frequency_x, base_frequency_y),
            (0.0, 0.0),
            "{authored}"
        );
    }

    for authored in [
        r##"<feTurbulence type="FractalNoise" stitchTiles="Stitch" numOctaves="1.0" seed="2px"/>"##,
        r##"<feTurbulence type=" fractalNoise " stitchTiles=" stitch " numOctaves="2147483648" seed="1 2"/>"##,
    ] {
        assert_eq!(
            primitive(authored),
            FilterPrimitive::Turbulence {
                kind: FilterTurbulenceKind::Turbulence,
                base_frequency_x: 0.0,
                base_frequency_y: 0.0,
                num_octaves: 1,
                seed: 0.0,
                stitch_tiles: false,
            },
            "invalid enumeration and scalar spellings retain each initial: {authored}"
        );
    }

    assert_eq!(
        primitive(r##"<feTurbulence type="fractalNoise" numOctaves="0"/>"##),
        FilterPrimitive::Turbulence {
            kind: FilterTurbulenceKind::FractalNoise,
            base_frequency_x: 0.0,
            base_frequency_y: 0.0,
            num_octaves: 0,
            seed: 0.0,
            stitch_tiles: false,
        },
        "zero reaches the fractal formula"
    );
    assert!(
        matches!(
            primitive(r##"<feTurbulence numOctaves="-1"/>"##),
            FilterPrimitive::SolidColor { color } if color.a() == 0.0
        ),
        "a negative octave count resolves to a bounded transparent image"
    );
}

#[test]
fn displacement_map_resolves_ordered_inputs_channels_and_horizontal_object_box_scale() {
    let source = |primitive: &str, primitive_units: &str| {
        document(&format!(
            r##"  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="{primitive_units}"
          x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#ff0080" result="color"/>
    <feFlood flood-color="#40c080" result="map"/>
    {primitive}
  </filter>
  <rect x="12" y="18" width="20" height="10" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let resolved = |primitive: &str, primitive_units: &str| {
        let frame = admit_both(&source(primitive, primitive_units));
        let filter = resolved_filter(&frame);
        let node = filter.program().iter().last().expect("displacement output");
        assert_eq!(node.inputs(), [FilterInput::Node(0), FilterInput::Node(1)]);
        node.primitive()
    };

    assert_eq!(
        resolved(
            r##"<feDisplacementMap in="color" in2="map" scale=".25," xChannelSelector="R" yChannelSelector="G"/>"##,
            "objectBoundingBox",
        ),
        FilterPrimitive::DisplacementMap {
            scale: 5.0,
            x_channel: FilterDisplacementChannel::Red,
            y_channel: FilterDisplacementChannel::Green,
        },
        "Blink's one native scalar uses target width for both displacement axes"
    );

    let channels = [
        ("R", FilterDisplacementChannel::Red),
        ("G", FilterDisplacementChannel::Green),
        ("B", FilterDisplacementChannel::Blue),
        ("A", FilterDisplacementChannel::Alpha),
    ];
    for (x_name, x_channel) in channels {
        for (y_name, y_channel) in channels {
            assert_eq!(
                resolved(
                    &format!(
                        r##"<feDisplacementMap in="color" in2="map" scale="-12.5" xChannelSelector="{x_name}" yChannelSelector="{y_name}"/>"##
                    ),
                    "userSpaceOnUse",
                ),
                FilterPrimitive::DisplacementMap {
                    scale: -12.5,
                    x_channel,
                    y_channel,
                }
            );
        }
    }

    for authored in [
        r##"<feDisplacementMap in="color" in2="map" scale="2px" xChannelSelector="r" yChannelSelector=" G "/>"##,
        r##"<feDisplacementMap in="color" in2="map" scale="calc(2)" xChannelSelector="initial" yChannelSelector="unset"/>"##,
        r##"<feDisplacementMap in="color" in2="map" scale="1 2" xChannelSelector="" yChannelSelector="bogus"/>"##,
    ] {
        assert_eq!(
            resolved(authored, "userSpaceOnUse"),
            FilterPrimitive::DisplacementMap {
                scale: 0.0,
                x_channel: FilterDisplacementChannel::Alpha,
                y_channel: FilterDisplacementChannel::Alpha,
            },
            "invalid spellings retain each initial: {authored}"
        );
    }
}

#[test]
fn generated_turbulence_can_paint_a_declared_empty_user_space_source() {
    let source = |primitive_units: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="{primitive_units}"
          x="8" y="8" width="48" height="48" color-interpolation-filters="sRGB">
    <feTurbulence baseFrequency=".07 .11" numOctaves="2" seed="3"/>
  </filter>
  <g filter="url(#f)"/>"##
        ))
    };
    let user = admit_both(&source("userSpaceOnUse"));
    let object = admit_both(&source("objectBoundingBox"));
    for frame in [&user, &object] {
        let filter = resolved_filter(frame);
        assert!(filter.source_is_transparent());
        let pixels = render_through_n0(frame, 64, 64);
        assert_ne!(at(&pixels, 24, 24), [255, 255, 255, 255]);
        assert_eq!(at(&pixels, 4, 4), [255, 255, 255, 255]);
    }
    assert_eq!(
        render_through_n0(&user, 64, 64),
        render_through_n0(&object, 64, 64),
        "base frequency itself is not primitiveUnits-scaled"
    );
}

#[test]
fn turbulence_and_displacement_precision_boundaries_refuse_before_paint() {
    let turbulence = r##"<filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feTurbulence baseFrequency=".07 .11" seed="3"/>
  </filter>"##;
    let displacement = r##"<filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feFlood flood-color="#ff0080" result="map"/>
    <feDisplacementMap in="SourceGraphic" in2="map" scale="16" xChannelSelector="R" yChannelSelector="B"/>
  </filter>"##;
    for (filter, target, reason) in [
        (
            turbulence,
            r##"<rect x="12" y="14" width="40" height="36" transform="rotate(17 32 32)" filter="url(#f)"/>"##,
            "procedural-filter transform precision boundary",
        ),
        (
            displacement,
            r##"<rect x="12" y="14" width="40" height="36" transform="skewX(17)" filter="url(#f)"/>"##,
            "displacement-filter transform precision boundary",
        ),
        (
            displacement,
            r##"<rect x="12" y="14" width="40" height="36" clip-path="url(#c)" filter="url(#f)"/>"##,
            "filtered clip-path precision boundary",
        ),
    ] {
        assert_target_skip(
            &document(&format!(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><clipPath id="c"><circle cx="32" cy="32" r="18"/></clipPath></defs>
  {filter}
  {target}"##
            )),
            reason,
        );
    }

    for transform in ["rotate(90 32 32)", "translate(64 0) scale(-1 1)"] {
        admit_both(&document(&format!(
            r##"  {turbulence}
  <rect x="12" y="14" width="40" height="36" transform="{transform}" filter="url(#f)"/>"##
        )));
    }
}

#[test]
fn blend_modes_lower_to_one_checked_two_input_operation() {
    let mode = |attribute: Option<&str>| {
        let attribute = attribute
            .map(|value| format!(r##" mode="{value}""##))
            .unwrap_or_default();
        let frame = admit_both(&document(&format!(
            r##"  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">
    <feFlood flood-color="#1fadd6" result="bg"/>
    <feFlood flood-color="#c43779" result="fg"/>
    <feBlend in="fg" in2="bg"{attribute}/>
  </filter>
  <rect width="64" height="64" filter="url(#f)"/>"##
        )));
        let filter = resolved_filter(&frame);
        let node = filter.program().iter().last().expect("one blend output");
        assert_eq!(node.inputs(), [FilterInput::Node(1), FilterInput::Node(0)]);
        assert_eq!(node.color_space(), FilterColorSpace::Srgb);
        let FilterPrimitive::Blend { mode } = node.primitive() else {
            panic!("the authored blend vocabulary resolves before the frame")
        };
        mode
    };

    for (name, expected) in [
        ("normal", FilterBlend::Normal),
        ("multiply", FilterBlend::Multiply),
        ("screen", FilterBlend::Screen),
        ("overlay", FilterBlend::Overlay),
        ("darken", FilterBlend::Darken),
        ("lighten", FilterBlend::Lighten),
        ("color-dodge", FilterBlend::ColorDodge),
        ("color-burn", FilterBlend::ColorBurn),
        ("hard-light", FilterBlend::HardLight),
        ("soft-light", FilterBlend::SoftLight),
        ("difference", FilterBlend::Difference),
        ("exclusion", FilterBlend::Exclusion),
        ("hue", FilterBlend::Hue),
        ("saturation", FilterBlend::Saturation),
        ("color", FilterBlend::Color),
        ("luminosity", FilterBlend::Luminosity),
    ] {
        assert_eq!(mode(Some(name)), expected, "{name}");
    }
    for fallback in [
        None,
        Some(""),
        Some("bogus"),
        Some("Multiply"),
        Some(" multiply "),
        Some("colorDodge"),
        Some("plus-lighter"),
        Some("initial"),
        Some("inherit"),
        Some("unset"),
        Some("revert"),
        Some("revert-layer"),
    ] {
        assert_eq!(mode(fallback), FilterBlend::Normal, "{fallback:?}");
    }
}

#[test]
fn blend_precision_patrols_name_mapping_and_clip_boundaries() {
    for (source, reason) in [
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#1fadd6" result="bg"/><feFlood flood-color="#c43779" result="fg"/>
    <feBlend in="fg" in2="bg" mode="soft-light"/>
  </filter>
  <rect x="8" y="9" width="48" height="46" transform="rotate(17 32 32)" filter="url(#f)"/>"##,
            ),
            "blend-filter transform precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><clipPath id="c"><circle cx="32" cy="32" r="19"/></clipPath></defs>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#1fadd6" result="bg"/><feFlood flood-color="#c43779" result="fg"/>
    <feBlend in="fg" in2="bg" mode="soft-light"/>
  </filter>
  <rect x="8" y="9" width="48" height="46" clip-path="url(#c)" filter="url(#f)"/>"##,
            ),
            "filtered clip-path precision boundary",
        ),
    ] {
        assert_target_skip(&source, reason);
    }

    admit_both(&document(
        r##"  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#1fadd6" result="bg"/><feFlood flood-color="#c43779" result="fg"/>
    <feBlend in="fg" in2="bg" mode="soft-light"/>
  </filter>
  <rect x="8" y="9" width="48" height="46" transform="translate(.375 .625) scale(.875)" filter="url(#f)"/>"##,
    ));
}

#[test]
fn translucent_source_multi_input_precision_boundary_is_named() {
    assert_target_skip(
        &document(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#e6a817" flood-opacity=".34" result="gold"/>
    <feComposite in="SourceGraphic" in2="gold" operator="atop"/>
  </filter>
  <rect x="8" y="9" width="48" height="46" rx="7" fill="#c43779" fill-opacity=".62" filter="url(#f)"/>"##,
        ),
        "translucent-source composition precision boundary",
    );

    admit_both(&document(
        r##"  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" color-interpolation-filters="sRGB">
    <feFlood flood-color="#1fadd6" flood-opacity=".47" result="bg"/>
    <feFlood flood-color="#c43779" flood-opacity=".62" result="fg"/>
    <feBlend in="fg" in2="bg" mode="soft-light" result="mixed"/>
    <feFlood flood-color="#e6a817" flood-opacity=".34" result="gold"/>
    <feComposite in="mixed" in2="gold" operator="atop"/>
  </filter>
  <rect x="8" y="9" width="48" height="46" rx="7" fill="#c43779" fill-opacity=".62" filter="url(#f)"/>"##,
    ));
}

#[test]
fn color_matrix_can_cross_channels_and_create_alpha_inside_its_hard_region() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse"
          x="8" y="8" width="48" height="48" color-interpolation-filters="sRGB">
    <feColorMatrix x="12" y="12" width="40" height="40"
      values="0 0 0 1 0  0 1 0 0 0  0 0 1 0 0  0 0 0 1 .25"/>
  </filter>
  <rect x="24" y="24" width="16" height="16" fill="#0ea5e9" filter="url(#f)"/>"##,
    ));
    let pixels = render_through_n0(&frame, 64, 64);
    assert_ne!(at(&pixels, 16, 16), [255, 255, 255, 255]);
    assert_eq!(
        at(&pixels, 9, 9),
        [255, 255, 255, 255],
        "alpha creation cannot escape the primitive crop"
    );
    assert_ne!(
        at(&pixels, 30, 30),
        [14, 165, 233, 255],
        "the alpha channel feeds the red output"
    );
}

#[test]
fn generated_color_matrix_input_does_not_inherit_source_layer_patrols() {
    admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">
    <feFlood flood-color="#0ea5e9" flood-opacity=".4" result="f"/>
    <feColorMatrix in="f"
      values=".5 0 0 0 0  0 .5 0 0 0  0 0 .5 0 0  0 0 0 1 .25"/>
  </filter>
  <g filter="url(#f)"><circle cx="20" cy="20" r="12"/><circle cx="42" cy="42" r="12"/></g>"##,
    ));
}

#[test]
fn color_matrix_precision_patrols_name_source_transform_and_spatial_boundaries() {
    for (source, reason) in [
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feColorMatrix/></filter>
  <circle cx="32" cy="32" r="16" fill="#0ea5e9" stroke="#0f172a" filter="url(#f)"/>"##,
            ),
            "color-matrix source-layer precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feColorMatrix values=".5 0 0 0 0 0 .5 0 0 0 0 0 .5 0 0 0 0 0 1 0"/></filter>
  <circle cx="32" cy="32" r="16" fill="#0ea5e9" transform="rotate(17 32 32)" filter="url(#f)"/>"##,
            ),
            "color-matrix transform precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feColorMatrix/><feGaussianBlur stdDeviation="2"/></filter>
  <circle cx="32" cy="32" r="16" fill="#0ea5e9" filter="url(#f)"/>"##,
            ),
            "composed-operation precision boundary",
        ),
    ] {
        assert_target_skip(&source, reason);
    }
}

#[test]
fn component_transfer_precision_patrols_name_paint_server_and_transform_boundaries() {
    for (source, reason) in [
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <defs><linearGradient id="g"><stop stop-color="blue"/><stop offset="1" stop-color="orange"/></linearGradient></defs>
  <filter id="f" color-interpolation-filters="sRGB"><feComponentTransfer><feFuncR type="linear" slope=".5"/></feComponentTransfer></filter>
  <circle cx="32" cy="32" r="16" fill="url(#g)" filter="url(#f)"/>"##,
            ),
            "table-filter paint-server precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feComponentTransfer><feFuncR type="linear" slope=".5"/></feComponentTransfer></filter>
  <rect x="12" y="12" width="40" height="40" rx="7" fill="#0ea5e9" transform="rotate(17 32 32)" filter="url(#f)"/>"##,
            ),
            "table-filter transform precision boundary",
        ),
    ] {
        assert_target_skip(&source, reason);
    }

    admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB">
    <feFlood flood-color="#0ea5e9"/>
    <feComponentTransfer><feFuncR type="linear" slope=".5"/></feComponentTransfer>
  </filter>
  <circle cx="32" cy="32" r="16" fill="#0f172a" transform="rotate(17 32 32)" filter="url(#f)"/>"##,
    ));
}

#[test]
fn filtered_descendants_refuse_same_scope_clip_and_partial_opacity() {
    let defs = r##"  <defs>
    <clipPath id="c"><circle cx="32" cy="32" r="19"/></clipPath>
    <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <feOffset dx="0" dy="0"/>
    </filter>
  </defs>"##;
    assert_target_skip(
        &document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
{defs}
  <g opacity=".63" clip-path="url(#c)">
    <rect x="5" y="5" width="54" height="54" fill="#406080" filter="url(#f)"/>
  </g>"##
        )),
        "effect-stack precision boundary",
    );

    admit_both(&document(&format!(
        r##"{defs}
  <g opacity=".63"><g clip-path="url(#c)">
    <rect x="5" y="5" width="54" height="54" fill="#406080" filter="url(#f)"/>
  </g></g>"##
    )));
}

#[test]
fn load_active_function_animation_refuses_before_authored_tables_escape() {
    let source = document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB">
    <feComponentTransfer>
      <feFuncR type="linear" slope="1"><set attributeName="slope" to="0"/></feFuncR>
    </feComponentTransfer>
  </filter>
  <rect x="8" y="8" width="48" height="48" fill="#0ea5e9" filter="url(#f)"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
        .expect_err("strict must refuse");
    assert!(
        strict.to_string().contains("animation element <set>"),
        "{strict}"
    );
    let best = SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
        .expect("best effort declares the overridden function");
    assert!(best.degradations().iter().any(|degradation| {
        degradation.action() == DegradationAction::Skipped
            && degradation
                .reason()
                .contains("<feFuncR> authored state is overridden")
    }));
}

#[test]
fn drop_shadow_number_defaults_clamps_and_units_follow_blink() {
    let source = |primitive: &str, filter_extra: &str, target: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB" {filter_extra}>{primitive}</filter>
  {target}"##
        ))
    };
    let target = r##"<rect x="20" y="20" width="16" height="8" fill="#0ea5e9" filter="url(#f)"/>"##;
    assert_eq!(
        admit_both(&source("<feDropShadow/>", "", target)),
        admit_both(&source(
            r##"<feDropShadow dx="2" dy="2" stdDeviation="2" flood-color="black" flood-opacity="1"/>"##,
            "",
            target,
        )),
        "missing fields use Blink's native initial 2/2/2 and black"
    );
    assert_eq!(
        admit_both(&source(
            r##"<feDropShadow dx="+4" dy="+2" stdDeviation="+2,+4"/>"##,
            "",
            target,
        )),
        admit_both(&source(
            r##"<feDropShadow dx="4" dy="2" stdDeviation="2 4"/>"##,
            "",
            target,
        )),
        "leading plus and comma separators preserve the number grammar"
    );
    assert_eq!(
        admit_both(&source(
            r##"<feDropShadow dx="4px" dy="25%" stdDeviation="calc(2)"/>"##,
            "",
            target,
        )),
        admit_both(&source("<feDropShadow/>", "", target)),
        "invalid number spellings return each field to its initial"
    );
    assert_eq!(
        admit_both(&source(
            r##"<feDropShadow dx="4" dy="2" stdDeviation="-2 4"/>"##,
            "",
            target,
        )),
        admit_both(&source(
            r##"<feDropShadow dx="4" dy="2" stdDeviation="0 4"/>"##,
            "",
            target,
        )),
        "negative blur axes clamp independently to zero"
    );
    assert_eq!(
        admit_both(&source(
            r##"<feDropShadow dx="4" dy="2" stdDeviation="2 4"/>"##,
            r##"primitiveUnits="userSpaceOnUse""##,
            target,
        )),
        admit_both(&source(
            r##"<feDropShadow dx=".25" dy=".25" stdDeviation=".125 .5"/>"##,
            r##"primitiveUnits="objectBoundingBox""##,
            target,
        )),
        "object-box numbers resolve against the target's two axes"
    );
}

#[test]
fn flood_opacity_percentage_keeps_css_parser_normalization_order() {
    let source = |opacity: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">
    <feFlood flood-color="red" flood-opacity="{opacity}"/>
  </filter>
  <rect width="64" height="64" filter="url(#f)"/>"##
        ))
    };

    let percentage = admit_both(&source("57.384267578125007%"));
    let equivalent_number = admit_both(&source(".57384267578125007"));
    let lower_f32_neighbor = admit_both(&source(".5738426446914673"));
    assert_eq!(
        percentage, equivalent_number,
        "CSS percentage normalization must divide before narrowing to f32"
    );
    assert_ne!(
        percentage, lower_f32_neighbor,
        "the authored percentage must not collapse onto the lower f32 neighbor"
    );
}

#[test]
fn offset_only_graphs_can_exceed_the_old_two_operation_boundary() {
    let source = |body: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{body}</filter>
  <rect x="20" y="20" width="24" height="24" fill="#0ea5e9" filter="url(#f)"/>"##
        ))
    };
    let chained = render_through_n0(
        &admit_both(&source(
            r##"<feOffset dx="1" result="a"/><feOffset in="a" dx="1" result="b"/><feOffset in="b" dx="1"/>"##,
        )),
        64,
        64,
    );
    let direct = render_through_n0(&admit_both(&source(r##"<feOffset dx="3"/>"##)), 64, 64);
    assert_eq!(chained, direct);
}

#[test]
fn safe_sigma_blur_graphs_can_exceed_the_retired_depth_boundary() {
    let source = |body: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64"
          color-interpolation-filters="sRGB">{body}</filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##
        ))
    };
    let direct = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur in="a" stdDeviation="2" result="b"/><feGaussianBlur in="b" stdDeviation="2"/>"##,
        )),
        64,
        64,
    );
    let through_merges = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feMerge result="m1"><feMergeNode in="a"/></feMerge><feGaussianBlur in="m1" stdDeviation="2" result="b"/><feMerge result="m2"><feMergeNode in="b"/></feMerge><feGaussianBlur in="m2" stdDeviation="2"/>"##,
        )),
        64,
        64,
    );
    assert_eq!(direct, through_merges);

    admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feGaussianBlur stdDeviation="1"/>
  </filter>
  <rect x="10" y="10" width="12" height="12" transform="scale(2)" fill="#16a34a" filter="url(#f)"/>"##,
    ));
}

#[test]
fn graph_inputs_names_units_and_color_spaces_follow_the_measured_blink_fallbacks() {
    let source = |primitive: &str, filter_extra: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64" {filter_extra}>
    {primitive}
  </filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="url(#f)"/>"##
        ))
    };
    let previous = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur stdDeviation="2"/>"##,
            "",
        )),
        64,
        64,
    );
    for second in [r##"in="a""##, r##"in="missing""##] {
        let pixels = render_through_n0(
            &admit_both(&source(
                &format!(
                    r##"<feGaussianBlur stdDeviation="2" result="a"/><feGaussianBlur {second} stdDeviation="2"/>"##
                ),
                "",
            )),
            64,
            64,
        );
        assert_eq!(pixels, previous, "{second} selects the previous result");
    }

    let user = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="3"/>"##,
            r##"primitiveUnits="userSpaceOnUse""##,
        )),
        64,
        64,
    );
    let object = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation=".125"/>"##,
            r##"primitiveUnits="objectBoundingBox""##,
        )),
        64,
        64,
    );
    assert_eq!(user, object, "object-box sigma resolves per target axis");

    let linear = render_through_n0(
        &admit_both(&source(r##"<feGaussianBlur stdDeviation="3"/>"##, "")),
        64,
        64,
    );
    let auto = render_through_n0(
        &admit_both(&source(
            r##"<feGaussianBlur stdDeviation="3" color-interpolation-filters="auto"/>"##,
            "",
        )),
        64,
        64,
    );
    assert_ne!(
        linear, auto,
        "missing is linearRGB while explicit auto is sRGB"
    );
}

#[test]
fn empty_invalid_and_wrong_kind_references_are_measured_nothings() {
    let source = |defs: &str, filter: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <defs>{defs}</defs>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" {filter}/>"##
        ))
    };
    let plain = render_through_n0(&admit_both(&source("", "")), 64, 64);
    for document in [
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(#missing)""##,
        ),
        source(r##"<linearGradient id="f"/>"##, r##"filter="url(#f)""##),
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(#f) trailing""##,
        ),
        source(
            r##"<filter id="f"><feGaussianBlur stdDeviation="3"/></filter>"##,
            r##"filter="url(/**/#f/**/)""##,
        ),
    ] {
        assert_eq!(render_through_n0(&admit_both(&document), 64, 64), plain);
    }

    let hidden = render_through_n0(
        &admit_both(&source(r##"<filter id="f"/>"##, r##"filter="url(#f)""##)),
        64,
        64,
    );
    assert_eq!(at(&hidden, 32, 32), [255, 255, 255, 255]);
}

#[test]
fn quoted_urls_share_the_url_branch_and_lists_refuse_by_name() {
    let source = |filter: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="3"/></filter>
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="{filter}"/>"##
        ))
    };

    assert_eq!(
        admit_both(&source("url('#f')")),
        admit_both(&source("url(#f)")),
        "quoted and unquoted URL tokens resolve the same resource"
    );

    for filter in [
        "url('#f') url('#f')",
        "url('#f') blur(1px)",
        "url('#f'), url('#f')",
    ] {
        assert_target_skip(&source(filter), "multiple filter operations");
    }

    let plain = render_through_n0(&admit_both(&source("none")), 64, 64);
    let invalid = render_through_n0(&admit_both(&source("url('#f') trailing")), 64, 64);
    assert_eq!(
        invalid, plain,
        "an invalid trailing ident drops the whole hint"
    );
}

#[test]
fn admitted_effect_order_is_clip_then_opacity_then_mask_then_filter() {
    let frame = admit_both(&document(
        r##"  <rect width="64" height="64" fill="white"/>
  <clipPath id="c"><rect x="8" y="8" width="48" height="48"/></clipPath>
  <mask id="m" mask-type="alpha"><rect width="64" height="64" fill="white"/></mask>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
    <feGaussianBlur stdDeviation="2"/>
  </filter>
  <g clip-path="url(#c)"><g opacity=".5"><g mask="url(#m)"><g filter="url(#f)">
    <rect x="16" y="16" width="32" height="32" fill="black"/>
    <rect x="24" y="16" width="24" height="32" fill="black"/>
  </g></g></g></g>"##,
    ));
    let tags: Vec<_> = frame
        .items
        .iter()
        .skip(1)
        .map(|item| match item {
            FrameItem::ScopeBegin(scope) => match &scope.effect {
                ScopeEffect::Clip(_) => "clip-begin",
                ScopeEffect::Opacity(_) => "opacity-begin",
                ScopeEffect::Filter(_) => "filter-begin",
            },
            FrameItem::MaskBegin(_) => "mask-begin",
            FrameItem::Node(_) => "node",
            FrameItem::MaskSource => "mask-source",
            FrameItem::MaskEnd => "mask-end",
            FrameItem::ScopeEnd => "scope-end",
        })
        .collect();
    assert_eq!(
        tags,
        [
            "clip-begin",
            "opacity-begin",
            "mask-begin",
            "filter-begin",
            "node",
            "node",
            "scope-end",
            "mask-source",
            "node",
            "mask-end",
            "scope-end",
            "scope-end",
        ]
    );
}

#[test]
fn unsupported_filter_routes_skip_the_whole_target_by_stable_name() {
    let target = |filter: &str, attribute: &str| {
        document(&format!(
            r##"  <rect width="64" height="64" fill="white"/>
  {filter}
  <rect x="20" y="20" width="24" height="24" fill="#16a34a" {attribute}/>"##
        ))
    };
    for (source, reason) in [
        (
            target(
                r##"<filter id="f"><feOffset dx="2.5"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "fractional displacement",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="128" height="128">
    <feOffset dx="1"/>
  </filter>
  <g transform="scale(.5)">
    <rect x="40" y="40" width="48" height="48" fill="#16a34a" filter="url(#f)"/>
  </g>"##,
            ),
            "fractional device-space displacement",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/><feOffset dx="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "combines feOffset with Gaussian blur",
        ),
        (
            target(
                r##"<filter id="f"><feFlood style="flood-color:red"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS flood-color on <feFlood>",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="calc(1 / 2)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS function",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="var(--o)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-opacity resolves through var()",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-opacity="inherit"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-opacity uses inherit",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="var(--c)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-color resolves through var()",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="inherit"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "flood-color uses inherit",
        ),
        (
            target(
                r##"<filter id="f"><feFlood flood-color="hsl(0 100% 50%)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "outside the admitted color slice",
        ),
        (
            target(
                r##"<filter id="f"><feDropShadow stdDeviation="1"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "small-kernel precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feDropShadow stdDeviation="2"/></filter>
  <g transform="rotate(19 32 32)" filter="url(#f)">
    <rect x="20" y="20" width="24" height="24" fill="#16a34a"/>
  </g>"##,
            ),
            "native-shadow transform precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
  <filter id="f" color-interpolation-filters="sRGB"><feDropShadow stdDeviation="2"/></filter>
  <rect x="20" y="20" width="24" height="24" fill="url(#g)" filter="url(#f)"/>"##,
            ),
            "native-shadow source-layer precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f" color-interpolation-filters="sRGB"><feDropShadow stdDeviation="2"/></filter>
  <g filter="url(#f)"><g opacity=".55"><rect x="20" y="20" width="24" height="24" fill="#16a34a"/></g></g>"##,
            ),
            "native-shadow source-layer precision boundary",
        ),
        (
            target(
                r##"<filter id="f"><feDropShadow stdDeviation="2" flood-color="#7c3aed"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "native-shadow color-conversion precision boundary",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow dx="33554432" stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "admitted native-shadow range",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow stdDeviation="1e999"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "finite resolved-filter range",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow style="flood-color:red"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS flood-color on <feDropShadow>",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow flood-color="var(--c)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "feDropShadow flood-color resolves through var()",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow flood-color="inherit"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "feDropShadow flood-color uses inherit",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow flood-color="hsl(0 100% 50%)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "feDropShadow flood-color is outside the admitted color slice",
        ),
        (
            target(
                r##"<filter id="f" color-interpolation-filters="sRGB"><feDropShadow flood-opacity="calc(1 / 2)"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "feDropShadow flood-opacity is a CSS function",
        ),
        (
            target(
                r##"<filter id="f" href="#base"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "href inheritance",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur width="0" stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "transparent graph result",
        ),
        (
            target(
                r##"<filter id="f" x="1em"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "unit, whose basis is not admitted",
        ),
        (
            target("", r##"filter="blur(2px)""##),
            "CSS filter functions",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url('#f') url('#f')""##,
            ),
            "multiple filter operations",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="var(--fx)" style="--fx:url(#f)""##,
            ),
            "filter presentation attribute uses var()",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <g filter="url(#f)">
    <rect x="20" y="20" width="24" height="24" fill="#16a34a" filter="inherit"/>
  </g>"##,
            ),
            "filter presentation attribute uses inherit",
        ),
        (
            target("", r##"filter="url(https://example.test/f.svg#f)""##),
            "external",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="/*x*/linearRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "contains a CSS comment",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="l\69 nearRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "contains a CSS escape",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" color-interpolation-filters="var(--space)" style="--space:sRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "color-interpolation-filters presentation attribute uses var()",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="2" style="color-interpolation-filters:sRGB"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "CSS color-interpolation-filters",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur stdDeviation="1"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "small-kernel precision boundary",
        ),
        (
            document(
                r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="3"/></filter>
  <g transform="scale(.5)">
    <rect x="40" y="40" width="24" height="24" fill="#16a34a" filter="url(#f)"/>
  </g>"##,
            ),
            "small-kernel precision boundary",
        ),
        (
            target(
                r##"<filter id="f" x="33554432"><feGaussianBlur stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "crosses the unimplemented Web used-length range",
        ),
        (
            target(
                r##"<filter id="f"><feGaussianBlur x="128" y="128" width="8" height="8" stdDeviation="2"/></filter>"##,
                r##"filter="url(#f)""##,
            ),
            "outside the effect region",
        ),
    ] {
        assert_target_skip(&source, reason);
    }
}

#[test]
fn root_and_css_filter_routes_remain_named_separate_boundaries() {
    let root = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" filter="url(#f)">
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <rect width="64" height="64" fill="black"/>
</svg>"##;
    for result in [
        SvgFrameSource::from_standalone_svg(root, viewport()),
        SvgFrameSource::from_standalone_svg_best_effort(root, viewport()),
    ] {
        let error = result.expect_err("root filter is a document-level boundary");
        assert!(error.to_string().contains("root <svg>"), "{error}");
    }

    let css = document(
        r##"  <rect width="64" height="64" fill="white"/>
  <filter id="f"><feGaussianBlur stdDeviation="2"/></filter>
  <rect x="20" y="20" width="24" height="24" fill="black"
        filter="url(#f)" style="filter:none"/>"##,
    );
    let strict = SvgFrameSource::from_standalone_svg(css.as_str(), viewport())
        .expect_err("CSS property ingress remains quarantined");
    assert!(strict.to_string().contains("declares filter"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(css.as_str(), viewport())
        .expect("best effort declares the CSS boundary");
    assert!(best.degradations().iter().any(|degradation| {
        degradation.action() == DegradationAction::Skipped
            && degradation.reason().contains("declares filter")
    }));
}
