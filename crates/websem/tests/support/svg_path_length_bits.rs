use super::*;
use crate::svg_path_length_metric::{
    Point, QUARTER_CONIC_WEIGHT, ellipse_length, path_length, rect_length,
};
use math2::Rectangle;
use rframe::{FillRule, PathCommand, PathData};
use skia_safe::{Path, PathBuilder, PathDirection, PathMeasure, Rect};
use std::sync::Arc;

#[test]
fn svg_number_accumulation_matches_chromium_149_bits() {
    assert_eq!(
        parse_svg_number("123456789.123456789").unwrap().to_bits(),
        0x4ceb_79a2
    );
    assert_eq!(
        parse_svg_number("1.654435761").unwrap().to_bits(),
        0x3fd3_c48e
    );
    assert_eq!(parse_svg_number("1e-45").unwrap().to_bits(), 1);
    assert_eq!(parse_svg_number("1e-46").unwrap().to_bits(), 0);
    assert_eq!(parse_svg_number("1e-1000").unwrap().to_bits(), 0);
    assert_eq!(parse_svg_number("-0").unwrap().to_bits(), 0x8000_0000);
    assert!(parse_svg_number("340282346638528859811704183484516925440").is_none());
    assert!(parse_svg_number("9999999999999999999999999999999999999999").is_none());
    assert!(parse_svg_number("1e39").is_none());
}

#[test]
fn svg_number_grammar_uses_only_svg_ascii_whitespace() {
    for (raw, expected) in [
        ("0", 0.0),
        ("+12", 12.0),
        ("-.5", -0.5),
        (".25", 0.25),
        ("1.5e2", 150.0),
        ("1.5E+2", 150.0),
        ("1.5e-2", 0.015),
        (" \t\n\r 17 \t, \r\n", 17.0),
        ("\u{c}17\u{c},\u{c}", 17.0),
    ] {
        assert_eq!(parse_svg_number(raw), Some(expected), "{raw:?}");
    }
    for raw in [
        "",
        " \t\n\r",
        ".",
        "1.",
        "+",
        "-",
        "e2",
        "1e",
        "1e+",
        "1e-",
        "1..0",
        "1,,",
        "1, 2",
        ",1",
        "NaN",
        "Infinity",
        "\u{a0}1",
        "1\u{a0}",
        "\u{2003}1",
    ] {
        assert!(parse_svg_number(raw).is_none(), "{raw:?}");
    }
}

#[test]
fn geometry_dispatch_and_dash_scale_cover_blink_branch_matrix() {
    let rect = Rectangle::from_xywh(0.0, 0.0, 10.0, 5.0);
    let positive = Geometry::Rect(rect);
    assert_same_bits(geometry_length(&positive), 30.0);

    let ellipse_rect = Rectangle::from_xywh(3.0, 7.0, 20.0, 12.0);
    let ellipse = Geometry::Ellipse(ellipse_rect);
    assert_same_bits(geometry_length(&ellipse), ellipse_length(ellipse_rect));

    let path = PathData::new(
        vec![
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::LineTo { x: 3.0, y: 4.0 },
        ],
        FillRule::NonZero,
    )
    .unwrap();
    let path = Geometry::Path(Arc::new(path));
    assert_same_bits(geometry_length(&path), 5.0);

    for (authored, expected) in [
        (None, 1.0),
        (Some("-15"), 1.0),
        (Some("15"), 2.0),
        (Some("60"), 0.5),
        (Some("0"), f32::MAX),
        (Some("-0"), f32::MAX),
        (Some(""), f32::MAX),
        (Some("malformed"), f32::MAX),
        (Some("1e-46"), f32::MAX),
        (Some("1e39"), f32::MAX),
    ] {
        assert_same_bits(dash_scale(&positive, authored), expected);
    }

    let zero = Geometry::Rect(Rectangle::from_xywh(4.0, 8.0, 0.0, 0.0));
    for (authored, expected) in [
        (None, 1.0),
        (Some("-1"), 1.0),
        (Some("0"), 0.0),
        (Some("15"), 0.0),
        (Some(""), 0.0),
    ] {
        assert_same_bits(dash_scale(&zero, authored), expected);
    }
}

