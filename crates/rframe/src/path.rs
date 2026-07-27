//! Resolved vector path geometry — the contract's second geometry kind.
//!
//! A producer resolves its own path syntax (SVG's `d` grammar, its relative
//! and shorthand commands, its arcs) and emits the **canonical absolute
//! command stream** here: every contour opens with an explicit move, every
//! coordinate is final and finite, and nothing carries source spelling. The
//! stream is deliberately narrower than the shapes a source can author:
//!
//! - **No arc command.** An SVG arc is authored syntax, not resolved geometry:
//!   every backend represents it as some sequence of curves, and *which*
//!   sequence decides the pixels. That choice belongs to the producer, which is
//!   the only party that can verify it against the browser it is matching.
//! - **No rational conics — yet.** The one producer that needs them is an SVG
//!   arc (Chromium rasterizes an arc through the same conics as an ellipse,
//!   measured), and it is not admitted yet. The contract grows when that
//!   producer arrives, not before.
//!
//! Construction is checked: [`PathData::new`] rejects a stream no resolved
//! path can be, and computes the tight local-space bounds once so producer and
//! consumer never compute them twice and disagree.

use std::sync::Arc;

use math2::Rectangle;

/// Which regions of a self-overlapping path the fill covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// One resolved path command in the node's local space, absolute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    /// Open a new contour at this point.
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    /// Quadratic Bézier through control point `(x1, y1)` to `(x, y)`.
    QuadTo {
        x1: f32,
        y1: f32,
        x: f32,
        y: f32,
    },
    /// Cubic Bézier through control points `(x1, y1)` and `(x2, y2)`.
    CubicTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x: f32,
        y: f32,
    },
    /// Close the open contour. A following drawing command needs its own
    /// explicit [`PathCommand::MoveTo`]; the producer resolves where the
    /// current point went.
    Close,
}

/// Why a command stream is not a resolved path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathDataError {
    /// No commands at all. A resolved path carries drawable geometry; a
    /// source that resolves to nothing is not a path node.
    Empty,
    /// A coordinate or control point is not finite.
    NonFiniteCoordinate { index: usize },
    /// A drawing or close command that does not continue an open contour —
    /// including a stream that does not begin with a move, a second close,
    /// and a drawing command after a close.
    NoOpenContour { index: usize },
    /// A contour opened here, never drew, and was never closed. Nothing can
    /// paint it — a bare move has no fill area, no stroke geometry, and no
    /// effect on the coverage of the rest of the path (measured) — so the
    /// producer resolves it away instead of carrying a visual fact that is not
    /// one.
    ///
    /// A contour that only moves *and then closes* is a different thing: it is
    /// a zero-length closed contour, it strokes as a cap-shaped dot, and it
    /// changes how the rest of the path is filled. Its canonical encoding is a
    /// move, a zero-length segment, and a close — which this type accepts.
    MoveOnlyContour { index: usize },
}

impl std::fmt::Display for PathDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathDataError::Empty => f.write_str("resolved path carries no commands"),
            PathDataError::NonFiniteCoordinate { index } => {
                write!(
                    f,
                    "resolved path command {index} has a non-finite coordinate"
                )
            }
            PathDataError::NoOpenContour { index } => write!(
                f,
                "resolved path command {index} does not continue an open contour"
            ),
            PathDataError::MoveOnlyContour { index } => write!(
                f,
                "resolved path contour opened at command {index} draws nothing"
            ),
        }
    }
}

impl std::error::Error for PathDataError {}

/// A checked resolved path: canonical absolute commands, the fill rule that
/// decides its interior, and the tight local-space bounds of its geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct PathData {
    commands: Arc<[PathCommand]>,
    fill_rule: FillRule,
    local_bounds: Rectangle,
    all_contours_closed: bool,
}

impl PathData {
    /// Check one canonical command stream and resolve its tight bounds.
    pub fn new(
        commands: impl Into<Arc<[PathCommand]>>,
        fill_rule: FillRule,
    ) -> Result<Self, PathDataError> {
        let commands = commands.into();
        let all_contours_closed = validate(&commands)?;
        let local_bounds = tight_bounds(&commands);
        Ok(Self {
            commands,
            fill_rule,
            local_bounds,
            all_contours_closed,
        })
    }

