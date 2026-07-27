//! The provisional source-neutral **resolved render contract** — `Frame`.
//!
//! This is *derived frame data*: normalized visual facts a producer emits
//! after it has resolved its own source. It is **not** an authored source of
//! truth, a file format, or a round-trip promise. It carries only what the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md) permits:
//! opaque identity/provenance, geometry and resolved bounds, transforms,
//! ordered paint stacks, and the frame viewport clip. It carries **no**
//! HTML/CSS/SVG syntax, no parser ASTs, no producer bindings, no backend
//! objects, and no serialization.
//!
//! It is deliberately minimal (solid-fill rectangles, ellipses, and paths) and
//! **breakable**: the enums grow as real producers force new visual facts, and
//! the sharing boundary moves *down* (toward the engine's private drawlist)
//! rather than admit a source-specific field.
//!
//! This crate is backend-free (enforced by `tests/architecture.rs`).

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::path::PathData;
use crate::stroke::Stroke;

/// Why a producer paint stack cannot enter the current solid-only contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolidPaintStackError {
    pub index: usize,
}

impl std::fmt::Display for SolidPaintStackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "visible paint {} is outside the ordinary-solid resolved scope",
            self.index
        )
    }
}

impl std::error::Error for SolidPaintStackError {}

/// A validated ordered stack of visible ordinary solid `cg` paints.
///
/// This is an admitted subset of the shared leaf vocabulary, not a competing
/// paint vocabulary. Construction removes paints with no visual effect and
/// rejects every visible variant or blend mode outside the current scope.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolidPaintStack(cg::Paints);

impl SolidPaintStack {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn solid(color: cg::CGColor) -> Self {
        if color.a == 0 {
            return Self::empty();
        }
        Self(cg::Paints::new([cg::Paint::Solid(
            cg::SolidPaint::new_color(color),
        )]))
    }

    pub fn try_from_paints(paints: cg::Paints) -> Result<Self, SolidPaintStackError> {
        let mut admitted = Vec::with_capacity(paints.len());
        for (index, paint) in paints.into_iter().enumerate() {
            if !paint.visible() {
                continue;
            }
            match paint {
                cg::Paint::Solid(solid) if solid.blend_mode == cg::BlendMode::Normal => {
                    admitted.push(cg::Paint::Solid(solid));
                }
                _ => return Err(SolidPaintStackError { index }),
            }
        }
        Ok(Self(cg::Paints::new(admitted)))
    }

    pub fn iter(&self) -> impl Iterator<Item = &cg::SolidPaint> {
        self.0.iter().map(|paint| match paint {
            cg::Paint::Solid(solid) => solid,
            _ => unreachable!("SolidPaintStack construction closes the variant set"),
        })
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Opaque source-neutral identity within one frame product.
///
/// A retained producer may preserve the value across related samples, but a
/// one-shot producer does not thereby promise cross-product continuity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity(u64);

impl Identity {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Opaque product-local provenance token for diagnostics and host projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Provenance(u64);

impl Provenance {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The complete source-neutral owner of one resolved visual fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualRef {
    identity: Identity,
    provenance: Provenance,
}

impl VisualRef {
    pub const fn new(identity: Identity, provenance: Provenance) -> Self {
        Self {
            identity,
            provenance,
        }
    }

    pub const fn identity(self) -> Identity {
        self.identity
    }

    pub const fn provenance(self) -> Provenance {
        self.provenance
    }
}

/// Resolved vector geometry, in the node's local space. Rectangles, ellipses,
/// and vector paths — which join as resolved command streams, never
/// rasterized early (see the amendment).
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect(Rectangle),
    /// The axis-aligned ellipse inscribed in this local-space rectangle.
    Ellipse(Rectangle),
    /// A checked canonical command stream with its fill rule. Shared rather
    /// than copied: one resolved path is read by every frame a retained
    /// producer emits.
    Path(Arc<PathData>),
}

impl Geometry {
    /// The local-space box the node's `bounds` maps: the rectangle itself,
    /// the ellipse's inscribing rectangle, or the path's tight extent.
    #[must_use]
    pub fn local_box(&self) -> Rectangle {
        match self {
            Geometry::Rect(rect) | Geometry::Ellipse(rect) => *rect,
            Geometry::Path(path) => path.local_bounds(),
        }
    }
}

/// One resolved node: identity, its local→frame transform, resolved geometry,
/// resolved bounds, the fill's paint stack, and an optional stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameNode {
    /// Source-neutral product identity and opaque diagnostic provenance.
    pub owner: VisualRef,
    /// Resolved transform mapping the node's local geometry into frame space.
    pub transform: AffineTransform,
    /// Resolved geometry, in local space.
    pub geometry: Geometry,
    /// Resolved axis-aligned bounds of the **geometry**, in frame space.
    ///
    /// A stroke paints outside this box (see [`Stroke::outset`]), so a consumer
    /// that needs the covered area — damage, culling — must inflate it. The
    /// field is the geometry's bounds and not the ink's because that is the
    /// quantity a producer can state exactly, and the exact-bounds law depends
    /// on it being exact.
    pub bounds: Rectangle,
    /// Ordered fill paint stack (bottom entry painted first).
    pub paints: SolidPaintStack,
    /// The resolved stroke, painted over the fill. `None` when nothing is
    /// stroked — construction never yields an invisible stroke.
    pub stroke: Option<Stroke>,
}

/// The resolved frame: an ordered list of nodes in painter order, plus the
/// frame's own bounds (the viewport the frame is clipped to).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Source-neutral product identity and opaque diagnostic provenance.
    pub owner: VisualRef,
    /// The frame viewport, in frame space. Content is clipped to it.
    pub bounds: Rectangle,
    /// Resolved nodes, in painter order (first painted first).
    pub nodes: Vec<FrameNode>,
}
