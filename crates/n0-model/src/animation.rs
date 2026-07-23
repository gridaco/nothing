//! n0 animation programs, typed effects, and atomic property-value sampling.
//!
//! Source frontends compile immutable typed tracks once. Source-neutral exact
//! time, timing, easing, and scalar sampling are adopted from
//! `animation-sampling`; this module owns document targets, typed property
//! curves, effect composition, and projection into ordinary
//! [`PropertyValues`].

use crate::math::Affine;
use crate::model::{
    AnchorEdge, AxisBinding, BlendMode, Color, Document, LensOp, NodeKey, Paint, Paints, SizeIntent,
};
use crate::path::{PathCommand, PathGeometry};
use crate::properties::{
    PropertyError, PropertyKey, PropertyTarget, PropertyValue, PropertyValueKind, PropertyValues,
};
use animation_sampling::internal::{
    active_progress, binary32_rational, easing_apply, keyframe_offset_cmp_ratio,
    keyframe_offset_rational, lerp_binary32_once_rational, round_rational_to_binary32,
};
use animation_sampling::{Contribution, ScalarSample};
pub use animation_sampling::{
    CubicBezier, CubicBezierError, CubicControl, Easing, FillMode, KeyframeOffset,
    KeyframeOffsetError, SampleTime, SampleTimeRangeError, ScalarCurve, ScalarCurveError,
    ScalarKeyframe, ScalarSegment, Timing, TimingError,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::sync::Arc;

/// One complete singleton-solid fill value at an exact curve offset.
///
/// The color is the value of an existing [`Paint::Solid`]. Sampling projects
/// the final composed result back to the ordinary ordered [`Paints`] property;
/// this is not a parallel color property or a nested paint-member target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorKeyframe {
    offset: KeyframeOffset,
    color: Color,
}

impl ColorKeyframe {
    pub const fn new(offset: KeyframeOffset, color: Color) -> Self {
        Self { offset, color }
    }

    pub const fn offset(self) -> KeyframeOffset {
        self.offset
    }

    pub const fn color(self) -> Color {
        self.color
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSegment {
    easing: Easing,
    end: ColorKeyframe,
}

impl ColorSegment {
    pub const fn new(easing: Easing, end: ColorKeyframe) -> Self {
        Self { easing, end }
    }

    pub fn easing(&self) -> Easing {
        self.easing.clone()
    }

    pub const fn end(&self) -> ColorKeyframe {
        self.end
    }
}

/// A keyframed legacy-sRGB solid-fill color.
///
/// Channels are interpolated straight (unpremultiplied) and remain exact and
/// unbounded while effects are accumulated and composed. The complete color
/// sandwich is clamped and quantized to the engine's RGBA8 [`Color`] only when
/// it is projected to [`PropertyValue::Paints`].
#[derive(Debug, Clone, PartialEq)]
pub struct ColorCurve {
    first: ColorKeyframe,
    segments: Box<[ColorSegment]>,
}

impl ColorCurve {
    pub fn new(first: ColorKeyframe, segments: Vec<ColorSegment>) -> Result<Self, ColorCurveError> {
        if segments.is_empty() {
            return Ok(Self::constant(first.color));
        }
        if first.offset != KeyframeOffset::ZERO {
            return Err(ColorCurveError::FirstOffsetMustBeZero {
                actual: first.offset,
            });
        }

        let mut previous = first.offset;
        for (segment_index, segment) in segments.iter().enumerate() {
            let current = segment.end.offset;
            if current <= previous {
                return Err(ColorCurveError::OffsetsNotStrictlyIncreasing {
                    previous_index: segment_index,
                    current_index: segment_index + 1,
                    previous,
                    current,
                });
            }
            previous = current;
        }
        if previous != KeyframeOffset::ONE {
            return Err(ColorCurveError::LastOffsetMustBeOne { actual: previous });
        }

        Ok(Self {
            first,
            segments: segments.into_boxed_slice(),
        })
    }

    pub fn linear(from: Color, to: Color) -> Self {
        Self {
            first: ColorKeyframe::new(KeyframeOffset::ZERO, from),
            segments: vec![ColorSegment::new(
                Easing::Linear,
                ColorKeyframe::new(KeyframeOffset::ONE, to),
            )]
            .into_boxed_slice(),
        }
    }

    pub fn constant(color: Color) -> Self {
        Self {
            first: ColorKeyframe::new(KeyframeOffset::ZERO, color),
            segments: Box::new([]),
        }
    }

    pub const fn first(&self) -> ColorKeyframe {
        self.first
    }

    pub fn segments(&self) -> &[ColorSegment] {
        &self.segments
    }

    pub fn keyframes(&self) -> impl Iterator<Item = &ColorKeyframe> {
        std::iter::once(&self.first).chain(self.segments.iter().map(|segment| &segment.end))
    }

    pub fn keyframe_count(&self) -> usize {
        1 + self.segments.len()
    }

    pub const fn first_color(&self) -> Color {
        self.first.color
    }

    pub fn last_color(&self) -> Color {
        self.segments
            .last()
            .map_or(self.first.color, |segment| segment.end.color)
    }

    fn sample(&self, numerator: u64, denominator: u64) -> ExactColor {
        debug_assert!(denominator > 0);
        debug_assert!(numerator < denominator);
        if self.segments.is_empty() || numerator == 0 {
            return ExactColor::from(self.first.color);
        }

        let segment_index = match self.segments.binary_search_by(|segment| {
            keyframe_offset_cmp_ratio(segment.end.offset, numerator, denominator)
        }) {
            Ok(index) => return ExactColor::from(self.segments[index].end.color),
            Err(index) => index,
        };
        let segment = self
            .segments
            .get(segment_index)
            .expect("an active progress is below the required terminal offset one");
        let start = segment_index
            .checked_sub(1)
            .map_or(self.first, |index| self.segments[index].end);
        let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
        let start_offset = keyframe_offset_rational(start.offset);
        let end_offset = keyframe_offset_rational(segment.end.offset);
        let local = (&progress - &start_offset) / (&end_offset - &start_offset);
        ExactColor::from(start.color).interpolate(
            &ExactColor::from(segment.end.color),
            &easing_apply(&segment.easing, &local),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCurveError {
    FirstOffsetMustBeZero {
        actual: KeyframeOffset,
    },
    LastOffsetMustBeOne {
        actual: KeyframeOffset,
    },
    OffsetsNotStrictlyIncreasing {
        previous_index: usize,
        current_index: usize,
        previous: KeyframeOffset,
        current: KeyframeOffset,
    },
}

impl std::fmt::Display for ColorCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstOffsetMustBeZero { actual } => write!(
                f,
                "the first color keyframe offset must be 0, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::LastOffsetMustBeOne { actual } => write!(
                f,
                "the last color keyframe offset must be 1, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::OffsetsNotStrictlyIncreasing {
                previous_index,
                current_index,
                previous,
                current,
            } => write!(
                f,
                "color keyframe offsets must increase strictly: index {previous_index} is {}/{}, index {current_index} is {}/{}",
                previous.numerator(),
                previous.denominator(),
                current.numerator(),
                current.denominator()
            ),
        }
    }
}

impl std::error::Error for ColorCurveError {}

/// One complete, validated path geometry value at an exact curve offset.
#[derive(Debug, Clone, PartialEq)]
pub struct PathKeyframe {
    offset: KeyframeOffset,
    path: Arc<PathGeometry>,
}

impl PathKeyframe {
    pub const fn new(offset: KeyframeOffset, path: Arc<PathGeometry>) -> Self {
        Self { offset, path }
    }

    pub const fn offset(&self) -> KeyframeOffset {
        self.offset
    }

    pub fn path(&self) -> &Arc<PathGeometry> {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    easing: Easing,
    end: PathKeyframe,
}

impl PathSegment {
    pub const fn new(easing: Easing, end: PathKeyframe) -> Self {
        Self { easing, end }
    }

    pub fn easing(&self) -> Easing {
        self.easing.clone()
    }

    pub const fn end(&self) -> &PathKeyframe {
        &self.end
    }
}

/// A smoothly interpolated, topology-stable path curve.
///
/// Curves are already normalized into the engine's absolute command
/// vocabulary. Frontends remain responsible for any stricter source-level
/// compatibility rule (for example, SVG distinguishes `H` from `L` before
/// both become an engine line command). Rational conics are intentionally not
/// admitted: interpolating a lowered conic is not SVG arc interpolation.
#[derive(Debug, Clone, PartialEq)]
pub struct PathCurve {
    first: PathKeyframe,
    segments: Box<[PathSegment]>,
}

impl PathCurve {
    pub fn new(first: PathKeyframe, segments: Vec<PathSegment>) -> Result<Self, PathCurveError> {
        first
            .path
            .validate()
            .map_err(|error| PathCurveError::InvalidPath {
                keyframe_index: 0,
                reason: error.to_string(),
            })?;
        validate_path_topology(&first.path, &first.path, 0)?;
        if segments.is_empty() {
            return Ok(Self {
                first: PathKeyframe::new(KeyframeOffset::ZERO, first.path),
                segments: Box::new([]),
            });
        }
        if first.offset != KeyframeOffset::ZERO {
            return Err(PathCurveError::FirstOffsetMustBeZero {
                actual: first.offset,
            });
        }

        let mut previous = first.offset;
        for (segment_index, segment) in segments.iter().enumerate() {
            let keyframe_index = segment_index + 1;
            segment
                .end
                .path
                .validate()
                .map_err(|error| PathCurveError::InvalidPath {
                    keyframe_index,
                    reason: error.to_string(),
                })?;
            validate_path_topology(&first.path, &segment.end.path, keyframe_index)?;
            if let Easing::CubicBezier(easing) = &segment.easing {
                for (control, value) in [
                    (CubicControl::Y1, easing.y1()),
                    (CubicControl::Y2, easing.y2()),
                ] {
                    if !(0.0..=1.0).contains(&value) {
                        return Err(PathCurveError::UnsafeCubicControl {
                            segment_index,
                            control,
                        });
                    }
                }
            }
            let current = segment.end.offset;
            if current <= previous {
                return Err(PathCurveError::OffsetsNotStrictlyIncreasing {
                    previous_index: segment_index,
                    current_index: keyframe_index,
                    previous,
                    current,
                });
            }
            previous = current;
        }
        if previous != KeyframeOffset::ONE {
            return Err(PathCurveError::LastOffsetMustBeOne { actual: previous });
        }

        Ok(Self {
            first,
            segments: segments.into_boxed_slice(),
        })
    }

    pub fn linear(from: Arc<PathGeometry>, to: Arc<PathGeometry>) -> Result<Self, PathCurveError> {
        Self::new(
            PathKeyframe::new(KeyframeOffset::ZERO, from),
            vec![PathSegment::new(
                Easing::Linear,
                PathKeyframe::new(KeyframeOffset::ONE, to),
            )],
        )
    }

    pub fn constant(path: Arc<PathGeometry>) -> Result<Self, PathCurveError> {
        Self::new(PathKeyframe::new(KeyframeOffset::ZERO, path), vec![])
    }

    pub const fn first(&self) -> &PathKeyframe {
        &self.first
    }

    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    pub fn keyframes(&self) -> impl Iterator<Item = &PathKeyframe> {
        std::iter::once(&self.first).chain(self.segments.iter().map(|segment| &segment.end))
    }

    pub fn first_path(&self) -> &Arc<PathGeometry> {
        &self.first.path
    }

    pub fn last_path(&self) -> &Arc<PathGeometry> {
        self.segments
            .last()
            .map_or(&self.first.path, |segment| &segment.end.path)
    }

    fn sample(&self, numerator: u64, denominator: u64) -> Arc<PathGeometry> {
        debug_assert!(denominator > 0);
        debug_assert!(numerator < denominator);
        if self.segments.is_empty() || numerator == 0 {
            return Arc::clone(&self.first.path);
        }

        let segment_index = match self.segments.binary_search_by(|segment| {
            keyframe_offset_cmp_ratio(segment.end.offset, numerator, denominator)
        }) {
            Ok(index) => return Arc::clone(&self.segments[index].end.path),
            Err(index) => index,
        };
        let segment = self
            .segments
            .get(segment_index)
            .expect("an active progress is below the required terminal offset one");
        let start = segment_index
            .checked_sub(1)
            .map_or(&self.first, |index| &self.segments[index].end);
        let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
        let start_offset = keyframe_offset_rational(start.offset);
        let end_offset = keyframe_offset_rational(segment.end.offset);
        let local = (&progress - &start_offset) / (&end_offset - &start_offset);
        interpolate_path(
            &start.path,
            &segment.end.path,
            &easing_apply(&segment.easing, &local),
        )
    }
}

fn validate_path_topology(
    first: &PathGeometry,
    candidate: &PathGeometry,
    keyframe_index: usize,
) -> Result<(), PathCurveError> {
    if first.commands.len() != candidate.commands.len() {
        return Err(PathCurveError::DifferentTopology { keyframe_index });
    }
    for (command_index, (left, right)) in first
        .commands
        .iter()
        .zip(candidate.commands.iter())
        .enumerate()
    {
        if matches!(left, PathCommand::ConicTo { .. })
            || matches!(right, PathCommand::ConicTo { .. })
        {
            return Err(PathCurveError::ConicNotInterpolable {
                keyframe_index,
                command_index,
            });
        }
        if std::mem::discriminant(left) != std::mem::discriminant(right) {
            return Err(PathCurveError::DifferentTopology { keyframe_index });
        }
    }
    Ok(())
}

fn interpolate_path(
    from: &Arc<PathGeometry>,
    to: &Arc<PathGeometry>,
    progress: &BigRational,
) -> Arc<PathGeometry> {
    debug_assert_eq!(from.commands.len(), to.commands.len());
    let lerp = |from, to| lerp_binary32_once_rational(from, to, progress);
    let commands = from
        .commands
        .iter()
        .zip(to.commands.iter())
        .map(|(from, to)| match (*from, *to) {
            (PathCommand::MoveTo { x: ax, y: ay }, PathCommand::MoveTo { x: bx, y: by }) => {
                PathCommand::MoveTo {
                    x: lerp(ax, bx),
                    y: lerp(ay, by),
                }
            }
            (PathCommand::LineTo { x: ax, y: ay }, PathCommand::LineTo { x: bx, y: by }) => {
                PathCommand::LineTo {
                    x: lerp(ax, bx),
                    y: lerp(ay, by),
                }
            }
            (
                PathCommand::QuadTo {
                    x1: ax1,
                    y1: ay1,
                    x: ax,
                    y: ay,
                },
                PathCommand::QuadTo {
                    x1: bx1,
                    y1: by1,
                    x: bx,
                    y: by,
                },
            ) => PathCommand::QuadTo {
                x1: lerp(ax1, bx1),
                y1: lerp(ay1, by1),
                x: lerp(ax, bx),
                y: lerp(ay, by),
            },
            (
                PathCommand::CubicTo {
                    x1: ax1,
                    y1: ay1,
                    x2: ax2,
                    y2: ay2,
                    x: ax,
                    y: ay,
                },
                PathCommand::CubicTo {
                    x1: bx1,
                    y1: by1,
                    x2: bx2,
                    y2: by2,
                    x: bx,
                    y: by,
                },
            ) => PathCommand::CubicTo {
                x1: lerp(ax1, bx1),
                y1: lerp(ay1, by1),
                x2: lerp(ax2, bx2),
                y2: lerp(ay2, by2),
                x: lerp(ax, bx),
                y: lerp(ay, by),
            },
            (PathCommand::Close, PathCommand::Close) => PathCommand::Close,
            _ => unreachable!("path curve topology is validated at construction"),
        })
        .collect::<Vec<_>>();
    PathGeometry::from_commands(commands)
        .expect("validated convex path interpolation remains a valid finite path")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathCurveError {
    FirstOffsetMustBeZero {
        actual: KeyframeOffset,
    },
    LastOffsetMustBeOne {
        actual: KeyframeOffset,
    },
    OffsetsNotStrictlyIncreasing {
        previous_index: usize,
        current_index: usize,
        previous: KeyframeOffset,
        current: KeyframeOffset,
    },
    InvalidPath {
        keyframe_index: usize,
        reason: String,
    },
    DifferentTopology {
        keyframe_index: usize,
    },
    ConicNotInterpolable {
        keyframe_index: usize,
        command_index: usize,
    },
    UnsafeCubicControl {
        segment_index: usize,
        control: CubicControl,
    },
}

impl std::fmt::Display for PathCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstOffsetMustBeZero { actual } => write!(
                f,
                "the first path keyframe offset must be 0, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::LastOffsetMustBeOne { actual } => write!(
                f,
                "the last path keyframe offset must be 1, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::OffsetsNotStrictlyIncreasing {
                previous_index,
                current_index,
                previous,
                current,
            } => write!(
                f,
                "path keyframe offsets must increase strictly: index {previous_index} is {}/{}, index {current_index} is {}/{}",
                previous.numerator(),
                previous.denominator(),
                current.numerator(),
                current.denominator()
            ),
            Self::InvalidPath {
                keyframe_index,
                reason,
            } => write!(f, "path keyframe {keyframe_index} is invalid: {reason}"),
            Self::DifferentTopology { keyframe_index } => write!(
                f,
                "path keyframe {keyframe_index} has incompatible normalized command topology"
            ),
            Self::ConicNotInterpolable {
                keyframe_index,
                command_index,
            } => write!(
                f,
                "path keyframe {keyframe_index} command {command_index} is a rational conic; lowered arc geometry is not smoothly interpolable"
            ),
            Self::UnsafeCubicControl {
                segment_index,
                control,
            } => write!(
                f,
                "path segment {segment_index} has {control:?} outside [0, 1]; path interpolation requires convex easing"
            ),
        }
    }
}

impl std::error::Error for PathCurveError {}

/// One complete property value selected at an exact discrete offset.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteKeyframe {
    offset: KeyframeOffset,
    value: PropertyValue,
}