    #[must_use]
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    #[must_use]
    pub const fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// The tight axis-aligned bounds of the geometry, in local space. Curve
    /// extrema are solved, not approximated by the control hull, so this is
    /// the box the ink actually occupies — the node's `bounds` is exactly
    /// this box mapped through its transform.
    #[must_use]
    pub const fn local_bounds(&self) -> Rectangle {
        self.local_bounds
    }

    /// Whether every contour ends with an explicit [`PathCommand::Close`].
    ///
    /// Invisible to a fill — an open contour fills as though closed — and
    /// decisive for a stroke, where a closed contour joins its ends and an
    /// open one caps them. Resolved here so no consumer re-derives it.
    #[must_use]
    pub const fn all_contours_closed(&self) -> bool {
        self.all_contours_closed
    }
}

/// Every contour opens with a move and draws at least once, and every
/// coordinate is finite. A producer that cannot honor this has not finished
/// resolving its source. Returns whether every contour closed explicitly.
fn validate(commands: &[PathCommand]) -> Result<bool, PathDataError> {
    if commands.is_empty() {
        return Err(PathDataError::Empty);
    }
    let mut open_at: Option<usize> = None;
    let mut drew = false;
    let mut all_contours_closed = true;
    for (index, command) in commands.iter().enumerate() {
        if !coordinates(command).iter().all(|value| value.is_finite()) {
            return Err(PathDataError::NonFiniteCoordinate { index });
        }
        match command {
            PathCommand::MoveTo { .. } => {
                if let Some(opened) = open_at {
                    if !drew {
                        return Err(PathDataError::MoveOnlyContour { index: opened });
                    }
                    // The previous contour drew and never closed.
                    all_contours_closed = false;
                }
                open_at = Some(index);
                drew = false;
            }
            PathCommand::LineTo { .. }
            | PathCommand::QuadTo { .. }
            | PathCommand::CubicTo { .. } => {
                if open_at.is_none() {
                    return Err(PathDataError::NoOpenContour { index });
                }
                drew = true;
            }
            PathCommand::Close => {
                if open_at.is_none() || !drew {
                    return Err(PathDataError::NoOpenContour { index });
                }
                open_at = None;
                drew = false;
            }
        }
    }
    match open_at {
        Some(opened) if !drew => Err(PathDataError::MoveOnlyContour { index: opened }),
        // A last contour that drew and never closed.
        Some(_) => Ok(false),
        None => Ok(all_contours_closed),
    }
}

/// Every f32 a command carries, in authored order.
fn coordinates(command: &PathCommand) -> Vec<f32> {
    match *command {
        PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => vec![x, y],
        PathCommand::QuadTo { x1, y1, x, y } => vec![x1, y1, x, y],
        PathCommand::CubicTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => vec![x1, y1, x2, y2, x, y],
        PathCommand::Close => Vec::new(),
    }
}

/// The tight bounds of a validated stream: the union of every segment's own
/// extent, with curve extrema solved. `Close` adds no extent — it returns to
/// a point the contour already contributed.
fn tight_bounds(commands: &[PathCommand]) -> Rectangle {
    let mut boxes: Vec<Rectangle> = Vec::with_capacity(commands.len());
    let mut current = [0.0f32, 0.0f32];
    for command in commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                current = [x, y];
                boxes.push(Rectangle::from_points(&[current]));
            }
            PathCommand::LineTo { x, y } => {
                let end = [x, y];
                boxes.push(Rectangle::from_points(&[current, end]));
                current = end;
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                let end = [x, y];
                boxes.push(quad_bounds(current, [x1, y1], end));
                current = end;
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let end = [x, y];
                boxes.push(math2::bezier_get_bbox(&math2::CubicBezierWithTangents {
                    a: current,
                    b: end,
                    ta: [x1 - current[0], y1 - current[1]],
                    tb: [x2 - end[0], y2 - end[1]],
                }));
                current = end;
            }
            PathCommand::Close => {}
        }
    }
    math2::union(&boxes)
}

