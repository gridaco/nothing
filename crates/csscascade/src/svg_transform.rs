//! The SVG `transform` attribute grammar, rewritten into CSS `transform`
//! text for presentation-hint injection.
//!
//! SVG 2 and CSS Transforms L1 §7 define the `transform` attribute as a
//! presentation attribute of the one CSS `transform` property, with its own
//! backwards-compatible grammar: unitless numbers, comma-wsp separators, and
//! a three-argument `rotate(a cx cy)` that the CSS grammar cannot spell.
//! This module owns that grammar boundary: a valid attribute list becomes
//! equivalent CSS text (§7.3's unit assignment — `px` on translations, `deg`
//! on angles, the 3-argument rotate expanded to its defining
//! translate·rotate·translate sandwich) and enters the cascade through the
//! same hint path as every other admitted presentation attribute; an invalid
//! list becomes no hint at all, which renders the element untransformed —
//! exactly Chromium's behavior for every measured malformed list.
//!
//! The accepted-vs-dropped boundary is measured, not assumed (the transform
//! rung's probe matrix, Chromium 149): arity is exact per function, one comma
//! at most separates arguments or functions, a leading, doubled, or trailing
//! comma invalidates the whole list, `10.` is not a number, units and
//! `!important` invalidate, and function names are case-sensitive. Two
//! lenient facts are equally measured and equally binding, because refusing
//! them would render untransformed where Chromium transforms: **numbers may
//! run together** wherever the next token is self-delimiting
//! (`translate(10-10)` is (10, −10); `10.5.5` is 10.5 then .5 — the
//! csswg-drafts#2623 posture no browser ever tightened), and **two functions
//! need no separator at all** (`translate(10 10)scale(2)` composes).
//!
//! The serialization is exact: every number round-trips through `f32` and
//! Rust's shortest-round-trip `Display`, so the value Stylo computes from the
//! rewritten text is the value the attribute authored.

/// Rewrite an SVG `transform` attribute value into CSS `transform` text.
///
/// `Some(css)` is a non-empty, Stylo-parseable CSS transform list; `None`
/// means the attribute contributes no hint — either an empty list (identity,
/// same as an absent attribute) or a malformed one (dropped whole, exactly as
/// Chromium drops it; a valid prefix contributes nothing).
pub(crate) fn transform_attribute_to_css(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut css = String::new();
    let mut index = 0usize;
    let mut functions = 0usize;
    // At most one comma separates two functions, and a comma is a
    // separator, not a terminator: consuming one obliges another function.
    let mut pending_comma = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if is_svg_whitespace(byte) {
            index += 1;
            continue;
        }
        if byte == b',' {
            if functions == 0 || pending_comma {
                return None;
            }
            pending_comma = true;
            index += 1;
            continue;
        }
        let name_start = index;
        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        if name_start == index {
            return None;
        }
        let name = &value[name_start..index];
        while index < bytes.len() && is_svg_whitespace(bytes[index]) {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'(' {
            return None;
        }
        index += 1;
        let args_start = index;
        while index < bytes.len() && bytes[index] != b')' {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let args = parse_arguments(&value[args_start..index])?;
        index += 1;
        pending_comma = false;
        functions += 1;

        if !css.is_empty() {
            css.push(' ');
        }
        use std::fmt::Write;
        match (name, args.as_slice()) {
            ("matrix", [a, b, c, d, e, f]) => {
                write!(css, "matrix({a}, {b}, {c}, {d}, {e}, {f})").unwrap();
            }
            ("translate", [tx]) => write!(css, "translate({tx}px)").unwrap(),
            ("translate", [tx, ty]) => write!(css, "translate({tx}px, {ty}px)").unwrap(),
            ("scale", [s]) => write!(css, "scale({s})").unwrap(),
            ("scale", [sx, sy]) => write!(css, "scale({sx}, {sy})").unwrap(),
            ("rotate", [a]) => write!(css, "rotate({a}deg)").unwrap(),
            // CSS Transforms L1 §7.3: "equivalent to an initial translation
            // by cx, cy, a rotation by a, followed by a translation by
            // −cx, −cy". The expansion is the mapping, not a paraphrase.
            ("rotate", [a, cx, cy]) => {
                let (ncx, ncy) = (-cx, -cy);
                write!(
                    css,
                    "translate({cx}px, {cy}px) rotate({a}deg) translate({ncx}px, {ncy}px)"
                )
                .unwrap();
            }
            ("skewX", [a]) => write!(css, "skewX({a}deg)").unwrap(),
            ("skewY", [a]) => write!(css, "skewY({a}deg)").unwrap(),
            _ => return None,
        }
    }
    if pending_comma {
        return None;
    }
    (functions > 0).then_some(css)
}

/// Parse one function's argument text into numbers.
///
/// Between two numbers the separator is comma-wsp — whitespace and/or at
/// most one comma, never leading, doubled, or trailing — **or nothing**,
/// where the next number announces itself with a sign or a dot (the
/// measured run-together leniency). Splitting on separators and skipping
/// empty tokens would silently accept the comma shapes Chromium rejects, so
/// an empty comma group is a hard error here, not a skip.
fn parse_arguments(args: &str) -> Option<Vec<f32>> {
    let bytes = args.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0usize;
    let mut pending_comma = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if is_svg_whitespace(byte) {
            index += 1;
            continue;
        }
        if byte == b',' {
            if numbers.is_empty() || pending_comma {
                return None;
            }
            pending_comma = true;
            index += 1;
            continue;
        }
        let (number, next) = scan_number(bytes, index)?;
        numbers.push(number);
        pending_comma = false;
        index = next;
    }
    if pending_comma {
        return None;
    }
    Some(numbers)
}