impl DiscreteKeyframe {
    pub const fn new(offset: KeyframeOffset, value: PropertyValue) -> Self {
        Self { offset, value }
    }

    pub const fn offset(&self) -> KeyframeOffset {
        self.offset
    }

    pub const fn value(&self) -> &PropertyValue {
        &self.value
    }
}

/// A format-neutral, hold-until-next-offset replacement curve.
///
/// The terminal offset may be below one, matching SMIL discrete keyTimes.
/// Frontends decide which properties admit this calculation mode.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteCurve {
    keyframes: Box<[DiscreteKeyframe]>,
}

impl DiscreteCurve {
    pub fn new(mut keyframes: Vec<DiscreteKeyframe>) -> Result<Self, DiscreteCurveError> {
        if keyframes.is_empty() {
            return Err(DiscreteCurveError::Empty);
        }
        let expected = keyframes[0].value.kind();
        for (keyframe_index, keyframe) in keyframes.iter().enumerate().skip(1) {
            let actual = keyframe.value.kind();
            if actual != expected {
                return Err(DiscreteCurveError::MixedValueKind {
                    keyframe_index,
                    expected,
                    actual,
                });
            }
        }
        if keyframes.len() == 1 {
            keyframes[0].offset = KeyframeOffset::ZERO;
            return Ok(Self {
                keyframes: keyframes.into_boxed_slice(),
            });
        }
        if keyframes[0].offset != KeyframeOffset::ZERO {
            return Err(DiscreteCurveError::FirstOffsetMustBeZero {
                actual: keyframes[0].offset,
            });
        }
        for index in 1..keyframes.len() {
            let previous = keyframes[index - 1].offset;
            let current = keyframes[index].offset;
            if current <= previous {
                return Err(DiscreteCurveError::OffsetsNotStrictlyIncreasing {
                    previous_index: index - 1,
                    current_index: index,
                    previous,
                    current,
                });
            }
        }
        Ok(Self {
            keyframes: keyframes.into_boxed_slice(),
        })
    }

    pub fn keyframes(&self) -> &[DiscreteKeyframe] {
        &self.keyframes
    }

    pub fn first_value(&self) -> &PropertyValue {
        &self.keyframes[0].value
    }

    pub fn last_value(&self) -> &PropertyValue {
        &self.keyframes[self.keyframes.len() - 1].value
    }

    fn sample(&self, numerator: u64, denominator: u64) -> &PropertyValue {
        debug_assert!(denominator > 0);
        debug_assert!(numerator < denominator);
        let index = match self.keyframes.binary_search_by(|keyframe| {
            keyframe_offset_cmp_ratio(keyframe.offset, numerator, denominator)
        }) {
            Ok(index) => index,
            Err(0) => 0,
            Err(index) => index - 1,
        };
        &self.keyframes[index].value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscreteCurveError {
    Empty,
    MixedValueKind {
        keyframe_index: usize,
        expected: PropertyValueKind,
        actual: PropertyValueKind,
    },
    FirstOffsetMustBeZero {
        actual: KeyframeOffset,
    },
    OffsetsNotStrictlyIncreasing {
        previous_index: usize,
        current_index: usize,
        previous: KeyframeOffset,
        current: KeyframeOffset,
    },
}

impl std::fmt::Display for DiscreteCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "a discrete curve requires at least one keyframe"),
            Self::MixedValueKind {
                keyframe_index,
                expected,
                actual,
            } => write!(
                f,
                "discrete keyframe {keyframe_index} has {actual:?}, expected {expected:?}"
            ),
            Self::FirstOffsetMustBeZero { actual } => write!(
                f,
                "the first discrete keyframe offset must be 0, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::OffsetsNotStrictlyIncreasing {
                previous_index,
                current_index,
                previous,
                current,
            } => write!(
                f,
                "discrete keyframe offsets must increase strictly: index {previous_index} is {}/{}, index {current_index} is {}/{}",
                previous.numerator(),
                previous.denominator(),
                current.numerator(),
                current.denominator()
            ),
        }
    }
}

impl std::error::Error for DiscreteCurveError {}

/// A two-value discrete transition whose switch is measured after easing.
/// This represents SVG's bounded incompatible-path fallback without
/// pretending that it is an authored discrete keyframe schedule.
#[derive(Debug, Clone, PartialEq)]
struct EasedDiscretePair {
    from: PropertyValue,
    to: PropertyValue,
    easing: Easing,
}

impl EasedDiscretePair {
    fn path(from: Arc<PathGeometry>, to: Arc<PathGeometry>, easing: Easing) -> Self {
        Self {
            from: PropertyValue::PathGeometry(from),
            to: PropertyValue::PathGeometry(to),
            easing,
        }
    }

