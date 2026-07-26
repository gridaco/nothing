//! The SVG `d` grammar → the resolved contract's canonical command stream.
//!
//! The frozen donor (`crates/htmlcss/src/svg/dom/path_d.rs`) supplied the
//! question list — which commands exist, what the shorthands reflect, how an
//! arc becomes cubics. Every *answer* here was measured against Chromium 149,
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
//! optional exponent that must carry a digit — then a finite `f32`. The
//! digit-after-the-dot requirement is the one worth naming: `M10. 10 …` renders
//! nothing in Chromium, so a trailing dot is invalid here too. (SVG 1.1's
//! grammar admitted `digit-sequence "."`; whatever any grammar says, the
//! browser is what this compiler matches.) `1e40` overflows to non-finite and
//! is an error; `1e30` is fine.
//!
//! ## Arcs refuse, and the measurement says why
//! `A`/`a` parses and then refuses by name. Blink's path *normalizer*
//! (`core/svg/svg_path_normalizer.cc`) decomposes an arc into cubics, and
//! following it produces a curve that is not what Chromium paints: the same
//! cubics, authored explicitly and rendered *by Chromium itself*, differ from
//! Chromium's `A` by 77 pixels at up to a 170-per-channel delta. What Chromium
//! paints instead is measurable — the half-ellipse arc
//! `M8 28 A24 20 0 0 1 56 28 Z` is **byte-identical** to
//! `<ellipse cx="32" cy="28" rx="24" ry="20">` over every row they share — so
//! an arc reaches the rasterizer as the ellipse's rational **conics**, which
//! no cubic can equal. Emitting conics is the arc rung's job (the contract has
//! no conic command yet, and the engine's oval tolerance is the class such a
//! curve would land in); until then the arc is a declared hole, never a
//! visibly different curve.
//!
//! ## Errors: the one deliberate divergence
//! Chromium renders an erroneous path's **valid prefix** (SVG2 §9.3.9) and
//! drops the rest. This slice refuses the whole path by name instead, so the
//! shape becomes one declared hole rather than an unbaked partial geometry.
//! Where the prefix is empty — no leading `moveto`, an error in the first
//! segment — the two agree exactly, because both paint nothing. Admitting the
//! prefix rule is a rung of its own, with its own fixtures.
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

use rframe::PathCommand;

/// Why a `d` value did not resolve to a command stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathDataError {
    /// The value stopped being valid SVG path data at this byte offset.
    /// Sufficient to excerpt the offending text without carrying a kilobyte of
    /// `d` in an error.
    Syntax { offset: usize },
    /// A command whose grammar parses but whose geometry this slice does not
    /// emit. Distinct from [`Self::Syntax`] on purpose: the document is
    /// correct and the engine is not finished.
    UnsupportedCommand { command: char },
}

/// Parse one `d` value into the canonical absolute command stream.
///
/// An empty (or whitespace-only) value is valid and resolves to no commands —
/// SVG's `d` initial value is `none`, which renders nothing.
pub(crate) fn parse_path_data(d: &str) -> Result<Vec<PathCommand>, PathDataError> {
    Parser::new(d).parse()
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
}

impl<'a> Parser<'a> {
    fn new(d: &'a str) -> Self {
        Self {
            bytes: d.as_bytes(),
            at: 0,
            commands: Vec::new(),
            current: (0.0, 0.0),
            subpath_start: (0.0, 0.0),
            open: None,
            last_cubic: None,
            last_quad: None,
        }
    }

    fn error<T>(&self) -> Result<T, PathDataError> {
        Err(PathDataError::Syntax { offset: self.at })
    }

    fn parse(mut self) -> Result<Vec<PathCommand>, PathDataError> {
        self.skip_wsp();
        if self.at >= self.bytes.len() {
            return Ok(Vec::new());
        }
        let mut first = true;
        while self.at < self.bytes.len() {
            let command = self.bytes[self.at];
            if !command.is_ascii_alphabetic() {
                return self.error();
            }
            // SVG2 §9.3.4: path data must begin with a moveto. Chromium
            // renders nothing at all otherwise (an empty valid prefix).
            if first && !matches!(command, b'M' | b'm') {
                return self.error();
            }
            first = false;
            self.at += 1;
            self.command(command)?;
            self.skip_wsp();
        }
        // A trailing contour that only moved contributes nothing.
        if let Some((index, false)) = self.open {
            debug_assert_eq!(index + 1, self.commands.len());
            self.commands.truncate(index);
        }
        Ok(self.commands)
    }

    /// One command letter and its complete argument list, including every
    /// implicit repeat of it.
    fn command(&mut self, command: u8) -> Result<(), PathDataError> {
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
                    // The whole argument list is parsed before refusing, so a
                    // malformed arc is reported as malformed rather than as
                    // unsupported.
                    let _rx = self.number()?;
                    let _ry = self.number()?;
                    let _angle = self.number()?;
                    let _large = self.flag()?;
                    let _sweep = self.flag()?;
                    let _end = self.coordinate_pair()?;
                    return Err(PathDataError::UnsupportedCommand {
                        command: char::from(command),
                    });
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
            Some((x, y)) => (2.0 * self.current.0 - x, 2.0 * self.current.1 - y),
            None => self.current,
        }
    }

    fn reset_reflection(&mut self) {
        self.last_cubic = None;
        self.last_quad = None;
    }

    // ─── emission (canonical form) ────────────────────────────────────────

    fn emit_move(&mut self, point: (f32, f32)) {
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
        self.open_contour();
        self.commands.push(PathCommand::LineTo {
            x: point.0,
            y: point.1,
        });
        self.drew();
        self.current = point;
    }

    fn emit_quad(&mut self, one: (f32, f32), end: (f32, f32)) {
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

    fn coordinate_pair(&mut self) -> Result<(f32, f32), PathDataError> {
        let x = self.number()?;
        let y = self.number()?;
        Ok((x, y))
    }

    fn number(&mut self) -> Result<f32, PathDataError> {
        self.skip_wsp();
        let start = self.at;
        if self.at < self.bytes.len() && matches!(self.bytes[self.at], b'+' | b'-') {
            self.at += 1;
        }
        let integer_digits = self.digits();
        let mut fraction_digits = 0;
        if self.at < self.bytes.len() && self.bytes[self.at] == b'.' {
            self.at += 1;
            fraction_digits = self.digits();
            // Blink requires a digit after the dot even though SVG's BNF
            // permits `10.`; a trailing dot renders nothing.
            if fraction_digits == 0 {
                self.at = start;
                return self.error();
            }
        }
        if integer_digits == 0 && fraction_digits == 0 {
            self.at = start;
            return self.error();
        }
        if self.at < self.bytes.len() && matches!(self.bytes[self.at], b'e' | b'E') {
            let exponent_at = self.at;
            self.at += 1;
            if self.at < self.bytes.len() && matches!(self.bytes[self.at], b'+' | b'-') {
                self.at += 1;
            }
            if self.digits() == 0 {
                self.at = exponent_at;
                return self.error();
            }
        }
        let token = &self.bytes[start..self.at];
        let parsed = std::str::from_utf8(token)
            .ok()
            .and_then(|token| token.parse::<f32>().ok())
            .filter(|value: &f32| value.is_finite());
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
    fn flag(&mut self) -> Result<bool, PathDataError> {
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
