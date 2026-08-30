//! One complete SVG `<number>` list with Blink-shaped separators.
//!
//! This parser owns only a list of numbers: no commands, flags, units, CSS
//! tokens, or error recovery. A syntax error clears the whole list. Number
//! arithmetic delegates to [`crate::svg_number`], so every SVG consumer that
//! uses this list keeps Chromium's ordered `f32` evaluation.

use crate::svg_number::{self, ExponentParts, NumberParts};

/// Parse a complete SVG number list.
///
/// Blink consumes SVG whitespace, then at most one comma, then whitespace
/// after each number. A lone trailing comma is therefore accepted. Adjacent
/// signs and decimal points can start the next number without a separator.
/// Any other residual syntax invalidates the complete list.
pub(crate) fn parse(raw: &str) -> Option<Vec<f32>> {
    if !raw.is_ascii() {
        return None;
    }

    let mut parser = Parser {
        bytes: raw.as_bytes(),
        at: 0,
    };
    parser.skip_whitespace();
    let mut values = Vec::new();
    while parser.at < parser.bytes.len() {
        values.push(parser.number()?);
        parser.skip_separator();
    }
    Some(values)
}

/// Parse exactly one SVG number with no list separator.
///
/// Consumers whose attribute grammar is a scalar use the same ordered
/// arithmetic as a number list without inheriting that list's measured lone
/// trailing-comma recovery.
pub(crate) fn parse_one(raw: &str) -> Option<f32> {
    if !raw.is_ascii() {
        return None;
    }

    let mut parser = Parser {
        bytes: raw.as_bytes(),
        at: 0,
    };
    parser.skip_whitespace();
    let value = parser.number()?;
    parser.skip_whitespace();
    (parser.at == parser.bytes.len()).then_some(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn number(&mut self) -> Option<f32> {
        let start = self.at;
        let negative = match self.bytes.get(self.at) {
            Some(b'+') => {
                self.at += 1;
                false
            }
            Some(b'-') => {
                self.at += 1;
                true
            }
            _ => false,
        };

        let integer_start = self.at;
        self.digits();
        let integer_digits = integer_start..self.at;

        let fraction_digits = if self.bytes.get(self.at) == Some(&b'.') {
            self.at += 1;
            let fraction_start = self.at;
            self.digits();
            if self.at == fraction_start {
                self.at = start;
                return None;
            }
            Some(fraction_start..self.at)
        } else {
            None
        };
        if integer_digits.is_empty() && fraction_digits.is_none() {
            self.at = start;
            return None;
        }

        let exponent = if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            let exponent_at = self.at;
            self.at += 1;
            let exponent_negative = match self.bytes.get(self.at) {
                Some(b'+') => {
                    self.at += 1;
                    false
                }
                Some(b'-') => {
                    self.at += 1;
                    true
                }
                _ => false,
            };
            let exponent_start = self.at;
            self.digits();
            if self.at == exponent_start {
                self.at = exponent_at;
                return None;
            }
            Some(ExponentParts {
                negative: exponent_negative,
                digits: exponent_start..self.at,
            })
        } else {
            None
        };

        let value = svg_number::evaluate(
            self.bytes,
            &NumberParts {
                negative,
                integer_digits,
                fraction_digits,
                exponent,
            },
        )?;
        Some(value)
    }

    fn digits(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c))
        {
            self.at += 1;
        }
    }

    fn skip_separator(&mut self) {
        self.skip_whitespace();
        if self.bytes.get(self.at) == Some(&b',') {
            self.at += 1;
            self.skip_whitespace();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_one};

    #[test]
    fn measured_separators_and_adjacent_numbers_are_preserved() {
        assert_eq!(parse(" +1,.5 -2e1, "), Some(vec![1.0, 0.5, -20.0]));
        assert_eq!(parse("1-2.5.25"), Some(vec![1.0, -2.5, 0.25]));
        assert_eq!(parse(""), Some(Vec::new()));
        assert_eq!(parse(" \t\n\r\x0c"), Some(Vec::new()));
    }

    #[test]
    fn one_trailing_comma_is_not_general_error_recovery() {
        assert_eq!(parse("1,"), Some(vec![1.0]));
        for raw in [",1", "1,,2", "1px", "1%", "1/*x*/ 2", "1.", "1e+"] {
            assert_eq!(parse(raw), None, "{raw}");
        }
    }

    #[test]
    fn evaluation_uses_blink_order_and_rejects_positive_overflow() {
        assert_eq!(
            parse("1.000000059604644775390625000000000000000000000001"),
            Some(vec![1.000_000_1])
        );
        assert_eq!(parse("1e999"), None);
        assert_eq!(parse("-1e-999"), Some(vec![-0.0]));
        assert_eq!(parse("1\u{00a0}2"), None);
    }

    #[test]
    fn scalar_grammar_shares_evaluation_without_list_recovery() {
        assert_eq!(parse_one(" \t+.5e1\n"), Some(5.0));
        assert_eq!(
            parse_one("0.057384267578125007").map(f32::to_bits),
            Some(0x3d6b_0bc6)
        );
        for raw in ["", "1,", "1 2", "1%", "1.", "1e+", "1\u{00a0}"] {
            assert_eq!(parse_one(raw), None, "{raw:?}");
        }
    }
}
