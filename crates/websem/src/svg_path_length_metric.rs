/*
 * Copyright 2006 The Android Open Source Project
 * Copyright 2008 The Android Open Source Project
 * Copyright 2015 Google Inc.
 * Copyright 2018 Google LLC
 * Copyright 2025 Google LLC
 * Copyright (c) 2011 Google Inc. All rights reserved.
 *
 * Modified and translated to Rust by n0 contributors from Skia revision
 * 53348aa333da02b77c4b5797e2de722f5abde7d0:
 *   src/core/SkContourMeasure.cpp
 *   src/core/SkGeometry.cpp
 *   src/core/SkPathBuilder.cpp
 *   src/core/SkPoint.cpp
 *   src/core/SkPathRawShapes.cpp
 *   include/core/SkScalar.h
 *   include/private/base/SkFloatingPoint.h
 *
 * This file contains BSD-3-Clause-derived code. The complete license and
 * exact upstream links are in ../NOTICE.md.
 */

//! Blink-compatible local-space lengths for resolved SVG geometry.
//!
//! Float operation order is part of the compatibility contract: SVG
//! `pathLength` scales authored values by this exact used length before Blink
//! hands dashes to Skia.

use math2::Rectangle;
use rframe::{PathCommand, PathData};

pub(super) type Point = [f32; 2];

const MAX_T_VALUE: i32 = 0x3fff_ffff;
const MAX_T_RECIPROCAL: f32 = 1.0 / MAX_T_VALUE as f32;
const CHEAP_DISTANCE_LIMIT: f32 = 0.5;
const MAX_RECURSION_DEPTH: u8 = 8;
pub(super) const QUARTER_CONIC_WEIGHT: f32 = 0.707_106_77;

/// Length of one canonical resolved path, using Blink's pinned Skia metric.
pub(super) fn path_length(path: &PathData) -> f32 {
    commands_length(path.commands().iter().copied())
}

/// Length of the exact rect contour Blink gives to Skia: upper-left, clockwise.
pub(super) fn rect_length(rect: Rectangle) -> f32 {
    let left = rect.x;
    let top = rect.y;
    let right = left + rect.width;
    let bottom = top + rect.height;
    commands_length([
        PathCommand::MoveTo { x: left, y: top },
        PathCommand::LineTo { x: right, y: top },
        PathCommand::LineTo {
            x: right,
            y: bottom,
        },
        PathCommand::LineTo { x: left, y: bottom },
        PathCommand::Close,
    ])
}

/// Length of Skia's four-conic oval, starting at the rightmost point clockwise.
pub(super) fn ellipse_length(rect: Rectangle) -> f32 {
    let left = rect.x;
    let top = rect.y;
    let right = left + rect.width;
    let bottom = top + rect.height;
    // SkRect::centerX/Y use a double intermediate to avoid float overflow.
    let center_x = midpoint(left, right);
    let center_y = midpoint(top, bottom);
    commands_length([
        PathCommand::MoveTo {
            x: right,
            y: center_y,
        },
        PathCommand::ConicTo {
            x1: right,
            y1: bottom,
            x: center_x,
            y: bottom,
            weight: QUARTER_CONIC_WEIGHT,
        },
        PathCommand::ConicTo {
            x1: left,
            y1: bottom,
            x: left,
            y: center_y,
            weight: QUARTER_CONIC_WEIGHT,
        },
        PathCommand::ConicTo {
            x1: left,
            y1: top,
            x: center_x,
            y: top,
            weight: QUARTER_CONIC_WEIGHT,
        },
        PathCommand::ConicTo {
            x1: right,
            y1: top,
            x: right,
            y: center_y,
            weight: QUARTER_CONIC_WEIGHT,
        },
        PathCommand::Close,
    ])
}

fn commands_length(commands: impl IntoIterator<Item = PathCommand>) -> f32 {
    let mut total = 0.0f32;
    let mut contour: Option<Contour> = None;

    for command in commands {
        match command {
            PathCommand::MoveTo { x, y } => {
                finish_contour(&mut total, contour.take());
                contour = Some(Contour::new([x, y]));
            }
            PathCommand::LineTo { x, y } => {
                contour
                    .as_mut()
                    .expect("PathData is validated")
                    .line_to([x, y]);
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                contour
                    .as_mut()
                    .expect("PathData is validated")
                    .quad_to([x1, y1], [x, y]);
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                contour.as_mut().expect("PathData is validated").cubic_to(
                    [x1, y1],
                    [x2, y2],
                    [x, y],
                );
            }
            PathCommand::ConicTo {
                x1,
                y1,
                x,
                y,
                weight,
            } => {
                contour
                    .as_mut()
                    .expect("PathData is validated")
                    .conic_to([x1, y1], [x, y], weight);
            }
            PathCommand::Close => {
                contour.as_mut().expect("PathData is validated").close();
            }
        }
    }
    finish_contour(&mut total, contour);
    total
}

fn finish_contour(total: &mut f32, contour: Option<Contour>) {
    let Some(contour) = contour else {
        return;
    };
    // SkContourMeasureIter skips empty and non-finite contour measures.
    if contour.has_measured_segment && contour.distance.is_finite() {
        *total += contour.distance;
    }
}

