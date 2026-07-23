//! The provisional source-neutral **resolved render contract** — `Frame`.
//!
//! This is *derived frame data*: normalized visual facts a producer emits
//! after it has resolved its own source. It is **not** an authored source of
//! truth, a file format, or a round-trip promise. It carries only what the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md) permits:
//! frame-local identity, geometry and resolved bounds, transforms, ordered
//! paint stacks, and clips. It carries **no** HTML/CSS/SVG syntax, no parser ASTs,
//! no producer bindings, no backend objects, and no serialization.
//!
//! It is deliberately minimal for the first slice (solid-fill rectangles) and
//! **breakable**: the enums grow as real producers force new visual facts, and
//! the sharing boundary moves *down* (toward the drawlist) rather than admit a
//! source-specific field.
//!
//! This module is Skia-free (enforced by `tests/architecture.rs`).

use math2::Rectangle;
use math2::transform::AffineTransform;

/// A resolved, straight-alpha RGBA color.
///
/// A deliberately minimal leaf, adopting **neither** existing paint vocabulary
/// (the `cg` crate's, nor the v2 engine's): the leaf-vocabulary seat is an
/// owner decision deferred to a later evidence spike, so this prototype keeps
/// the question open rather than pre-committing. See the Web-First Amendment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// One paint in an ordered paint stack. Solid only for the first slice.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Solid(Color),
}

/// An ordered paint stack, painted bottom entry first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintStack {
    pub paints: Vec<Paint>,
}

impl PaintStack {
    pub fn solid(color: Color) -> Self {
        Self {
            paints: vec![Paint::Solid(color)],
        }
    }
}

/// Resolved vector geometry, in the node's local space. Rectangles only for
/// the first slice; vector paths join here (they are not rasterized early —
/// see the amendment) as the corpus grows.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    Rect(Rectangle),
}

/// A source-neutral node identity within one frame product.
///
/// The first slice assigns these deterministically but does not preserve them
/// across source edits. They are therefore not yet valid cross-frame damage or
/// cache keys. Stable cross-frame identity and provenance enter only when a
/// real incremental producer forces that contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// One resolved node: identity, its local→frame transform, resolved geometry,
/// resolved bounds, an ordered paint stack, and an optional clip.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameNode {
    /// Source-neutral identity within this frame product.
    pub id: NodeId,
    /// Resolved transform mapping the node's local geometry into frame space.
    pub transform: AffineTransform,
    /// Resolved geometry, in local space.
    pub geometry: Geometry,
    /// Resolved axis-aligned bounds, in frame space.
    pub bounds: Rectangle,
    /// Ordered paint stack (bottom entry painted first).
    pub paints: PaintStack,
    /// Optional clip geometry, in frame space.
    pub clip: Option<Geometry>,
}

/// The resolved frame: an ordered list of nodes in painter order, plus the
/// frame's own bounds (the viewport the frame is clipped to).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The frame viewport, in frame space. Content is clipped to it.
    pub bounds: Rectangle,
    /// Resolved nodes, in painter order (first painted first).
    pub nodes: Vec<FrameNode>,
}
