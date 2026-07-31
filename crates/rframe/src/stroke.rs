//! The resolved stroke — a second painted fact per node, beside the fill.
//!
//! A stroke is carried, not derived: the producer resolves the paint, the width
//! in the node's local space, and the three shapes a corner or an end can take,
//! and the consumer paints exactly that.
//!
//! **Centred, with no alignment field.** A Web stroke straddles its geometry:
//! half the width falls inside the outline and half outside. That is the only
//! alignment any Web source can express, so the contract does not carry one;
//! an inside- or outside-aligned stroke would grow this type when a producer
//! that needs it arrives.
//!
//! **In local space.** The width is a length in the same space as the geometry,
//! so the node's transform scales it — including non-uniformly, which turns the
//! pen elliptical. That is what a browser does (measured: `scale(2,1)` on a
//! stroked rect yields a stroke twice as wide horizontally as vertically), and
//! it follows from transforming the stroke *outline* rather than the width.
//!
//! **An invisible stroke is not a stroke.** Construction *resolves* a width or
//! a paint stack that cannot paint to `Ok(None)` — not an error, because
//! painting nothing is a perfectly good answer — so a node's `Option<Stroke>`
//! is `None` whenever nothing would be drawn and no consumer re-derives that.
//! It errors only for a value no stroke can have: a negative or non-finite
//! width, a negative or non-finite miter limit, or a reach that cannot be
//! represented.

use crate::frame::PaintStack;

/// The shape of a stroked contour's open ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeCap {
    /// Ends exactly at the endpoint, adding nothing.
    #[default]
    Butt,
    /// A half-disc of the stroke's own radius.
    Round,
    /// A half-square extending by the stroke's radius.
    Square,
}

/// The shape of a stroked contour's corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeJoin {
    /// Extend both edges to their intersection, subject to
    /// [`Stroke::miter_limit`].
    #[default]
    Miter,
    /// An arc of the stroke's own radius.
    Round,
    /// Cut the corner off straight.
    Bevel,
}

/// Why a resolved stroke is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeError {
    /// The width is negative or not finite. A width of *zero* is not an error
    /// — it paints nothing, which the node carries as `None`.
    InvalidWidth,
    /// The miter limit is not finite and non-negative.
    InvalidMiterLimit,
    /// The width is finite but how far the stroke reaches
    /// ([`Stroke::outset`]) is not, so no consumer could state the area it
    /// covers.
    UnrepresentableReach,
}

impl std::fmt::Display for StrokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrokeError::InvalidWidth => {
                f.write_str("resolved stroke width must be finite and positive")
            }
            StrokeError::InvalidMiterLimit => {
                f.write_str("resolved stroke miter limit must be finite and non-negative")
            }
            StrokeError::UnrepresentableReach => {
                f.write_str("resolved stroke reaches beyond a representable distance")
            }
        }
    }
}

impl std::error::Error for StrokeError {}

/// One resolved stroke: what to paint, how wide, and what its corners and ends
/// look like.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    paints: PaintStack,
    width: f32,
    cap: StrokeCap,
    join: StrokeJoin,
    miter_limit: f32,
}

impl Stroke {
    /// Resolve one stroke, or `None` when nothing would be painted.
    ///
    /// The miter limit is carried as resolved, including a value below 1, which
    /// no miter can satisfy — a backend turns that into a bevel, and choosing
    /// differently here would diverge from the browser this contract's producer
    /// is matching.
    pub fn new(
        paints: PaintStack,
        width: f32,
        cap: StrokeCap,
        join: StrokeJoin,
        miter_limit: f32,
    ) -> Result<Option<Self>, StrokeError> {
        if !miter_limit.is_finite() || miter_limit < 0.0 {
            return Err(StrokeError::InvalidMiterLimit);
        }
        if !width.is_finite() || width < 0.0 {
            return Err(StrokeError::InvalidWidth);
        }
        if width == 0.0 || paints.is_empty() {
            return Ok(None);
        }
        let stroke = Self {
            paints,
            width,
            cap,
            join,
            miter_limit,
        };
        // A width can be finite while its *reach* is not: half of `f32::MAX`
        // times a miter limit of 4 overflows. A consumer that cannot compute
        // the covered area cannot honestly carry the stroke, and an
        // unrepresentable coverage rectangle would reach the damage policy as
        // an infinity rather than as a refusal.
        if !stroke.outset().is_finite() {
            return Err(StrokeError::UnrepresentableReach);
        }
        Ok(Some(stroke))
    }

    #[must_use]
    pub const fn paints(&self) -> &PaintStack {
        &self.paints
    }

    /// The stroke width, in the node's local space.
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn cap(&self) -> StrokeCap {
        self.cap
    }

    #[must_use]
    pub const fn join(&self) -> StrokeJoin {
        self.join
    }

    #[must_use]
    pub const fn miter_limit(&self) -> f32 {
        self.miter_limit
    }