    const fn from(&self) -> &PropertyValue {
        &self.from
    }

    const fn to(&self) -> &PropertyValue {
        &self.to
    }

    fn sample(&self, numerator: u64, denominator: u64) -> &PropertyValue {
        let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
        if easing_apply(&self.easing, &progress) < BigRational::new(BigInt::one(), BigInt::from(2))
        {
            &self.from
        } else {
            &self.to
        }
    }
}

/// One SVG-compatible 2D transform operation.
///
/// This remains typed through interpolation and repeat accumulation. It is
/// converted to an affine lens operation only after sampling, so scale,
/// rotation centers, and accumulation never depend on matrix decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    Translate,
    Scale,
    Rotate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransformValue {
    Translate {
        x: f32,
        y: f32,
    },
    Scale {
        x: f32,
        y: f32,
    },
    Rotate {
        degrees: f32,
        center_x: f32,
        center_y: f32,
    },
}

impl TransformValue {
    pub const fn kind(self) -> TransformKind {
        match self {
            Self::Translate { .. } => TransformKind::Translate,
            Self::Scale { .. } => TransformKind::Scale,
            Self::Rotate { .. } => TransformKind::Rotate,
        }
    }

    pub const fn components(self) -> [f32; 3] {
        match self {
            Self::Translate { x, y } | Self::Scale { x, y } => [x, y, 0.0],
            Self::Rotate {
                degrees,
                center_x,
                center_y,
            } => [degrees, center_x, center_y],
        }
    }

    fn from_components(kind: TransformKind, [first, second, third]: [f32; 3]) -> Self {
        match kind {
            TransformKind::Translate => Self::Translate {
                x: first,
                y: second,
            },
            TransformKind::Scale => Self::Scale {
                x: first,
                y: second,
            },
            TransformKind::Rotate => Self::Rotate {
                degrees: first,
                center_x: second,
                center_y: third,
            },
        }
    }

    fn interpolate(self, to: Self, progress: &BigRational) -> Self {
        assert_eq!(
            self.kind(),
            to.kind(),
            "transform curve kinds are validated"
        );
        let from = self.components();
        let to = to.components();
        Self::from_components(
            self.kind(),
            std::array::from_fn(|index| {
                lerp_binary32_once_rational(from[index], to[index], progress)
            }),
        )
    }

    fn accumulated(self, terminal: Self, repeat_index: u64) -> Option<Self> {
        assert_eq!(
            self.kind(),
            terminal.kind(),
            "transform curve kinds are validated"
        );
        if repeat_index == 0 {
            return Some(self);
        }
        let value = self.components();
        let terminal = terminal.components();
        let components = std::array::from_fn(|index| {
            let exact = binary32_rational(value[index])
                + binary32_rational(terminal[index])
                    * BigRational::from_integer(BigInt::from(repeat_index));
            if exact.is_zero()
                && value[index] == 0.0
                && value[index].is_sign_negative()
                && terminal[index] == 0.0
                && terminal[index].is_sign_negative()
            {
                -0.0
            } else {
                round_rational_to_binary32(&exact)
            }
        });
        components
            .iter()
            .all(|value| value.is_finite())
            .then(|| Self::from_components(self.kind(), components))
    }

    fn into_lens_op(self) -> Option<LensOp> {
        let matrix = match self {
            Self::Translate { x, y } => Affine::translate(x, y),
            Self::Scale { x, y } => Affine::scale(x, y),
            Self::Rotate {
                degrees,
                center_x,
                center_y,
            } => Affine::translate(center_x, center_y)
                .then(&Affine::rotate_deg(degrees))
                .then(&Affine::translate(-center_x, -center_y)),
        };
        let m = [matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f];
        m.iter()
            .all(|value| value.is_finite())
            .then_some(LensOp::Matrix { m })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformKeyframe {
    offset: KeyframeOffset,
    value: TransformValue,
}

impl TransformKeyframe {
    pub const fn new(offset: KeyframeOffset, value: TransformValue) -> Self {
        Self { offset, value }
    }

    pub const fn offset(self) -> KeyframeOffset {
        self.offset
    }

    pub const fn value(self) -> TransformValue {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformSegment {
    easing: Easing,
    end: TransformKeyframe,
}

impl TransformSegment {
    pub const fn new(easing: Easing, end: TransformKeyframe) -> Self {
        Self { easing, end }
    }

    pub fn easing(&self) -> Easing {
        self.easing.clone()
    }

    pub const fn end(&self) -> TransformKeyframe {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformCurve {
    first: TransformKeyframe,
    segments: Box<[TransformSegment]>,
}

impl TransformCurve {
    pub fn new(
        first: TransformKeyframe,
        segments: Vec<TransformSegment>,
    ) -> Result<Self, TransformCurveError> {
        if !first
            .value
            .components()
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(TransformCurveError::NonFiniteValue { keyframe_index: 0 });
        }
        if segments.is_empty() {
            return Ok(Self {
                first: TransformKeyframe::new(KeyframeOffset::ZERO, first.value),
                segments: Box::new([]),
            });
        }
        if first.offset != KeyframeOffset::ZERO {
            return Err(TransformCurveError::FirstOffsetMustBeZero {
                actual: first.offset,
            });
        }

        let kind = first.value.kind();
        let mut previous = first.offset;
        for (segment_index, segment) in segments.iter().enumerate() {
            let keyframe_index = segment_index + 1;
            if segment.end.value.kind() != kind {
                return Err(TransformCurveError::MixedKinds {
                    expected: kind,
                    keyframe_index,
                    actual: segment.end.value.kind(),
                });
            }
            if !segment
                .end
                .value
                .components()
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(TransformCurveError::NonFiniteValue { keyframe_index });
            }
            let current = segment.end.offset;
            if current <= previous {
                return Err(TransformCurveError::OffsetsNotStrictlyIncreasing {
                    previous_index: segment_index,
                    current_index: keyframe_index,
                    previous,
                    current,
                });
            }
            previous = current;
        }
        if previous != KeyframeOffset::ONE {
            return Err(TransformCurveError::LastOffsetMustBeOne { actual: previous });
        }
        Ok(Self {
            first,
            segments: segments.into_boxed_slice(),
        })
    }

    pub fn linear(from: TransformValue, to: TransformValue) -> Result<Self, TransformCurveError> {
        Self::new(
            TransformKeyframe::new(KeyframeOffset::ZERO, from),
            vec![TransformSegment::new(
                Easing::Linear,
                TransformKeyframe::new(KeyframeOffset::ONE, to),
            )],
        )
    }

    pub fn constant(value: TransformValue) -> Result<Self, TransformCurveError> {
        Self::new(TransformKeyframe::new(KeyframeOffset::ZERO, value), vec![])
    }

    pub const fn first(&self) -> TransformKeyframe {
        self.first
    }

    pub fn segments(&self) -> &[TransformSegment] {
        &self.segments
    }

    pub fn keyframes(&self) -> impl Iterator<Item = &TransformKeyframe> {
        std::iter::once(&self.first).chain(self.segments.iter().map(|segment| &segment.end))
    }

    pub fn keyframe_count(&self) -> usize {
        1 + self.segments.len()
    }

    pub const fn kind(&self) -> TransformKind {
        self.first.value.kind()
    }

    pub const fn first_value(&self) -> TransformValue {
        self.first.value
    }

    pub fn last_value(&self) -> TransformValue {
        self.segments
            .last()
            .map_or(self.first.value, |segment| segment.end.value)
    }

    fn sample(&self, numerator: u64, denominator: u64) -> TransformValue {
        debug_assert!(denominator > 0);
        debug_assert!(numerator < denominator);
        if self.segments.is_empty() || numerator == 0 {
            return self.first.value;
        }
        let segment_index = match self.segments.binary_search_by(|segment| {
            keyframe_offset_cmp_ratio(segment.end.offset, numerator, denominator)
        }) {
            Ok(index) => return self.segments[index].end.value,
            Err(index) => index,
        };
        let segment = &self.segments[segment_index];
        let start = segment_index
            .checked_sub(1)
            .map_or(self.first, |index| self.segments[index].end);
        let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
        let start_offset = keyframe_offset_rational(start.offset);
        let end_offset = keyframe_offset_rational(segment.end.offset);
        let local = (&progress - &start_offset) / (&end_offset - &start_offset);
        start
            .value
            .interpolate(segment.end.value, &easing_apply(&segment.easing, &local))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformCurveError {
    FirstOffsetMustBeZero {
        actual: KeyframeOffset,
    },
    LastOffsetMustBeOne {
        actual: KeyframeOffset,
    },
    OffsetsNotStrictlyIncreasing {
        previous_index: usize,
        current_index: usize,
        previous: KeyframeOffset,
        current: KeyframeOffset,
    },
    MixedKinds {
        expected: TransformKind,
        keyframe_index: usize,
        actual: TransformKind,
    },
    NonFiniteValue {
        keyframe_index: usize,
    },
}

impl std::fmt::Display for TransformCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstOffsetMustBeZero { actual } => write!(
                f,
                "the first transform keyframe offset must be 0, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::LastOffsetMustBeOne { actual } => write!(
                f,
                "the last transform keyframe offset must be 1, found {}/{}",
                actual.numerator(),
                actual.denominator()
            ),
            Self::OffsetsNotStrictlyIncreasing {
                previous_index,
                current_index,
                previous,
                current,
            } => write!(
                f,
                "transform keyframe offsets must increase strictly: index {previous_index} is {}/{}, index {current_index} is {}/{}",
                previous.numerator(),
                previous.denominator(),
                current.numerator(),
                current.denominator()
            ),
            Self::MixedKinds {
                expected,
                keyframe_index,
                actual,
            } => write!(
                f,
                "transform keyframe {keyframe_index} is {actual:?}, expected {expected:?}"
            ),
            Self::NonFiniteValue { keyframe_index } => {
                write!(f, "transform keyframe {keyframe_index} contains a non-finite component")
            }
        }
    }
}

impl std::error::Error for TransformCurveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    AxisStart,
    FixedSize,
    Opacity,
    /// A complete singleton-solid value for the existing `Fills` aggregate.
    SolidFill,
    LensTransform,
    PathGeometry,
}

/// Trust-boundary ceiling for one exact-rational solid-fill sandwich.
///
/// Exact channel denominators can grow across additive effects. Bounding the
/// target stack keeps sampling work finite without weakening the ordinary
/// `Paints` model or introducing floating-point fallback.
pub const MAX_SOLID_FILL_EFFECTS_PER_TARGET: usize = 256;

/// How one effect combines with the lower-priority sandwich result.
///
/// This is format-neutral. SVG `additive="replace|sum"` is one frontend
/// spelling; future frontends may lower their own composition vocabulary to
/// the same operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeOperation {
    Replace,
    Add,
    /// Interpolate from the live lower sandwich value without cutting it off.
    InterpolateLiveUnderlying,
}

/// How later iterations combine with the effect's simple value.
///
/// `Accumulate` adds `repeat_index * final_keyframe` before the effect is
/// composed into its target sandwich.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationCompositeOperation {
    Replace,
    Accumulate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationValueOperation {
    Add,
    Accumulate,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlyingValueShape {
    StartPin,
    CenterPin,
    EndPin,
    AxisSpan,
    FixedSize,
    AutoSize,
    Number,
    EmptyPaints,
    SolidPaint,
    InactiveSolidPaint,
    BlendedSolidPaint,
    NonSolidPaint,
    PaintStack,
    LensOps,
    PathGeometry,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationValueError {
    UnsupportedUnderlying {
        target: PropertyTarget,
        expected: TrackKind,
        actual: UnderlyingValueShape,
    },
    InvalidUnderlying {
        target: PropertyTarget,
        kind: TrackKind,
        reason: ScalarDomainError,
    },
    NonFiniteResult {
        target: PropertyTarget,
        operation: AnimationValueOperation,
    },
}

impl std::fmt::Display for AnimationValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedUnderlying {
                target,
                expected,
                actual,
            } => {
                if *expected == TrackKind::SolidFill {
                    write!(
                        f,
                        "animation target {target:?} needs exactly one active, normal-blend solid paint as its complete Fills underlying value, found {actual:?}"
                    )
                } else {
                    write!(
                        f,
                        "animation target {target:?} needs a compatible {expected:?} underlying value, found {actual:?}"
                    )
                }
            }
            Self::InvalidUnderlying {
                target,
                kind,
                reason,
            } => write!(
                f,
                "animation target {target:?} has an invalid {kind:?} underlying scalar: {reason:?}"
            ),
            Self::NonFiniteResult { target, operation } => write!(
                f,
                "animation {operation:?} produced a non-finite value for {target:?}"
            ),
        }
    }
}

