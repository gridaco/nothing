//! Resolved image filtering — one source-neutral, checked effect program.
//!
//! A producer resolves its own lookup, units, names, inheritance, and source
//! grammar before constructing this program. The contract keeps only the
//! image facts a painter needs: a local operation space, hard regions, a
//! bounded acyclic node list, and explicit inputs. No authored identifier or
//! backend object crosses this seam.

use std::sync::Arc;

use cg::CGColor32F;
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

/// One resolved two-input compositing rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterComposite {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Lighter,
    Arithmetic { k1: f32, k2: f32, k3: f32, k4: f32 },
}

/// One source-neutral blend function over a foreground and backdrop image.
///
/// The enum is deliberately owned by the filter contract. Paint-stack blend
/// vocabulary is a different operation even where its members have the same
/// names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterBlend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// One source-neutral morphology operation over premultiplied image channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMorphology {
    Erode,
    Dilate,
}

/// One source-neutral procedural-noise formula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterTurbulenceKind {
    Turbulence,
    FractalNoise,
}

/// One non-premultiplied channel selected from a displacement image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDisplacementChannel {
    Red,
    Green,
    Blue,
    Alpha,
}

/// Four exact byte lookup tables for one non-premultiplied RGBA operation.
///
/// Channel order is named at construction and access, so a producer cannot
/// leak its source grammar or make the painter infer an ordering convention.
/// The fixed array shape is the contract's length check: every admitted
/// channel maps all 256 possible input bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterChannelTables(Arc<[[u8; 256]; 4]>);

impl FilterChannelTables {
    #[must_use]
    pub fn new(red: [u8; 256], green: [u8; 256], blue: [u8; 256], alpha: [u8; 256]) -> Self {
        Self(Arc::new([red, green, blue, alpha]))
    }

    #[must_use]
    pub fn red(&self) -> &[u8; 256] {
        &self.0[0]
    }

    #[must_use]
    pub fn green(&self) -> &[u8; 256] {
        &self.0[1]
    }

    #[must_use]
    pub fn blue(&self) -> &[u8; 256] {
        &self.0[2]
    }

    #[must_use]
    pub fn alpha(&self) -> &[u8; 256] {
        &self.0[3]
    }
}

/// The admitted operation vocabulary of a resolved filter node.
///
/// This enum grows only when a producer proves a new operation through the
/// same resolved seam. It states only operations a producer has proved, not
/// the eventual family.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterPrimitive {
    GaussianBlur {
        sigma_x: f32,
        sigma_y: f32,
    },
    Offset {
        dx: f32,
        dy: f32,
    },
    /// A bounded solid source. Its components are resolved sRGB values; the
    /// float alpha preserves a separately multiplied source alpha and
    /// opacity until the raster target quantizes once.
    SolidColor {
        color: CGColor32F,
    },
    Composite {
        operator: FilterComposite,
    },
    /// Blend `inputs[0]` as the foreground over `inputs[1]` as the backdrop.
    Blend {
        mode: FilterBlend,
    },
    /// One resolved shadow operation. The painter draws the shadow below the
    /// input and preserves the input as the operation's foreground.
    DropShadow {
        dx: f32,
        dy: f32,
        sigma_x: f32,
        sigma_y: f32,
        /// Resolved sRGB shadow colour. The operation converts its colour
        /// channels into its declared interpolation space at paint time.
        color: CGColor32F,
    },
    /// One row-major 4x5 matrix over non-premultiplied RGBA.
    ColorMatrix {
        matrix: [f32; 20],
    },
    /// Four exact byte lookups over non-premultiplied RGBA.
    ComponentTransfer {
        tables: FilterChannelTables,
    },
    /// Per-channel extrema over one rectangular, axis-aligned kernel.
    Morphology {
        operator: FilterMorphology,
        radius_x: f32,
        radius_y: f32,
    },
    /// A bounded four-channel procedural-noise source. The octave count is
    /// already capped to the resolved algorithm's meaningful range.
    Turbulence {
        kind: FilterTurbulenceKind,
        base_frequency_x: f32,
        base_frequency_y: f32,
        num_octaves: u8,
        seed: f32,
        stitch_tiles: bool,
    },
    /// Displace `inputs[0]` by non-premultiplied channels from `inputs[1]`.
    DisplacementMap {
        scale: f32,
        x_channel: FilterDisplacementChannel,
        y_channel: FilterDisplacementChannel,
    },
    Merge,
}

