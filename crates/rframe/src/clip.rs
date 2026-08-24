//! Resolved geometric clipping — source-neutral path-strategy facts.
//!
//! A producer resolves every resource lookup, coordinate system, and source
//! grammar before constructing this type. The contract receives only geometry
//! and local-to-frame transforms. One [`ClipLayer`] is the union of its
//! geometries; a [`ClipPath`] is the intersection of its layers. That normal
//! form carries a referenced clipPath and any clipPath-to-clipPath chain
//! without carrying an id, URL, DOM node, or backend path.
//!
//! This is deliberately not a mask. Text/raster fallback, per-child clipping,
//! alpha masks, and any image-backed effect have no representation here. A
//! producer that needs one must refuse rather than smuggle a resource through
//! the resolved frame.

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::frame::Geometry;

/// The maximum number of geometric contributors in one path-strategy union.
///
/// The bound keeps construction and backend path operations finite. It also
/// makes a producer's switch to a different semantic strategy explicit rather
/// than allowing an unbounded list to change meaning downstream.
pub const MAX_CLIP_GEOMETRIES_PER_LAYER: usize = 42;

/// The maximum number of chained clip layers in one resolved effect.
pub const MAX_CLIP_LAYERS: usize = 64;

/// Why resolved clip geometry cannot cross the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipGeometryError {
    /// The local rectangle carried by the geometry is non-finite or has a
    /// negative extent.
    InvalidGeometry,
    /// The local-to-frame transform has a non-finite component.
    NonFiniteTransform,
    /// Transforming the local geometry box does not produce finite frame
    /// bounds.
    NonFiniteBounds,
}

impl std::fmt::Display for ClipGeometryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidGeometry => "clip geometry has invalid local bounds",
            Self::NonFiniteTransform => "clip geometry has a non-finite transform",
            Self::NonFiniteBounds => "clip geometry has non-finite frame bounds",
        })
    }
}

impl std::error::Error for ClipGeometryError {}

/// One resolved geometric contributor to a clip union.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipGeometry {
    transform: AffineTransform,
    geometry: Geometry,
    bounds: Rectangle,
}

impl ClipGeometry {
    /// Check one geometry and its local-to-frame transform.
    pub fn new(transform: AffineTransform, geometry: Geometry) -> Result<Self, ClipGeometryError> {
        let local = geometry.local_box();
        if !valid_rectangle(local) {
            return Err(ClipGeometryError::InvalidGeometry);
        }
        if !transform
            .matrix
            .iter()
            .flatten()
            .all(|component| component.is_finite())
        {
            return Err(ClipGeometryError::NonFiniteTransform);
        }
        let bounds = math2::rect_transform(local, &transform);
        if !valid_rectangle(bounds) {
            return Err(ClipGeometryError::NonFiniteBounds);
        }
        Ok(Self {
            transform,
            geometry,
            bounds,
        })
    }

    /// The resolved transform from geometry-local coordinates to frame space.
    #[must_use]
    pub const fn transform(&self) -> AffineTransform {
        self.transform
    }

    /// The resolved local geometry.
    #[must_use]
    pub const fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// Its transformed axis-aligned geometry bounds in frame space.
    #[must_use]
    pub const fn bounds(&self) -> Rectangle {
        self.bounds
    }
}

/// Why a union layer cannot cross the resolved contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipLayerError {
    pub count: usize,
}

impl std::fmt::Display for ClipLayerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "clip union has {} geometries; the resolved path-strategy limit is {}",
            self.count, MAX_CLIP_GEOMETRIES_PER_LAYER
        )
    }
}

impl std::error::Error for ClipLayerError {}

/// One union of resolved clip geometries.
///
/// An empty layer is meaningful: it is a valid clip that admits no pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipLayer {
    geometries: Arc<[ClipGeometry]>,
    bounds: Option<Rectangle>,
}

impl ClipLayer {
    pub fn new(geometries: impl Into<Arc<[ClipGeometry]>>) -> Result<Self, ClipLayerError> {
        let geometries = geometries.into();
        if geometries.len() > MAX_CLIP_GEOMETRIES_PER_LAYER {
            return Err(ClipLayerError {
                count: geometries.len(),
            });
        }
        let bounds = geometries
            .iter()
            .filter_map(|geometry| non_empty(geometry.bounds()))
            .reduce(union);
        Ok(Self { geometries, bounds })
    }

    #[must_use]
    pub fn geometries(&self) -> &[ClipGeometry] {
        &self.geometries
    }

    /// Conservative frame-space bounds of this union, or `None` when its
    /// contributors have no area.
    #[must_use]
    pub const fn bounds(&self) -> Option<Rectangle> {
        self.bounds
    }
}

/// Why a resolved clip path cannot cross the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipPathError {
    /// `clip-path: none` is represented by omitting the scope, never by an
    /// empty list of layers.
    NoLayers,
    TooManyLayers {
        count: usize,
    },
}

impl std::fmt::Display for ClipPathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLayers => formatter.write_str("a resolved clip path has no layers"),
            Self::TooManyLayers { count } => write!(
                formatter,
                "clip path has {count} layers; the resolved limit is {MAX_CLIP_LAYERS}"
            ),
        }
    }
}

impl std::error::Error for ClipPathError {}

/// A resolved geometric clip: intersect each path-union layer in order.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipPath {
    layers: Arc<[ClipLayer]>,
    bounds: Option<Rectangle>,
}

impl ClipPath {
    pub fn new(layers: impl Into<Arc<[ClipLayer]>>) -> Result<Self, ClipPathError> {
        let layers = layers.into();
        if layers.is_empty() {
            return Err(ClipPathError::NoLayers);
        }
        if layers.len() > MAX_CLIP_LAYERS {
            return Err(ClipPathError::TooManyLayers {
                count: layers.len(),
            });
        }
        let mut bounds = layers[0].bounds();
        for layer in &layers[1..] {
            bounds = match (bounds, layer.bounds()) {
                (Some(left), Some(right)) => intersection(left, right),
                _ => None,
            };
        }
        Ok(Self { layers, bounds })
    }

    #[must_use]
    pub fn layers(&self) -> &[ClipLayer] {
        &self.layers
    }

    /// Conservative frame-space bounds of the complete intersection. `None`
    /// means the resolved clip admits no pixels.
    #[must_use]
    pub const fn bounds(&self) -> Option<Rectangle> {
        self.bounds
    }
}

fn valid_rectangle(rectangle: Rectangle) -> bool {
    rectangle.x.is_finite()
        && rectangle.y.is_finite()
        && rectangle.width.is_finite()
        && rectangle.height.is_finite()
        && rectangle.width >= 0.0
        && rectangle.height >= 0.0
        && (rectangle.x + rectangle.width).is_finite()
        && (rectangle.y + rectangle.height).is_finite()
}

fn non_empty(rectangle: Rectangle) -> Option<Rectangle> {
    (rectangle.width > 0.0 && rectangle.height > 0.0).then_some(rectangle)
}

fn union(left: Rectangle, right: Rectangle) -> Rectangle {
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    let right_edge = (left.x + left.width).max(right.x + right.width);
    let bottom_edge = (left.y + left.height).max(right.y + right.height);
    Rectangle::from_xywh(x, y, right_edge - x, bottom_edge - y)
}

fn intersection(left: Rectangle, right: Rectangle) -> Option<Rectangle> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom_edge = (left.y + left.height).min(right.y + right.height);
    (right_edge > x && bottom_edge > y)
        .then(|| Rectangle::from_xywh(x, y, right_edge - x, bottom_edge - y))
}
