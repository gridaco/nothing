//! Model-boundary laws for ordered radial circles, independent of a painter.

use n0_model::model::*;
use n0_model::n0_xml::{self, PrintError};
use n0_model::properties::{PropertyKey, PropertyTarget, PropertyValue, PropertyValues, ValueView};
use n0_model::renderability::validate_paint;

const SOURCE: &str = r##"<grida version="0"><container><rect width="10" height="10"><fill><gradient kind="radial"><stop offset="0" color="#000"/><stop offset="1" color="#fff"/></gradient></fill></rect></container></grida>"##;

fn source() -> (Document, NodeId) {
    let doc = n0_xml::parse(SOURCE).unwrap();
    let container = doc.get(doc.root).children[0];
    let rect = doc.get(container).children[0];
    (doc, rect)
}

fn circle(x: f32, y: f32, radius: f32) -> RadialGradientCircle {
    RadialGradientCircle {
        center: (x, y),
        radius,
    }
}

fn paint(geometry: RadialGradientGeometry) -> Paint {
    Paint::RadialGradient(RadialGradientPaint {
        geometry: Some(geometry),
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: Color::BLACK.into(),
            },
            GradientStop {
                offset: 1.0,
                color: Color(0xffff_ffff).into(),
            },
        ],
        ..Default::default()
    })
}

fn projected(
    doc: &Document,
    node: NodeId,
    paint: Paint,
) -> Result<PropertyValues, n0_model::properties::PropertyError> {
    PropertyValues::new(
        doc,
        [(
            PropertyTarget::new(doc.key_of(node).unwrap(), PropertyKey::Fills),
            PropertyValue::Paints(Paints::new([paint])),
        )],
    )
}

#[test]
fn zero_exterior_reversed_and_equal_circles_are_valid_facts() {
    let (doc, rect) = source();
    for (start, end) in [
        (circle(0.5, 0.5, 0.0), circle(0.5, 0.5, 0.5)),
        (circle(-2.25, 3.5, 1.75), circle(0.5, 0.5, 0.0)),
        (circle(0.0, 0.0, 0.0), circle(1.0, 1.0, 0.0)),
        (circle(0.5, 0.5, 0.5), circle(0.5, 0.5, 0.5)),
        (circle(0.5, 0.5, 0.0), circle(0.5, 0.5, 0.0)),
        (circle(f32::MAX, -f32::MAX, f32::MAX), circle(0.0, 0.0, 0.0)),
    ] {
        let paint = paint(RadialGradientGeometry { start, end });
        validate_paint(&paint).unwrap();
        let values = projected(&doc, rect, paint.clone()).unwrap();
        assert_eq!(ValueView::new(&doc, &values).unwrap().fills(rect)[0], paint);
    }
}

#[test]
fn every_nonfinite_component_and_negative_radius_is_rejected_at_admission() {
    let (doc, rect) = source();
    for is_end in [false, true] {
        for component in 0..3 {
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
                if component != 2 && invalid == -1.0 {
                    continue;
                }
                let mut geometry = RadialGradientGeometry {
                    start: circle(0.25, 0.375, 0.125),
                    end: circle(0.5, 0.625, 0.75),
                };
                let circle = if is_end {
                    &mut geometry.end
                } else {
                    &mut geometry.start
                };
                match component {
                    0 => circle.center.0 = invalid,
                    1 => circle.center.1 = invalid,
                    _ => circle.radius = invalid,
                }
                let paint = paint(geometry);
                let message = validate_paint(&paint).unwrap_err().to_string();
                let role = if is_end { "end" } else { "start" };
                let field = if component == 2 { "radius" } else { "center" };
                assert!(
                    message.contains(&format!("radial gradient {role} circle {field}")),
                    "{message}"
                );
                assert!(
                    projected(&doc, rect, paint).is_err(),
                    "{role} {field}={invalid}"
                );
            }
        }
    }
}

