//! The resolved stroke — a second painted fact per node, beside the fill.
//!
//! A stroke is carried, not derived: the producer resolves the paint, the width
//! in one declared construction space, the three shapes a corner or an end can
//! take, and an optional dash pattern, and the consumer paints exactly that.
//!
//! **Centred, with no alignment field.** A Web stroke straddles its geometry:
//! half the width falls inside the outline and half outside. That is the only
//! alignment any Web source can express, so the contract does not carry one;
//! an inside- or outside-aligned stroke would grow this type when a producer
//! that needs it arrives.
//!
//! **Construction space is explicit.** An ordinary stroke is widened and
//! dashed in the geometry's local space before the node transform, so a
//! non-uniform transform turns its pen elliptical. A frame-space stroke first
//! maps the centerline through the node transform, then applies the same scalar
//! width, dash intervals, and phase in resolved frame coordinates. The host
//! view remains downstream of both meanings; this contract never bakes a
//! camera into a frame.
//!
//! **An invisible stroke is not a stroke.** Construction *resolves* a width or
//! a paint stack that cannot paint to `Ok(None)` — not an error, because
//! painting nothing is a perfectly good answer — so a node's `Option<Stroke>`
//! is `None` whenever nothing would be drawn and no consumer re-derives that.
//! It errors only for a value no stroke can have: a negative or non-finite
//! width or a negative or non-finite miter limit. A butt-capped dash cycle
//! whose painted intervals are all zero is invisible too; round and square
//! caps can paint dots at those same zero-length intervals, so those strokes
//! remain present.

use crate::frame::{PaintAlphaFactor, PaintStack};

/// The coordinate system in which a stroke is dashed and widened.
///
/// This is a source-neutral resolved fact. It names neither SVG syntax nor a
/// backend strategy: any producer may require a stroke whose construction is
/// fixed before or after the node's local-to-frame mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrokeSpace {
    /// Construct the stroke around local geometry, then apply the node
    /// transform to the complete outline.
    #[default]
    Local,
    /// Apply the node transform to the centerline, then construct the stroke
    /// in resolved frame coordinates. A later host view still applies.
    Frame,
}

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

/// Why an interval sequence is not one resolved stroke dash cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeDashIntervalsError {
    /// A present cycle must contain at least one painted-gap pair. Absence,
    /// rather than an empty present value, states a solid stroke.
    Empty,
    /// A resolved cycle contains complete painted-gap pairs. Repetition of an
    /// odd authored list belongs to the producer and has already happened.
    OddIntervalCount { count: usize },
    /// Every interval is a finite distance in its stroke's construction space.
    NonFiniteInterval { index: usize },
    /// Every interval is non-negative. Zero-length intervals remain
    /// meaningful under round and square caps.
    NegativeInterval { index: usize },
    /// The intervals are individually finite, but their sum is not finite in
    /// `f32`, so a consumer could not represent the repeating cycle.
    UnrepresentableCycleLength,
}

impl std::fmt::Display for StrokeDashIntervalsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrokeDashIntervalsError::Empty => {
                f.write_str("resolved stroke dash intervals must not be empty")
            }
            StrokeDashIntervalsError::OddIntervalCount { count } => write!(
                f,
                "resolved stroke dash intervals must contain an even number of entries, got {count}"
            ),
            StrokeDashIntervalsError::NonFiniteInterval { index } => {
                write!(f, "resolved stroke dash interval {index} must be finite")
            }
            StrokeDashIntervalsError::NegativeInterval { index } => write!(
                f,
                "resolved stroke dash interval {index} must be non-negative"
            ),
            StrokeDashIntervalsError::UnrepresentableCycleLength => f.write_str(
                "resolved stroke dash interval cycle length must be finite and positive",
            ),
        }
    }
}

impl std::error::Error for StrokeDashIntervalsError {}

