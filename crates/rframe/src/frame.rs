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
//! ellipses, and paths, composited flat or through checked opacity, clip,
//! mask, and image-filter effects) and
//! **breakable**: the enums grow as real producers force new visual facts, and
//! the sharing boundary moves *down* (toward the engine's private drawlist)
//! rather than admit a source-specific field. One such move is decided, not
//! pending: the D-M text stage joined **low**, so this contract admits no
//! shaped-text fact — Web text arrives as resolved outline geometry ([the
//! text-stage
//! evidence](../../../docs/wg/consolidation/n0-join-point.md#the-text-stage-evidence)).
//! That decision leaned on this crate's *standing* identity — no fact that
//! references a resource — which phase 3 (images) may revisit on its own
//! evidence. `tests/architecture.rs` holds both refusals, each under its own
//! provenance.
//!
//! This crate is backend-free (enforced by `tests/architecture.rs`).

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;

use crate::mask::Mask;
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

/// Why a value cannot be carried as a post-paint alpha factor.
///
/// The factor is a normalized multiplier, so its complete domain is the
/// closed unit interval. A producer resolves a zero factor to no paint when it
/// attaches the factor to a [`PaintStack`]; keeping zero valid here lets that
/// normalization remain explicit and checked rather than relying on a raw
/// scalar at the call site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintAlphaFactorError {
    pub value: f32,
}

impl std::fmt::Display for PaintAlphaFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "paint alpha factor {} is outside the closed unit interval",
            self.value
        )
    }
}

impl std::error::Error for PaintAlphaFactorError {}

/// A checked factor applied after a paint entry's own alpha materializes.
///
/// This is not the paint's intrinsic opacity and not group opacity. A
/// consumer first materializes each [`cg::Paint`]'s own alpha, then multiplies
/// that result by this factor before coverage and source-over compositing. In
/// particular, it must not multiply the factor back into a gradient's own
/// opacity, because that changes the order of the two alpha operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintAlphaFactor(f32);

impl PaintAlphaFactor {
    /// The identity factor carried by every ordinary paint stack.
    pub const IDENTITY: Self = Self(1.0);

    /// Check one finite factor in `[0, 1]`.
    pub fn new(value: f32) -> Result<Self, PaintAlphaFactorError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            // Erase negative zero as a second spelling of the factor that
            // normalizes a stack to no paint.
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        } else {
            Err(PaintAlphaFactorError { value })
        }
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for PaintAlphaFactor {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A validated ordered stack of visible normal-blend `cg` paints: solids,
/// linear gradients, and radial gradients, plus one uniform post-paint alpha
/// factor.
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
///
/// The alpha factor applies independently to every entry, after that entry's
/// own alpha materializes and before it composites over the entries below it.
/// It is therefore not opacity over the already-composited stack and creates
/// no isolated group. A producer that needs to modulate the stack's composite
/// states a [`Scope`] instead. This order is equally defined for a one-paint
/// and a multi-paint stack; the factor never changes paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStack {
    paints: cg::Paints,
    alpha_factor: PaintAlphaFactor,
}

impl Default for PaintStack {
    fn default() -> Self {
        Self {
            paints: cg::Paints::default(),
            alpha_factor: PaintAlphaFactor::IDENTITY,
        }
    }
}

impl PaintStack {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn solid(color: cg::CGColor) -> Self {
        if color.a == 0 {
            return Self::empty();
        }
        Self {
            paints: cg::Paints::new([cg::Paint::Solid(cg::SolidPaint::new_color(color))]),
            alpha_factor: PaintAlphaFactor::IDENTITY,
        }
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
        Ok(Self {
            paints: cg::Paints::new(admitted),
            alpha_factor: PaintAlphaFactor::IDENTITY,
        })
    }

    /// Attach the factor applied after every entry's intrinsic paint alpha.
    ///
    /// A zero factor, or a factor attached to an already-empty stack,
    /// canonicalizes to [`PaintStack::empty`]: a resolved stack that paints
    /// nothing carries neither dormant paints nor a meaningless factor.
    #[must_use]
    pub fn with_alpha_factor(self, alpha_factor: PaintAlphaFactor) -> Self {
        if self.is_empty() || alpha_factor.get() == 0.0 {
            Self::empty()
        } else {
            Self {
                alpha_factor,
                ..self
            }
        }
    }

    /// The factor applied after each entry's own paint alpha materializes.
    #[must_use]
    pub const fn alpha_factor(&self) -> PaintAlphaFactor {
        self.alpha_factor
    }

    pub fn iter(&self) -> impl Iterator<Item = &cg::Paint> {
        self.paints.iter()
    }

