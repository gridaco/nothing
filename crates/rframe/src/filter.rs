//! Resolved image filtering — one source-neutral, checked effect program.
//!
//! A producer resolves its own lookup, units, names, inheritance, and source
//! grammar before constructing this program. The contract keeps only the
//! image facts a painter needs: a local operation space, hard regions, a
//! bounded acyclic node list, and explicit inputs. No authored identifier or
//! backend object crosses this seam.

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;

/// The largest resolved filter program carried by one compositing scope.
///
/// This is a contract bound, not a source-language limit. A producer whose
/// graph exceeds it refuses before constructing a frame.
pub const MAX_FILTER_NODES: usize = 256;

/// The pixel interpolation space in which one operation executes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterColorSpace {
    Srgb,
    LinearRgb,
}

/// One already-resolved input to a filter node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterInput {
    /// The isolated scope's original composite.
    Source,
    /// The alpha channel of the isolated scope's original composite, with
    /// every color channel cleared.
    SourceAlpha,
    /// The output of an earlier node in the same program.
    Node(usize),
}

/// The admitted operation vocabulary of a resolved filter node.
///
/// This enum grows only when a producer proves a new operation through the
/// same resolved seam. It deliberately starts with the first implemented
/// operation instead of predicting the eventual family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterPrimitive {
    GaussianBlur { sigma_x: f32, sigma_y: f32 },
}

/// One operation in a checked filter program.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterNode {
    input: FilterInput,
    region: Rectangle,
    color_space: FilterColorSpace,
    primitive: FilterPrimitive,
}

impl FilterNode {
    #[must_use]
    pub const fn new(
        input: FilterInput,
        region: Rectangle,
        color_space: FilterColorSpace,
        primitive: FilterPrimitive,
    ) -> Self {
        Self {
            input,
            region,
            color_space,
            primitive,
        }
    }

    #[must_use]
    pub const fn input(&self) -> FilterInput {
        self.input
    }

    #[must_use]
    pub const fn region(&self) -> Rectangle {
        self.region
    }

    #[must_use]
    pub const fn color_space(&self) -> FilterColorSpace {
        self.color_space
    }

    #[must_use]
    pub const fn primitive(&self) -> FilterPrimitive {
        self.primitive
    }
}

/// Why a resolved filter program is not a trustworthy render fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterProgramError {
    Empty,
    TooManyNodes,
    InputIsNotEarlier { node: usize, input: usize },
    InvalidRegion { node: usize },
    InvalidGaussianBlur { node: usize },
}

impl std::fmt::Display for FilterProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("a filter program must contain an operation"),
            Self::TooManyNodes => write!(
                f,
                "a filter program exceeds the {MAX_FILTER_NODES}-node contract bound"
            ),
            Self::InputIsNotEarlier { node, input } => write!(
                f,
                "filter node {node} reads node {input}, which is not earlier in the program"
            ),
            Self::InvalidRegion { node } => {
                write!(f, "filter node {node} has a non-finite or empty region")
            }
            Self::InvalidGaussianBlur { node } => write!(
                f,
                "filter node {node} has a non-finite or negative Gaussian sigma"
            ),
        }
    }
}

impl std::error::Error for FilterProgramError {}

/// A bounded acyclic list whose last node is the program output.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterProgram(Arc<[FilterNode]>);

impl FilterProgram {
    pub fn new(nodes: Arc<[FilterNode]>) -> Result<Self, FilterProgramError> {
        if nodes.is_empty() {
            return Err(FilterProgramError::Empty);
        }
        if nodes.len() > MAX_FILTER_NODES {
            return Err(FilterProgramError::TooManyNodes);
        }
        for (index, node) in nodes.iter().enumerate() {
            if let FilterInput::Node(input) = node.input
                && input >= index
            {
                return Err(FilterProgramError::InputIsNotEarlier { node: index, input });
            }
            if !valid_rect(node.region) || node.region.width <= 0.0 || node.region.height <= 0.0 {
                return Err(FilterProgramError::InvalidRegion { node: index });
            }
            match node.primitive {
                FilterPrimitive::GaussianBlur { sigma_x, sigma_y }
                    if !sigma_x.is_finite()
                        || !sigma_y.is_finite()
                        || sigma_x < 0.0
                        || sigma_y < 0.0 =>
                {
                    return Err(FilterProgramError::InvalidGaussianBlur { node: index });
                }
                FilterPrimitive::GaussianBlur { .. } => {}
            }
        }
        Ok(Self(nodes))
    }

    pub fn iter(&self) -> impl Iterator<Item = &FilterNode> {
        self.0.iter()
    }
}

/// Why a checked program cannot be applied as one resolved filter effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterError {
    InvalidTransform,
    InvalidRegion,
}

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransform => f.write_str("a filter transform must be finite"),
            Self::InvalidRegion => {
                f.write_str("a filter region must be finite and strictly positive")
            }
        }
    }
}

impl std::error::Error for FilterError {}

/// One checked filter program in its local operation space.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    transform: AffineTransform,
    region: Rectangle,
    program: FilterProgram,
}

impl Filter {
    pub fn new(
        transform: AffineTransform,
        region: Rectangle,
        program: FilterProgram,
    ) -> Result<Self, FilterError> {
        if !transform.matrix.into_iter().flatten().all(f32::is_finite) {
            return Err(FilterError::InvalidTransform);
        }
        if !valid_rect(region) || region.width <= 0.0 || region.height <= 0.0 {
            return Err(FilterError::InvalidRegion);
        }
        Ok(Self {
            transform,
            region,
            program,
        })
    }

    #[must_use]
    pub const fn transform(&self) -> AffineTransform {
        self.transform
    }

    #[must_use]
    pub const fn region(&self) -> Rectangle {
        self.region
    }

    #[must_use]
    pub const fn program(&self) -> &FilterProgram {
        &self.program
    }
}

fn valid_rect(rect: Rectangle) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f32::is_finite)
}
