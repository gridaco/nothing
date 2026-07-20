//! Shape building for rendering
//!
//! Model-agnostic shape abstraction ([`PainterShape`]) and path boolean
//! merging. The model-facing shape construction (`build_shape` and the
//! boolean-operation walks) lives in [`super::compile`] — the painter's
//! single model seam (gridaco/nothing#31).

use crate::shape::*;
use crate::{backends::skia::IntoSkia, cg::prelude::*};
use skia_safe::{Path, RRect, Rect};

/// Internal universal Painter's shape abstraction for optimized drawing
/// Virtual nodes like Group, BooleanOperation are not Painter's shapes, they use different methods.
#[derive(Debug, Clone)]
pub struct PainterShape {
    pub rect: Rect,
    pub rect_shape: Option<Rect>,
    pub rrect: Option<RRect>,
    pub oval: Option<Rect>,
    pub path: Option<Path>,
}

impl PainterShape {
    pub fn empty() -> Self {
        Self {
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            rect_shape: None,
            rrect: None,
            oval: None,
            path: None,
        }
    }
    /// Construct a plain rectangle shape
    pub fn from_rect(rect: impl Into<Rect>) -> Self {
        let r: Rect = rect.into();
        Self {
            rect: r,
            rect_shape: Some(r),
            rrect: None,
            oval: None,
            path: None,
        }
    }
    /// Construct a rounded rectangle shape
    pub fn from_rrect(rrect: RRect) -> Self {
        Self {
            rect: *rrect.rect(),
            rect_shape: None,
            rrect: Some(rrect),
            oval: None,
            path: None,
        }
    }
    /// Construct an oval/ellipse shape
    pub fn from_oval(rect: Rect) -> Self {
        Self {
            rect,
            rect_shape: None,
            rrect: None,
            oval: Some(rect),
            path: None,
        }
    }
    /// Construct a path-based shape (bounding rect must be provided)
    pub fn from_path(path: Path) -> Self {
        Self {
            rect: *path.bounds(),
            rect_shape: None,
            rrect: None,
            oval: None,
            path: Some(path),
        }
    }

    pub fn from_shape(shape: &Shape) -> Self {
        match shape {
            Shape::Ellipse(shape) => {
                PainterShape::from_oval(Rect::from_xywh(0.0, 0.0, shape.width, shape.height))
            }
            Shape::Rect(shape) => {
                PainterShape::from_rect(Rect::from_xywh(0.0, 0.0, shape.width, shape.height))
            }
            Shape::RRect(shape) => PainterShape::from_rrect(shape.into()),
            _ => PainterShape::from_path(shape.into()),
        }
    }

    /// Extract corner radii from the shape
    ///
    /// Returns corner radii [top-left, top-right, bottom-right, bottom-left]
    /// for shapes that support them (RRect). Returns uniform zeros for other shapes.
    ///
    /// # Returns
    /// - For RRect: Actual corner radii extracted from the shape (uses x-radius)
    /// - For Rect, Oval, Path: [0.0, 0.0, 0.0, 0.0] (no rounded corners)
    pub fn corner_radii(&self) -> [f32; 4] {
        if let Some(rrect) = &self.rrect {
            // Extract radii from RRect using radii_ref()
            // Returns [UpperLeft, UpperRight, LowerRight, LowerLeft]
            let radii = rrect.radii_ref();
            [
                radii[0].x, // top-left (UpperLeft)
                radii[1].x, // top-right (UpperRight)
                radii[2].x, // bottom-right (LowerRight)
                radii[3].x, // bottom-left (LowerLeft)
            ]
        } else {
            // No rounded corners for other shape types
            [0.0, 0.0, 0.0, 0.0]
        }
    }

    /// Return a new shape expanded (positive) or contracted (negative)
    /// uniformly on all sides. Corner radii are adjusted by the same
    /// amount (clamped to ≥ 0), matching CSS `box-shadow` spread.
    ///
    /// For ovals and arbitrary paths the expansion is approximated via
    /// the bounding rect.
    pub fn expanded_by(&self, amount: f32) -> Self {
        let rect = self.rect.with_outset((amount, amount));
        if let Some(rrect) = &self.rrect {
            let r = rrect.radii_ref();
            let adjust = |p: &skia_safe::Point| {
                skia_safe::Point::new((p.x + amount).max(0.0), (p.y + amount).max(0.0))
            };
            let new_radii = [adjust(&r[0]), adjust(&r[1]), adjust(&r[2]), adjust(&r[3])];
            let mut rr = RRect::new();
            rr.set_rect_radii(rect, &new_radii);
            Self::from_rrect(rr)
        } else if self.rect_shape.is_some() {
            Self::from_rect(rect)
        } else if self.oval.is_some() {
            Self::from_oval(rect)
        } else {
            Self::from_rect(rect)
        }
    }