    /// How far the stroke can reach outside the geometry it follows, in local
    /// space. Consumers need this to know what a stroked node covers: the
    /// node's `bounds` is its *geometry*, and ink lies outside it.
    ///
    /// Two independent reaches, whichever is larger — a corner and an end are
    /// different places and either can be the farthest:
    ///
    /// - the **join**: a miter's tip reaches `miter_limit` half-widths from the
    ///   corner when the backend admits it and is cut back to the bevel
    ///   otherwise, so the limit bounds it either way; round and bevel reach
    ///   exactly the radius.
    /// - the **cap**: a square cap's two far corners sit at `radius · √2` from
    ///   the endpoint (offset `radius` along the travel direction *and*
    ///   `radius` across it), so on a segment that is not axis-aligned it
    ///   reaches farther than the radius — measured: a 16-wide square-capped
    ///   segment ending at (44,44) inks out to x=55, which is `44 + 8√2`.
    ///   Butt and round caps reach the radius.
    ///
    /// This is a direction-free uniform inflation, so `radius · √2` is the
    /// tight bound for the square cap rather than a safety margin.
    #[must_use]
    pub fn outset(&self) -> f32 {
        let radius = self.width / 2.0;
        let join = match self.join {
            StrokeJoin::Miter => self.miter_limit.max(1.0),
            StrokeJoin::Round | StrokeJoin::Bevel => 1.0,
        };
        let cap = match self.cap {
            StrokeCap::Square => std::f32::consts::SQRT_2,
            StrokeCap::Butt | StrokeCap::Round => 1.0,
        };
        radius * join.max(cap)
    }
}

#[cfg(test)]
mod tests {
    use cg::CGColor;

    use super::*;

    fn black() -> PaintStack {
        PaintStack::solid(CGColor::from_rgb(0, 0, 0))
    }

    #[test]
    fn a_stroke_that_paints_nothing_is_not_a_stroke() {
        assert_eq!(
            Stroke::new(black(), 0.0, StrokeCap::Butt, StrokeJoin::Miter, 4.0),
            Ok(None),
            "a zero width paints nothing"
        );
        assert_eq!(
            Stroke::new(
                PaintStack::empty(),
                8.0,
                StrokeCap::Butt,
                StrokeJoin::Miter,
                4.0
            ),
            Ok(None),
            "no paint paints nothing"
        );
        assert_eq!(
            Stroke::new(
                PaintStack::solid(CGColor::TRANSPARENT),
                8.0,
                StrokeCap::Butt,
                StrokeJoin::Miter,
                4.0
            ),
            Ok(None),
            "a fully transparent paint normalizes away before it reaches here"
        );
    }

    #[test]
    fn an_unpaintable_stroke_refuses_by_name() {
        assert_eq!(
            Stroke::new(black(), -8.0, StrokeCap::Butt, StrokeJoin::Miter, 4.0),
            Err(StrokeError::InvalidWidth)
        );
        assert_eq!(
            Stroke::new(
                black(),
                f32::INFINITY,
                StrokeCap::Butt,
                StrokeJoin::Miter,
                4.0
            ),
            Err(StrokeError::InvalidWidth)
        );
        assert_eq!(
            Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Miter, f32::NAN),
            Err(StrokeError::InvalidMiterLimit)
        );
        assert_eq!(
            Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Miter, -1.0),
            Err(StrokeError::InvalidMiterLimit)
        );
    }

    /// A miter limit below 1 is carried, not corrected: it is what the browser
    /// resolved, and the backend turns it into a bevel.
    #[test]
    fn a_miter_limit_below_one_is_carried_as_resolved() {
        let stroke = Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Miter, 0.5)
            .expect("valid")
            .expect("paints");
        assert_eq!(stroke.miter_limit(), 0.5);
        assert_eq!(
            stroke.outset(),
            4.0,
            "and it never shrinks the covered area below the bevel's"
        );
    }

    #[test]
    fn the_outset_bounds_the_join_and_the_cap() {
        let miter = Stroke::new(black(), 8.0, StrokeCap::Butt, StrokeJoin::Miter, 4.0)
            .expect("valid")
            .expect("paints");
        assert_eq!(miter.outset(), 16.0, "four half-widths at the limit");
        for join in [StrokeJoin::Round, StrokeJoin::Bevel] {
            let stroke = Stroke::new(black(), 8.0, StrokeCap::Butt, join, 4.0)
                .expect("valid")
                .expect("paints");
            assert_eq!(stroke.outset(), 4.0, "{join:?} reaches the radius");
        }
        // A square cap reaches farther than the radius, so a round join does
        // not bound it — the reach is the larger of the two, not the join's.
        let square = Stroke::new(black(), 8.0, StrokeCap::Square, StrokeJoin::Round, 4.0)
            .expect("valid")
            .expect("paints");
        assert_eq!(square.outset(), 4.0 * std::f32::consts::SQRT_2);
        assert!(
            square.outset() > 4.0,
            "the corner of a square cap is outside the radius"
        );
        // And a generous miter still wins when it is the larger reach.
        let both = Stroke::new(black(), 8.0, StrokeCap::Square, StrokeJoin::Miter, 4.0)
            .expect("valid")
            .expect("paints");
        assert_eq!(both.outset(), 16.0);
    }

    /// A width can be finite while its reach is not. The contract refuses that
    /// rather than handing a consumer an infinite coverage rectangle.
    #[test]
    fn a_width_whose_reach_cannot_be_represented_refuses() {
        assert_eq!(
            Stroke::new(
                black(),
                f32::MAX,
                StrokeCap::Butt,
                StrokeJoin::Miter,
                f32::MAX
            ),
            Err(StrokeError::UnrepresentableReach)
        );
        // The same width with a reach that *is* representable is admitted, so
        // the refusal is about the reach and not about the magnitude.
        assert!(
            Stroke::new(black(), f32::MAX, StrokeCap::Butt, StrokeJoin::Round, 4.0)
                .expect("valid")
                .is_some()
        );
    }
}