/// One operation in a checked filter program.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterNode {
    inputs: Arc<[FilterInput]>,
    region: Rectangle,
    color_space: FilterColorSpace,
    primitive: FilterPrimitive,
}

impl FilterNode {
    #[must_use]
    pub fn new(
        inputs: Arc<[FilterInput]>,
        region: Rectangle,
        color_space: FilterColorSpace,
        primitive: FilterPrimitive,
    ) -> Self {
        Self {
            inputs,
            region,
            color_space,
            primitive,
        }
    }

    #[must_use]
    pub fn inputs(&self) -> &[FilterInput] {
        &self.inputs
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
    pub fn primitive(&self) -> FilterPrimitive {
        self.primitive.clone()
    }
}

/// Why a resolved filter program is not a trustworthy render fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterProgramError {
    Empty,
    TooManyNodes,
    InputIsNotEarlier {
        node: usize,
        input: usize,
    },
    InvalidInputCount {
        node: usize,
        expected: usize,
        actual: usize,
    },
    InvalidRegion {
        node: usize,
    },
    InvalidGaussianBlur {
        node: usize,
    },
    InvalidOffset {
        node: usize,
    },
    InvalidComposite {
        node: usize,
    },
    InvalidDropShadow {
        node: usize,
    },
    InvalidColorMatrix {
        node: usize,
    },
    InvalidMorphology {
        node: usize,
    },
    InvalidTurbulence {
        node: usize,
    },
    InvalidDisplacementMap {
        node: usize,
    },
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
            Self::InvalidInputCount {
                node,
                expected,
                actual,
            } => write!(
                f,
                "filter node {node} has {actual} inputs; its operation requires {expected}"
            ),
            Self::InvalidRegion { node } => {
                write!(f, "filter node {node} has a non-finite or empty region")
            }
            Self::InvalidGaussianBlur { node } => write!(
                f,
                "filter node {node} has a non-finite or negative Gaussian sigma"
            ),
            Self::InvalidOffset { node } => {
                write!(f, "filter node {node} has a non-finite offset")
            }
            Self::InvalidComposite { node } => {
                write!(
                    f,
                    "filter node {node} has a non-finite arithmetic coefficient"
                )
            }
            Self::InvalidDropShadow { node } => write!(
                f,
                "filter node {node} has a non-finite offset or non-finite/negative shadow sigma"
            ),
            Self::InvalidColorMatrix { node } => {
                write!(
                    f,
                    "filter node {node} has a non-finite color-matrix coefficient"
                )
            }
            Self::InvalidMorphology { node } => write!(
                f,
                "filter node {node} has a non-finite or negative morphology radius"
            ),
            Self::InvalidTurbulence { node } => write!(
                f,
                "filter node {node} has a non-finite/negative frequency, non-finite seed, or more than nine octaves"
            ),
            Self::InvalidDisplacementMap { node } => {
                write!(f, "filter node {node} has a non-finite displacement scale")
            }
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
            let expected_inputs = match node.primitive {
                FilterPrimitive::GaussianBlur { .. }
                | FilterPrimitive::Offset { .. }
                | FilterPrimitive::DropShadow { .. }
                | FilterPrimitive::ColorMatrix { .. }
                | FilterPrimitive::ComponentTransfer { .. }
                | FilterPrimitive::Morphology { .. } => Some(1),
                FilterPrimitive::SolidColor { .. } | FilterPrimitive::Turbulence { .. } => Some(0),
                FilterPrimitive::Composite { .. }
                | FilterPrimitive::Blend { .. }
                | FilterPrimitive::DisplacementMap { .. } => Some(2),
                FilterPrimitive::Merge => None,
            };
            if let Some(expected) = expected_inputs
                && node.inputs.len() != expected
            {
                return Err(FilterProgramError::InvalidInputCount {
                    node: index,
                    expected,
                    actual: node.inputs.len(),
                });
            }
            for input in node.inputs.iter() {
                if let FilterInput::Node(input) = *input
                    && input >= index
                {
                    return Err(FilterProgramError::InputIsNotEarlier { node: index, input });
                }
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
                FilterPrimitive::Offset { dx, dy } if !dx.is_finite() || !dy.is_finite() => {
                    return Err(FilterProgramError::InvalidOffset { node: index });
                }
                FilterPrimitive::Composite {
                    operator: FilterComposite::Arithmetic { k1, k2, k3, k4 },
                } if ![k1, k2, k3, k4].into_iter().all(f32::is_finite) => {
                    return Err(FilterProgramError::InvalidComposite { node: index });
                }
                FilterPrimitive::DropShadow {
                    dx,
                    dy,
                    sigma_x,
                    sigma_y,
                    ..
                } if ![dx, dy, sigma_x, sigma_y].into_iter().all(f32::is_finite)
                    || sigma_x < 0.0
                    || sigma_y < 0.0 =>
                {
                    return Err(FilterProgramError::InvalidDropShadow { node: index });
                }
                FilterPrimitive::ColorMatrix { matrix }
                    if !matrix.into_iter().all(f32::is_finite) =>
                {
                    return Err(FilterProgramError::InvalidColorMatrix { node: index });
                }
                FilterPrimitive::Morphology {
                    radius_x, radius_y, ..
                } if !radius_x.is_finite()
                    || !radius_y.is_finite()
                    || radius_x < 0.0
                    || radius_y < 0.0 =>
                {
                    return Err(FilterProgramError::InvalidMorphology { node: index });
                }
                FilterPrimitive::Turbulence {
                    base_frequency_x,
                    base_frequency_y,
                    num_octaves,
                    seed,
                    ..
                } if !base_frequency_x.is_finite()
                    || !base_frequency_y.is_finite()
                    || base_frequency_x < 0.0
                    || base_frequency_y < 0.0
                    || num_octaves > 9
                    || !seed.is_finite() =>
                {
                    return Err(FilterProgramError::InvalidTurbulence { node: index });
                }
                FilterPrimitive::DisplacementMap { scale, .. } if !scale.is_finite() => {
                    return Err(FilterProgramError::InvalidDisplacementMap { node: index });
                }
                FilterPrimitive::Offset { .. }
                | FilterPrimitive::SolidColor { .. }
                | FilterPrimitive::Composite { .. }
                | FilterPrimitive::Blend { .. }
                | FilterPrimitive::DropShadow { .. }
                | FilterPrimitive::ColorMatrix { .. }
                | FilterPrimitive::ComponentTransfer { .. }
                | FilterPrimitive::Morphology { .. }
                | FilterPrimitive::Turbulence { .. }
                | FilterPrimitive::DisplacementMap { .. }
                | FilterPrimitive::Merge => {}
            }
        }
        Ok(Self(nodes))
    }

    pub fn iter(&self) -> impl Iterator<Item = &FilterNode> {
        self.0.iter()
    }

    /// Whether this graph may paint when its isolated source is fully
    /// transparent.
    ///
    /// This is deliberately conservative. It lets a consumer retain an
    /// otherwise empty compositing scope whenever a generated source or an
    /// additive coefficient could create visible output.
    #[must_use]
    pub fn may_paint_transparent_input(&self) -> bool {
        self.0.iter().any(|node| match node.primitive() {
            FilterPrimitive::SolidColor { color } => color.a() > 0.0,
            FilterPrimitive::Composite {
                operator: FilterComposite::Arithmetic { k4, .. },
            } => k4 > 0.0,
            FilterPrimitive::ColorMatrix { matrix } => matrix[19] > 0.0,
            FilterPrimitive::ComponentTransfer { tables } => tables.alpha()[0] > 0,
            FilterPrimitive::Turbulence { .. } => true,
            FilterPrimitive::GaussianBlur { .. }
            | FilterPrimitive::Offset { .. }
            | FilterPrimitive::Composite { .. }
            | FilterPrimitive::Blend { .. }
            | FilterPrimitive::DropShadow { .. }
            | FilterPrimitive::Morphology { .. }
            | FilterPrimitive::DisplacementMap { .. }
            | FilterPrimitive::Merge => false,
        })
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
    source_is_transparent: bool,
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
            source_is_transparent: false,
        })
    }

    /// Declare that this invocation's isolated source image is fully
    /// transparent. Generated/additive nodes may still paint; a consumer must
    /// materialize an explicit transparent source rather than a lazy backend
    /// input sentinel for that case.
    #[must_use]
    pub fn with_transparent_source(mut self) -> Self {
        self.source_is_transparent = true;
        self
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

    #[must_use]
    pub const fn source_is_transparent(&self) -> bool {
        self.source_is_transparent
    }
}

fn valid_rect(rect: Rectangle) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f32::is_finite)
}