    pub fn to_path(&self) -> Path {
        if let Some(rect) = self.rect_shape {
            Path::rect(rect, None)
        } else if let Some(rrect) = &self.rrect {
            Path::rrect(rrect, None)
        } else if let Some(oval) = &self.oval {
            Path::oval(oval, None)
        } else if let Some(existing_path) = &self.path {
            existing_path.clone()
        } else {
            // Fallback to rect if no specific shape is set
            Path::rect(self.rect, None)
        }
    }

    /// Draw the shape directly on the canvas using the most efficient Skia
    /// primitive for the shape type.
    ///
    /// For simple shapes (rect, rrect, oval), this avoids creating an
    /// intermediate `Path` object and uses Skia's specialized GPU draw calls
    /// (`draw_rect`, `draw_rrect`, `draw_oval`) which have lower overhead
    /// than `draw_path`.
    #[inline]
    pub fn draw_on_canvas(&self, canvas: &skia_safe::Canvas, paint: &skia_safe::Paint) {
        if let Some(rect) = self.rect_shape {
            canvas.draw_rect(rect, paint);
        } else if let Some(rrect) = &self.rrect {
            canvas.draw_rrect(rrect, paint);
        } else if let Some(oval) = &self.oval {
            canvas.draw_oval(oval, paint);
        } else if let Some(existing_path) = &self.path {
            canvas.draw_path(existing_path, paint);
        } else {
            canvas.draw_rect(self.rect, paint);
        }
    }

    /// Clip the canvas to this shape using the most efficient Skia primitive.
    ///
    /// For rect/rrect shapes, uses `clip_rect`/`clip_rrect` which are faster
    /// than `clip_path`. Falls back to `clip_path` for ovals and complex paths.
    #[inline]
    pub fn clip_on_canvas(&self, canvas: &skia_safe::Canvas) {
        if let Some(rect) = self.rect_shape {
            canvas.clip_rect(rect, None, true);
        } else if let Some(rrect) = &self.rrect {
            canvas.clip_rrect(rrect, None, true);
        } else if let Some(oval) = &self.oval {
            canvas.clip_path(&Path::oval(oval, None), None, true);
        } else if let Some(existing_path) = &self.path {
            canvas.clip_path(existing_path, None, true);
        } else {
            canvas.clip_rect(self.rect, None, true);
        }
    }

    pub fn is_closed(&self) -> bool {
        if let Some(path) = &self.path {
            path.is_last_contour_closed()
        } else {
            true
        }
    }
}

/// Merges multiple shapes into a single path using boolean operations.
///
/// This function takes a list of shapes and their corresponding boolean operations,
/// and merges them into a single path. The first shape is used as the base,
/// and subsequent shapes are combined using the specified operations.
///
/// # Parameters
///
/// - `shapes`: A slice of tuples containing (PainterShape, BooleanPathOperation)
///   The first shape is used as the base, subsequent shapes are combined with the base
///   using their respective operations.
///
/// # Returns
///
/// A merged `Path` representing the result of all boolean operations.
/// If no shapes are provided, returns an empty path.
///
/// # Example
///
/// ```rust,ignore
/// let shapes = vec![
///     (shape1, BooleanPathOperation::Union),
///     (shape2, BooleanPathOperation::Intersection),
/// ];
/// let merged_path = merge_shapes(&shapes);
/// ```
pub fn merge_shapes(shapes: &[(PainterShape, BooleanPathOperation)]) -> Path {
    if shapes.is_empty() {
        return Path::new();
    }

    let mut result = shapes[0].0.to_path();

    for (shape, operation) in shapes.iter().skip(1) {
        let shape_path = shape.to_path();
        if let Some(merged) = Path::op(&result, &shape_path, (*operation).into_skia()) {
            result = merged;
        }
    }

    result
}