    pub fn len(&self) -> usize {
        self.paints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paints.is_empty()
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
/// painter order. Balance, nesting depth, and meaningful content are
/// invariants of [`FrameItems`] construction, so a consumer matching on this
/// enum never meets a dangling boundary. The one meaningful empty span is a
/// filter whose declared transparent source can itself generate output.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameItem {
    /// One resolved painted node.
    Node(FrameNode),
    /// The following items, up to the matching [`FrameItem::ScopeEnd`],
    /// composite as one isolated group under this scope's effect.
    ScopeBegin(Scope),
    /// Closes the innermost open scope.
    ScopeEnd,
    /// Opens an isolated target composite for one resolved mask.
    MaskBegin(Mask),
    /// Ends the target phase and begins painting the mask source. This marker
    /// is valid only at the direct item depth of its matching
    /// [`FrameItem::MaskBegin`]. The source phase may be empty: a valid empty
    /// mask is transparent black and masks every target pixel.
    MaskSource,
    /// Closes the mask-source phase and composites the masked target.
    MaskEnd,
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
    /// empty group is not a resolved visual fact — except for a filter whose
    /// declared transparent source can itself generate output.
    EmptyScope { index: usize },
    /// The [`FrameItem::ScopeBegin`] at this index nests deeper than
    /// [`MAX_SCOPE_DEPTH`].
    ScopeTooDeep { index: usize },
    /// A mask begins after the checked nesting limit is already reached.
    MaskTooDeep { index: usize },
    /// A mask-source marker has no directly open target phase to switch.
    UnexpectedMaskSource { index: usize },
    /// A mask-source marker follows an empty target phase. There is no visual
    /// target fact for such a mask to affect.
    EmptyMaskTarget { index: usize },
    /// A mask end has no directly open source phase to close.
    UnexpectedMaskEnd { index: usize },
    /// A [`FrameItem::MaskBegin`] at this index never reaches a matching end.
    UnclosedMask { index: usize },
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
            FrameItemsError::MaskTooDeep { index } => write!(
                f,
                "the mask begun at item {index} nests deeper than {MAX_SCOPE_DEPTH}"
            ),
            FrameItemsError::UnexpectedMaskSource { index } => {
                write!(
                    f,
                    "item {index} begins a mask source without a directly open target"
                )
            }
            FrameItemsError::EmptyMaskTarget { index } => {
                write!(
                    f,
                    "the mask begun at item {index} encloses no target content"
                )
            }
            FrameItemsError::UnexpectedMaskEnd { index } => {
                write!(f, "item {index} ends no directly open mask source")
            }
            FrameItemsError::UnclosedMask { index } => {
                write!(f, "the mask begun at item {index} is never closed")
            }
        }
    }
}

impl std::error::Error for FrameItemsError {}

/// A checked painter-ordered item stream: every scope is balanced, meaningful,
/// and nested within [`MAX_SCOPE_DEPTH`]. Only a source-generating filter may
/// be meaningful without enclosed items.
///
/// Like [`PathData`], this is a checked type: construction proves the
/// invariants once, and a consumer trusts them rather than re-deriving
/// them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameItems(Vec<FrameItem>);

impl FrameItems {
    pub fn try_new(items: Vec<FrameItem>) -> Result<Self, FrameItemsError> {
        #[derive(Clone, Copy)]
        enum OpenKind {
            Scope { permits_empty: bool },
            MaskTarget,
            MaskSource,
        }

        #[derive(Clone, Copy)]
        struct Open {
            index: usize,
            kind: OpenKind,
            has_content: bool,
        }

        fn complete_item(open: &mut [Open]) {
            if let Some(parent) = open.last_mut() {
                parent.has_content = true;
            }
        }

        let mut open: Vec<Open> = Vec::new();
        for (index, item) in items.iter().enumerate() {
            match item {
                FrameItem::Node(_) => complete_item(&mut open),
                FrameItem::ScopeBegin(scope) => {
                    if open.len() >= MAX_SCOPE_DEPTH {
                        return Err(FrameItemsError::ScopeTooDeep { index });
                    }
                    open.push(Open {
                        index,
                        kind: OpenKind::Scope {
                            permits_empty: matches!(
                                &scope.effect,
                                crate::scope::ScopeEffect::Filter(filter)
                                    if filter.source_is_transparent()
                                        && filter.program().may_paint_transparent_input()
                            ),
                        },
                        has_content: false,
                    });
                }
                FrameItem::ScopeEnd => {
                    let Some(current) = open.last().copied() else {
                        return Err(FrameItemsError::UnopenedScopeEnd { index });
                    };
                    let OpenKind::Scope { permits_empty } = current.kind else {
                        return Err(FrameItemsError::UnopenedScopeEnd { index });
                    };
                    if !current.has_content && !permits_empty {
                        return Err(FrameItemsError::EmptyScope {
                            index: current.index,
                        });
                    }
                    open.pop();
                    complete_item(&mut open);
                }
                FrameItem::MaskBegin(_) => {
                    if open.len() >= MAX_SCOPE_DEPTH {
                        return Err(FrameItemsError::MaskTooDeep { index });
                    }
                    open.push(Open {
                        index,
                        kind: OpenKind::MaskTarget,
                        has_content: false,
                    });
                }
                FrameItem::MaskSource => {
                    let Some(current) = open.last_mut() else {
                        return Err(FrameItemsError::UnexpectedMaskSource { index });
                    };
                    if !matches!(current.kind, OpenKind::MaskTarget) {
                        return Err(FrameItemsError::UnexpectedMaskSource { index });
                    }
                    if !current.has_content {
                        return Err(FrameItemsError::EmptyMaskTarget {
                            index: current.index,
                        });
                    }
                    current.kind = OpenKind::MaskSource;
                    current.has_content = false;
                }
                FrameItem::MaskEnd => {
                    let Some(current) = open.last().copied() else {
                        return Err(FrameItemsError::UnexpectedMaskEnd { index });
                    };
                    if !matches!(current.kind, OpenKind::MaskSource) {
                        return Err(FrameItemsError::UnexpectedMaskEnd { index });
                    }
                    open.pop();
                    complete_item(&mut open);
                }
            }
        }
        if let Some(current) = open.first() {
            return Err(match current.kind {
                OpenKind::Scope { .. } => FrameItemsError::UnclosedScope {
                    index: current.index,
                },
                OpenKind::MaskTarget | OpenKind::MaskSource => FrameItemsError::UnclosedMask {
                    index: current.index,
                },
            });
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
            FrameItem::ScopeBegin(_)
            | FrameItem::ScopeEnd
            | FrameItem::MaskBegin(_)
            | FrameItem::MaskSource
            | FrameItem::MaskEnd => None,
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
