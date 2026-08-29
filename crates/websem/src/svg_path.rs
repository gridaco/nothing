//! The SVG `d` grammar → the resolved contract's canonical command stream.
//!
//! The frozen donor (`crates/htmlcss/src/svg/dom/path_d.rs`) supplied the
//! question list — which commands exist, what the shorthands reflect, how an
//! arc reaches a resolved curve. Every *answer* here was measured against
//! Chromium 149,
//! because the donor's tokenizer is lenient in exactly the places a browser is
//! not. Nothing below is a claim about what a specification says; each rule is
//! the behaviour a browser was observed to have.
//!
//! ## The separator rule
//! A number consumes leading **whitespace** only, then its digits, then
//! trailing whitespace *and at most one comma* and more whitespace. That one
//! rule reproduces every measured case: `M10 10,L…` parses (the number ate the
//! comma — a comma may therefore sit between a coordinate list and the next
//! command letter), while `M,10 10`, `M10,,10`, and a leading `,M…` are all
//! errors.
//!
//! ## The number rule
//! Sign, digits, an optional fraction whose dot **must** carry a digit, and an
//! optional exponent that must carry a digit — then Blink's ordered `f32`
//! evaluation: integer digits right-to-left, fraction digits left-to-right,
//! then the exponent. A one-shot ideal-decimal conversion selects the wrong
//! neighbouring float for valid tokens in both rounding directions. The
//! digit-after-the-dot requirement is separately visible: `M10. 10 …` stops
//! before a complete segment in Chromium, so a trailing dot is invalid here
//! too. (SVG 1.1's grammar admitted `digit-sequence "."`; whatever any grammar
//! says, the browser is what this compiler matches.) `1e40` is an error;
//! `1e30` is finite.
//!
//! ## Arcs follow the pinned path builder
//! Blink forwards `A`/`a` to Chromium's pinned Skia path builder, not through
//! its cubic normalizer. That builder uses `f32` arithmetic and emits at most
//! three rational conics of at most 120 degrees. The authored angle is not
//! reduced. At numeric extremes the builder can return before appending a
//! segment; prior ink then survives and the logical SVG current point still
//! advances. That is distinct from an ordinary derived non-finite verb, which
//! invalidates the whole path. Both outcomes are committed pixel laws.
//!
//! ## Errors finalize the valid prefix
//! Chromium renders an erroneous path's **valid prefix** (SVG2 §9.3.9) and
//! drops the rest. Every fully defined segment is emitted immediately; a
//! partial repeated command contributes nothing. Where the prefix is empty —
//! no leading `moveto`, an error in the first segment — the resolved geometry
//! is the correct nothing. A trailing move-only contour is neutral and is
//! removed during finalization.
//!
//! ## Canonical form
//! The contract requires every contour to open with an explicit move and to
//! draw at least once ([`rframe::PathData`]). SVG does not, so this producer
//! normalizes — and each normalization was measured to be pixel-neutral in
//! Chromium *before* being applied, on geometry whose edges are anti-aliased,
//! because integer axis-aligned edges hide exactly this class of difference:
//!
//! - a contour that only **moves** is dropped (neutral);
//! - a **second** `Z`, closing nothing, is dropped (neutral);
//! - a drawing command after a `Z` gets the explicit move to the subpath start
//!   that SVG leaves implicit (neutral).
//!
//! One shape that looks like the first is not: `M x y Z` is a zero-length
//! *closed* contour, it is **not** neutral, and it is resolved rather than
//! dropped. See [`Parser::emit_close`].

use crate::svg_number::{self, ExponentParts, NumberParts};
use rframe::PathCommand;

/// The producer-side result of one authored `d` scan.
///
/// `commands` is the canonical raster path that may cross into `rframe`.
/// `marker_elements` retains the authored vertex topology Blink uses for
/// marker placement: move-only contours survive, quadratics are normalized to
/// one cubic, and one authored arc contributes at most one synthetic cubic.
/// Neither this type nor the marker projection crosses the resolved contract.
#[derive(Debug, Default)]
pub(crate) struct ParsedPath {
    pub(crate) commands: Vec<PathCommand>,
    marker_elements: Vec<MarkerPathElement>,
    /// Derived non-finite ordinary geometry made the raster path invalid.
    /// Marker painting for that extreme class is not admitted by inference.
    marker_projection_poisoned: bool,
}

impl ParsedPath {
    pub(crate) fn marker_positions(&self) -> Result<Vec<MarkerPosition>, &'static str> {
        if self.marker_projection_poisoned {
            return Err(
                "marker geometry crosses a derived non-finite path boundary whose vertex projection is not admitted",
            );
        }
        Ok(build_marker_positions(&self.marker_elements))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerType {
    Start,
    Mid,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarkerPosition {
    pub(crate) kind: MarkerType,
    pub(crate) origin: (f32, f32),
    pub(crate) angle: f32,
}

/// Authored marker topology after Blink's path normalizer, but before marker
/// positions and tangent angles are built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MarkerPathElement {
    MoveTo((f32, f32)),
    LineTo((f32, f32)),
    CubicTo {
        one: (f32, f32),
        two: (f32, f32),
        end: (f32, f32),
    },
    Close,
}

/// Why one SVG numeric grammar stopped parsing.
///
/// Path data itself consumes its valid prefix when this occurs. Point lists
/// use the same scanner but finalize their own list-level error semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceSyntaxError {
    /// The source value stopped being valid at this byte offset.
    /// Sufficient to excerpt the offending text without carrying a kilobyte of
    /// `d` in an error.
    Syntax { offset: usize },
}

/// Parse one `d` value into its canonical absolute valid-prefix stream.
///
/// An empty (or whitespace-only) value is valid and resolves to no commands —
/// SVG's `d` initial value is `none`, which also has an empty valid prefix.
/// Any syntax error ends parsing after the last fully defined segment.
pub(crate) fn parse_path_data(d: &str) -> Vec<PathCommand> {
    parse_path(d).commands
}

/// Parse both producer projections from one token scan.
pub(crate) fn parse_path(d: &str) -> ParsedPath {
    Parser::new(d).parse_path_prefix()
}

