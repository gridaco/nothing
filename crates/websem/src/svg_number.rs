//! Blink-shaped evaluation of an already-lexed SVG number.
//!
//! SVG features own their surrounding grammar and separators. This module
//! owns only the ordered `f32` arithmetic Chromium 149's SVG parser applies
//! after identifying a sign, integer digits, fraction digits, and exponent.

use std::ops::Range;

#[derive(Debug)]
pub(crate) struct NumberParts {
    pub(crate) negative: bool,
    pub(crate) integer_digits: Range<usize>,
    pub(crate) fraction_digits: Option<Range<usize>>,
    pub(crate) exponent: Option<ExponentParts>,
}

#[derive(Debug)]
pub(crate) struct ExponentParts {
    pub(crate) negative: bool,
    pub(crate) digits: Range<usize>,
}

/// Evaluate one lexically valid SVG number in Blink's operation order.
///
/// Integer digits accumulate from least significant to most significant;
/// fraction and exponent digits accumulate from left to right. Every
/// intermediate is `f32`, while the decimal exponent's power uses the same
/// double-precision `pow` conversion Blink reaches before narrowing.
pub(crate) fn evaluate(bytes: &[u8], parts: &NumberParts) -> Option<f32> {
    let integer = evaluate_integer(bytes, &parts.integer_digits)?;
    let fraction = parts
        .fraction_digits
        .as_ref()
        .map_or(0.0_f32, |digits| evaluate_fraction(bytes, digits));

    let unsigned = integer + fraction;
    let sign = if parts.negative { -1.0_f32 } else { 1.0_f32 };
    let mut number = unsigned * sign;

    if let Some(exponent_parts) = &parts.exponent {
        let magnitude = evaluate_exponent(bytes, &exponent_parts.digits);
        if !exponent_parts.negative && magnitude > 38.0_f32 {
            return None;
        }

        let exponent = if exponent_parts.negative {
            -magnitude
        } else {
            magnitude
        };
        if exponent != 0.0_f32 {
            let scale = (10.0_f64).powf(exponent as f64) as f32;
            number *= scale;
        }
    }

    number.is_finite().then_some(number)
}

fn evaluate_integer(bytes: &[u8], digits: &Range<usize>) -> Option<f32> {
    let mut accumulator = 0.0_f32;
    let mut place = 1.0_f32;

    for index in (digits.start..digits.end).rev() {
        let digit = f32::from(bytes[index] - b'0');
        let term = place * digit;
        accumulator += term;
        place *= 10.0_f32;
    }

    accumulator.is_finite().then_some(accumulator)
}

fn evaluate_fraction(bytes: &[u8], digits: &Range<usize>) -> f32 {
    let mut accumulator = 0.0_f32;
    let mut place = 1.0_f32;

    for &byte in &bytes[digits.start..digits.end] {
        place *= 0.1_f32;
        let digit = f32::from(byte - b'0');
        let term = digit * place;
        accumulator += term;
    }

    accumulator
}

fn evaluate_exponent(bytes: &[u8], digits: &Range<usize>) -> f32 {
    let mut accumulator = 0.0_f32;

    for &byte in &bytes[digits.start..digits.end] {
        accumulator *= 10.0_f32;
        accumulator += f32::from(byte - b'0');
    }

    accumulator
}
