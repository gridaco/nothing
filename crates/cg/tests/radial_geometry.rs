//! Serialization contract for the ordered radial-circle value, without a backend.

use cg::{RadialGradientCircle, RadialGradientGeometry, RadialGradientPaint};
use serde_json::json;

#[test]
fn absent_geometry_preserves_the_old_json_field_set_and_input() {
    let old = json!({
        "active": true,
        "transform": {"matrix": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]},
        "stops": [],
        "opacity": 1.0,
        "blend_mode": "normal",
        "tile_mode": "clamp"
    });
    let paint: RadialGradientPaint = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(paint, RadialGradientPaint::default());
    assert_eq!(paint.geometry, None);
    assert_eq!(serde_json::to_value(&paint).unwrap(), old);
    assert_eq!(
        serde_json::to_string(&paint).unwrap(),
        r#"{"active":true,"transform":{"matrix":[[1.0,0.0,0.0],[0.0,1.0,0.0]]},"stops":[],"opacity":1.0,"blend_mode":"normal","tile_mode":"clamp"}"#
    );
}

#[test]
fn debug_preserves_absent_values_but_never_hides_present_geometry() {
    let mut paint = RadialGradientPaint::default();
    assert_eq!(format!("{paint:?}"), "RadialGradientPaint { active: true, transform: AffineTransform { matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] }, stops: [], opacity: 1.0, blend_mode: Normal, tile_mode: Clamp }");
    paint.geometry = Some(RadialGradientGeometry {
        start: RadialGradientCircle {
            center: (-2.0, 1.0),
            radius: 0.75,
        },
        end: RadialGradientCircle {
            center: (0.5, 0.5),
            radius: 0.0,
        },
    });
    let debug = format!("{paint:?}");
    assert!(debug.contains("geometry: RadialGradientGeometry { start: RadialGradientCircle { center: (-2.0, 1.0), radius: 0.75 }, end: RadialGradientCircle { center: (0.5, 0.5), radius: 0.0 } }"));
}

fn circle_bits(circle: RadialGradientCircle) -> [u32; 3] {
    [
        circle.center.0.to_bits(),
        circle.center.1.to_bits(),
        circle.radius.to_bits(),
    ]
}

#[test]
fn present_geometry_round_trips_without_alignment_conversion_or_circle_reordering() {
    // A subnormal, signed zero, and a radius larger than the point-sized end
    // discriminate arithmetic normalization and inferred circle order.
    let paint = RadialGradientPaint {
        geometry: Some(RadialGradientGeometry {
            start: RadialGradientCircle {
                center: (f32::from_bits(1), -0.0),
                radius: 1.75,
            },
            end: RadialGradientCircle {
                center: (3.5, -2.25),
                radius: 0.0,
            },
        }),
        ..Default::default()
    };
    let encoded = serde_json::to_string(&paint).unwrap();
    let decoded: RadialGradientPaint = serde_json::from_str(&encoded).unwrap();
    let geometry = decoded.geometry.unwrap();
    assert_eq!(circle_bits(geometry.start), [1, 0x8000_0000, 0x3fe0_0000]);
    assert_eq!(circle_bits(geometry.end), [0x4060_0000, 0xc010_0000, 0]);
    assert_eq!(decoded, paint);
    assert_eq!(
        serde_json::to_value(&decoded).unwrap()["geometry"]["end"],
        json!({"center": [3.5, -2.25], "radius": 0.0})
    );
}

#[test]
fn explicit_implicit_circle_values_remain_present() {
    let geometry = RadialGradientGeometry {
        start: RadialGradientCircle {
            center: (0.5, 0.5),
            radius: 0.0,
        },
        end: RadialGradientCircle {
            center: (0.5, 0.5),
            radius: 0.5,
        },
    };
    let paint = RadialGradientPaint {
        geometry: Some(geometry),
        ..Default::default()
    };
    let decoded: RadialGradientPaint =
        serde_json::from_value(serde_json::to_value(paint).unwrap()).unwrap();
    assert_eq!(decoded.geometry, Some(geometry));
}

#[test]
fn a_present_geometry_requires_both_complete_circles() {
    for incomplete in [
        json!({"start": {"center": [0.5, 0.5], "radius": 0.0}}),
        json!({"end": {"center": [0.5, 0.5], "radius": 0.5}}),
        json!({"start": {"center": [0.5, 0.5]}, "end": {"center": [0.5, 0.5], "radius": 0.5}}),
        json!({"start": {"center": [0.5, 0.5], "radius": 0.0}, "end": {"radius": 0.5}}),
    ] {
        let mut json = serde_json::to_value(RadialGradientPaint::default()).unwrap();
        json["geometry"] = incomplete;
        assert!(serde_json::from_value::<RadialGradientPaint>(json).is_err());
    }
}