/// Parse one `points` list (SVG2 §10.4) into Chromium's used coordinate pairs,
/// through the same number scanner as path data so the two grammars cannot
/// drift.
///
/// The separator rules are Blink's, measured against Chromium 149: a
/// trailing separator after the last complete pair is accepted (unlike the
/// `viewBox` grammar), a leading or doubled comma is an error, a trailing
/// dot needs a digit, and a sign starts a new number (`32-56` is two).
/// A final unmatched x coordinate, with or without one trailing comma, is the
/// one recoverable error: it is dropped and all complete pairs remain. Every
/// lexical or numeric failure clears the whole list, including one after an
/// unmatched x; no malformed-list prefix survives.
///
/// An empty (or whitespace-only) value is valid and resolves to no points,
/// which renders nothing.
pub(crate) fn parse_points(value: &str) -> Vec<(f32, f32)> {
    let mut parser = Parser::new(value);
    parser.skip_wsp();
    let mut points = Vec::new();
    while parser.at < parser.bytes.len() {
        let Ok(x) = parser.number() else {
            return Vec::new();
        };
        // Blink emits only complete pairs. Reaching the end after x is the
        // SVGPointList odd-count exception, including when x consumed one
        // trailing comma.
        if parser.at == parser.bytes.len() {
            return points;
        }
        let Ok(y) = parser.number() else {
            return Vec::new();
        };
        points.push((x, y));
    }
    points
}

/// The five ASCII characters Blink's SVG parsers treat as whitespace
/// (`IsHTMLSpace`), which is also this compiler's attribute whitespace set.
const fn is_wsp(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0C)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    commands: Vec<PathCommand>,
    marker_elements: Vec<MarkerPathElement>,
    current: (f32, f32),
    subpath_start: (f32, f32),
    /// The open contour's `MoveTo` index and whether it has drawn yet.
    /// `None` between a `Z` and the next drawing command.
    open: Option<(usize, bool)>,
    /// Absolute control point a smooth cubic (`S`/`s`) reflects, when the
    /// previous command was itself a cubic.
    last_cubic: Option<(f32, f32)>,
    /// The same for a smooth quadratic (`T`/`t`).
    last_quad: Option<(f32, f32)>,
    /// Skia discards a path containing a non-finite ordinary verb. The source
    /// parser must still advance its semantic current point, but no resolved
    /// frame command may carry that value.
    poisoned: bool,
}

impl<'a> Parser<'a> {
    fn new(d: &'a str) -> Self {
        Self {
            bytes: d.as_bytes(),
            at: 0,
            commands: Vec::new(),
            marker_elements: Vec::new(),
            current: (0.0, 0.0),
            subpath_start: (0.0, 0.0),
            open: None,
            last_cubic: None,
            last_quad: None,
            poisoned: false,
        }
    }

    fn error<T>(&self) -> Result<T, SourceSyntaxError> {
        Err(SourceSyntaxError::Syntax { offset: self.at })
    }

    fn parse_path_prefix(mut self) -> ParsedPath {
        self.skip_wsp();
        if self.at >= self.bytes.len() {
            return ParsedPath::default();
        }
        let mut first = true;
        while self.at < self.bytes.len() {
            let command = self.bytes[self.at];
            if !command.is_ascii_alphabetic() {
                break;
            }
            // SVG2 §9.3.4: path data must begin with a moveto. Chromium
            // renders nothing at all otherwise (an empty valid prefix).
            if first && !matches!(command, b'M' | b'm') {
                break;
            }
            first = false;
            self.at += 1;
            if self.command(command).is_err() {
                break;
            }
            self.skip_wsp();
        }
        self.finish()
    }

    fn finish(mut self) -> ParsedPath {
        if self.poisoned {
            return ParsedPath {
                commands: Vec::new(),
                marker_elements: Vec::new(),
                marker_projection_poisoned: true,
            };
        }
        // A trailing contour that only moved contributes nothing.
        if let Some((index, false)) = self.open {
            debug_assert_eq!(index + 1, self.commands.len());
            self.commands.truncate(index);
        }
        ParsedPath {
            commands: self.commands,
            marker_elements: self.marker_elements,
            marker_projection_poisoned: false,
        }
    }

    /// One command letter and its complete argument list, including every
    /// implicit repeat of it.
    fn command(&mut self, command: u8) -> Result<(), SourceSyntaxError> {
        let relative = command.is_ascii_lowercase();
        let mut repeat = 0usize;
        loop {
            match command {
                b'M' | b'm' => {
                    let (x, y) = self.coordinate_pair()?;
                    // An implicit repeat after a moveto is a lineto — the one
                    // command whose repeat changes meaning.
                    if repeat == 0 {
                        let point = self.absolute(relative, x, y);
                        self.emit_move(point);
                    } else {
                        let point = self.absolute(relative, x, y);
                        self.emit_line(point);
                    }
                    self.reset_reflection();
                }
                b'L' | b'l' => {
                    let (x, y) = self.coordinate_pair()?;
                    let point = self.absolute(relative, x, y);
                    self.emit_line(point);
                    self.reset_reflection();
                }
                b'H' | b'h' => {
                    let x = self.number()?;
                    let point = if relative {
                        (self.current.0 + x, self.current.1)
                    } else {
                        (x, self.current.1)
                    };
                    self.emit_line(point);
                    self.reset_reflection();
                }
                b'V' | b'v' => {
                    let y = self.number()?;
                    let point = if relative {
                        (self.current.0, self.current.1 + y)
                    } else {
                        (self.current.0, y)
                    };
                    self.emit_line(point);
                    self.reset_reflection();
                }
                b'C' | b'c' => {
                    let one = self.coordinate_pair()?;
                    let two = self.coordinate_pair()?;
                    let end = self.coordinate_pair()?;
                    let one = self.absolute(relative, one.0, one.1);
                    let two = self.absolute(relative, two.0, two.1);
                    let end = self.absolute(relative, end.0, end.1);
                    self.emit_cubic(one, two, end);
                }
                b'S' | b's' => {
                    let two = self.coordinate_pair()?;
                    let end = self.coordinate_pair()?;
                    let two = self.absolute(relative, two.0, two.1);
                    let end = self.absolute(relative, end.0, end.1);
                    let one = self.reflect(self.last_cubic);
                    self.emit_cubic(one, two, end);
                }
                b'Q' | b'q' => {
                    let one = self.coordinate_pair()?;
                    let end = self.coordinate_pair()?;
                    let one = self.absolute(relative, one.0, one.1);
                    let end = self.absolute(relative, end.0, end.1);
                    self.emit_quad(one, end);
                }
                b'T' | b't' => {
                    let end = self.coordinate_pair()?;
                    let end = self.absolute(relative, end.0, end.1);
                    let one = self.reflect(self.last_quad);
                    self.emit_quad(one, end);
                }
                b'A' | b'a' => {
                    let rx = self.number()?;
                    let ry = self.number()?;
                    let angle = self.number()?;
                    let large = self.flag()?;
                    let sweep = self.flag()?;
                    let end = self.coordinate_pair()?;
                    let end = self.absolute(relative, end.0, end.1);
                    self.emit_arc((rx, ry), angle, large, sweep, end);
                    self.reset_reflection();
                }
                b'Z' | b'z' => {
                    self.emit_close();
                    self.reset_reflection();
                    // Close takes no arguments, so it never repeats.
                    return Ok(());
                }
                _ => return self.error(),
            }
            repeat += 1;
            if !self.peek_argument() {
                return Ok(());
            }
        }
    }