/// Scan one SVG `<number-token>` at `start`: `sign? (digits ('.' digits)? |
/// '.' digits) (('e'|'E') sign? digits)?` — a dot carries digits, so `10.`
/// is not a number and invalidates the list (measured: Chromium's transform
/// scanner drops it, unlike its path-data scanner). The scan is maximal, so
/// where one number ends, only whitespace, a comma, a `)`, a sign, or a dot
/// can follow — which is exactly what makes run-together lists unambiguous.
fn scan_number(bytes: &[u8], start: usize) -> Option<(f32, usize)> {
    let mut index = start;
    if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
        index += 1;
    }
    let int_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let has_int = index > int_start;
    let mut has_fraction = false;
    if index < bytes.len() && bytes[index] == b'.' {
        let fraction_start = index + 1;
        let mut cursor = fraction_start;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == fraction_start {
            return None;
        }
        has_fraction = true;
        index = cursor;
    }
    if !has_int && !has_fraction {
        return None;
    }
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        let mut cursor = index + 1;
        if cursor < bytes.len() && matches!(bytes[cursor], b'+' | b'-') {
            cursor += 1;
        }
        let exponent_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == exponent_start {
            return None;
        }
        index = cursor;
    }
    let text = std::str::from_utf8(&bytes[start..index]).ok()?;
    let number = text.parse::<f32>().ok()?;
    // Overflow is invalidity, not clamping: Chromium drops
    // `translate(1e999)` (measured), and a non-finite number could not
    // serialize back into CSS anyway.
    number.is_finite().then_some((number, index))
}

/// The five ASCII characters the SVG grammar calls whitespace.
const fn is_svg_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0C)
}

#[cfg(test)]
mod tests {
    use super::transform_attribute_to_css as rewrite;