/// One checked stroke dash interval cycle.
///
/// The immutable intervals are path distances in their owning stroke's
/// declared construction space. They alternate painted, unpainted, painted,
/// unpainted, beginning with paint, and the cycle restarts at the beginning of
/// every contour. A present cycle is non-empty and even-length; every interval
/// is finite and non-negative; and their `f32` sum is finite and positive.
///
/// Source syntax does not cross this type. A producer has already resolved
/// units, percentages, and odd-list repetition. This type deliberately owns
/// neither a phase nor a path-calibration fact; [`StrokeDash`] pairs it with a
/// resolved phase when one is present.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeDashIntervals {
    intervals: Box<[f32]>,
}

impl StrokeDashIntervals {
    /// Check one resolved interval cycle.
    ///
    /// An all-zero cycle normalizes to `Ok(None)`: dash absence is the one
    /// spelling of a solid stroke. Empty input is invalid because a present
    /// value may not provide a second spelling for absence.
    pub fn new(intervals: Vec<f32>) -> Result<Option<Self>, StrokeDashIntervalsError> {
        if intervals.is_empty() {
            return Err(StrokeDashIntervalsError::Empty);
        }
        if !intervals.len().is_multiple_of(2) {
            return Err(StrokeDashIntervalsError::OddIntervalCount {
                count: intervals.len(),
            });
        }

        for (index, interval) in intervals.iter().copied().enumerate() {
            if !interval.is_finite() {
                return Err(StrokeDashIntervalsError::NonFiniteInterval { index });
            }
            if interval < 0.0 {
                return Err(StrokeDashIntervalsError::NegativeInterval { index });
            }
        }

        let cycle_length = intervals.iter().copied().sum::<f32>();
        if !cycle_length.is_finite() {
            return Err(StrokeDashIntervalsError::UnrepresentableCycleLength);
        }
        if cycle_length == 0.0 {
            return Ok(None);
        }

        Ok(Some(Self {
            intervals: intervals.into_boxed_slice(),
        }))
    }

    /// The alternating painted and unpainted construction-space distances.
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.intervals
    }

    fn has_positive_painted_interval(&self) -> bool {
        self.intervals
            .chunks_exact(2)
            .any(|paint_and_gap| paint_and_gap[0] > 0.0)
    }

    fn cycle_length(&self) -> f32 {
        self.intervals.iter().copied().sum()
    }
}

/// Why a checked dash cycle cannot be paired with one resolved phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeDashError {
    /// A resolved construction-space phase must be finite before it can be
    /// reduced into the finite cycle.
    NonFinitePhase,
}

impl std::fmt::Display for StrokeDashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrokeDashError::NonFinitePhase => {
                f.write_str("resolved stroke dash phase must be finite")
            }
        }
    }
}

impl std::error::Error for StrokeDashError {}

/// One checked, source-neutral stroke dash pattern.
///
/// The intervals and phase are path distances in their owning stroke's
/// declared construction space. At contour distance `s`, the alternating
/// cycle is observed at `s + phase`: a positive phase advances into the cycle.
/// The same phase restarts at the beginning of every contour.
///
/// Construction is the sole normalization owner. It reduces every finite
/// phase modulo the positive cycle length into the canonical half-open range
/// `[0, cycle_length)`, so periodically equivalent patterns compare equal and
/// no consumer needs to reinterpret a signed or multi-cycle phase. The
/// contract carries no source syntax, percentage bases, transforms, or
/// path-length calibration.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeDash {
    intervals: StrokeDashIntervals,
    phase: f32,
}

impl StrokeDash {
    /// Pair a checked positive cycle with one finite construction-space phase.
    pub fn new(intervals: StrokeDashIntervals, phase: f32) -> Result<Self, StrokeDashError> {
        if !phase.is_finite() {
            return Err(StrokeDashError::NonFinitePhase);
        }

        let cycle_length = intervals.cycle_length();
        let phase = phase.rem_euclid(cycle_length);
        // Floating-point `rem_euclid` may round a result at the upper edge to
        // the divisor. Keep the public invariant strictly half-open, and erase
        // negative zero as the second spelling of zero phase.
        let phase = if phase == 0.0 || phase >= cycle_length {
            0.0
        } else {
            phase
        };

        Ok(Self { intervals, phase })
    }

    const fn zero_phase(intervals: StrokeDashIntervals) -> Self {
        Self {
            intervals,
            phase: 0.0,
        }
    }