    fn absolute(&self, relative: bool, x: f32, y: f32) -> (f32, f32) {
        if relative {
            (self.current.0 + x, self.current.1 + y)
        } else {
            (x, y)
        }
    }

    /// A smooth command's first control point: the previous curve's control
    /// point reflected about the current point, or the current point itself
    /// when the previous command was not the matching curve kind.
    fn reflect(&self, previous: Option<(f32, f32)>) -> (f32, f32) {
        match previous {
            Some((x, y)) => (
                self.current.0 + (self.current.0 - x),
                self.current.1 + (self.current.1 - y),
            ),
            None => self.current,
        }
    }

    fn reset_reflection(&mut self) {
        self.last_cubic = None;
        self.last_quad = None;
    }

    // ─── emission (canonical form) ────────────────────────────────────────

    fn emit_move(&mut self, point: (f32, f32)) {
        if self.poisoned || !finite_point(point) {
            self.poisoned |= !finite_point(point);
            self.commands.clear();
            self.marker_elements.clear();
            self.open = None;
            self.current = point;
            self.subpath_start = point;
            return;
        }
        self.marker_elements.push(MarkerPathElement::MoveTo(point));
        // The contour this move replaces drew nothing, so it is not a visual
        // fact; its move is the last command emitted.
        if let Some((index, false)) = self.open {
            debug_assert_eq!(index + 1, self.commands.len());
            self.commands.truncate(index);
        }
        self.commands.push(PathCommand::MoveTo {
            x: point.0,
            y: point.1,
        });
        self.open = Some((self.commands.len() - 1, false));
        self.current = point;
        self.subpath_start = point;
    }

    /// Reopen the contour SVG leaves implicit after a `Z`: the current point is
    /// the closed contour's start, and the canonical stream says so.
    fn open_contour(&mut self) {
        if self.poisoned {
            return;
        }
        if self.open.is_none() {
            self.commands.push(PathCommand::MoveTo {
                x: self.subpath_start.0,
                y: self.subpath_start.1,
            });
            self.open = Some((self.commands.len() - 1, false));
            self.current = self.subpath_start;
        }
    }

    fn drew(&mut self) {
        if let Some((index, _)) = self.open {
            self.open = Some((index, true));
        }
    }

    fn emit_line(&mut self, point: (f32, f32)) {
        if self.poisoned || !finite_point(point) {
            self.poisoned |= !finite_point(point);
            self.commands.clear();
            self.marker_elements.clear();
            self.open = None;
            self.current = point;
            return;
        }
        self.marker_elements.push(MarkerPathElement::LineTo(point));
        self.emit_raster_line(point);
    }

    fn emit_raster_line(&mut self, point: (f32, f32)) {
        self.open_contour();
        self.commands.push(PathCommand::LineTo {
            x: point.0,
            y: point.1,
        });
        self.drew();
        self.current = point;
    }

    fn emit_quad(&mut self, one: (f32, f32), end: (f32, f32)) {
        if self.poisoned || !finite_point(one) || !finite_point(end) {
            self.poisoned |= !finite_point(one) || !finite_point(end);
            self.commands.clear();
            self.marker_elements.clear();
            self.open = None;
            self.current = end;
            self.last_quad = Some(one);
            self.last_cubic = None;
            return;
        }
        // Blink normalizes one quadratic to one cubic with these f32 blend
        // operations before its marker tangent builder sees it.
        let start = if self.open.is_none() {
            self.subpath_start
        } else {
            self.current
        };
        let one_third = 1.0 / 3.0_f32;
        let cubic_one = (
            (start.0 + 2.0 * one.0) * one_third,
            (start.1 + 2.0 * one.1) * one_third,
        );
        let cubic_two = (
            (end.0 + 2.0 * one.0) * one_third,
            (end.1 + 2.0 * one.1) * one_third,
        );
        self.marker_elements.push(MarkerPathElement::CubicTo {
            one: cubic_one,
            two: cubic_two,
            end,
        });
        self.open_contour();
        self.commands.push(PathCommand::QuadTo {
            x1: one.0,
            y1: one.1,
            x: end.0,
            y: end.1,
        });
        self.drew();
        self.current = end;
        self.last_quad = Some(one);
        self.last_cubic = None;
    }

    fn emit_cubic(&mut self, one: (f32, f32), two: (f32, f32), end: (f32, f32)) {
        if self.poisoned || !finite_point(one) || !finite_point(two) || !finite_point(end) {
            self.poisoned |= !finite_point(one) || !finite_point(two) || !finite_point(end);
            self.commands.clear();
            self.marker_elements.clear();
            self.open = None;
            self.current = end;
            self.last_cubic = Some(two);
            self.last_quad = None;
            return;
        }
        self.marker_elements
            .push(MarkerPathElement::CubicTo { one, two, end });
        self.open_contour();
        self.commands.push(PathCommand::CubicTo {
            x1: one.0,
            y1: one.1,
            x2: two.0,
            y2: two.1,
            x: end.0,
            y: end.1,
        });
        self.drew();
        self.current = end;
        self.last_cubic = Some(two);
        self.last_quad = None;
    }