#[test]
fn model_copy_keeps_direct_coordinate_bits_and_circle_order() {
    let (doc, rect) = source();
    let geometry = RadialGradientGeometry {
        start: circle(f32::from_bits(1), -0.0, 1.75),
        end: circle(3.5, -2.25, 0.0),
    };
    let values = projected(&doc, rect, paint(geometry)).unwrap().clone();
    let view = ValueView::new(&doc, &values).unwrap();
    let Paint::RadialGradient(paint) = &view.fills(rect)[0] else {
        panic!("radial fact")
    };
    let geometry = paint.geometry.unwrap();
    assert_eq!(
        [
            geometry.start.center.0.to_bits(),
            geometry.start.center.1.to_bits(),
            geometry.start.radius.to_bits()
        ],
        [1, 0x8000_0000, 0x3fe0_0000]
    );
    assert_eq!(
        [
            geometry.end.center.0.to_bits(),
            geometry.end.center.1.to_bits(),
            geometry.end.radius.to_bits()
        ],
        [0x4060_0000, 0xc010_0000, 0]
    );
}

#[test]
fn draft_zero_preserves_absence_and_refuses_every_present_circle_pair() {
    let (mut doc, rect) = source();
    let printed = n0_xml::print(&doc).unwrap();
    let roundtrip = n0_xml::parse(&printed).unwrap();
    assert_eq!(doc, roundtrip);
    let Paint::RadialGradient(original) = &doc.get(rect).fills[0] else {
        panic!("radial fact")
    };
    assert_eq!(original.geometry, None);

    for geometry in [
        RadialGradientGeometry {
            start: circle(0.5, 0.5, 0.0),
            end: circle(0.5, 0.5, 0.5),
        },
        RadialGradientGeometry {
            start: circle(-1.0, 2.0, 0.75),
            end: circle(0.5, 0.5, 0.0),
        },
    ] {
        for active in [false, true] {
            let mut paint = paint(geometry);
            let Paint::RadialGradient(radial) = &mut paint else {
                unreachable!()
            };
            radial.active = active;
            doc.get_mut(rect).fills = Paints::new([paint]);
            assert!(
                matches!(n0_xml::print(&doc), Err(PrintError::InvalidDocument(message))
                if message.contains("ordered circle geometry is not representable in Draft 0"))
            );
            assert_eq!(
                n0_model::textir::try_print(&doc).unwrap_err().0,
                format!(
                    "node {rect} has a paint stack the historical TextIr dialect cannot represent"
                )
            );
        }
    }
}

#[test]
fn draft_zero_cannot_drop_ordered_circles_from_strokes_or_run_overrides() {
    let geometry = RadialGradientGeometry {
        start: circle(-1.0, 2.0, 0.75),
        end: circle(0.5, 0.5, 0.0),
    };
    let (mut doc, rect) = source();
    let mut stroke = Stroke::default_for(&doc.get(rect).payload).unwrap();
    stroke.paints = Paints::new([paint(geometry)]);
    doc.get_mut(rect).strokes.push(stroke);
    assert!(
        matches!(n0_xml::print(&doc), Err(PrintError::InvalidDocument(message))
        if message.contains("ordered circle geometry is not representable in Draft 0"))
    );

    let mut text = n0_xml::parse(r##"<grida version="0"><container><text><tspan fill="#000">a</tspan></text></container></grida>"##).unwrap();
    let container = text.get(text.root).children[0];
    let id = text.get(container).children[0];
    let Payload::AttributedText {
        attributed_string, ..
    } = &mut text.get_mut(id).payload
    else {
        panic!("run-bearing text");
    };
    attributed_string.runs[0].fills = Some(Paints::new([paint(geometry)]));
    assert!(
        matches!(n0_xml::print(&text), Err(PrintError::InvalidDocument(message))
        if message.contains("ordered circle geometry is not representable in Draft 0"))
    );
}