struct Contour {
    start: Point,
    current: Point,
    last_measured: Point,
    distance: f32,
    has_measured_segment: bool,
}

impl Contour {
    fn new(start: Point) -> Self {
        Self {
            start,
            current: start,
            last_measured: start,
            distance: 0.0,
            has_measured_segment: false,
        }
    }

    fn line_to(&mut self, end: Point) {
        let before = self.distance;
        self.distance = add_line(self.current, end, self.distance);
        self.record_endpoint_if_advanced(before, end);
        self.current = end;
    }

    fn quad_to(&mut self, control: Point, end: Point) {
        let before = self.distance;
        self.distance = add_quad(
            [self.current, control, end],
            self.distance,
            0,
            MAX_T_VALUE,
            0,
        );
        self.record_endpoint_if_advanced(before, end);
        self.current = end;
    }

    fn cubic_to(&mut self, control_1: Point, control_2: Point, end: Point) {
        let before = self.distance;
        self.distance = add_cubic(
            [self.current, control_1, control_2, end],
            self.distance,
            0,
            MAX_T_VALUE,
            0,
        );
        self.record_endpoint_if_advanced(before, end);
        self.current = end;
    }

    fn conic_to(&mut self, control: Point, end: Point, weight: f32) {
        if weight == 1.0 {
            // SkPathBuilder canonicalizes a unit-weight conic to a quadratic.
            self.quad_to(control, end);
            return;
        }
        let before = self.distance;
        let conic = Conic::new(self.current, control, end, weight);
        self.distance = add_conic(&conic, self.distance, 0, self.current, MAX_T_VALUE, end, 0);
        self.record_endpoint_if_advanced(before, end);
        self.current = end;
    }

    fn close(&mut self) {
        let before = self.distance;
        // SkContourMeasure closes from the last endpoint whose segment changed
        // the accumulated f32 distance.
        self.distance = add_line(self.last_measured, self.start, self.distance);
        self.record_endpoint_if_advanced(before, self.start);
        self.current = self.start;
    }

    fn record_endpoint_if_advanced(&mut self, before: f32, endpoint: Point) {
        if self.distance > before {
            self.last_measured = endpoint;
            self.has_measured_segment = true;
        }
    }
}

fn add_line(start: Point, end: Point, distance: f32) -> f32 {
    distance + point_distance(start, end)
}

fn add_quad(
    points: [Point; 3],
    mut distance: f32,
    min_t: i32,
    max_t: i32,
    recursion_depth: u8,
) -> f32 {
    if recursion_depth < MAX_RECURSION_DEPTH
        && tspan_big_enough(max_t - min_t)
        && quad_too_curvy(points)
    {
        let chopped = chop_quad_at_half(points);
        let half_t = (min_t + max_t) >> 1;
        let next_depth = recursion_depth + 1;
        distance = add_quad(
            [chopped[0], chopped[1], chopped[2]],
            distance,
            min_t,
            half_t,
            next_depth,
        );
        add_quad(
            [chopped[2], chopped[3], chopped[4]],
            distance,
            half_t,
            max_t,
            next_depth,
        )
    } else {
        distance + point_distance(points[0], points[2])
    }
}

fn add_cubic(
    points: [Point; 4],
    mut distance: f32,
    min_t: i32,
    max_t: i32,
    recursion_depth: u8,
) -> f32 {
    if recursion_depth < MAX_RECURSION_DEPTH
        && tspan_big_enough(max_t - min_t)
        && cubic_too_curvy(points)
    {
        let chopped = chop_cubic_at_half(points);
        let half_t = (min_t + max_t) >> 1;
        let next_depth = recursion_depth + 1;
        distance = add_cubic(
            [chopped[0], chopped[1], chopped[2], chopped[3]],
            distance,
            min_t,
            half_t,
            next_depth,
        );
        add_cubic(
            [chopped[3], chopped[4], chopped[5], chopped[6]],
            distance,
            half_t,
            max_t,
            next_depth,
        )
    } else {
        distance + point_distance(points[0], points[3])
    }
}

#[allow(clippy::too_many_arguments)]
fn add_conic(
    conic: &Conic,
    mut distance: f32,
    min_t: i32,
    min_point: Point,
    max_t: i32,
    max_point: Point,
    recursion_depth: u8,
) -> f32 {
    let half_t = (min_t + max_t) >> 1;
    let half_point = conic.eval(t_value_to_scalar(half_t));
    if !point_is_finite(half_point) {
        return distance;
    }
    if recursion_depth < MAX_RECURSION_DEPTH
        && tspan_big_enough(max_t - min_t)
        && conic_too_curvy(min_point, half_point, max_point)
    {
        let next_depth = recursion_depth + 1;
        distance = add_conic(
            conic, distance, min_t, min_point, half_t, half_point, next_depth,
        );
        add_conic(
            conic, distance, half_t, half_point, max_t, max_point, next_depth,
        )
    } else {
        distance + point_distance(min_point, max_point)
    }
}