    /// Close the open contour — or, when it has not drawn, resolve the
    /// zero-length **closed** contour that `M x y Z` actually is.
    ///
    /// A bare `M x y` contributes nothing and is dropped. `M x y Z` is a
    /// different fact, and the difference is measurable in two ways. It
    /// strokes: a zero-length closed subpath paints a cap-shaped dot. And it
    /// changes the *fill* of the rest of the path — Chromium's coverage of the
    /// surviving geometry shifts by up to 35/255 when the contour is dropped,
    /// because an extra contour is an extra contour to the scan converter.
    /// Encoding it as an explicit zero-length segment is exact: Chromium
    /// renders `M x y Z` byte-identically to `M x y L x y Z`, and the contract
    /// already carries that form.
    ///
    /// A `Z` with no open contour at all — a second `Z` — really is inert
    /// (measured), so it emits nothing.
    fn emit_close(&mut self) {
        if self.poisoned {
            self.open = None;
            self.current = self.subpath_start;
            return;
        }
        // Blink's authored path stream preserves close commands even when a
        // second close is raster-neutral. The marker builder still observes
        // that authored vertex.
        if !self.marker_elements.is_empty() {
            self.marker_elements.push(MarkerPathElement::Close);
        }
        match self.open {
            Some((_, true)) => {
                self.commands.push(PathCommand::Close);
                self.open = None;
            }
            Some((_, false)) => {
                self.commands.push(PathCommand::LineTo {
                    x: self.subpath_start.0,
                    y: self.subpath_start.1,
                });
                self.commands.push(PathCommand::Close);
                self.open = None;
            }
            None => {}
        }
        self.current = self.subpath_start;
    }

    fn emit_conic(&mut self, one: (f32, f32), end: (f32, f32), weight: f32) {
        if self.poisoned
            || !finite_point(one)
            || !finite_point(end)
            || !weight.is_finite()
            || weight <= 0.0
        {
            self.poisoned |=
                !finite_point(one) || !finite_point(end) || !weight.is_finite() || weight <= 0.0;
            self.commands.clear();
            self.marker_elements.clear();
            self.open = None;
            self.current = end;
            return;
        }
        self.open_contour();
        self.commands.push(PathCommand::ConicTo {
            x1: one.0,
            y1: one.1,
            x: end.0,
            y: end.1,
            weight,
        });
        self.drew();
        self.current = end;
    }

    /// Resolve one elliptical arc to the conic segments Chromium's rasterizer
    /// draws it through. Every rule below is a Chromium 149 measurement:
    ///
    /// - coincident endpoints elide the segment entirely;
    /// - a zero radius degenerates to a straight line, byte-identical to the
    ///   authored `L`;
    /// - negative radii take their absolute value, byte-identical to the
    ///   positive spelling;
    /// - too-small radii scale up uniformly until the endpoints fit,
    ///   byte-identical to authoring the scaled radii;
    /// - the rotation angle feeds Skia's snapped `f32` trigonometry as authored;
    /// - Skia's finite arithmetic may produce either no conic or a poisoned
    ///   path at numeric extremes, and those are distinct outcomes;
    /// - the sweep splits into at most three conics of at most 120 degrees,
    ///   and the last segment reuses the authored endpoint exactly.
    fn emit_arc(
        &mut self,
        radii: (f32, f32),
        angle: f32,
        large: bool,
        sweep: bool,
        end: (f32, f32),
    ) {
        let start = if self.open.is_none() {
            // An arc after a close continues from the closed contour's start,
            // which is where `emit_conic`'s implicit reopen will put the pen.
            self.subpath_start
        } else {
            self.current
        };
        // Marker topology is independent from Skia's raster path. Blink's
        // normalizer turns a coincident-endpoint or zero-radius arc into one
        // line for marker placement even when the raster builder elides it.
        let marker_element = marker_arc_element(start, radii, angle, large, sweep, end);
        self.marker_elements.push(marker_element);
        if start == end {
            return;
        }
        match resolve_arc_like_skia(start, radii, angle, large, sweep, end) {
            ArcResolution::Line => self.emit_raster_line(end),
            ArcResolution::NoOp => self.current = end,
            ArcResolution::Conics(conics) => {
                for (control, endpoint, weight) in conics {
                    self.emit_conic(control, endpoint, weight);
                }
            }
        }
    }

    // ─── tokenizer ───────────────────────────────────────────────────────

    fn skip_wsp(&mut self) {
        while self.at < self.bytes.len() && is_wsp(self.bytes[self.at]) {
            self.at += 1;
        }
    }

    /// Blink's `SkipOptionalSVGSpacesOrDelimiter`, run after every number:
    /// whitespace, then at most one comma, then whitespace. Consuming a second
    /// comma here would admit `M10,,10`, which Chromium rejects.
    fn skip_trailing_separator(&mut self) {
        self.skip_wsp();
        if self.at < self.bytes.len() && self.bytes[self.at] == b',' {
            self.at += 1;
            self.skip_wsp();
        }
    }

    /// Whether another argument follows — the implicit-repeat test. Separators
    /// were already consumed by the previous number.
    fn peek_argument(&mut self) -> bool {
        self.at < self.bytes.len()
            && matches!(self.bytes[self.at], b'+' | b'-' | b'.' | b'0'..=b'9')
    }

    fn coordinate_pair(&mut self) -> Result<(f32, f32), SourceSyntaxError> {
        let x = self.number()?;
        let y = self.number()?;
        Ok((x, y))
    }