impl std::error::Error for AnimationValueError {}

impl TrackKind {
    fn underlying_scalar(
        self,
        target: PropertyTarget,
        value: &PropertyValue,
    ) -> Result<f32, AnimationValueError> {
        let scalar = match (self, value) {
            (
                Self::AxisStart,
                PropertyValue::AxisBinding(AxisBinding::Pin {
                    anchor: AnchorEdge::Start,
                    offset,
                }),
            ) => Some(*offset),
            (Self::FixedSize, PropertyValue::SizeIntent(SizeIntent::Fixed(value))) => Some(*value),
            (Self::Opacity, PropertyValue::Number(value)) => Some(*value),
            (Self::SolidFill | Self::LensTransform | Self::PathGeometry, _) => None,
            _ => None,
        };
        let scalar = scalar.ok_or_else(|| AnimationValueError::UnsupportedUnderlying {
            target,
            expected: self,
            actual: underlying_value_shape(value),
        })?;
        if let Some(reason) = scalar_domain_error(self, scalar) {
            return Err(AnimationValueError::InvalidUnderlying {
                target,
                kind: self,
                reason,
            });
        }
        Ok(scalar)
    }

    fn underlying_color(
        self,
        target: PropertyTarget,
        value: &PropertyValue,
    ) -> Result<ExactColor, AnimationValueError> {
        let PropertyValue::Paints(paints) = value else {
            return Err(AnimationValueError::UnsupportedUnderlying {
                target,
                expected: self,
                actual: underlying_value_shape(value),
            });
        };
        let [Paint::Solid(paint)] = paints.as_slice() else {
            return Err(AnimationValueError::UnsupportedUnderlying {
                target,
                expected: self,
                actual: underlying_value_shape(value),
            });
        };
        if !paint.active || paint.blend_mode != BlendMode::Normal {
            return Err(AnimationValueError::UnsupportedUnderlying {
                target,
                expected: self,
                actual: underlying_value_shape(value),
            });
        }
        Ok(ExactColor::from(paint.color))
    }

    fn project_scalar(self, value: f32) -> PropertyValue {
        match self {
            Self::AxisStart => PropertyValue::AxisBinding(AxisBinding::start(value)),
            Self::FixedSize => PropertyValue::SizeIntent(SizeIntent::Fixed(value)),
            // SVG opacity addition is allowed to escape the property domain
            // internally. The existing document model remains normalized:
            // clamp once, after the complete sandwich has been composed.
            Self::Opacity => PropertyValue::Number(value.clamp(0.0, 1.0)),
            Self::SolidFill => unreachable!("solid fill tracks do not project scalar values"),
            Self::LensTransform => unreachable!("transform tracks do not project scalar values"),
            Self::PathGeometry => unreachable!("path tracks do not project scalar values"),
        }
    }
}

fn underlying_value_shape(value: &PropertyValue) -> UnderlyingValueShape {
    match value {
        PropertyValue::AxisBinding(AxisBinding::Pin {
            anchor: AnchorEdge::Start,
            ..
        }) => UnderlyingValueShape::StartPin,
        PropertyValue::AxisBinding(AxisBinding::Pin {
            anchor: AnchorEdge::Center,
            ..
        }) => UnderlyingValueShape::CenterPin,
        PropertyValue::AxisBinding(AxisBinding::Pin {
            anchor: AnchorEdge::End,
            ..
        }) => UnderlyingValueShape::EndPin,
        PropertyValue::AxisBinding(AxisBinding::Span { .. }) => UnderlyingValueShape::AxisSpan,
        PropertyValue::SizeIntent(SizeIntent::Fixed(_)) => UnderlyingValueShape::FixedSize,
        PropertyValue::SizeIntent(SizeIntent::Auto) => UnderlyingValueShape::AutoSize,
        PropertyValue::Number(_) => UnderlyingValueShape::Number,
        PropertyValue::Paints(paints) => match paints.as_slice() {
            [] => UnderlyingValueShape::EmptyPaints,
            [Paint::Solid(paint)] if !paint.active => UnderlyingValueShape::InactiveSolidPaint,
            [Paint::Solid(paint)] if paint.blend_mode != BlendMode::Normal => {
                UnderlyingValueShape::BlendedSolidPaint
            }
            [Paint::Solid(_)] => UnderlyingValueShape::SolidPaint,
            [_] => UnderlyingValueShape::NonSolidPaint,
            _ => UnderlyingValueShape::PaintStack,
        },
        PropertyValue::LensOps(_) => UnderlyingValueShape::LensOps,
        PropertyValue::PathGeometry(_) => UnderlyingValueShape::PathGeometry,
        _ => UnderlyingValueShape::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    From,
    To,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarDomainError {
    NotFinite,
    Negative,
    OutsideUnitInterval,
    OutsideFiniteBinary32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackError {
    EmptySource,
    WrongProperty {
        source: String,
        kind: TrackKind,
        actual: PropertyKey,
    },
    InvalidEndpoint {
        source: String,
        kind: TrackKind,
        endpoint: Endpoint,
        reason: ScalarDomainError,
    },
    InvalidKeyframe {
        source: String,
        kind: TrackKind,
        keyframe_index: usize,
        reason: ScalarDomainError,
    },
    UnsafeCubicControl {
        source: String,
        kind: TrackKind,
        segment_index: usize,
        control: CubicControl,
        reason: ScalarDomainError,
    },
    InvalidTransformProjection {
        source: String,
        keyframe_index: usize,
    },
    InvalidEffectValue {
        source: String,
        keyframe_index: usize,
        expected: PropertyValueKind,
        actual: PropertyValueKind,
    },
    InvalidPathGeometry {
        source: String,
        keyframe_index: usize,
        reason: String,
    },
    InvalidComposition {
        source: String,
        effect: TrackEffectKind,
        composite: CompositeOperation,
        iteration_composite: IterationCompositeOperation,
    },
}

impl std::fmt::Display for TrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackError::EmptySource => write!(f, "animation track source identity must not be empty"),
            TrackError::WrongProperty {
                source,
                kind,
                actual,
            } => write!(
                f,
                "animation track {source} of kind {kind:?} cannot target {actual:?}"
            ),
            TrackError::InvalidEndpoint {
                source,
                kind,
                endpoint,
                reason,
            } => write!(
                f,
                "animation track {source} has invalid {endpoint:?} endpoint for {kind:?}: {reason:?}"
            ),
            TrackError::InvalidKeyframe {
                source,
                kind,
                keyframe_index,
                reason,
            } => write!(
                f,
                "animation track {source} has invalid keyframe {keyframe_index} for {kind:?}: {reason:?}"
            ),
            TrackError::UnsafeCubicControl {
                source,
                kind,
                segment_index,
                control,
                reason,
            } => write!(
                f,
                "animation track {source} segment {segment_index} has an unsafe {control:?} property-space control for {kind:?}: {reason:?}"
            ),
            TrackError::InvalidTransformProjection {
                source,
                keyframe_index,
            } => write!(
                f,
                "animation track {source} transform keyframe {keyframe_index} cannot be represented as a finite affine value"
            ),
            TrackError::InvalidEffectValue {
                source,
                keyframe_index,
                expected,
                actual,
            } => write!(
                f,
                "animation track {source} keyframe {keyframe_index} has {actual:?}, expected {expected:?}"
            ),
            TrackError::InvalidPathGeometry {
                source,
                keyframe_index,
                reason,
            } => write!(
                f,
                "animation track {source} path keyframe {keyframe_index} is invalid: {reason}"
            ),
            TrackError::InvalidComposition {
                source,
                effect,
                composite,
                iteration_composite,
            } => write!(
                f,
                "animation track {source} effect {effect:?} does not admit {composite:?} composition with {iteration_composite:?} iteration composition"
            ),
        }
    }
}

impl std::error::Error for TrackError {}

/// The closed value/effect vocabulary sampled by one track.
#[derive(Debug, Clone, PartialEq)]
enum TrackEffect {
    /// Context-free scalar keyframes.
    ScalarCurve(ScalarCurve),
    /// Interpolate from the live lower sandwich value to `target`.
    ScalarFromLiveUnderlying { target: f32, easing: Easing },
    /// Context-free singleton-solid fill keyframes.
    SolidFillCurve(ColorCurve),
    /// Interpolate a singleton-solid lower fill to `target`.
    SolidFillFromLiveUnderlying { target: Color, easing: Easing },
    /// One typed 2D transform operation per keyframe.
    TransformCurve(TransformCurve),
    /// Compatible normalized path geometry interpolated component-wise.
    PathCurve(PathCurve),
    /// Complete property values selected by an authored discrete schedule.
    DiscreteCurve(DiscreteCurve),
    /// A bounded two-value discrete fallback measured after easing.
    EasedDiscretePair(EasedDiscretePair),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackEffectKind {
    ScalarCurve,
    ScalarFromLiveUnderlying,
    SolidFillCurve,
    SolidFillFromLiveUnderlying,
    TransformCurve,
    PathCurve,
    DiscreteCurve,
    EasedDiscretePair,
}

impl TrackEffect {
    const fn kind(&self) -> TrackEffectKind {
        match self {
            Self::ScalarCurve(_) => TrackEffectKind::ScalarCurve,
            Self::ScalarFromLiveUnderlying { .. } => TrackEffectKind::ScalarFromLiveUnderlying,
            Self::SolidFillCurve(_) => TrackEffectKind::SolidFillCurve,
            Self::SolidFillFromLiveUnderlying { .. } => {
                TrackEffectKind::SolidFillFromLiveUnderlying
            }
            Self::TransformCurve(_) => TrackEffectKind::TransformCurve,
            Self::PathCurve(_) => TrackEffectKind::PathCurve,
            Self::DiscreteCurve(_) => TrackEffectKind::DiscreteCurve,
            Self::EasedDiscretePair(_) => TrackEffectKind::EasedDiscretePair,
        }
    }

