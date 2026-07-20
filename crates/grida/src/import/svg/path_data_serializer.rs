//! SVG path-data serialization for the import IR.
//!
//! The legacy importer originally round-tripped tiny-skia segments through a
//! Skia path solely to obtain `SkParsePath::ToSVGString` output. This module
//! owns that representation contract without involving a graphics backend.

use usvg::tiny_skia_path::{Path, PathSegment, Point};

struct ScientificBuffer {
    bytes: [u8; 16],
    len: usize,
}

impl ScientificBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 16],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("Rust float formatting emits ASCII")
    }
}

impl std::fmt::Write for ScientificBuffer {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(std::fmt::Error)?;
        let destination = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        destination.copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// Serialize tiny-skia path segments with the legacy importer's exact path-data
/// spelling. A present `offset` translates every point before iteration.
pub(super) fn serialize(path: &Path, offset: Option<(f32, f32)>) -> String {
    // Skia treats a zero translation as the identity, without performing
    // additions that could erase signed zeroes.
    let offset = offset.filter(|(x, y)| *x != 0.0 || *y != 0.0);
    let mut output = String::new();
    let mut segments = path.segments().peekable();
    let mut contour_start = Point::zero();
    let mut last_point = Point::zero();
    let mut svg_origin = Point::zero();

    while let Some(segment) = segments.next() {
        match segment {
            PathSegment::MoveTo(point) => {
                // Skia's iterator does not expose a terminal move-only contour.
                if segments.peek().is_none() {
                    break;
                }
                let point = map_point(point, offset);
                contour_start = point;
                last_point = point;
                append_command(&mut output, 'M', &[point], &mut svg_origin);
            }
            PathSegment::LineTo(point) => {
                let point = map_point(point, offset);
                last_point = point;
                append_command(&mut output, 'L', &[point], &mut svg_origin);
            }
            PathSegment::QuadTo(control, point) => {
                let control = map_point(control, offset);
                let point = map_point(point, offset);
                last_point = point;
                append_command(&mut output, 'Q', &[control, point], &mut svg_origin);
            }
            PathSegment::CubicTo(control_1, control_2, point) => {
                let control_1 = map_point(control_1, offset);
                let control_2 = map_point(control_2, offset);
                let point = map_point(point, offset);
                last_point = point;
                append_command(
                    &mut output,
                    'C',
                    &[control_1, control_2, point],
                    &mut svg_origin,
                );
            }
            PathSegment::Close => {
                // Skia compares the already-translated points. Translation can
                // round distinct source points to the same f32, in which case
                // its iterator suppresses the otherwise implicit closing line.
                if last_point != contour_start
                    && !point_has_nan(last_point)
                    && !point_has_nan(contour_start)
                {
                    append_command(&mut output, 'L', &[contour_start], &mut svg_origin);
                }
                last_point = contour_start;
                output.push('Z');
            }
        }
    }

    output
}

fn map_point(point: Point, offset: Option<(f32, f32)>) -> Point {
    match offset {
        Some((x, y)) => Point::from_xy(point.x + x, point.y + y),
        None => point,
    }
}

fn point_has_nan(point: Point) -> bool {
    point.x.is_nan() || point.y.is_nan()
}

fn append_command(output: &mut String, command: char, points: &[Point], svg_origin: &mut Point) {
    output.push(command);
    let mut separator = "";
    for point in points {
        output.push_str(separator);
        append_scalar(output, point.x - svg_origin.x);
        output.push(' ');
        append_scalar(output, point.y - svg_origin.y);
        separator = " ";
    }

    let point = points.last().expect("path command always has a point");
    // Absolute Skia path-data encoding retains a signed-zero origin derived
    // from the preceding endpoint. Reproduce that observable spelling.
    *svg_origin = Point::from_xy(point.x * 0.0, point.y * 0.0);
}

/// Append Skia's scalar text spelling (`snprintf("%.8g", value)`) without
/// depending on its C++ utility layer.
fn append_scalar(output: &mut String, value: f32) {
    if value.is_nan() {
        output.push_str("nan");
        return;
    }
    if value.is_infinite() {
        if value.is_sign_negative() {
            output.push('-');
        }
        output.push_str("inf");
        return;
    }

    let negative = value.is_sign_negative();
    let mut rounded = ScientificBuffer::new();
    std::fmt::write(&mut rounded, format_args!("{:.7e}", value.abs()))
        .expect("eight-digit f32 scientific spelling fits the stack buffer");
    let (coefficient, exponent) = rounded
        .as_str()
        .split_once('e')
        .expect("Rust lower-exponential formatting always includes an exponent");
    let exponent: i32 = exponent
        .parse()
        .expect("Rust lower-exponential formatting emits an integer exponent");
    let mut digits = [0_u8; 8];
    let mut digits_len = 0;
    for byte in coefficient.bytes().filter(|&byte| byte != b'.') {
        digits[digits_len] = byte;
        digits_len += 1;
    }
    debug_assert_eq!(digits_len, digits.len());

    if negative {
        output.push('-');
    }

    if (-4..8).contains(&exponent) {
        append_fixed(output, &digits, exponent);
    } else {
        append_scientific(output, &digits, exponent);
    }
}

fn append_fixed(output: &mut String, digits: &[u8; 8], exponent: i32) {
    let decimal_index = exponent + 1;
    if decimal_index <= 0 {
        output.push_str("0.");
        for _ in decimal_index..0 {
            output.push('0');
        }
        output.push_str(digits_str(digits));
        trim_fraction(output);
    } else if decimal_index as usize >= digits.len() {
        output.push_str(digits_str(digits));
        for _ in digits.len()..decimal_index as usize {
            output.push('0');
        }
    } else {
        let decimal_index = decimal_index as usize;
        output.push_str(digits_str(&digits[..decimal_index]));
        output.push('.');
        output.push_str(digits_str(&digits[decimal_index..]));
        trim_fraction(output);
    }
}

fn append_scientific(output: &mut String, digits: &[u8; 8], exponent: i32) {
    output.push(digits[0] as char);
    let mut fraction_end = digits.len();
    while fraction_end > 1 && digits[fraction_end - 1] == b'0' {
        fraction_end -= 1;
    }
    if fraction_end > 1 {
        output.push('.');
        output.push_str(digits_str(&digits[1..fraction_end]));
    }
    output.push('e');
    if exponent < 0 {
        output.push('-');
    } else {
        output.push('+');
    }
    let magnitude = exponent.unsigned_abs();
    debug_assert!(magnitude < 100);
    if magnitude < 10 {
        output.push('0');
    } else {
        output.push((b'0' + (magnitude / 10) as u8) as char);
    }
    output.push((b'0' + (magnitude % 10) as u8) as char);
}

fn digits_str(digits: &[u8]) -> &str {
    std::str::from_utf8(digits).expect("scalar digits are ASCII")
}

fn trim_fraction(output: &mut String) {
    while output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{PathBuilder as SkPathBuilder, Point as SkPoint};
    use usvg::tiny_skia_path::PathBuilder;

    fn skia_legacy_serialize(path: &Path, offset: Option<(f32, f32)>) -> String {
        let mut builder = SkPathBuilder::new();
        for segment in path.segments() {
            match segment {
                PathSegment::MoveTo(point) => {
                    builder.move_to((point.x, point.y));
                }
                PathSegment::LineTo(point) => {
                    builder.line_to((point.x, point.y));
                }
                PathSegment::QuadTo(control, point) => {
                    builder.quad_to((control.x, control.y), (point.x, point.y));
                }
                PathSegment::CubicTo(control_1, control_2, point) => {
                    builder.cubic_to(
                        (control_1.x, control_1.y),
                        (control_2.x, control_2.y),
                        (point.x, point.y),
                    );
                }
                PathSegment::Close => {
                    builder.close();
                }
            }
        }
        let path = builder.detach();
        match offset {
            Some(offset) => path.make_offset(offset).to_svg(),
            None => path.to_svg(),
        }
    }

    fn format_scalar(value: f32) -> String {
        let mut output = String::new();
        append_scalar(&mut output, value);
        output
    }

    fn skia_format_scalar(value: f32) -> String {
        let mut builder = SkPathBuilder::new();
        builder.move_to(SkPoint::new(value, 0.0));
        builder.line_to(SkPoint::new(0.0, 1.0));
        let path_data = builder.detach().to_svg();
        path_data[1..path_data.find(' ').expect("move x/y separator")].to_owned()
    }

    #[test]
    fn path_data_serializer_spells_commands_and_close_like_skia() {
        let mut builder = PathBuilder::new();
        builder.move_to(1.25, -2.5);
        builder.line_to(3.0, 4.0);
        builder.quad_to(5.0, 6.0, 7.0, 8.0);
        builder.cubic_to(9.0, 10.0, 11.0, 12.0, 13.0, 14.0);
        builder.close();
        let path = builder.finish().unwrap();

        assert_eq!(
            serialize(&path, None),
            "M1.25 -2.5L3 4Q5 6 7 8C9 10 11 12 13 14L1.25 -2.5Z"
        );
    }

    #[test]
    fn path_data_serializer_handles_contours_offsets_and_terminal_moves() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(1.0, 0.0);
        builder.close();
        builder.move_to(2.0, 2.0);
        builder.line_to(3.0, 3.0);
        builder.move_to(4.0, 4.0);
        let path = builder.finish().unwrap();

        assert_eq!(
            serialize(&path, Some((-0.25, 1.5))),
            "M-0.25 1.5L0.75 1.5L-0.25 1.5ZM1.75 3.5L2.75 4.5"
        );
        assert_eq!(
            serialize(&path, Some((-0.25, 1.5))),
            skia_legacy_serialize(&path, Some((-0.25, 1.5)))
        );
    }