    /// The checked alternating interval cycle.
    #[must_use]
    pub const fn intervals(&self) -> &StrokeDashIntervals {
        &self.intervals
    }

    /// The canonical construction-space phase in `[0, cycle_length)`.
    #[must_use]
    pub const fn phase(&self) -> f32 {
        self.phase
    }
}

/// Why a resolved stroke is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeError {
    /// The width is negative or not finite. A width of *zero* is not an error
    /// — it paints nothing, which the node carries as `None`.
    InvalidWidth,
    /// The miter limit is not finite and non-negative.
    InvalidMiterLimit,
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
    space: StrokeSpace,
    cap: StrokeCap,
    join: StrokeJoin,
    miter_limit: f32,
    dash: Option<StrokeDash>,
}

impl Stroke {
    /// Resolve one stroke, or `None` when nothing would be painted.
    ///
    /// This compatibility constructor always creates a solid stroke. Use
    /// [`Stroke::new_with_dash`] for a checked dash pattern;
    /// [`Stroke::new_with_dash_intervals`] remains the zero-phase compatibility
    /// spelling.
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
        Self::new_with_dash_intervals(paints, width, cap, join, miter_limit, None)
    }

    /// Resolve one optionally dashed stroke, or `None` when nothing would be
    /// painted.
    ///
    /// Dash absence states a solid stroke. A present cycle has already been
    /// checked by [`StrokeDashIntervals::new`]; this constructor never accepts
    /// raw intervals and therefore cannot bypass that validation. It is the
    /// zero-phase compatibility spelling; use [`Stroke::new_with_dash`] to
    /// carry a nonzero phase.
    pub fn new_with_dash_intervals(
        paints: PaintStack,
        width: f32,
        cap: StrokeCap,
        join: StrokeJoin,
        miter_limit: f32,
        dash_intervals: Option<StrokeDashIntervals>,
    ) -> Result<Option<Self>, StrokeError> {
        let dash = dash_intervals.map(StrokeDash::zero_phase);
        Self::new_with_dash(paints, width, cap, join, miter_limit, dash)
    }

    /// Resolve one stroke with an optional checked dash pattern, or `None`
    /// when nothing would be painted.
    ///
    /// Dash absence states a solid stroke. A present value pairs a checked
    /// positive interval cycle with its canonical construction-space phase, so
    /// phase cannot exist without a cycle.
    pub fn new_with_dash(
        paints: PaintStack,
        width: f32,
        cap: StrokeCap,
        join: StrokeJoin,
        miter_limit: f32,
        dash: Option<StrokeDash>,
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
        if cap == StrokeCap::Butt
            && dash
                .as_ref()
                .is_some_and(|dash| !dash.intervals.has_positive_painted_interval())
        {
            return Ok(None);
        }
        Ok(Some(Self {
            paints,
            width,
            space: StrokeSpace::Local,
            cap,
            join,
            miter_limit,
            dash,
        }))
    }

    #[must_use]
    pub const fn paints(&self) -> &PaintStack {
        &self.paints
    }

    /// Attach the factor applied after every stroke-paint entry's own alpha.
    ///
    /// This changes only the stroke's [`PaintStack`]; width, construction
    /// space, cap, join, miter limit, and dash remain exactly as resolved. A
    /// zero factor normalizes the paint stack away, so the complete stroke
    /// becomes `None` rather than retain an invisible stroke that violates this
    /// type's invariant.
    #[must_use]
    pub fn with_paint_alpha_factor(mut self, alpha_factor: PaintAlphaFactor) -> Option<Self> {
        self.paints = self.paints.with_alpha_factor(alpha_factor);
        (!self.paints.is_empty()).then_some(self)
    }

    /// Select the coordinate system in which this stroke is dashed and
    /// widened. Construction defaults to [`StrokeSpace::Local`].
    #[must_use]
    pub fn with_space(mut self, space: StrokeSpace) -> Self {
        self.space = space;
        self
    }

    /// The coordinate system in which width, dash intervals, and phase are
    /// consumed around the centerline.
    #[must_use]
    pub const fn space(&self) -> StrokeSpace {
        self.space
    }

    /// The stroke width, in the stroke's declared construction space.
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

    /// The checked dash pattern, or `None` for a solid stroke.
    #[must_use]
    pub const fn dash(&self) -> Option<&StrokeDash> {
        self.dash.as_ref()
    }

    /// The checked dash interval cycle, or `None` for a solid stroke.
    ///
    /// This compatibility view omits the phase. New consumers that paint a
    /// dash pattern should read [`Stroke::dash`] as one indivisible fact.
    #[must_use]
    pub const fn dash_intervals(&self) -> Option<&StrokeDashIntervals> {
        match &self.dash {
            Some(dash) => Some(&dash.intervals),
            None => None,
        }
    }

    /// How far the stroke can reach outside the geometry it follows, in its
    /// declared construction space. Consumers need this to know what a stroked
    /// node covers: the node's `bounds` is its *geometry*, and ink lies outside
    /// it.
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
    /// mathematically tight bound for the square cap rather than a policy
    /// margin.
    ///
    /// The returned representation is always finite. The carried `f32` width
    /// and miter limit are converted exactly to `f64` before any arithmetic;
    /// widening this derived value does not widen either carried fact. The
    /// irrational square-cap product is advanced by one `f64` step so its
    /// representation rounds outward rather than inside the mathematical
    /// bound.
    #[must_use]
    pub fn outset(&self) -> f64 {
        let radius = f64::from(self.width) / 2.0;
        let join_reach = radius
            * match self.join {
                StrokeJoin::Miter => f64::from(self.miter_limit).max(1.0),
                StrokeJoin::Round | StrokeJoin::Bevel => 1.0,
            };
        let cap_reach = match self.cap {
            StrokeCap::Square => (radius * std::f64::consts::SQRT_2).next_up(),
            StrokeCap::Butt | StrokeCap::Round => radius,
        };
        join_reach.max(cap_reach)
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
    fn paint_alpha_factor_changes_only_the_stack_and_zero_removes_the_stroke() {
        let intervals = StrokeDashIntervals::new(vec![3.0, 2.0])
            .expect("valid intervals")
            .expect("positive cycle");
        let dash = StrokeDash::new(intervals, -7.0).expect("finite phase");
        let original = Stroke::new_with_dash(
            black(),
            8.0,
            StrokeCap::Square,
            StrokeJoin::Bevel,
            2.5,
            Some(dash),
        )
        .expect("valid stroke")
        .expect("visible stroke");

        let half = PaintAlphaFactor::new(0.5).expect("half alpha");
        let mut expected = original.clone();
        expected.paints = expected.paints.with_alpha_factor(half);
        assert_eq!(
            original
                .clone()
                .with_paint_alpha_factor(half)
                .expect("a nonzero factor keeps the stroke"),
            expected,
            "every non-paint stroke fact stays bit-for-bit unchanged"
        );

        let zero = PaintAlphaFactor::new(0.0).expect("checked zero");
        assert_eq!(original.with_paint_alpha_factor(zero), None);
    }

    #[test]
    fn construction_space_defaults_local_and_changes_no_other_stroke_fact() {
        let intervals = StrokeDashIntervals::new(vec![3.0, 2.0])
            .expect("valid intervals")
            .expect("positive cycle");
        let dash = StrokeDash::new(intervals, -7.0).expect("finite phase");
        let local = Stroke::new_with_dash(
            black(),
            8.0,
            StrokeCap::Square,
            StrokeJoin::Bevel,
            2.5,
            Some(dash),
        )
        .expect("valid stroke")
        .expect("visible stroke");
        assert_eq!(local.space(), StrokeSpace::Local);

        let frame = local.clone().with_space(StrokeSpace::Frame);
        let mut expected = local.clone();
        expected.space = StrokeSpace::Frame;
        assert_eq!(frame, expected);
        assert_eq!(frame.space(), StrokeSpace::Frame);
        assert_eq!(
            local.space(),
            StrokeSpace::Local,
            "the builder is persistent"
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
        assert_eq!(square.outset(), (4.0 * std::f64::consts::SQRT_2).next_up());
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
}