    /// The 23 malformed lists the compiler refused before drop-semantics,
    /// re-baked for the rung: Chromium drops every one (probes b01–b23),
    /// so every one contributes no hint.
    #[test]
    fn every_measured_malformed_list_drops() {
        for list in [
            "translate(10, abc)",
            "translate()",
            "translate(1,2,3)",
            "scale(1,2,3)",
            "rotate(45,1)",
            "rotate(45,1,2,3)",
            "skewX()",
            "skewX(10,20)",
            "matrix(1,0,0,1,0)",
            "matrix(1,0,0,1,0,0,0)",
            "translate(10.)",
            "shear(10)",
            "translate 10",
            "translate(10",
            "translate(NaN)",
            "translate(1e999)",
            "translate(1,,2)",
            "translate(,1)",
            "translate(1,)",
            "translate(1 2,)",
            "matrix(1,0,0,1,,0)",
            ",translate(1,2)",
            "translate(1,2),,scale(2)",
            // The separator edges measured for drop-semantics: a trailing
            // list-level comma drops (b31, b32), and function names are
            // case-sensitive (b33).
            "translate(10 10),",
            "translate(10 10) ,",
            "Translate(10 10)",
            // Units and !important are CSS-only spellings; the attribute
            // grammar drops both (b29, b30).
            "translate(10px, 10px)",
            "translate(10 10) !important",
        ] {
            assert_eq!(rewrite(list), None, "{list:?} must contribute no hint");
        }
    }

    /// The measured leniency: run-together numbers (csswg-drafts#2623) and
    /// functions without separators (probes b24–b28) transform in Chromium,
    /// so dropping them would render untransformed where Chromium moves.
    #[test]
    fn run_together_numbers_and_functions_rewrite() {
        assert_eq!(
            rewrite("translate(10-10)").unwrap(),
            "translate(10px, -10px)"
        );
        assert_eq!(
            rewrite("translate(10+10)").unwrap(),
            "translate(10px, 10px)"
        );
        assert_eq!(
            rewrite("translate(10.5.5)").unwrap(),
            "translate(10.5px, 0.5px)"
        );
        assert_eq!(
            rewrite("translate(.5.5)").unwrap(),
            "translate(0.5px, 0.5px)"
        );
        assert_eq!(
            rewrite("translate(10 10)scale(2)").unwrap(),
            "translate(10px, 10px) scale(2)"
        );
    }

    #[test]
    fn every_function_maps_with_its_css_units() {
        assert_eq!(
            rewrite("matrix(1,2,3,4,5,6)").unwrap(),
            "matrix(1, 2, 3, 4, 5, 6)"
        );
        assert_eq!(rewrite("translate(10)").unwrap(), "translate(10px)");
        assert_eq!(rewrite("scale(2)").unwrap(), "scale(2)");
        assert_eq!(rewrite("scale(2 3)").unwrap(), "scale(2, 3)");
        assert_eq!(rewrite("rotate(45)").unwrap(), "rotate(45deg)");
        assert_eq!(rewrite("skewX(30)").unwrap(), "skewX(30deg)");
        assert_eq!(rewrite("skewY(30)").unwrap(), "skewY(30deg)");
        // Scientific notation is part of the attribute's <number-token>.
        assert_eq!(
            rewrite("translate(1e1 1E1)").unwrap(),
            "translate(10px, 10px)"
        );
    }

    /// §7.3's defining expansion, verbatim as the mapping.
    #[test]
    fn three_argument_rotate_expands_to_its_translate_sandwich() {
        assert_eq!(
            rewrite("rotate(90 18 14)").unwrap(),
            "translate(18px, 14px) rotate(90deg) translate(-18px, -14px)"
        );
    }

    /// An empty or whitespace-only list authors no function: no hint, the
    /// identity — exactly as an absent attribute.
    #[test]
    fn an_empty_list_contributes_no_hint() {
        assert_eq!(rewrite(""), None);
        assert_eq!(rewrite("  \t\n"), None);
    }

    /// Whitespace between a function name and its parenthesis is grammar
    /// (§7.2 `wsp*`), and comma-wsp between functions still holds.
    #[test]
    fn function_separators_accept_wsp_and_one_comma() {
        assert_eq!(
            rewrite("translate (10 10)").unwrap(),
            "translate(10px, 10px)"
        );
        assert_eq!(
            rewrite("translate(1,2) , scale(2)").unwrap(),
            "translate(1px, 2px) scale(2)"
        );
    }
}