    fn admits_composition(
        &self,
        composite: CompositeOperation,
        iteration_composite: IterationCompositeOperation,
    ) -> bool {
        match self {
            Self::ScalarCurve(_) | Self::SolidFillCurve(_) | Self::TransformCurve(_) => {
                composite != CompositeOperation::InterpolateLiveUnderlying
            }
            Self::ScalarFromLiveUnderlying { .. } | Self::SolidFillFromLiveUnderlying { .. } => {
                composite == CompositeOperation::InterpolateLiveUnderlying
                    && iteration_composite == IterationCompositeOperation::Replace
            }
            Self::PathCurve(_) | Self::DiscreteCurve(_) | Self::EasedDiscretePair(_) => {
                composite == CompositeOperation::Replace
                    && iteration_composite == IterationCompositeOperation::Replace
            }
        }
    }
}

/// One prevalidated typed effect. Constructors are separated by output shape
/// so an authored frontend cannot pair a property with the wrong runtime
/// representation.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    source: String,
    target: PropertyTarget,
    kind: TrackKind,
    effect: TrackEffect,
    timing: Timing,
    fill: FillMode,
    composite: CompositeOperation,
    iteration_composite: IterationCompositeOperation,
}

impl Track {
    pub fn axis_start(
        source: impl Into<String>,
        target: PropertyTarget,
        from: f32,
        to: f32,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_linear(source, target, TrackKind::AxisStart, from, to, timing, fill)
    }

    pub fn fixed_size(
        source: impl Into<String>,
        target: PropertyTarget,
        from: f32,
        to: f32,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_linear(source, target, TrackKind::FixedSize, from, to, timing, fill)
    }

    pub fn opacity(
        source: impl Into<String>,
        target: PropertyTarget,
        from: f32,
        to: f32,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_linear(source, target, TrackKind::Opacity, from, to, timing, fill)
    }

    pub fn solid_fill(
        source: impl Into<String>,
        target: PropertyTarget,
        from: Color,
        to: Color,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_solid_fill_curve(source, target, ColorCurve::linear(from, to), timing, fill)
    }

