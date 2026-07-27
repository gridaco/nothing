//! The SVG `transform` attribute grammar: a token list in, one affine out.
//!
//! Separated from the compiler because it is exactly that and nothing else —
//! no document, no cascade, no error type, no element. It reads a string and
//! returns `Some(affine)` or `None`, which is why it can be read, tested and
//! reasoned about without the compiler around it.
//!
//! What it deliberately does *not* inherit from the frozen donor is leniency;
//! the two tightenings are documented on [`parse_transform_list`].

use math2::transform::AffineTransform;

use crate::svg::{dots_carry_digits, trim_svg_whitespace};

/// Parse an SVG `transform` list into one affine.
///
/// The tokenizer shape is the frozen donor's
/// (`crates/htmlcss/src/svg/dom/attrs.rs`, `parse_transform`), re-expressed
/// onto [`AffineTransform`] with two deliberate tightenings, because the
/// donor's leniency is exactly the silent-divergence shape this engine
/// refuses:
///
/// - **Every number must parse.** The donor filters unparseable arguments
///   out of its list, so `translate(10, abc)` silently becomes
///   `translate(10, 0)` — a different mapping than any browser computes.
///   Here one bad number invalidates the list.
/// - **Arity is exact** per SVG2 §8.3: `translate(tx [ty])`,
///   `scale(sx [sy])`, `rotate(a [cx cy])`, `skewX(a)`, `skewY(a)`,
///   `matrix(a b c d e f)`. The donor accepts any count and defaults the
///   rest.
///
/// The number grammar is the same one every other attribute read uses
/// ([`dots_carry_digits`]), so a Rust-superset token like `10.` is
/// invalid here too. A malformed list refuses by name rather than
/// silently mapping a subset of it — the posture [`parse_viewbox`] and
/// [`parse_preserve_aspect_ratio`] already set.
pub(crate) fn parse_transform_list(value: &str) -> Option<AffineTransform> {
    let bytes = value.as_bytes();
    let mut composed = AffineTransform::identity();
    let mut index = 0usize;
    let mut functions = 0usize;
    while index < bytes.len() {
        // Between two functions SVG's separator is `comma-wsp`: whitespace
        // and/or at most one comma. Consuming any run of both would accept
        // a leading comma and a doubled `,,`, which Chromium rejects — and
        // a rejected list paints the element untransformed, so accepting
        // one here would place content the browser leaves in place.
        let mut commas = 0usize;
        while index < bytes.len() {
            if is_svg_whitespace(bytes[index]) {
                index += 1;
            } else if bytes[index] == b',' {
                commas += 1;
                if commas > 1 || functions == 0 {
                    return None;
                }
                index += 1;
            } else {
                break;
            }
        }
        if index >= bytes.len() {
            break;
        }
        // A comma is a separator, not a terminator: it must be followed by
        // another function.
        if functions > 0 && commas == 0 && index > 0 && !is_svg_whitespace(bytes[index - 1]) {
            return None;
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
        let args = &value[args_start..index];
        index += 1;
        functions += 1;

        let numbers = parse_number_list(args)?;

        let step = match (name, numbers.as_slice()) {
            ("matrix", [a, b, c, d, e, f]) => AffineTransform::from_acebdf(*a, *c, *e, *b, *d, *f),
            ("translate", [tx]) => AffineTransform::from_acebdf(1.0, 0.0, *tx, 0.0, 1.0, 0.0),
            ("translate", [tx, ty]) => AffineTransform::from_acebdf(1.0, 0.0, *tx, 0.0, 1.0, *ty),
            ("scale", [s]) => AffineTransform::from_acebdf(*s, 0.0, 0.0, 0.0, *s, 0.0),
            ("scale", [sx, sy]) => AffineTransform::from_acebdf(*sx, 0.0, 0.0, 0.0, *sy, 0.0),
            ("rotate", [degrees]) => rotate_transform(*degrees),
            ("rotate", [degrees, cx, cy]) => {
                AffineTransform::from_acebdf(1.0, 0.0, *cx, 0.0, 1.0, *cy)
                    .compose(&rotate_transform(*degrees))
                    .compose(&AffineTransform::from_acebdf(
                        1.0, 0.0, -*cx, 0.0, 1.0, -*cy,
                    ))
            }
            ("skewX", [degrees]) => {
                AffineTransform::from_acebdf(1.0, degrees.to_radians().tan(), 0.0, 0.0, 1.0, 0.0)
            }
            ("skewY", [degrees]) => {
                AffineTransform::from_acebdf(1.0, 0.0, 0.0, degrees.to_radians().tan(), 1.0, 0.0)
            }
            _ => return None,
        };
        composed = composed.compose(&step);
    }
    // An empty or whitespace-only list authored no function; SVG treats it
    // as the identity, and so does an absent attribute.
    (functions > 0 || trim_svg_whitespace(value).is_empty()).then_some(composed)
}

/// Parse a transform function's argument list under SVG's `comma-wsp`
/// separator grammar: numbers separated by whitespace and/or **at most one**
/// comma, with no leading or trailing comma.
///
/// Splitting on separators and skipping empty tokens — the obvious
/// implementation, and the frozen donor's — would silently accept
/// `translate(1,,2)`, `translate(,1)` and `translate(1,)` as
/// `translate(1,2)` / `translate(1)`, each of which Chromium rejects
/// outright (painting the element untransformed). An empty token is
/// therefore a hard error here, not a skip.
fn parse_number_list(args: &str) -> Option<Vec<f32>> {
    let trimmed = trim_svg_whitespace(args);
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let mut numbers = Vec::new();
    for group in trimmed.split(',') {
        // One comma may separate numbers, so each comma-delimited group
        // must itself hold at least one whitespace-separated number: an
        // empty group is a doubled, leading, or trailing comma.
        let mut tokens = group.split_ascii_whitespace().peekable();
        tokens.peek()?;
        for token in tokens {
            if !dots_carry_digits(token) {
                return None;
            }
            let number = token.parse::<f32>().ok()?;
            if !number.is_finite() {
                return None;
            }
            numbers.push(number);
        }
    }
    Some(numbers)
}

/// A rotation about the origin.
///
/// A quarter turn is produced from its integer matrix rather than from
/// `sin`/`cos`: in f32 the cosine of a right angle is `-4.37e-8`, not
/// zero, so the generic path shears and shifts the shape by a fraction of
/// a unit where the exact matrix does not.
///
/// Two guards keep the shortcut honest. The multiple-of-90 test uses `%`,
/// which is exact in f32, rather than comparing a quotient to its
/// truncation — past `90 * 2^23` every quotient is integral by
/// construction, so a quotient test would snap arbitrary large angles onto
/// one of four exact matrices. The magnitude bound then keeps the quadrant
/// index meaningful, since reducing a huge quotient mod 4 is not.
fn rotate_transform(degrees: f32) -> AffineTransform {
    /// Well past any authored angle, and far below where f32 spacing makes
    /// the quadrant reduction lossy.
    const EXACT_QUARTER_TURN_LIMIT: f32 = 360.0 * 1024.0;
    if degrees.abs() <= EXACT_QUARTER_TURN_LIMIT && degrees % 90.0 == 0.0 {
        let (sin, cos) = match (degrees / 90.0).rem_euclid(4.0) as i32 {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        };
        return AffineTransform::from_acebdf(cos, -sin, 0.0, sin, cos, 0.0);
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    AffineTransform::from_acebdf(cos, -sin, 0.0, sin, cos, 0.0)
}

/// The five ASCII characters the SVG grammar calls whitespace.
const fn is_svg_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0C)
}
