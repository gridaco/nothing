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
//! It is deliberately minimal (solid- and gradient-filled rectangles,
//! ellipses, and paths, composited flat or through isolated opacity
//! scopes) and
//! **breakable**: the enums grow as real producers force new visual facts, and
//! the sharing boundary moves *down* (toward the engine's private drawlist)
//! rather than admit a source-specific field.
//!
//! This crate is backend-free (enforced by `tests/architecture.rs`).

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::path::PathData;
use crate::scope::Scope;
use crate::stroke::Stroke;

/// Why a producer paint stack cannot enter the admitted resolved contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintStackError {
    pub index: usize,
}

impl std::fmt::Display for PaintStackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "visible paint {} is outside the admitted resolved paint scope",
            self.index
        )
    }
}

impl std::error::Error for PaintStackError {}

/// A validated ordered stack of visible normal-blend `cg` paints: solids,
/// linear gradients, and radial gradients.
///
/// This is an admitted subset of the shared leaf vocabulary, not a competing
/// paint vocabulary. Construction removes paints with no visual effect and
/// rejects every visible variant or blend mode outside the admitted set —
/// sweep and diamond gradients, image paints, and non-normal blends stay
/// producer refusals.
///
/// A gradient's geometry is stated in the unit square of the item's **paint
/// box** — the tight local bounds of the geometry the stack paints (for a
/// stroke, the stroked geometry's own box, never the stroke's inked reach; a
/// degenerate box axis is the consumer's stable one-pixel interval). The
/// gradient's transform composes in that unit space. A producer resolves its
/// source's coordinate systems into these unit-box facts; no source vocabulary
/// (units, references, spread keywords) crosses the contract.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintStack(cg::Paints);

impl PaintStack {
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

    pub fn try_from_paints(paints: cg::Paints) -> Result<Self, PaintStackError> {
        let mut admitted = Vec::with_capacity(paints.len());
        for (index, paint) in paints.into_iter().enumerate() {
            if !paint.visible() {
                continue;
            }
            if paint.blend_mode() != cg::BlendMode::Normal {
                return Err(PaintStackError { index });
            }
            match paint {
                cg::Paint::Solid(_)
                | cg::Paint::LinearGradient(_)
                | cg::Paint::RadialGradient(_) => {
                    admitted.push(paint);
                }
                _ => return Err(PaintStackError { index }),
            }
        }
        Ok(Self(cg::Paints::new(admitted)))
    }

    pub fn iter(&self) -> impl Iterator<Item = &cg::Paint> {
        self.0.iter()
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
    pub paints: PaintStack,
    /// The resolved stroke, painted over the fill. `None` when nothing is
    /// stroked — construction never yields an invisible stroke.
    pub stroke: Option<Stroke>,
}

/// One entry of the painter-ordered item stream: a painted node, or a
/// scope boundary.
///
/// A scope's contents are the items between its begin and its end — a
/// contiguous span, because an isolated group *is* a contiguous span of
/// painter order. Balance, nesting depth, and non-emptiness are invariants
/// of [`FrameItems`] construction, so a consumer matching on this enum
/// never meets a dangling boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameItem {
    /// One resolved painted node.
    Node(FrameNode),
    /// The following items, up to the matching [`FrameItem::ScopeEnd`],
    /// composite as one isolated group under this scope's effect.
    ScopeBegin(Scope),
    /// Closes the innermost open scope.
    ScopeEnd,
}

/// The deepest scope nesting a checked stream admits. Mirrors the
/// producer-side container bound so a within-slice source can never
/// out-nest its own contract.
pub const MAX_SCOPE_DEPTH: usize = 64;

/// Why a producer item stream cannot enter the admitted resolved contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameItemsError {
    /// A [`FrameItem::ScopeEnd`] at this index closes no open scope.
    UnopenedScopeEnd { index: usize },
    /// The [`FrameItem::ScopeBegin`] at this index is never closed.
    UnclosedScope { index: usize },
    /// The [`FrameItem::ScopeBegin`] at this index encloses nothing. An
    /// empty group is not a resolved visual fact — a producer whose scope
    /// resolved to nothing states nothing.
    EmptyScope { index: usize },
    /// The [`FrameItem::ScopeBegin`] at this index nests deeper than
    /// [`MAX_SCOPE_DEPTH`].
    ScopeTooDeep { index: usize },
}

impl std::fmt::Display for FrameItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameItemsError::UnopenedScopeEnd { index } => {
                write!(f, "item {index} ends a scope no begin opened")
            }
            FrameItemsError::UnclosedScope { index } => {
                write!(f, "the scope begun at item {index} is never closed")
            }
            FrameItemsError::EmptyScope { index } => {
                write!(f, "the scope begun at item {index} encloses nothing")
            }
            FrameItemsError::ScopeTooDeep { index } => write!(
                f,
                "the scope begun at item {index} nests deeper than {MAX_SCOPE_DEPTH}"
            ),
        }
    }
}

impl std::error::Error for FrameItemsError {}

/// A checked painter-ordered item stream: every scope is balanced,
/// non-empty, and nested within [`MAX_SCOPE_DEPTH`].
///
/// Like [`PathData`], this is a checked type: construction proves the
/// invariants once, and a consumer trusts them rather than re-deriving
/// them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameItems(Vec<FrameItem>);

impl FrameItems {
    pub fn try_new(items: Vec<FrameItem>) -> Result<Self, FrameItemsError> {
        let mut open = Vec::new();
        for (index, item) in items.iter().enumerate() {
            match item {
                FrameItem::Node(_) => {}
                FrameItem::ScopeBegin(_) => {
                    if open.len() >= MAX_SCOPE_DEPTH {
                        return Err(FrameItemsError::ScopeTooDeep { index });
                    }
                    open.push(index);
                }
                FrameItem::ScopeEnd => {
                    let Some(begin) = open.pop() else {
                        return Err(FrameItemsError::UnopenedScopeEnd { index });
                    };
                    if begin + 1 == index {
                        return Err(FrameItemsError::EmptyScope { index: begin });
                    }
                }
            }
        }
        if let Some(&begin) = open.first() {
            return Err(FrameItemsError::UnclosedScope { index: begin });
        }
        Ok(Self(items))
    }

    /// A scope-free stream is trivially checked.
    pub fn from_nodes(nodes: Vec<FrameNode>) -> Self {
        Self(nodes.into_iter().map(FrameItem::Node).collect())
    }

    pub fn iter(&self) -> impl Iterator<Item = &FrameItem> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The painted nodes in painter order, flattened across scopes.
    pub fn nodes(&self) -> impl Iterator<Item = &FrameNode> {
        self.0.iter().filter_map(|item| match item {
            FrameItem::Node(node) => Some(node),
            FrameItem::ScopeBegin(_) | FrameItem::ScopeEnd => None,
        })
    }
}

impl<'a> IntoIterator for &'a FrameItems {
    type Item = &'a FrameItem;
    type IntoIter = std::slice::Iter<'a, FrameItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// The resolved frame: a checked item stream in painter order, plus the
/// frame's own bounds (the viewport the frame is clipped to).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Source-neutral product identity and opaque diagnostic provenance.
    pub owner: VisualRef,
    /// The frame viewport, in frame space. Content is clipped to it.
    pub bounds: Rectangle,
    /// Painter-ordered items (first painted first).
    pub items: FrameItems,
}

impl Frame {
    /// The painted nodes in painter order, flattened across scopes —
    /// indexable convenience for laws and diagnostics; a consumer that
    /// composites walks [`Frame::items`] instead.
    pub fn nodes(&self) -> Vec<&FrameNode> {
        self.items.nodes().collect()
    }
}