#[test]
fn dash_scale_is_formed_before_the_dash_member_is_multiplied() {
    let geometry = Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, 10.0, 5.0));
    let author = parse_svg_number("1.654435761").unwrap();
    let member = parse_svg_number("123456789.123456789").unwrap();
    let scale = dash_scale(&geometry, Some("1.654435761"));
    assert_eq!(scale.to_bits(), 0x4191_1087);

    let scaled_after_resolution = member * scale;
    let algebraically_reordered = (member * 30.0) / author;
    assert_eq!(scaled_after_resolution.to_bits(), 0x4f05_6f19);
    assert_eq!(algebraically_reordered.to_bits(), 0x4f05_6f18);
}

#[test]
fn line_close_and_multi_contour_match_pinned_skia_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo { x: 3.25, y: -7.5 },
        PathCommand::LineTo { x: 91.75, y: 13.0 },
        PathCommand::LineTo { x: 91.75, y: 13.0 },
        PathCommand::Close,
        PathCommand::MoveTo { x: -40.0, y: 20.0 },
        PathCommand::LineTo { x: -40.0, y: 20.0 },
        PathCommand::Close,
        PathCommand::MoveTo { x: 1.0, y: 2.0 },
        PathCommand::LineTo { x: 8.0, y: 19.0 },
    ]);
}

#[test]
fn quadratic_matches_pinned_skia_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo { x: -13.25, y: 4.5 },
        PathCommand::QuadTo {
            x1: 113.75,
            y1: -207.5,
            x: 281.125,
            y: 16.25,
        },
    ]);
}

#[test]
fn cubic_matches_pinned_skia_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo { x: 2.0, y: 3.0 },
        PathCommand::CubicTo {
            x1: 200.25,
            y1: -400.5,
            x2: -310.75,
            y2: 511.125,
            x: 123.5,
            y: 29.25,
        },
    ]);
}

#[test]
fn rational_and_unit_weight_conics_match_pinned_skia_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo { x: 80.0, y: 17.0 },
        PathCommand::ConicTo {
            x1: 80.0,
            y1: 113.0,
            x: 11.0,
            y: 113.0,
            weight: QUARTER_CONIC_WEIGHT,
        },
        PathCommand::ConicTo {
            x1: -120.5,
            y1: -41.25,
            x: 17.75,
            y: 9.5,
            weight: 1.0,
        },
    ]);
}

#[test]
fn maximum_depth_curve_matches_pinned_skia_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo {
            x: -1.0e18,
            y: 1.0e18,
        },
        PathCommand::CubicTo {
            x1: 1.0e18,
            y1: -1.0e18,
            x2: -1.0e18,
            y2: -1.0e18,
            x: 1.0e18,
            y: 1.0e18,
        },
    ]);
}

#[test]
fn large_finite_distance_matches_skia_double_fallback_bits() {
    assert_path_matches_skia(vec![
        PathCommand::MoveTo {
            x: -1.0e20,
            y: -1.0e20,
        },
        PathCommand::LineTo {
            x: 1.0e20,
            y: 1.0e20,
        },
    ]);
}

#[test]
fn deterministic_curve_matrix_matches_pinned_skia_bits() {
    let mut state = 0x1495_3348u32;
    let weights = [0.25, QUARTER_CONIC_WEIGHT, 0.875, 1.0, 1.75, 8.0];
    for index in 0..64 {
        let points = std::array::from_fn::<_, 9, _>(|_| random_point(&mut state));
        let mut commands = vec![
            PathCommand::MoveTo {
                x: points[0][0],
                y: points[0][1],
            },
            PathCommand::QuadTo {
                x1: points[1][0],
                y1: points[1][1],
                x: points[2][0],
                y: points[2][1],
            },
            PathCommand::CubicTo {
                x1: points[3][0],
                y1: points[3][1],
                x2: points[4][0],
                y2: points[4][1],
                x: points[5][0],
                y: points[5][1],
            },
            PathCommand::ConicTo {
                x1: points[6][0],
                y1: points[6][1],
                x: points[7][0],
                y: points[7][1],
                weight: weights[index % weights.len()],
            },
            PathCommand::LineTo {
                x: points[8][0],
                y: points[8][1],
            },
        ];
        if index % 2 == 0 {
            commands.push(PathCommand::Close);
        }
        assert_path_matches_skia(commands);
    }
}

