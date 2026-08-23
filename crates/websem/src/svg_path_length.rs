//! SVG `pathLength` source semantics for dash calibration.
//!
//! This module owns the attribute-value grammar, invalid-value behavior, scale
//! dispatch, and the decision to apply one local-space factor to resolved dash
//! facts. Document parsing remains in `csscascade`; the Skia-derived geometry
//! metric is isolated in the private `svg_path_length_metric` sibling.

use crate::svg_number::{self, ExponentParts, NumberParts};
use crate::svg_path_length_metric;
use rframe::Geometry;

/// Blink's f32 scale applied to resolved dash members and phase when a
/// `pathLength` attribute participates. Absence and valid negative values do
/// not participate; malformed present values reset the SVGNumber to zero.
pub(crate) fn dash_scale(geometry: &Geometry, authored: Option<&str>) -> f32 {
    let Some(author_length) = authored_path_length(authored) else {
        return 1.0;
    };
    let computed_length = geometry_length(geometry);
    if computed_length == 0.0 {
        return 0.0;
    }
    let scale = computed_length / author_length.abs();
    if scale > f32::MAX { f32::MAX } else { scale }
}

fn geometry_length(geometry: &Geometry) -> f32 {
    match geometry {
        Geometry::Rect(rect) => svg_path_length_metric::rect_length(*rect),
        Geometry::Ellipse(rect) => svg_path_length_metric::ellipse_length(*rect),
        Geometry::Path(path) => svg_path_length_metric::path_length(path),
    }
}

/// `None` means that Blink leaves dash values unchanged. A present syntax
/// error is different: SVGNumber falls back to zero, which participates in
/// scaling.
fn authored_path_length(authored: Option<&str>) -> Option<f32> {
    let raw = authored?;
    match parse_svg_number(raw) {
        Some(value) if value < 0.0 => None,
        Some(value) => Some(value),
        None => Some(0.0),
    }
}

/// Parse the SVG-number subset used by `pathLength`.
///
/// The grammar is recognized before any arithmetic is performed. This keeps
/// syntax decisions independent from the deliberately ordered `f32`
/// evaluation below, and keeps this feature-local parser from becoming a
/// second general SVG tokenizer.
fn parse_svg_number(raw: &str) -> Option<f32> {
    let parts = lex_number(raw)?;
    evaluate_number(raw.as_bytes(), &parts)
}

fn lex_number(raw: &str) -> Option<NumberParts> {
    if !raw.is_ascii() {
        return None;
    }

    let bytes = raw.as_bytes();
    let mut cursor = skip_whitespace(bytes, 0);

    let negative = match bytes.get(cursor) {
        Some(b'+') => {
            cursor += 1;
            false
        }
        Some(b'-') => {
            cursor += 1;
            true
        }
        _ => false,
    };

    let integer_start = cursor;
    cursor = skip_digits(bytes, cursor);
    let integer_digits = integer_start..cursor;

    let fraction_digits = if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let fraction_start = cursor;
        cursor = skip_digits(bytes, cursor);
        if cursor == fraction_start {
            return None;
        }
        Some(fraction_start..cursor)
    } else {
        None
    };

    if integer_digits.is_empty() && fraction_digits.is_none() {
        return None;
    }

    let exponent = if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        cursor += 1;
        let exponent_negative = match bytes.get(cursor) {
            Some(b'+') => {
                cursor += 1;
                false
            }
            Some(b'-') => {
                cursor += 1;
                true
            }
            _ => false,
        };

        let exponent_start = cursor;
        cursor = skip_digits(bytes, cursor);
        if cursor == exponent_start {
            return None;
        }

        Some(ExponentParts {
            negative: exponent_negative,
            digits: exponent_start..cursor,
        })
    } else {
        None
    };

    cursor = skip_whitespace(bytes, cursor);
    if bytes.get(cursor) == Some(&b',') {
        cursor += 1;
        cursor = skip_whitespace(bytes, cursor);
    }
    if cursor != bytes.len() {
        return None;
    }

    Some(NumberParts {
        negative,
        integer_digits,
        fraction_digits,
        exponent,
    })
}

fn skip_digits(bytes: &[u8], mut cursor: usize) -> usize {
    while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
        cursor += 1;
    }
    cursor
}

fn skip_whitespace(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| is_svg_whitespace(*byte))
    {
        cursor += 1;
    }
    cursor
}

fn is_svg_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

fn evaluate_number(bytes: &[u8], parts: &NumberParts) -> Option<f32> {
    svg_number::evaluate(bytes, parts)
}

#[cfg(test)]
#[path = "../tests/support/svg_path_length_bits.rs"]
mod tests;