    pub fn axis_start_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: ScalarCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_curve(source, target, TrackKind::AxisStart, curve, timing, fill)
    }

    pub fn fixed_size_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: ScalarCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_curve(source, target, TrackKind::FixedSize, curve, timing, fill)
    }

    pub fn opacity_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: ScalarCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_curve(source, target, TrackKind::Opacity, curve, timing, fill)
    }

    pub fn solid_fill_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: ColorCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_solid_fill_curve(source, target, curve, timing, fill)
    }

    pub fn axis_start_from_live_underlying(
        source: impl Into<String>,
        target: PropertyTarget,
        to: f32,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_scalar_from_live_underlying(
            source,
            target,
            TrackKind::AxisStart,
            to,
            easing,
            timing,
            fill,
        )
    }

    pub fn fixed_size_from_live_underlying(
        source: impl Into<String>,
        target: PropertyTarget,
        to: f32,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_scalar_from_live_underlying(
            source,
            target,
            TrackKind::FixedSize,
            to,
            easing,
            timing,
            fill,
        )
    }

    pub fn opacity_from_live_underlying(
        source: impl Into<String>,
        target: PropertyTarget,
        to: f32,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        Self::new_scalar_from_live_underlying(
            source,
            target,
            TrackKind::Opacity,
            to,
            easing,
            timing,
            fill,
        )
    }

    pub fn solid_fill_from_live_underlying(
        source: impl Into<String>,
        target: PropertyTarget,
        to: Color,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::SolidFill)?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::SolidFill,
            effect: TrackEffect::SolidFillFromLiveUnderlying { target: to, easing },
            timing,
            fill,
            composite: CompositeOperation::InterpolateLiveUnderlying,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    pub fn lens_transform_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: TransformCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::LensTransform)?;
        validate_transform_curve_domain(&source, &curve)?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::LensTransform,
            effect: TrackEffect::TransformCurve(curve),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    pub fn path_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: PathCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::PathGeometry)?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::PathGeometry,
            effect: TrackEffect::PathCurve(curve),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    pub fn path_discrete_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: DiscreteCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::PathGeometry)?;
        validate_discrete_path_values(
            &source,
            curve.keyframes().iter().map(|keyframe| keyframe.value()),
        )?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::PathGeometry,
            effect: TrackEffect::DiscreteCurve(curve),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    pub fn path_discrete_fallback(
        source: impl Into<String>,
        target: PropertyTarget,
        from: Arc<PathGeometry>,
        to: Arc<PathGeometry>,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::PathGeometry)?;
        let effect = EasedDiscretePair::path(from, to, easing);
        validate_discrete_path_values(&source, [effect.from(), effect.to()].into_iter())?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::PathGeometry,
            effect: TrackEffect::EasedDiscretePair(effect),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    fn new_linear(
        source: impl Into<String>,
        target: PropertyTarget,
        kind: TrackKind,
        from: f32,
        to: f32,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, kind)?;
        validate_endpoint(&source, kind, Endpoint::From, from)?;
        validate_endpoint(&source, kind, Endpoint::To, to)?;
        Ok(Self {
            source,
            target,
            kind,
            effect: TrackEffect::ScalarCurve(
                ScalarCurve::linear(from, to)
                    .expect("validated scalar endpoints are finite curve values"),
            ),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    fn new_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        kind: TrackKind,
        curve: ScalarCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, kind)?;
        validate_curve_domain(&source, kind, &curve)?;
        Ok(Self {
            source,
            target,
            kind,
            effect: TrackEffect::ScalarCurve(curve),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    fn new_scalar_from_live_underlying(
        source: impl Into<String>,
        target: PropertyTarget,
        kind: TrackKind,
        to: f32,
        easing: Easing,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, kind)?;
        validate_endpoint(&source, kind, Endpoint::To, to)?;
        validate_live_underlying_easing(&source, kind, &easing)?;
        Ok(Self {
            source,
            target,
            kind,
            effect: TrackEffect::ScalarFromLiveUnderlying { target: to, easing },
            timing,
            fill,
            composite: CompositeOperation::InterpolateLiveUnderlying,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    fn new_solid_fill_curve(
        source: impl Into<String>,
        target: PropertyTarget,
        curve: ColorCurve,
        timing: Timing,
        fill: FillMode,
    ) -> Result<Self, TrackError> {
        let source = source.into();
        validate_track_identity(&source, target, TrackKind::SolidFill)?;
        Ok(Self {
            source,
            target,
            kind: TrackKind::SolidFill,
            effect: TrackEffect::SolidFillCurve(curve),
            timing,
            fill,
            composite: CompositeOperation::Replace,
            iteration_composite: IterationCompositeOperation::Replace,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn target(&self) -> PropertyTarget {
        self.target
    }

    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    pub const fn effect_kind(&self) -> TrackEffectKind {
        self.effect.kind()
    }

    pub fn scalar_curve(&self) -> Option<&ScalarCurve> {
        match &self.effect {
            TrackEffect::ScalarCurve(curve) => Some(curve),
            _ => None,
        }
    }

    pub fn transform_curve(&self) -> Option<&TransformCurve> {
        match &self.effect {
            TrackEffect::TransformCurve(curve) => Some(curve),
            _ => None,
        }
    }

    pub fn color_curve(&self) -> Option<&ColorCurve> {
        match &self.effect {
            TrackEffect::SolidFillCurve(curve) => Some(curve),
            _ => None,
        }
    }

    pub fn path_curve_value(&self) -> Option<&PathCurve> {
        match &self.effect {
            TrackEffect::PathCurve(curve) => Some(curve),
            _ => None,
        }
    }

    pub fn discrete_curve(&self) -> Option<&DiscreteCurve> {
        match &self.effect {
            TrackEffect::DiscreteCurve(curve) => Some(curve),
            _ => None,
        }
    }

    pub const fn timing(&self) -> Timing {
        self.timing
    }

    pub const fn fill(&self) -> FillMode {
        self.fill
    }

    pub const fn composite(&self) -> CompositeOperation {
        self.composite
    }

    pub const fn iteration_composite(&self) -> IterationCompositeOperation {
        self.iteration_composite
    }

    /// Select effect and iteration composition when that pair is meaningful
    /// for this effect class.
    pub fn with_composition(
        mut self,
        composite: CompositeOperation,
        iteration_composite: IterationCompositeOperation,
    ) -> Result<Self, TrackError> {
        if !self
            .effect
            .admits_composition(composite, iteration_composite)
        {
            return Err(TrackError::InvalidComposition {
                source: self.source.clone(),
                effect: self.effect.kind(),
                composite,
                iteration_composite,
            });
        }
        self.composite = composite;
        self.iteration_composite = iteration_composite;
        Ok(self)
    }

    fn scalar_value(&self, value: f32) -> PropertyValue {
        match self.kind {
            TrackKind::AxisStart => PropertyValue::AxisBinding(AxisBinding::start(value)),
            TrackKind::FixedSize => PropertyValue::SizeIntent(SizeIntent::Fixed(value)),
            TrackKind::Opacity => PropertyValue::Number(value),
            TrackKind::SolidFill => {
                unreachable!("solid fill tracks do not project scalar values")
            }
            TrackKind::LensTransform => {
                unreachable!("transform tracks do not project scalar values")
            }
            TrackKind::PathGeometry => {
                unreachable!("path tracks do not project scalar values")
            }
        }
    }

    fn endpoint_values(&self) -> Vec<PropertyValue> {
        match &self.effect {
            TrackEffect::ScalarCurve(curve) => curve
                .keyframes()
                .map(|keyframe| self.scalar_value(keyframe.value()))
                .collect(),
            TrackEffect::ScalarFromLiveUnderlying { target, .. } => {
                vec![self.scalar_value(*target)]
            }
            TrackEffect::SolidFillCurve(curve) => curve
                .keyframes()
                .map(|keyframe| PropertyValue::Paints(Paints::solid(keyframe.color)))
                .collect(),
            TrackEffect::SolidFillFromLiveUnderlying { target, .. } => {
                vec![PropertyValue::Paints(Paints::solid(*target))]
            }
            TrackEffect::TransformCurve(curve) => curve
                .keyframes()
                .map(|keyframe| {
                    PropertyValue::LensOps(vec![keyframe
                        .value
                        .into_lens_op()
                        .expect("track validation checks transform projection")])
                })
                .collect(),
            TrackEffect::PathCurve(curve) => curve
                .keyframes()
                .map(|keyframe| PropertyValue::PathGeometry(Arc::clone(keyframe.path())))
                .collect(),
            TrackEffect::DiscreteCurve(curve) => curve
                .keyframes()
                .iter()
                .map(|keyframe| keyframe.value().clone())
                .collect(),
            TrackEffect::EasedDiscretePair(effect) => {
                vec![effect.from().clone(), effect.to().clone()]
            }
        }
    }

    fn contribution(&self, time: SampleTime) -> Option<Contribution> {
        self.timing.contribution(time, self.fill)
    }

    fn overwrites_underlying(&self) -> bool {
        self.composite == CompositeOperation::Replace
    }

    fn needs_underlying(&self) -> bool {
        matches!(
            self.composite,
            CompositeOperation::Add | CompositeOperation::InterpolateLiveUnderlying
        )
    }
}

fn validate_track_identity(
    source: &str,
    target: PropertyTarget,
    kind: TrackKind,
) -> Result<(), TrackError> {
    if source.is_empty() {
        return Err(TrackError::EmptySource);
    }
    let property_matches = match kind {
        TrackKind::AxisStart => matches!(target.property, PropertyKey::X | PropertyKey::Y),
        TrackKind::FixedSize => matches!(target.property, PropertyKey::Width | PropertyKey::Height),
        TrackKind::Opacity => target.property == PropertyKey::Opacity,
        TrackKind::SolidFill => target.property == PropertyKey::Fills,
        TrackKind::LensTransform => target.property == PropertyKey::LensOps,
        TrackKind::PathGeometry => target.property == PropertyKey::PathGeometry,
    };
    if !property_matches {
        return Err(TrackError::WrongProperty {
            source: source.to_owned(),
            kind,
            actual: target.property,
        });
    }
    Ok(())
}

fn validate_discrete_path_values<'a>(
    source: &str,
    values: impl IntoIterator<Item = &'a PropertyValue>,
) -> Result<(), TrackError> {
    for (keyframe_index, value) in values.into_iter().enumerate() {
        match value {
            PropertyValue::PathGeometry(path) => {
                path.validate()
                    .map_err(|error| TrackError::InvalidPathGeometry {
                        source: source.to_owned(),
                        keyframe_index,
                        reason: error.to_string(),
                    })?;
            }
            _ => {
                return Err(TrackError::InvalidEffectValue {
                    source: source.to_owned(),
                    keyframe_index,
                    expected: PropertyValueKind::PathGeometry,
                    actual: value.kind(),
                });
            }
        }
    }
    Ok(())
}

fn validate_endpoint(
    source: &str,
    kind: TrackKind,
    endpoint: Endpoint,
    value: f32,
) -> Result<(), TrackError> {
    let reason = scalar_domain_error(kind, value);
    match reason {
        Some(reason) => Err(TrackError::InvalidEndpoint {
            source: source.to_owned(),
            kind,
            endpoint,
            reason,
        }),
        None => Ok(()),
    }
}

fn validate_curve_domain(
    source: &str,
    kind: TrackKind,
    curve: &ScalarCurve,
) -> Result<(), TrackError> {
    for (keyframe_index, keyframe) in curve.keyframes().enumerate() {
        if let Some(reason) = scalar_domain_error(kind, keyframe.value()) {
            return Err(TrackError::InvalidKeyframe {
                source: source.to_owned(),
                kind,
                keyframe_index,
                reason,
            });
        }
    }

    let mut start = curve.first();
    for (segment_index, segment) in curve.segments().iter().enumerate() {
        let end = segment.end();
        if let Easing::CubicBezier(easing) = segment.easing() {
            let from = binary32_rational(start.value());
            let to = binary32_rational(end.value());
            let delta = &to - &from;
            for (control, timing_control) in [
                (CubicControl::Y1, easing.y1()),
                (CubicControl::Y2, easing.y2()),
            ] {
                let property_control = &from + &delta * binary32_rational(timing_control);
                if let Some(reason) = rational_domain_error(kind, &property_control) {
                    return Err(TrackError::UnsafeCubicControl {
                        source: source.to_owned(),
                        kind,
                        segment_index,
                        control,
                        reason,
                    });
                }
            }
        }
        start = end;
    }
    Ok(())
}

fn validate_live_underlying_easing(
    source: &str,
    kind: TrackKind,
    easing: &Easing,
) -> Result<(), TrackError> {
    let Easing::CubicBezier(easing) = easing else {
        return Ok(());
    };
    // The lower endpoint is known only at sample time. Constraining timing-y
    // to the unit interval makes every result a convex interpolation between
    // two already-valid values, so no deferred domain check is required.
    for (control, value) in [
        (CubicControl::Y1, easing.y1()),
        (CubicControl::Y2, easing.y2()),
    ] {
        if !(0.0..=1.0).contains(&value) {
            return Err(TrackError::UnsafeCubicControl {
                source: source.to_owned(),
                kind,
                segment_index: 0,
                control,
                reason: ScalarDomainError::OutsideUnitInterval,
            });
        }
    }
    Ok(())
}

fn validate_transform_curve_domain(source: &str, curve: &TransformCurve) -> Result<(), TrackError> {
    for (keyframe_index, keyframe) in curve.keyframes().enumerate() {
        if keyframe.value.into_lens_op().is_none() {
            return Err(TrackError::InvalidTransformProjection {
                source: source.to_owned(),
                keyframe_index,
            });
        }
    }

    let mut start = curve.first;
    for (segment_index, segment) in curve.segments.iter().enumerate() {
        if let Easing::CubicBezier(easing) = &segment.easing {
            let from = start.value.components();
            let to = segment.end.value.components();
            for component in 0..3 {
                let from = binary32_rational(from[component]);
                let delta = binary32_rational(to[component]) - &from;
                for (control, timing_control) in [
                    (CubicControl::Y1, easing.y1()),
                    (CubicControl::Y2, easing.y2()),
                ] {
                    let property_control = &from + &delta * binary32_rational(timing_control);
                    if let Some(reason) =
                        rational_domain_error(TrackKind::LensTransform, &property_control)
                    {
                        return Err(TrackError::UnsafeCubicControl {
                            source: source.to_owned(),
                            kind: TrackKind::LensTransform,
                            segment_index,
                            control,
                            reason,
                        });
                    }
                }
            }
        }
        start = segment.end;
    }
    Ok(())
}

fn scalar_domain_error(kind: TrackKind, value: f32) -> Option<ScalarDomainError> {
    if !value.is_finite() {
        return Some(ScalarDomainError::NotFinite);
    }
    match kind {
        TrackKind::AxisStart => None,
        TrackKind::FixedSize if value < 0.0 => Some(ScalarDomainError::Negative),
        TrackKind::FixedSize => None,
        TrackKind::Opacity if !(0.0..=1.0).contains(&value) => {
            Some(ScalarDomainError::OutsideUnitInterval)
        }
        TrackKind::Opacity => None,
        TrackKind::SolidFill => unreachable!("solid fill tracks have no scalar domain"),
        TrackKind::LensTransform => None,
        TrackKind::PathGeometry => unreachable!("path tracks have no scalar domain"),
    }
}

fn rational_domain_error(kind: TrackKind, value: &BigRational) -> Option<ScalarDomainError> {
    let zero = BigRational::zero();
    let maximum = binary32_rational(f32::MAX);
    match kind {
        TrackKind::AxisStart if value < &-&maximum || value > &maximum => {
            Some(ScalarDomainError::OutsideFiniteBinary32)
        }
        TrackKind::AxisStart => None,
        TrackKind::FixedSize if value < &zero => Some(ScalarDomainError::Negative),
        TrackKind::FixedSize if value > &maximum => Some(ScalarDomainError::OutsideFiniteBinary32),
        TrackKind::FixedSize => None,
        TrackKind::Opacity if value < &zero || value > &BigRational::one() => {
            Some(ScalarDomainError::OutsideUnitInterval)
        }
        TrackKind::Opacity => None,
        TrackKind::SolidFill => unreachable!("solid fill tracks have no scalar domain"),
        TrackKind::LensTransform if value < &-&maximum || value > &maximum => {
            Some(ScalarDomainError::OutsideFiniteBinary32)
        }
        TrackKind::LensTransform => None,
        TrackKind::PathGeometry => unreachable!("path tracks have no scalar domain"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SampledTransform {
    value: TransformValue,
    repeat_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactColor {
    /// Straight legacy-sRGB red, green, blue, and alpha in unbounded byte
    /// channel space. Values are exact until the complete sandwich projects.
    channels: [BigRational; 4],
}

impl From<Color> for ExactColor {
    fn from(color: Color) -> Self {
        let argb = color.argb();
        Self {
            channels: [
                BigRational::from_integer(BigInt::from((argb >> 16) & 0xff)),
                BigRational::from_integer(BigInt::from((argb >> 8) & 0xff)),
                BigRational::from_integer(BigInt::from(argb & 0xff)),
                BigRational::from_integer(BigInt::from(argb >> 24)),
            ],
        }
    }
}

impl ExactColor {
    fn interpolate(&self, to: &Self, progress: &BigRational) -> Self {
        if progress.is_zero() || self == to {
            return self.clone();
        }
        if progress.is_one() {
            return to.clone();
        }
        Self {
            channels: std::array::from_fn(|index| {
                &self.channels[index] + (&to.channels[index] - &self.channels[index]) * progress
            }),
        }
    }

    fn add(&self, effect: &Self) -> Self {
        Self {
            channels: std::array::from_fn(|index| &self.channels[index] + &effect.channels[index]),
        }
    }

    fn accumulated(&self, terminal: Color, repeat_index: u64) -> Self {
        if repeat_index == 0 {
            return self.clone();
        }
        let terminal = Self::from(terminal);
        let repeat_index = BigRational::from_integer(BigInt::from(repeat_index));
        Self {
            channels: std::array::from_fn(|index| {
                &self.channels[index] + &terminal.channels[index] * &repeat_index
            }),
        }
    }

    fn into_color(self) -> Color {
        let [red, green, blue, alpha] = self.channels.map(round_color_channel);
        Color(
            (u32::from(alpha) << 24)
                | (u32::from(red) << 16)
                | (u32::from(green) << 8)
                | u32::from(blue),
        )
    }
}

fn round_color_channel(value: BigRational) -> u8 {
    let zero = BigRational::zero();
    let maximum = BigRational::from_integer(BigInt::from(255_u16));
    let value = if value < zero {
        zero
    } else if value > maximum {
        maximum
    } else {
        value
    };
    // Presentation conversion for non-negative channels is nearest integer,
    // with an exact half rounded upward (matching SVG color presentation in
    // Chromium and Rust's positive `f32::round`).
    let numerator = value.numer();
    let denominator = value.denom();
    let rounded: BigInt = (numerator * 2_u8 + denominator) / (denominator * 2_u8);
    rounded
        .to_u8()
        .expect("a clamped color channel is an integer from zero through 255")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampledColor {
    value: ExactColor,
    repeat_index: u64,
}

fn stack_underlying_scalar(
    document: &Document,
    stack: &[Track],
) -> Result<f32, AnimationValueError> {
    let track = &stack[0];
    let node = document
        .node_for_key(track.target.node)
        .expect("target validity is checked before composition");
    let value = track
        .target
        .property
        .spec()
        .base_value(node)
        .expect("target applicability is checked before composition");
    track.kind.underlying_scalar(track.target, &value)
}

fn stack_underlying_color(
    document: &Document,
    stack: &[Track],
) -> Result<ExactColor, AnimationValueError> {
    let track = &stack[0];
    let node = document
        .node_for_key(track.target.node)
        .expect("target validity is checked before composition");
    let value = track
        .target
        .property
        .spec()
        .base_value(node)
        .expect("target applicability is checked before composition");
    track.kind.underlying_color(track.target, &value)
}

fn stack_underlying_lens_ops(document: &Document, stack: &[Track]) -> Vec<LensOp> {
    let track = &stack[0];
    let node = document
        .node_for_key(track.target.node)
        .expect("target validity is checked before composition");
    match track
        .target
        .property
        .spec()
        .base_value(node)
        .expect("target applicability is checked before composition")
    {
        PropertyValue::LensOps(ops) => ops,
        _ => unreachable!("lens transform tracks target the LensOps property"),
    }
}

fn validate_stack_underlying_lens_ops(
    document: &Document,
    stack: &[Track],
) -> Result<(), PropertyError> {
    let target = stack[0].target;
    PropertyValues::new(
        document,
        [(
            target,
            PropertyValue::LensOps(stack_underlying_lens_ops(document, stack)),
        )],
    )
    .map(|_| ())
}

fn sampled_transform_curve(curve: &TransformCurve, contribution: Contribution) -> SampledTransform {
    let value = active_progress(contribution).map_or_else(
        || curve.last_value(),
        |(numerator, denominator)| curve.sample(numerator, denominator),
    );
    SampledTransform {
        value,
        repeat_index: contribution.repeat_index(),
    }
}

fn sampled_color_curve(curve: &ColorCurve, contribution: Contribution) -> SampledColor {
    let value = active_progress(contribution).map_or_else(
        || ExactColor::from(curve.last_color()),
        |(numerator, denominator)| curve.sample(numerator, denominator),
    );
    SampledColor {
        value,
        repeat_index: contribution.repeat_index(),
    }
}

fn scalar_from_live_underlying_value(
    underlying: f32,
    target: f32,
    easing: &Easing,
    contribution: Contribution,
) -> f32 {
    let Some((numerator, denominator)) = active_progress(contribution) else {
        return target;
    };
    let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
    lerp_binary32_once_rational(underlying, target, &easing_apply(easing, &progress))
}

fn color_from_live_underlying_value(
    underlying: &ExactColor,
    target: Color,
    easing: &Easing,
    contribution: Contribution,
) -> ExactColor {
    let Some((numerator, denominator)) = active_progress(contribution) else {
        return ExactColor::from(target);
    };
    let progress = BigRational::new(BigInt::from(numerator), BigInt::from(denominator));
    underlying.interpolate(&ExactColor::from(target), &easing_apply(easing, &progress))
}

fn accumulated_scalar(track: &Track, sampled: ScalarSample) -> Result<f32, AnimationValueError> {
    if track.iteration_composite == IterationCompositeOperation::Replace
        || sampled.repeat_index() == 0
    {
        return Ok(sampled.value());
    }

    let terminal = match &track.effect {
        TrackEffect::ScalarCurve(curve) => curve.last_value(),
        _ => unreachable!("only scalar curves use iteration accumulation"),
    };
    let exact = binary32_rational(sampled.value())
        + binary32_rational(terminal)
            * BigRational::from_integer(BigInt::from(sampled.repeat_index()));
    let value = if exact.is_zero()
        && sampled.value() == 0.0
        && sampled.value().is_sign_negative()
        && terminal == 0.0
        && terminal.is_sign_negative()
    {
        -0.0
    } else {
        round_rational_to_binary32(&exact)
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or(AnimationValueError::NonFiniteResult {
            target: track.target,
            operation: AnimationValueOperation::Accumulate,
        })
}

fn accumulated_transform(
    track: &Track,
    sampled: SampledTransform,
) -> Result<TransformValue, AnimationValueError> {
    if track.iteration_composite == IterationCompositeOperation::Replace
        || sampled.repeat_index == 0
    {
        return Ok(sampled.value);
    }
    let terminal = match &track.effect {
        TrackEffect::TransformCurve(curve) => curve.last_value(),
        _ => unreachable!("only transform curves use transform accumulation"),
    };
    sampled
        .value
        .accumulated(terminal, sampled.repeat_index)
        .ok_or(AnimationValueError::NonFiniteResult {
            target: track.target,
            operation: AnimationValueOperation::Accumulate,
        })
}

fn accumulated_color(track: &Track, sampled: SampledColor) -> ExactColor {
    if track.iteration_composite == IterationCompositeOperation::Replace
        || sampled.repeat_index == 0
    {
        return sampled.value;
    }
    let terminal = match &track.effect {
        TrackEffect::SolidFillCurve(curve) => curve.last_color(),
        _ => unreachable!("only color curves use color accumulation"),
    };
    sampled.value.accumulated(terminal, sampled.repeat_index)
}

fn projected_transform(
    track: &Track,
    value: TransformValue,
) -> Result<LensOp, AnimationValueError> {
    value
        .into_lens_op()
        .ok_or(AnimationValueError::NonFiniteResult {
            target: track.target,
            operation: AnimationValueOperation::Project,
        })
}

fn add_scalar(
    target: PropertyTarget,
    underlying: f32,
    effect: f32,
) -> Result<f32, AnimationValueError> {
    let exact = binary32_rational(underlying) + binary32_rational(effect);
    let value = if exact.is_zero()
        && underlying == 0.0
        && underlying.is_sign_negative()
        && effect == 0.0
        && effect.is_sign_negative()
    {
        -0.0
    } else {
        round_rational_to_binary32(&exact)
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or(AnimationValueError::NonFiniteResult {
            target,
            operation: AnimationValueOperation::Add,
        })
}

/// Immutable, document-bound effect stacks in canonical target
/// order.
///
/// The constructor receives tracks in low-to-high effect priority. Canonical
/// target grouping preserves that relative order within each target. Priority
/// is a program-level relation supplied by the frontend, not an intrinsic
/// property of a [`Track`].
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationProgram {
    compiler_id: String,
    document_root: NodeKey,
    tracks: Vec<Track>,
}

impl AnimationProgram {
    pub fn new(
        document: &Document,
        compiler_id: impl Into<String>,
        mut tracks_low_to_high_priority: Vec<Track>,
    ) -> Result<Self, ProgramError> {
        let compiler_id = compiler_id.into();
        if compiler_id.is_empty() {
            return Err(ProgramError::EmptyCompilerId);
        }
        let document_root = document
            .key_of(document.root)
            .ok_or(ProgramError::DocumentHasNoLiveRoot)?;

        // Stable target grouping preserves the caller's low-to-high priority
        // order inside every effect stack.
        tracks_low_to_high_priority.sort_by_key(Track::target);
        let tracks = tracks_low_to_high_priority;

        for stack in tracks.chunk_by(|left, right| left.target == right.target) {
            if stack[0].kind == TrackKind::SolidFill
                && stack.len() > MAX_SOLID_FILL_EFFECTS_PER_TARGET
            {
                return Err(ProgramError::TooManySolidFillEffects {
                    target: stack[0].target,
                    count: stack.len(),
                    maximum: MAX_SOLID_FILL_EFFECTS_PER_TARGET,
                });
            }
        }

        for track in &tracks {
            let endpoint_values = track.endpoint_values();
            let keyframe_count = endpoint_values.len();
            for (keyframe_index, value) in endpoint_values.into_iter().enumerate() {
                PropertyValues::new(document, [(track.target, value)]).map_err(|error| {
                    if matches!(
                        track.effect,
                        TrackEffect::ScalarFromLiveUnderlying { .. }
                            | TrackEffect::SolidFillFromLiveUnderlying { .. }
                    ) {
                        ProgramError::InvalidEndpoint {
                            source: track.source.clone(),
                            endpoint: Endpoint::To,
                            error,
                        }
                    } else if keyframe_count == 2 {
                        ProgramError::InvalidEndpoint {
                            source: track.source.clone(),
                            endpoint: if keyframe_index == 0 {
                                Endpoint::From
                            } else {
                                Endpoint::To
                            },
                            error,
                        }
                    } else {
                        ProgramError::InvalidKeyframe {
                            source: track.source.clone(),
                            keyframe_index,
                            error,
                        }
                    }
                })?;
            }
        }

        for stack in tracks.chunk_by(|left, right| left.target == right.target) {
            if !stack.iter().any(Track::needs_underlying) {
                continue;
            }
            let sources = || unique_sources(stack.iter().filter(|track| track.needs_underlying()));
            if stack[0].kind == TrackKind::LensTransform {
                validate_stack_underlying_lens_ops(document, stack).map_err(|error| {
                    ProgramError::InvalidValues {
                        sources: sources(),
                        error,
                    }
                })?;
            } else if stack[0].kind == TrackKind::SolidFill {
                stack_underlying_color(document, stack).map_err(|error| {
                    ProgramError::InvalidComposition {
                        sources: sources(),
                        error,
                    }
                })?;
            } else {
                stack_underlying_scalar(document, stack).map_err(|error| {
                    ProgramError::InvalidComposition {
                        sources: sources(),
                        error,
                    }
                })?;
            }
        }

        Ok(Self {
            compiler_id,
            document_root,
            tracks,
        })
    }

    pub fn empty(
        document: &Document,
        compiler_id: impl Into<String>,
    ) -> Result<Self, ProgramError> {
        Self::new(document, compiler_id, vec![])
    }

    pub fn compiler_id(&self) -> &str {
        &self.compiler_id
    }

    pub const fn document_root(&self) -> NodeKey {
        self.document_root
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Target-major effect stacks. Tracks inside each slice are ordered
    /// from lowest to highest effect priority.
    pub fn effect_stacks(&self) -> impl Iterator<Item = &[Track]> {
        self.tracks
            .chunk_by(|left, right| left.target == right.target)
    }

    /// Sample all tracks atomically. Any stale target or invalid combined
    /// effective state rejects the complete value set.
    pub fn sample(
        &self,
        document: &Document,
        time: SampleTime,
    ) -> Result<PropertyValues, SampleError> {
        let actual = document.key_of(document.root);
        if actual != Some(self.document_root) {
            return Err(SampleError::DocumentMismatch {
                compiler_id: self.compiler_id.clone(),
                time,
                expected: self.document_root,
                actual,
            });
        }

        // Program validity is independent from contribution state. A stale or
        // newly inapplicable target must not disappear behind pre-begin or
        // post-remove inactivity and silently turn Sample into Base.
        for stack in self.effect_stacks() {
            let target = stack[0].target;
            let error = match document.node_for_key(target.node) {
                None => Some(PropertyError::StaleTarget { target }),
                Some(node) if !target.property.spec().applies_to(node) => {
                    Some(PropertyError::Inapplicable {
                        target,
                        applicability: target.property.spec().applicability,
                    })
                }
                Some(_) => None,
            };
            if let Some(error) = error {
                return Err(SampleError::InvalidValues {
                    compiler_id: self.compiler_id.clone(),
                    time,
                    sources: unique_sources(stack.iter()),
                    error,
                });
            }
            if stack.iter().any(Track::needs_underlying) {
                let sources =
                    || unique_sources(stack.iter().filter(|track| track.needs_underlying()));
                if stack[0].kind == TrackKind::LensTransform {
                    validate_stack_underlying_lens_ops(document, stack).map_err(|error| {
                        SampleError::InvalidValues {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: sources(),
                            error,
                        }
                    })?;
                } else if stack[0].kind == TrackKind::SolidFill {
                    stack_underlying_color(document, stack).map_err(|error| {
                        SampleError::InvalidComposition {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: sources(),
                            error,
                        }
                    })?;
                } else {
                    stack_underlying_scalar(document, stack).map_err(|error| {
                        SampleError::InvalidComposition {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: sources(),
                            error,
                        }
                    })?;
                }
            }
        }

        let mut entries = Vec::new();
        let mut contributors = Vec::new();
        let mut sampled = Vec::new();
        for stack in self.effect_stacks() {
            sampled.clear();
            sampled.extend(stack.iter().filter_map(|track| {
                track
                    .contribution(time)
                    .map(|contribution| (track, contribution))
            }));
            if sampled.is_empty() {
                continue;
            }

            let replacement = sampled
                .iter()
                .rposition(|(track, _)| track.overwrites_underlying());
            let influential = &sampled[replacement.unwrap_or(0)..];
            let sources = || unique_sources(influential.iter().map(|(track, _)| *track));
            let track = influential.last().expect("sampled stack is non-empty").0;
            if track.kind == TrackKind::PathGeometry {
                let (track, contribution) = *influential
                    .last()
                    .expect("a contributing path stack is non-empty");
                let value = match &track.effect {
                    TrackEffect::PathCurve(curve) => active_progress(contribution).map_or_else(
                        || PropertyValue::PathGeometry(Arc::clone(curve.last_path())),
                        |(numerator, denominator)| {
                            PropertyValue::PathGeometry(curve.sample(numerator, denominator))
                        },
                    ),
                    TrackEffect::DiscreteCurve(curve) => active_progress(contribution).map_or_else(
                        || curve.last_value().clone(),
                        |(numerator, denominator)| curve.sample(numerator, denominator).clone(),
                    ),
                    TrackEffect::EasedDiscretePair(effect) => active_progress(contribution)
                        .map_or_else(
                            || effect.to().clone(),
                            |(numerator, denominator)| {
                                effect.sample(numerator, denominator).clone()
                            },
                        ),
                    _ => unreachable!("path targets accept only path geometry effects"),
                };
                entries.push((track.target, value));
            } else if track.kind == TrackKind::LensTransform {
                let mut value = replacement
                    .is_none()
                    .then(|| stack_underlying_lens_ops(document, stack));
                for &(track, contribution) in influential {
                    let TrackEffect::TransformCurve(curve) = &track.effect else {
                        unreachable!("LensOps targets accept only transform curves")
                    };
                    let sampled = sampled_transform_curve(curve, contribution);
                    let effect = accumulated_transform(track, sampled)
                        .and_then(|value| projected_transform(track, value))
                        .map_err(|error| SampleError::InvalidComposition {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: unique_sources(std::iter::once(track)),
                            error,
                        })?;
                    match track.composite {
                        CompositeOperation::Replace => value = Some(vec![effect]),
                        CompositeOperation::Add => value
                            .as_mut()
                            .expect("additive transform stacks have an underlying list")
                            .push(effect),
                        CompositeOperation::InterpolateLiveUnderlying => {
                            unreachable!("transform curves reject live-underlying composition")
                        }
                    }
                }
                entries.push((
                    track.target,
                    PropertyValue::LensOps(value.expect("a contributing stack produces a value")),
                ));
            } else if track.kind == TrackKind::SolidFill {
                let mut value = match replacement {
                    Some(_) => None,
                    None => Some(stack_underlying_color(document, stack).map_err(|error| {
                        SampleError::InvalidComposition {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: sources(),
                            error,
                        }
                    })?),
                };

                for &(track, contribution) in influential {
                    match &track.effect {
                        TrackEffect::SolidFillCurve(curve) => {
                            let sampled = sampled_color_curve(curve, contribution);
                            let effect = accumulated_color(track, sampled);
                            value = Some(match track.composite {
                                CompositeOperation::Replace => effect,
                                CompositeOperation::Add => value
                                    .as_ref()
                                    .expect("additive fill-color stacks have an underlying color")
                                    .add(&effect),
                                CompositeOperation::InterpolateLiveUnderlying => {
                                    unreachable!("color curves reject live-underlying composition")
                                }
                            });
                        }
                        TrackEffect::SolidFillFromLiveUnderlying { target, easing } => {
                            value = Some(color_from_live_underlying_value(
                                value.as_ref().expect(
                                    "live-underlying color effects retain their lower sandwich value",
                                ),
                                *target,
                                easing,
                                contribution,
                            ));
                        }
                        _ => unreachable!("fill-color targets accept only color effects"),
                    }
                }
                entries.push((
                    track.target,
                    PropertyValue::Paints(Paints::solid(
                        value
                            .expect("a contributing fill-color stack produces a value")
                            .into_color(),
                    )),
                ));
            } else {
                let mut value = match replacement {
                    Some(_) => None,
                    None => Some(stack_underlying_scalar(document, stack).map_err(|error| {
                        SampleError::InvalidComposition {
                            compiler_id: self.compiler_id.clone(),
                            time,
                            sources: sources(),
                            error,
                        }
                    })?),
                };

                for (index, &(track, contribution)) in influential.iter().enumerate() {
                    match &track.effect {
                        TrackEffect::ScalarCurve(curve) => {
                            let sampled = curve.sample(contribution);
                            let effect = accumulated_scalar(track, sampled).map_err(|error| {
                                SampleError::InvalidComposition {
                                    compiler_id: self.compiler_id.clone(),
                                    time,
                                    sources: unique_sources(std::iter::once(track)),
                                    error,
                                }
                            })?;
                            value = Some(match track.composite {
                                CompositeOperation::Replace => effect,
                                CompositeOperation::Add => add_scalar(
                                    track.target,
                                    value.expect("additive stacks have an underlying scalar"),
                                    effect,
                                )
                                .map_err(|error| SampleError::InvalidComposition {
                                    compiler_id: self.compiler_id.clone(),
                                    time,
                                    sources: unique_sources(
                                        influential[..=index].iter().map(|(track, _)| *track),
                                    ),
                                    error,
                                })?,
                                CompositeOperation::InterpolateLiveUnderlying => {
                                    unreachable!("scalar curves reject live-underlying composition")
                                }
                            });
                        }
                        TrackEffect::ScalarFromLiveUnderlying { target, easing } => {
                            value = Some(scalar_from_live_underlying_value(
                                value.expect(
                                    "live-underlying effects retain their lower sandwich value",
                                ),
                                *target,
                                easing,
                                contribution,
                            ));
                        }
                        TrackEffect::SolidFillCurve(_)
                        | TrackEffect::SolidFillFromLiveUnderlying { .. }
                        | TrackEffect::TransformCurve(_)
                        | TrackEffect::PathCurve(_)
                        | TrackEffect::DiscreteCurve(_)
                        | TrackEffect::EasedDiscretePair(_) => {
                            unreachable!("scalar targets accept only scalar effects")
                        }
                    }
                }
                entries.push((
                    track.target,
                    track.kind.project_scalar(
                        value.expect("a contributing scalar stack produces a value"),
                    ),
                ));
            }
            contributors.extend(influential.iter().map(|(track, _)| *track));
        }
        PropertyValues::new(document, entries).map_err(|error| SampleError::InvalidValues {
            compiler_id: self.compiler_id.clone(),
            time,
            sources: implicated_sources(&contributors, &error),
            error,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgramError {
    EmptyCompilerId,
    DocumentHasNoLiveRoot,
    TooManySolidFillEffects {
        target: PropertyTarget,
        count: usize,
        maximum: usize,
    },
    InvalidEndpoint {
        source: String,
        endpoint: Endpoint,
        error: PropertyError,
    },
    InvalidKeyframe {
        source: String,
        keyframe_index: usize,
        error: PropertyError,
    },
    InvalidValues {
        sources: Vec<String>,
        error: PropertyError,
    },
    InvalidComposition {
        sources: Vec<String>,
        error: AnimationValueError,
    },
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::EmptyCompilerId => {
                write!(f, "animation compiler identity must not be empty")
            }
            ProgramError::DocumentHasNoLiveRoot => {
                write!(f, "animation program document has no live root")
            }
            ProgramError::TooManySolidFillEffects {
                target,
                count,
                maximum,
            } => write!(
                f,
                "animation target {target:?} has {count} solid-fill effects; the exact-channel limit is {maximum}"
            ),
            ProgramError::InvalidEndpoint {
                source,
                endpoint,
                error,
            } => write!(
                f,
                "animation track {source} has an invalid {endpoint:?} endpoint: {error}"
            ),
            ProgramError::InvalidKeyframe {
                source,
                keyframe_index,
                error,
            } => write!(
                f,
                "animation track {source} has an invalid keyframe {keyframe_index}: {error}"
            ),
            ProgramError::InvalidValues { sources, error } => write!(
                f,
                "animation tracks {sources:?} have invalid underlying values: {error}"
            ),
            ProgramError::InvalidComposition { sources, error } => write!(
                f,
                "animation tracks {sources:?} have invalid composition: {error}"
            ),
        }
    }
}

impl std::error::Error for ProgramError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProgramError::InvalidEndpoint { error, .. }
            | ProgramError::InvalidKeyframe { error, .. }
            | ProgramError::InvalidValues { error, .. } => Some(error),
            ProgramError::InvalidComposition { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SampleError {
    DocumentMismatch {
        compiler_id: String,
        time: SampleTime,
        expected: NodeKey,
        actual: Option<NodeKey>,
    },
    InvalidValues {
        compiler_id: String,
        time: SampleTime,
        sources: Vec<String>,
        error: PropertyError,
    },
    InvalidComposition {
        compiler_id: String,
        time: SampleTime,
        sources: Vec<String>,
        error: AnimationValueError,
    },
}

impl std::fmt::Display for SampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleError::DocumentMismatch {
                compiler_id,
                time,
                expected,
                actual,
            } => write!(
                f,
                "animation program {compiler_id} failed at {}ns: compiled for {expected:?}, found {actual:?}",
                time.nanoseconds()
            ),
            SampleError::InvalidValues {
                compiler_id,
                time,
                sources,
                error,
            } => write!(
                f,
                "animation program {compiler_id} failed at {}ns for {sources:?}: {error}",
                time.nanoseconds()
            ),
            SampleError::InvalidComposition {
                compiler_id,
                time,
                sources,
                error,
            } => write!(
                f,
                "animation program {compiler_id} failed composition at {}ns for {sources:?}: {error}",
                time.nanoseconds()
            ),
        }
    }
}

impl std::error::Error for SampleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SampleError::InvalidValues { error, .. } => Some(error),
            SampleError::InvalidComposition { error, .. } => Some(error),
            _ => None,
        }
    }
}

fn implicated_sources(tracks: &[&Track], error: &PropertyError) -> Vec<String> {
    let implicated = match error {
        PropertyError::DuplicateTarget { target }
        | PropertyError::StaleTarget { target }
        | PropertyError::WrongValueKind { target, .. }
        | PropertyError::Inapplicable { target, .. }
        | PropertyError::InvalidValue { target, .. } => tracks
            .iter()
            .copied()
            .filter(|track| track.target == *target)
            .collect::<Vec<_>>(),
        PropertyError::InvalidEffectiveState {
            node, properties, ..
        } => tracks
            .iter()
            .copied()
            .filter(|track| {
                track.target.node == *node && properties.contains(&track.target.property)
            })
            .collect::<Vec<_>>(),
    };
    unique_sources(implicated)
}

fn unique_sources<'a>(tracks: impl IntoIterator<Item = &'a Track>) -> Vec<String> {
    let mut sources = Vec::new();
    for track in tracks {
        if !sources.iter().any(|source| source == &track.source) {
            sources.push(track.source.clone());
        }
    }
    sources
}