#[test]
fn rect_helper_matches_skia_raw_shape_bits() {
    for rect in [
        Rectangle::from_xywh(13.25, -7.75, 201.5, 89.125),
        Rectangle::from_xywh(-4.0, 8.0, 0.0, 31.5),
    ] {
        let mut builder = PathBuilder::new();
        builder.add_rect(
            Rect::from_xywh(rect.x, rect.y, rect.width, rect.height),
            Some(PathDirection::CW),
            Some(0),
        );
        assert_same_bits(rect_length(rect), skia_length(&builder.detach()));
    }
}

#[test]
fn ellipse_helper_matches_skia_raw_shape_bits() {
    for rect in [
        Rectangle::from_xywh(13.25, -7.75, 201.5, 89.125),
        Rectangle::from_xywh(-4.0, 8.0, 0.0, 31.5),
        Rectangle::from_xywh(16_777_216.0, -16_777_216.0, 13.0, 9.0),
    ] {
        let mut builder = PathBuilder::new();
        builder.add_oval(
            Rect::from_xywh(rect.x, rect.y, rect.width, rect.height),
            Some(PathDirection::CW),
            Some(1),
        );
        assert_same_bits(ellipse_length(rect), skia_length(&builder.detach()));
    }
}

#[test]
fn ellipse_helper_matches_chromium_149_dom_metric_bits_at_same_positions() {
    // Skia's f32 conic evaluation is translation-sensitive, so a DOM metric
    // pin is meaningful only when the helper receives the identical bounds.
    for (label, rect, expected_bits) in [
        (
            "circle-r12-at-40-30",
            Rectangle::from_xywh(28.0, 18.0, 24.0, 24.0),
            0x4295_d321,
        ),
        (
            "ellipse-16-10-at-80-30",
            Rectangle::from_xywh(64.0, 20.0, 32.0, 20.0),
            0x42a4_7d93,
        ),
        (
            "ellipse-24-12-at-140-30",
            Rectangle::from_xywh(116.0, 18.0, 48.0, 24.0),
            0x42e7_0f2c,
        ),
    ] {
        assert_eq!(ellipse_length(rect).to_bits(), expected_bits, "{label}");
    }
}

fn assert_path_matches_skia(commands: Vec<PathCommand>) {
    let path = PathData::new(commands.clone(), FillRule::NonZero).unwrap();
    let mut builder = PathBuilder::new();
    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } => {
                builder.move_to((x, y));
            }
            PathCommand::LineTo { x, y } => {
                builder.line_to((x, y));
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                builder.quad_to((x1, y1), (x, y));
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                builder.cubic_to((x1, y1), (x2, y2), (x, y));
            }
            PathCommand::ConicTo {
                x1,
                y1,
                x,
                y,
                weight,
            } => {
                builder.conic_to((x1, y1), (x, y), weight);
            }
            PathCommand::Close => {
                builder.close();
            }
        }
    }
    assert_same_bits(path_length(&path), skia_length(&builder.detach()));
}

fn skia_length(path: &Path) -> f32 {
    let mut measure = PathMeasure::new(path, false, None);
    let mut total = measure.length();
    while measure.next_contour() {
        total += measure.length();
    }
    total
}

fn assert_same_bits(actual: f32, expected: f32) {
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "actual {actual:?} ({:#010x}), Skia {expected:?} ({:#010x})",
        actual.to_bits(),
        expected.to_bits(),
    );
}

fn random_point(state: &mut u32) -> Point {
    [random_coordinate(state), random_coordinate(state)]
}

fn random_coordinate(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let centered = i64::from(*state >> 8) - (1_i64 << 23);
    centered as f32 / 137.0
}