    #[test]
    fn path_data_serializer_does_not_duplicate_an_existing_close_line() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(1.0, 0.0);
        builder.line_to(0.0, 0.0);
        builder.close();
        let path = builder.finish().unwrap();

        assert_eq!(serialize(&path, None), "M0 0L1 0L0 0Z");
    }

    #[test]
    fn path_data_serializer_compares_close_points_after_offset_rounding() {
        let mut builder = PathBuilder::new();
        builder.move_to(-1.0e30, 0.0);
        builder.line_to(-1.0e30 + 1.0e24, 1.0);
        builder.move_to(1.0, 0.0);
        builder.line_to(2.0, 0.0);
        builder.close();
        let path = builder.finish().unwrap();
        let offset = Some((1.0e30, 0.0));

        let actual = serialize(&path, offset);
        assert_eq!(actual, skia_legacy_serialize(&path, offset));
        assert!(actual.ends_with("M1e+30 0L1e+30 0Z"));
        assert!(!actual.ends_with("L1e+30 0L1e+30 0Z"));
    }

    #[test]
    fn path_data_serializer_preserves_skia_signed_zero_context() {
        let mut builder = PathBuilder::new();
        builder.move_to(-1.0, -1.0);
        builder.line_to(-0.0, -0.0);
        builder.line_to(0.0, 0.0);
        let path = builder.finish().unwrap();

        assert_eq!(serialize(&path, None), skia_legacy_serialize(&path, None));
        assert_eq!(serialize(&path, None), "M-1 -1L0 0L0 0");

        let mut builder = PathBuilder::new();
        builder.move_to(-0.0, -0.0);
        builder.line_to(1.0, 1.0);
        let path = builder.finish().unwrap();
        assert_eq!(serialize(&path, None), "M-0 -0L1 1");
        assert_eq!(serialize(&path, None), skia_legacy_serialize(&path, None));
        assert_eq!(
            serialize(&path, Some((0.0, -0.0))),
            skia_legacy_serialize(&path, Some((0.0, -0.0)))
        );
    }

    #[test]
    fn path_data_serializer_keeps_zero_contours_and_post_close_moves() {
        let mut builder = PathBuilder::new();
        builder.move_to(2.0, 3.0);
        builder.close();
        builder.line_to(4.0, 5.0);
        let path = builder.finish().unwrap();

        assert_eq!(serialize(&path, None), "M2 3ZM2 3L4 5");
        assert_eq!(serialize(&path, None), skia_legacy_serialize(&path, None));
    }

    #[test]
    fn path_data_serializer_matches_skia_scalar_spelling() {
        let finite_cases = [
            (0x0000_0000, "0"),
            (0x8000_0000, "-0"),
            (0x0000_0001, "1.4012985e-45"),
            (0x007f_ffff, "1.1754942e-38"),
            (0x0080_0000, "1.1754944e-38"),
            (0x38d1_b717, "9.9999997e-05"),
            (0x38d1_b718, "0.0001"),
            (0x4cbe_bc1f, "99999992"),
            (0x4cbe_bc20, "1e+08"),
            (0x7f7f_ffff, "3.4028235e+38"),
            (0xff7f_ffff, "-3.4028235e+38"),
        ];

        for (bits, expected) in finite_cases {
            let value = f32::from_bits(bits);
            assert_eq!(format_scalar(value), expected, "scalar bits {bits:08x}");
            assert_eq!(
                format_scalar(value),
                skia_format_scalar(value),
                "scalar bits {bits:08x}"
            );
        }

        assert_eq!(format_scalar(f32::INFINITY), "inf");
        assert_eq!(format_scalar(f32::NEG_INFINITY), "-inf");
        assert_eq!(format_scalar(f32::NAN), "nan");
        assert_eq!(format_scalar(f32::from_bits(0xffc0_0001)), "nan");
    }

    #[test]
    fn path_data_serializer_matches_skia_for_stratified_f32_bits() {
        let mut bits = 0x243f_6a88_u32;
        for _ in 0..65_536 {
            bits = bits.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let value = f32::from_bits(bits);
            if !value.is_finite() {
                continue;
            }
            assert_eq!(
                format_scalar(value),
                skia_format_scalar(value),
                "scalar bits {bits:08x}"
            );
        }
    }

    #[test]
    fn path_data_serializer_matches_skia_for_segment_sequences() {
        let mut state = 0x1319_8a2e_u32;
        let mut coordinate = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state as i32) as f32 / 131_072.0
        };

        for index in 0..128 {
            let mut builder = PathBuilder::new();
            builder.move_to(coordinate(), coordinate());
            builder.line_to(coordinate(), coordinate());
            builder.quad_to(coordinate(), coordinate(), coordinate(), coordinate());
            builder.cubic_to(
                coordinate(),
                coordinate(),
                coordinate(),
                coordinate(),
                coordinate(),
                coordinate(),
            );
            if index % 2 == 0 {
                builder.close();
            }
            builder.move_to(coordinate(), coordinate());
            builder.line_to(coordinate(), coordinate());
            if index % 3 == 0 {
                builder.move_to(coordinate(), coordinate());
            }
            let path = builder.finish().unwrap();
            let offset = Some((coordinate() / 17.0, coordinate() / 31.0));

            assert_eq!(
                serialize(&path, offset),
                skia_legacy_serialize(&path, offset),
                "path sequence {index}"
            );
        }
    }
}