    fn number(&mut self) -> Result<f32, SourceSyntaxError> {
        self.skip_wsp();
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
        let mut fraction_digits = None;
        if self.at < self.bytes.len() && self.bytes[self.at] == b'.' {
            self.at += 1;
            let fraction_start = self.at;
            self.digits();
            // Blink requires a digit after the dot even though SVG's BNF
            // permits `10.`; a trailing dot renders nothing.
            if self.at == fraction_start {
                self.at = start;
                return self.error();
            }
            fraction_digits = Some(fraction_start..self.at);
        }
        if integer_digits.is_empty() && fraction_digits.is_none() {
            self.at = start;
            return self.error();
        }
        let exponent = if self.at < self.bytes.len() && matches!(self.bytes[self.at], b'e' | b'E') {
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
                return self.error();
            }
            Some(ExponentParts {
                negative: exponent_negative,
                digits: exponent_start..self.at,
            })
        } else {
            None
        };
        let parsed = svg_number::evaluate(
            self.bytes,
            &NumberParts {
                negative,
                integer_digits,
                fraction_digits,
                exponent,
            },
        );
        let Some(value) = parsed else {
            self.at = start;
            return self.error();
        };
        self.skip_trailing_separator();
        Ok(value)
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_digit() {
            self.at += 1;
        }
        self.at - start
    }

    /// An arc's large/sweep flag: exactly one `0` or `1`, with no sign and no
    /// fraction, so `A22 22 0 0154 32` packs both flags and the endpoint.
    fn flag(&mut self) -> Result<bool, SourceSyntaxError> {
        self.skip_wsp();
        let flag = match self.bytes.get(self.at) {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => return self.error(),
        };
        self.at += 1;
        self.skip_trailing_separator();
        Ok(flag)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MarkerElementType {
    Move,
    Line,
    Cubic,
    Close,
}

#[derive(Clone, Copy, Default)]
struct MarkerSegmentData {
    start_tangent: (f32, f32),
    end_tangent: (f32, f32),
    position: (f32, f32),
}

/// Blink's marker-position builder, kept producer-side because its authored
/// path topology has no place in the resolved frame contract.
fn build_marker_positions(elements: &[MarkerPathElement]) -> Vec<MarkerPosition> {
    struct Builder {
        positions: Vec<MarkerPosition>,
        last_moveto_index: usize,
        last_element_type: MarkerElementType,
        origin: (f32, f32),
        subpath_start: (f32, f32),
        in_slope: (f32, f32),
        out_slope: (f32, f32),
        last_moveto_out_slope: (f32, f32),
    }

    impl Builder {
        fn compute_quad_tangents(
            data: &mut MarkerSegmentData,
            start: (f32, f32),
            control: (f32, f32),
            end: (f32, f32),
        ) {
            data.start_tangent = subtract(control, start);
            data.end_tangent = subtract(end, control);
            if is_zero(data.start_tangent) {
                data.start_tangent = data.end_tangent;
            } else if is_zero(data.end_tangent) {
                data.end_tangent = data.start_tangent;
            }
        }

        fn features(&self, element: MarkerPathElement) -> (MarkerElementType, MarkerSegmentData) {
            let mut data = MarkerSegmentData::default();
            let kind = match element {
                MarkerPathElement::MoveTo(position) => {
                    data.position = position;
                    data.start_tangent = subtract(position, self.origin);
                    data.end_tangent = data.start_tangent;
                    MarkerElementType::Move
                }
                MarkerPathElement::LineTo(position) => {
                    data.position = position;
                    data.start_tangent = subtract(position, self.origin);
                    data.end_tangent = data.start_tangent;
                    MarkerElementType::Line
                }
                MarkerPathElement::CubicTo { one, two, end } => {
                    data.position = end;
                    data.start_tangent = subtract(one, self.origin);
                    data.end_tangent = subtract(end, two);
                    if is_zero(data.start_tangent) {
                        Self::compute_quad_tangents(&mut data, one, two, end);
                    } else if is_zero(data.end_tangent) {
                        Self::compute_quad_tangents(&mut data, self.origin, one, two);
                    }
                    MarkerElementType::Cubic
                }
                MarkerPathElement::Close => {
                    let mut tangent = subtract(self.subpath_start, self.origin);
                    if self.last_element_type != MarkerElementType::Move && is_zero(tangent) {
                        tangent = self.last_moveto_out_slope;
                    }
                    data.position = self.subpath_start;
                    data.start_tangent = tangent;
                    data.end_tangent = tangent;
                    MarkerElementType::Close
                }
            };
            (kind, data)
        }

        fn current_angle(&self, ends_subpath: bool) -> f32 {
            if self.last_element_type == MarkerElementType::Close {
                return bisecting_angle(self.in_slope, self.out_slope);
            }
            if ends_subpath {
                return slope_degrees(self.in_slope);
            }
            if self.last_element_type == MarkerElementType::Move {
                return slope_degrees(self.out_slope);
            }
            bisecting_angle(self.in_slope, self.out_slope)
        }

        fn update_angle(&mut self, ends_subpath: bool) {
            if self.last_element_type == MarkerElementType::Close {
                self.out_slope = self.last_moveto_out_slope;
            }
            let angle = self.current_angle(ends_subpath);
            if self.last_element_type == MarkerElementType::Close {
                self.positions[self.last_moveto_index].angle = angle;
            }
            self.positions.last_mut().expect("a prior marker").angle = angle;
        }

        fn update(&mut self, element: MarkerPathElement) {
            let (kind, segment) = self.features(element);
            self.out_slope = segment.start_tangent;
            if self.last_element_type == MarkerElementType::Move {
                self.last_moveto_out_slope = self.out_slope;
            }
            let starts_new_subpath = kind == MarkerElementType::Move;
            if !self.positions.is_empty() {
                self.update_angle(starts_new_subpath);
            }
            self.in_slope = segment.end_tangent;
            self.origin = segment.position;
            if starts_new_subpath {
                self.subpath_start = segment.position;
                self.last_moveto_index = self.positions.len();
            }
            self.last_element_type = kind;
            self.positions.push(MarkerPosition {
                kind: if self.positions.is_empty() {
                    MarkerType::Start
                } else {
                    MarkerType::Mid
                },
                origin: self.origin,
                angle: 0.0,
            });
        }

        fn finish(mut self) -> Vec<MarkerPosition> {
            if self.positions.is_empty() {
                return self.positions;
            }
            self.update_angle(true);
            self.positions.last_mut().expect("one marker").kind = MarkerType::End;
            self.positions
        }
    }

    let mut builder = Builder {
        positions: Vec::with_capacity(elements.len()),
        last_moveto_index: 0,
        last_element_type: MarkerElementType::Move,
        origin: (0.0, 0.0),
        subpath_start: (0.0, 0.0),
        in_slope: (0.0, 0.0),
        out_slope: (0.0, 0.0),
        last_moveto_out_slope: (0.0, 0.0),
    };
    for element in elements {
        builder.update(*element);
    }
    builder.finish()
}

pub(crate) fn line_marker_positions(start: (f32, f32), end: (f32, f32)) -> Vec<MarkerPosition> {
    build_marker_positions(&[
        MarkerPathElement::MoveTo(start),
        MarkerPathElement::LineTo(end),
    ])
}

pub(crate) fn points_marker_positions(points: &[(f32, f32)], closed: bool) -> Vec<MarkerPosition> {
    let Some((first, rest)) = points.split_first() else {
        return Vec::new();
    };
    let mut elements = Vec::with_capacity(points.len() + usize::from(closed));
    elements.push(MarkerPathElement::MoveTo(*first));
    for point in rest {
        elements.push(MarkerPathElement::LineTo(*point));
    }
    if closed {
        elements.push(MarkerPathElement::Close);
    }
    build_marker_positions(&elements)
}

fn subtract(left: (f32, f32), right: (f32, f32)) -> (f32, f32) {
    (left.0 - right.0, left.1 - right.1)
}

fn is_zero(vector: (f32, f32)) -> bool {
    vector == (0.0, 0.0)
}

fn slope_degrees(vector: (f32, f32)) -> f32 {
    vector.1.atan2(vector.0).to_degrees()
}

fn bisecting_angle(in_slope: (f32, f32), out_slope: (f32, f32)) -> f32 {
    let mut in_angle = f64::from(slope_degrees(in_slope));
    let out_angle = f64::from(slope_degrees(out_slope));
    let diff = in_angle - out_angle;
    if diff > 180.0 || diff <= -180.0 {
        in_angle += 360.0;
    }
    ((in_angle + out_angle) / 2.0) as f32
}

/// Normalize one authored arc to the single synthetic cubic Blink feeds to
/// its marker builder: first control from the first decomposed cubic, and
/// second control/end from the last. Failure becomes one line segment.
fn marker_arc_element(
    start: (f32, f32),
    radii: (f32, f32),
    angle: f32,
    large: bool,
    sweep: bool,
    end: (f32, f32),
) -> MarkerPathElement {
    let curves = decompose_marker_arc_to_cubics(start, radii, angle, large, sweep, end);
    let (Some(first), Some(last)) = (curves.first(), curves.last()) else {
        return MarkerPathElement::LineTo(end);
    };
    MarkerPathElement::CubicTo {
        one: first.0,
        two: last.1,
        end: last.2,
    }
}

type MarkerCubic = ((f32, f32), (f32, f32), (f32, f32));

/// Port of Blink's `SVGPathNormalizer::DecomposeArcToCubic` arithmetic.
/// The raster path intentionally keeps its independent pinned-Skia conic
/// route; this helper exists only for authored marker tangents.
fn decompose_marker_arc_to_cubics(
    start: (f32, f32),
    radii: (f32, f32),
    angle: f32,
    large: bool,
    sweep: bool,
    end: (f32, f32),
) -> Vec<MarkerCubic> {
    let mut rx = radii.0.abs();
    let mut ry = radii.1.abs();
    if rx == 0.0 || ry == 0.0 || start == end {
        return Vec::new();
    }

    let midpoint_delta = ((start.0 - end.0) * 0.5, (start.1 - end.1) * 0.5);
    let transformed_midpoint = rotate_point_f32(midpoint_delta, -angle);
    let square_rx = rx * rx;
    let square_ry = ry * ry;
    let square_x = transformed_midpoint.0 * transformed_midpoint.0;
    let square_y = transformed_midpoint.1 * transformed_midpoint.1;
    let radii_scale = square_x / square_rx + square_y / square_ry;
    if radii_scale > 1.0 {
        let scale = radii_scale.sqrt();
        rx *= scale;
        ry *= scale;
    }
    if ![rx, ry].into_iter().all(f32::is_finite) {
        return Vec::new();
    }

    let point1 = ellipse_to_unit(start, rx, ry, angle);
    let point2 = ellipse_to_unit(end, rx, ry, angle);
    let mut delta = subtract(point2, point1);
    let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
    let scale_factor_squared = f64::from((1.0 / length_squared) - 0.25).max(0.0);
    let mut scale_factor = scale_factor_squared.sqrt() as f32;
    if sweep == large {
        scale_factor = -scale_factor;
    }
    delta.0 *= scale_factor;
    delta.1 *= scale_factor;
    let mut center = ((point1.0 + point2.0) * 0.5, (point1.1 + point2.1) * 0.5);
    center.0 -= delta.1;
    center.1 += delta.0;

    let theta1 = (point1.1 - center.1).atan2(point1.0 - center.0);
    let theta2 = (point2.1 - center.1).atan2(point2.0 - center.0);
    let mut theta_arc = theta2 - theta1;
    if theta_arc < 0.0 && sweep {
        theta_arc += std::f32::consts::TAU;
    } else if theta_arc > 0.0 && !sweep {
        theta_arc -= std::f32::consts::TAU;
    }
    if !theta_arc.is_finite() {
        return Vec::new();
    }

    let segments = (theta_arc.abs() / (std::f32::consts::FRAC_PI_2 + 0.001)).ceil() as usize;
    if segments == 0 {
        return Vec::new();
    }
    let mut curves = Vec::with_capacity(segments);
    for index in 0..segments {
        let start_theta = theta1 + index as f32 * theta_arc / segments as f32;
        let end_theta = theta1 + (index + 1) as f32 * theta_arc / segments as f32;
        let tangent = (8.0 / 6.0_f32) * (0.25 * (end_theta - start_theta)).tan();
        if !tangent.is_finite() {
            return Vec::new();
        }
        let (sin_start, cos_start) = start_theta.sin_cos();
        let (sin_end, cos_end) = end_theta.sin_cos();
        let control_one_unit = (
            cos_start - tangent * sin_start + center.0,
            sin_start + tangent * cos_start + center.1,
        );
        let target_unit = (cos_end + center.0, sin_end + center.1);
        let control_two_unit = (
            target_unit.0 + tangent * sin_end,
            target_unit.1 - tangent * cos_end,
        );
        curves.push((
            unit_to_ellipse(control_one_unit, rx, ry, angle),
            unit_to_ellipse(control_two_unit, rx, ry, angle),
            unit_to_ellipse(target_unit, rx, ry, angle),
        ));
    }
    curves
}

fn rotate_point_f32(point: (f32, f32), degrees: f32) -> (f32, f32) {
    let radians = f64::from(degrees).to_radians();
    let (sin, cos) = radians.sin_cos();
    (
        (f64::from(point.0) * cos - f64::from(point.1) * sin) as f32,
        (f64::from(point.0) * sin + f64::from(point.1) * cos) as f32,
    )
}

fn ellipse_to_unit(point: (f32, f32), rx: f32, ry: f32, angle: f32) -> (f32, f32) {
    let rotated = rotate_point_f32(point, -angle);
    (rotated.0 / rx, rotated.1 / ry)
}

fn unit_to_ellipse(point: (f32, f32), rx: f32, ry: f32, angle: f32) -> (f32, f32) {
    rotate_point_f32((point.0 * rx, point.1 * ry), angle)
}

type ResolvedConic = ((f32, f32), (f32, f32), f32);

enum ArcResolution {
    Line,
    NoOp,
    Conics(Vec<ResolvedConic>),
}

#[derive(Clone, Copy)]
struct SkiaAffine2 {
    scale_x: f32,
    skew_x: f32,
    skew_y: f32,
    scale_y: f32,
}

impl SkiaAffine2 {
    /// `Scale(sx, sy) * Rotate(degrees)`, matching `setScale().preRotate()`.
    fn scale_pre_rotate(scale_x: f32, scale_y: f32, degrees: f32) -> Self {
        let (sin, cos) = skia_sin_cos(degrees);
        Self {
            scale_x: scale_x * cos,
            skew_x: scale_x * -sin,
            skew_y: scale_y * sin,
            scale_y: scale_y * cos,
        }
    }

    /// `Rotate(degrees) * Scale(sx, sy)`, matching `setRotate().preScale()`.
    fn rotate_pre_scale(degrees: f32, scale_x: f32, scale_y: f32) -> Self {
        let (sin, cos) = skia_sin_cos(degrees);
        Self {
            scale_x: cos * scale_x,
            skew_x: -sin * scale_y,
            skew_y: sin * scale_x,
            scale_y: cos * scale_y,
        }
    }

    fn map(self, point: (f32, f32)) -> (f32, f32) {
        (
            point.0 * self.scale_x + point.1 * self.skew_x,
            point.0 * self.skew_y + point.1 * self.scale_y,
        )
    }
}

/// Chromium 149 forwards an SVG arc to pinned Skia's `SkPathBuilder::arcTo`.
/// This is that builder's `f32` construction, projected into the producer's
/// source-neutral conic vocabulary. A `NoOp` is distinct from a poisoned
/// ordinary path command: Skia can abandon an arc before appending a verb and
/// leave every preceding segment intact.
fn resolve_arc_like_skia(
    start: (f32, f32),
    radii: (f32, f32),
    angle: f32,
    large: bool,
    sweep: bool,
    end: (f32, f32),
) -> ArcResolution {
    if radii.0 == 0.0 || radii.1 == 0.0 {
        return ArcResolution::Line;
    }

    let mut rx = radii.0.abs();
    let mut ry = radii.1.abs();
    let midpoint_distance = ((start.0 - end.0) * 0.5, (start.1 - end.1) * 0.5);
    let transformed_midpoint =
        SkiaAffine2::scale_pre_rotate(1.0, 1.0, -angle).map(midpoint_distance);

    let square_rx = rx * rx;
    let square_ry = ry * ry;
    let square_x = transformed_midpoint.0 * transformed_midpoint.0;
    let square_y = transformed_midpoint.1 * transformed_midpoint.1;
    let radii_scale = square_x / square_rx + square_y / square_ry;
    if radii_scale > 1.0 {
        let scale = radii_scale.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let to_unit = SkiaAffine2::scale_pre_rotate(1.0 / rx, 1.0 / ry, -angle);
    let mut unit_start = to_unit.map(start);
    let mut unit_end = to_unit.map(end);
    let mut delta = (unit_end.0 - unit_start.0, unit_end.1 - unit_start.1);
    let distance = delta.0 * delta.0 + delta.1 * delta.1;
    let raw_scale_factor_squared = 1.0 / distance - 0.25;
    // `std::max(NaN, 0)` retains its first NaN argument; `f32::max` does not.
    let scale_factor_squared = if raw_scale_factor_squared < 0.0 {
        0.0
    } else {
        raw_scale_factor_squared
    };
    let mut scale_factor = scale_factor_squared.sqrt();
    if sweep == large {
        scale_factor = -scale_factor;
    }
    delta.0 *= scale_factor;
    delta.1 *= scale_factor;

    let mut center = (
        (unit_start.0 + unit_end.0) * 0.5,
        (unit_start.1 + unit_end.1) * 0.5,
    );
    center.0 += -delta.1;
    center.1 += delta.0;
    unit_start.0 -= center.0;
    unit_start.1 -= center.1;
    unit_end.0 -= center.0;
    unit_end.1 -= center.1;

    let theta1 = unit_start.1.atan2(unit_start.0);
    let theta2 = unit_end.1.atan2(unit_end.0);
    let mut theta_arc = theta2 - theta1;
    let tau = std::f32::consts::PI * 2.0;
    if theta_arc < 0.0 && sweep {
        theta_arc += tau;
    } else if theta_arc > 0.0 && !sweep {
        theta_arc -= tau;
    }

    if theta_arc.abs() < std::f32::consts::PI / 1_000_000.0 {
        return ArcResolution::Line;
    }
    // A non-finite angle reaches Skia's saturated segment count, then a
    // non-finite tangent, and returns before appending a conic.
    if !theta_arc.is_finite() {
        return ArcResolution::NoOp;
    }

    let segment_span = (2.0 * std::f32::consts::PI) / 3.0;
    let segments = (theta_arc.abs() / segment_span).ceil() as usize;
    debug_assert!((1..=3).contains(&segments));
    let theta_width = theta_arc / segments as f32;
    let tangent = (0.5 * theta_width).tan();
    if !tangent.is_finite() {
        return ArcResolution::NoOp;
    }

    let weight = (0.5 + theta_width.cos() * 0.5).sqrt();
    let expect_integers = (std::f32::consts::FRAC_PI_2 - theta_width.abs()).abs() <= 1.0 / 4096.0
        && rx == rx.floor()
        && ry == ry.floor()
        && end.0 == end.0.floor()
        && end.1 == end.1.floor();
    let from_unit = SkiaAffine2::rotate_pre_scale(angle, rx, ry);
    let mut start_theta = theta1;
    let mut conics = Vec::with_capacity(segments);
    for index in 0..segments {
        let end_theta = start_theta + theta_width;
        let (sin_end, cos_end) = skia_sin_cos_radians(end_theta);
        let mut unit_endpoint = (cos_end, sin_end);
        unit_endpoint.0 += center.0;
        unit_endpoint.1 += center.1;
        let mut unit_control = unit_endpoint;
        unit_control.0 += tangent * sin_end;
        unit_control.1 += -tangent * cos_end;

        let mut control = from_unit.map(unit_control);
        let mut endpoint = from_unit.map(unit_endpoint);
        if expect_integers {
            control = (skia_round(control.0), skia_round(control.1));
            endpoint = (skia_round(endpoint.0), skia_round(endpoint.1));
        }
        if index + 1 == segments {
            endpoint = end;
        }
        conics.push((control, endpoint, weight));
        start_theta = end_theta;
    }
    ArcResolution::Conics(conics)
}

fn skia_sin_cos(degrees: f32) -> (f32, f32) {
    skia_sin_cos_radians(degrees * (std::f32::consts::PI / 180.0))
}

fn skia_sin_cos_radians(radians: f32) -> (f32, f32) {
    let sin = radians.sin();
    let cos = radians.cos();
    (
        if sin.abs() <= 1.0 / 65_536.0 {
            0.0
        } else {
            sin
        },
        if cos.abs() <= 1.0 / 65_536.0 {
            0.0
        } else {
            cos
        },
    )
}

fn skia_round(value: f32) -> f32 {
    (f64::from(value) + 0.5).floor() as f32
}

fn finite_point(point: (f32, f32)) -> bool {
    point.0.is_finite() && point.1.is_finite()
}

#[cfg(test)]
mod marker_tests {
    use super::*;

    fn assert_position(actual: MarkerPosition, kind: MarkerType, origin: (f32, f32), angle: f32) {
        assert_eq!(actual.kind, kind);
        assert_eq!(actual.origin, origin);
        assert!(
            (actual.angle - angle).abs() <= 0.000_01,
            "expected angle {angle}, got {}",
            actual.angle
        );
    }

    #[test]
    fn one_scan_projects_raster_commands_and_authored_vertices() {
        let parsed = parse_path("M0 0L10 0L10 10");
        assert_eq!(
            parsed.commands,
            [
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::LineTo { x: 10.0, y: 0.0 },
                PathCommand::LineTo { x: 10.0, y: 10.0 },
            ]
        );
        let positions = parsed.marker_positions().expect("finite marker projection");
        assert_eq!(positions.len(), 3);
        assert_position(positions[0], MarkerType::Start, (0.0, 0.0), 0.0);
        assert_position(positions[1], MarkerType::Mid, (10.0, 0.0), 45.0);
        assert_position(positions[2], MarkerType::End, (10.0, 10.0), 90.0);
    }

    #[test]
    fn start_and_end_belong_to_the_whole_path_not_each_subpath() {
        let positions = parse_path("M0 0L10 0M20 0L20 10")
            .marker_positions()
            .expect("finite marker projection");
        assert_eq!(positions.len(), 4);
        assert_position(positions[0], MarkerType::Start, (0.0, 0.0), 0.0);
        assert_position(positions[1], MarkerType::Mid, (10.0, 0.0), 0.0);
        assert_position(positions[2], MarkerType::Mid, (20.0, 0.0), 90.0);
        assert_position(positions[3], MarkerType::End, (20.0, 10.0), 90.0);
    }

    #[test]
    fn close_backpatches_the_start_and_keeps_the_duplicate_vertex() {
        let positions = parse_path("M0 0L10 0L10 10Z")
            .marker_positions()
            .expect("finite marker projection");
        assert_eq!(positions.len(), 4);
        assert_position(positions[0], MarkerType::Start, (0.0, 0.0), -67.5);
        assert_position(positions[1], MarkerType::Mid, (10.0, 0.0), 45.0);
        assert_position(positions[2], MarkerType::Mid, (10.0, 10.0), 157.5);
        assert_position(positions[3], MarkerType::End, (0.0, 0.0), -67.5);
    }

    #[test]
    fn marker_only_shapes_survive_raster_geometry_elision() {
        let move_only = parse_path("M12 20");
        assert!(move_only.commands.is_empty());
        let positions = move_only
            .marker_positions()
            .expect("move-only marker projection");
        assert_eq!(positions.len(), 1);
        assert_position(
            positions[0],
            MarkerType::End,
            (12.0, 20.0),
            slope_degrees((12.0, 20.0)),
        );

        let polyline = points_marker_positions(&[(12.0, 20.0)], false);
        assert_eq!(polyline.len(), 1);
        assert_eq!(polyline[0].kind, MarkerType::End);

        let polygon = points_marker_positions(&[(12.0, 20.0)], true);
        assert_eq!(
            polygon
                .iter()
                .map(|position| position.kind)
                .collect::<Vec<_>>(),
            [MarkerType::Start, MarkerType::End]
        );
    }

    #[test]
    fn one_authored_arc_is_one_marker_segment_even_when_raster_decomposes() {
        let parsed = parse_path("M8 32A24 12 0 1 1 56 32");
        assert!(
            parsed.commands.len() > parsed.marker_elements.len(),
            "the raster conics may not become authored marker vertices"
        );
        assert_eq!(parsed.marker_elements.len(), 2);
        let MarkerPathElement::CubicTo { end, .. } = parsed.marker_elements[1] else {
            panic!("an authored arc must have one synthetic cubic marker segment")
        };
        assert!((end.0 - 56.0).abs() < 0.000_01 && (end.1 - 32.0).abs() < 0.000_01);
        let positions = parsed.marker_positions().expect("finite arc projection");
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].kind, MarkerType::Start);
        assert_eq!(positions[1].kind, MarkerType::End);
    }

    #[test]
    fn exact_opposite_tangents_take_blinks_lower_bisector_branch() {
        let positions = parse_path("M8 32L32 32L8 32")
            .marker_positions()
            .expect("finite marker projection");
        assert_position(positions[1], MarkerType::Mid, (32.0, 32.0), 270.0);
    }
}