fn quad_too_curvy(points: [Point; 3]) -> bool {
    let dx = points[1][0] * 0.5 - ((points[0][0] + points[2][0]) * 0.5) * 0.5;
    let dy = points[1][1] * 0.5 - ((points[0][1] + points[2][1]) * 0.5) * 0.5;
    cpp_max(dx.abs(), dy.abs()) > CHEAP_DISTANCE_LIMIT
}

fn cubic_too_curvy(points: [Point; 4]) -> bool {
    cheap_distance_exceeds_limit(
        points[1],
        scalar_interp(points[0][0], points[3][0], 1.0 / 3.0),
        scalar_interp(points[0][1], points[3][1], 1.0 / 3.0),
    ) || cheap_distance_exceeds_limit(
        points[2],
        scalar_interp(points[0][0], points[3][0], 2.0 / 3.0),
        scalar_interp(points[0][1], points[3][1], 2.0 / 3.0),
    )
}

fn cheap_distance_exceeds_limit(point: Point, x: f32, y: f32) -> bool {
    cpp_max((x - point[0]).abs(), (y - point[1]).abs()) > CHEAP_DISTANCE_LIMIT
}

fn conic_too_curvy(first: Point, middle: Point, last: Point) -> bool {
    let middle_ends = [(first[0] + last[0]) * 0.5, (first[1] + last[1]) * 0.5];
    let delta = [middle[0] - middle_ends[0], middle[1] - middle_ends[1]];
    cpp_max(delta[0].abs(), delta[1].abs()) > CHEAP_DISTANCE_LIMIT
}

fn chop_quad_at_half(points: [Point; 3]) -> [Point; 5] {
    let p01 = point_mix_half(points[0], points[1]);
    let p12 = point_mix_half(points[1], points[2]);
    [points[0], p01, point_mix_half(p01, p12), p12, points[2]]
}

fn chop_cubic_at_half(points: [Point; 4]) -> [Point; 7] {
    let ab = point_mix_half(points[0], points[1]);
    let bc = point_mix_half(points[1], points[2]);
    let cd = point_mix_half(points[2], points[3]);
    let abc = point_mix_half(ab, bc);
    let bcd = point_mix_half(bc, cd);
    [
        points[0],
        ab,
        abc,
        point_mix_half(abc, bcd),
        bcd,
        cd,
        points[3],
    ]
}

fn point_mix_half(a: Point, b: Point) -> Point {
    [(b[0] - a[0]) * 0.5 + a[0], (b[1] - a[1]) * 0.5 + a[1]]
}

fn scalar_interp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn point_distance(a: Point, b: Point) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let magnitude_squared = dx * dx + dy * dy;
    if magnitude_squared.is_finite() {
        magnitude_squared.sqrt()
    } else {
        let dx = f64::from(dx);
        let dy = f64::from(dy);
        (dx * dx + dy * dy).sqrt() as f32
    }
}

fn tspan_big_enough(span: i32) -> bool {
    span >> 10 != 0
}

fn t_value_to_scalar(t: i32) -> f32 {
    t as f32 * MAX_T_RECIPROCAL
}

fn midpoint(a: f32, b: f32) -> f32 {
    (0.5f64 * (f64::from(a) + f64::from(b))) as f32
}

fn point_is_finite(point: Point) -> bool {
    point[0].is_finite() && point[1].is_finite()
}

/// `std::max(a, b)` returns `a` when the comparison is unordered, unlike
/// Rust's `f32::max`, which deliberately suppresses a single NaN.
fn cpp_max(a: f32, b: f32) -> f32 {
    if a < b { b } else { a }
}

struct Conic {
    numerator_a: Point,
    numerator_b: Point,
    numerator_c: Point,
    denominator_a: f32,
    denominator_b: f32,
}

impl Conic {
    fn new(start: Point, control: Point, end: Point, weight: f32) -> Self {
        let weighted_control = [control[0] * weight, control[1] * weight];
        let twice_weighted_control = [
            weighted_control[0] + weighted_control[0],
            weighted_control[1] + weighted_control[1],
        ];
        let numerator_a = [
            end[0] - twice_weighted_control[0] + start[0],
            end[1] - twice_weighted_control[1] + start[1],
        ];
        let weighted_delta = [
            weighted_control[0] - start[0],
            weighted_control[1] - start[1],
        ];
        let numerator_b = [
            weighted_delta[0] + weighted_delta[0],
            weighted_delta[1] + weighted_delta[1],
        ];
        let denominator_delta = weight - 1.0;
        let denominator_b = denominator_delta + denominator_delta;
        Self {
            numerator_a,
            numerator_b,
            numerator_c: start,
            denominator_a: 0.0 - denominator_b,
            denominator_b,
        }
    }

    fn eval(&self, t: f32) -> Point {
        let denominator = (self.denominator_a * t + self.denominator_b) * t + 1.0;
        [
            ((self.numerator_a[0] * t + self.numerator_b[0]) * t + self.numerator_c[0])
                / denominator,
            ((self.numerator_a[1] * t + self.numerator_b[1]) * t + self.numerator_c[1])
                / denominator,
        ]
    }
}