/// A quadratic's tight extent: its endpoints plus the interior stationary
/// point of each axis, solved from the derivative rather than approximated by
/// the control point (which lies outside the curve).
fn quad_bounds(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2]) -> Rectangle {
    let mut points = vec![p0, p2];
    for axis in 0..2 {
        let denominator = p0[axis] - 2.0 * p1[axis] + p2[axis];
        if denominator == 0.0 {
            continue;
        }
        let t = (p0[axis] - p1[axis]) / denominator;
        if t > 0.0 && t < 1.0 {
            let mt = 1.0 - t;
            let mut point = [0.0f32, 0.0f32];
            for component in 0..2 {
                point[component] =
                    mt * mt * p0[component] + 2.0 * mt * t * p1[component] + t * t * p2[component];
            }
            points.push(point);
        }
    }
    Rectangle::from_points(&points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_canonical_stream_resolves_its_tight_bounds() {
        let path = PathData::new(
            vec![
                PathCommand::MoveTo { x: 10.0, y: 10.0 },
                PathCommand::LineTo { x: 30.0, y: 10.0 },
                PathCommand::Close,
            ],
            FillRule::NonZero,
        )
        .expect("canonical stream");
        assert_eq!(
            path.local_bounds(),
            Rectangle::from_xywh(10.0, 10.0, 20.0, 0.0)
        );
        assert_eq!(path.fill_rule(), FillRule::NonZero);
        assert_eq!(path.commands().len(), 3);
    }

    /// The control point of a curve lies outside it, so the resolved bounds
    /// must be the solved extent, not the control hull.
    #[test]
    fn curve_bounds_are_the_solved_extent_not_the_control_hull() {
        let quad = PathData::new(
            vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::QuadTo {
                    x1: 50.0,
                    y1: 100.0,
                    x: 100.0,
                    y: 0.0,
                },
            ],
            FillRule::NonZero,
        )
        .expect("canonical quad");
        // The apex of this symmetric quadratic is at half the control height.
        assert_eq!(
            quad.local_bounds(),
            Rectangle::from_xywh(0.0, 0.0, 100.0, 50.0)
        );

        let cubic = PathData::new(
            vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::CubicTo {
                    x1: 0.0,
                    y1: 100.0,
                    x2: 100.0,
                    y2: 100.0,
                    x: 100.0,
                    y: 0.0,
                },
            ],
            FillRule::NonZero,
        )
        .expect("canonical cubic");
        assert_eq!(
            cubic.local_bounds(),
            Rectangle::from_xywh(0.0, 0.0, 100.0, 75.0)
        );
    }

    #[test]
    fn a_stream_that_no_resolved_path_can_be_is_refused_by_index() {
        assert_eq!(
            PathData::new(Vec::new(), FillRule::NonZero),
            Err(PathDataError::Empty)
        );
        assert_eq!(
            PathData::new(
                vec![PathCommand::LineTo { x: 1.0, y: 1.0 }],
                FillRule::NonZero
            ),
            Err(PathDataError::NoOpenContour { index: 0 }),
            "a stream must open a contour before it draws"
        );
        assert_eq!(
            PathData::new(
                vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::LineTo { x: 1.0, y: 1.0 },
                    PathCommand::Close,
                    PathCommand::LineTo { x: 2.0, y: 2.0 },
                ],
                FillRule::NonZero
            ),
            Err(PathDataError::NoOpenContour { index: 3 }),
            "after a close the producer resolves the next contour's own move"
        );
        assert_eq!(
            PathData::new(
                vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::LineTo { x: 1.0, y: 1.0 },
                    PathCommand::Close,
                    PathCommand::Close,
                ],
                FillRule::NonZero
            ),
            Err(PathDataError::NoOpenContour { index: 3 }),
            "a second close has nothing to close"
        );
        assert_eq!(
            PathData::new(
                vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::MoveTo { x: 1.0, y: 1.0 },
                    PathCommand::LineTo { x: 2.0, y: 2.0 },
                ],
                FillRule::NonZero
            ),
            Err(PathDataError::MoveOnlyContour { index: 0 }),
            "a contour that draws nothing is the producer's to drop"
        );
        assert_eq!(
            PathData::new(
                vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::LineTo { x: 1.0, y: 1.0 },
                    PathCommand::MoveTo { x: 2.0, y: 2.0 },
                ],
                FillRule::NonZero
            ),
            Err(PathDataError::MoveOnlyContour { index: 2 }),
            "including a trailing one"
        );
        assert_eq!(
            PathData::new(
                vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::LineTo {
                        x: f32::INFINITY,
                        y: 1.0
                    },
                ],
                FillRule::NonZero
            ),
            Err(PathDataError::NonFiniteCoordinate { index: 1 })
        );
    }
}
