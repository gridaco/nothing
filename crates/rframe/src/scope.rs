//! The compositing scope: the group fact of the resolved contract.
//!
//! Opacity, blend, and filter scopes state that the items they enclose
//! composite as **one isolated group** — their effect applies to the group's
//! composite, never per item. A clip scope instead constrains paint coverage
//! without isolation. A fact a producer *could* state on one paint pass is
//! stated there instead. [`crate::PaintAlphaFactor`], for
//! example, modulates each paint entry without isolation after its own alpha
//! materializes. A scope is the byte-distinct fact that no such per-paint
//! statement can express — a translucent group whose contents overlap, or a
//! fill and its stroke composited together.
//!
//! What a scope refuses is what this crate refuses: an effect that retains an
//! unresolved lookup or external handle stays a producer refusal by name.
//! Geometric clipping is carried as resolved path coverage. Image masking uses
//! its own checked two-phase [`crate::Mask`] contract. Image filtering enters
//! only as a fully resolved, bounded [`crate::FilterProgram`]; authored lookup
//! and names never cross the contract.

use crate::clip::ClipPath;
use crate::filter::Filter;
use crate::frame::VisualRef;

/// Why an opacity cannot be a scope fact.
///
/// A scope opacity lives in the **open** unit interval: for an opacity-only
/// effect, `1` is identity (the producer omits the scope) and `0` composites
/// nothing (the producer emits nothing). A [`ScopeBlend`] expresses unit
/// opacity with `None` while retaining its isolation boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeOpacityError {
    pub value: f32,
}

impl std::fmt::Display for ScopeOpacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scope opacity {} is outside the open unit interval",
            self.value
        )
    }
}

impl std::error::Error for ScopeOpacityError {}

/// A checked group opacity: finite and strictly between 0 and 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeOpacity(f32);

impl ScopeOpacity {
    pub fn new(value: f32) -> Result<Self, ScopeOpacityError> {
        if value.is_finite() && value > 0.0 && value < 1.0 {
            Ok(Self(value))
        } else {
            Err(ScopeOpacityError { value })
        }
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

/// The admitted blend functions for an isolated group's final composition.
///
/// These operate on unpremultiplied group and backdrop color channels; alpha
/// uses source-over for all three modes. This vocabulary is deliberately
/// narrower than [`cg::BlendMode`] and separate from [`crate::FilterBlend`]:
/// it admits neither arbitrary leaf blends nor two-image filter operations.
/// Additive composition and backdrop-preserving groups are inexpressible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeBlendMode {
    /// Use the completed group's color unchanged: `B(b, s) = s`.
    Normal,
    /// Multiply backdrop and completed-group color: `B(b, s) = b * s`.
    Multiply,
    /// Complement the product of complements: `B(b, s) = b + s - b * s`.
    Screen,
}

/// One isolated group's combined final blend and optional opacity.
///
/// Children paint in order against transparent black. Their completed
/// composite is the source; the enclosing composite at this position in
/// painter order is the backdrop. Apply the optional opacity to the completed
/// source's premultiplied color and alpha, then blend and composite it over
/// that backdrop using [`ScopeBlendMode`]. Opacity and blend belong to this
/// **one** final operation: splitting them into nested scopes changes the
/// backdrop seen by the blend and can add an intermediate quantization step.
/// Neither operation is distributed over child paints, fills, or strokes.
///
/// `None` means unit opacity, including for [`ScopeBlendMode::Normal`]. Such a
/// scope still isolates descendants and must not be erased just because its
/// final blend and opacity are neutral. Painting children directly into the
/// enclosing backdrop is represented by **no scope**. Zero opacity resolves
/// to no emitted group before construction, as with [`ScopeEffect::Opacity`].
///
/// Non-normal blending depends on the enclosing backdrop even when all
/// enclosed items are unchanged. Equality of this fact and its children does
/// not prove equality of the final blended result across different backdrops.
/// This names visual meaning, never a layer allocation, backdrop copy, cache
/// policy, or authored group.
///
/// ```
/// use rframe::{ScopeBlend, ScopeBlendMode, ScopeOpacity};
///
/// let isolated = ScopeBlend::new(ScopeBlendMode::Normal, None);
/// assert_eq!(isolated.opacity(), None); // unit opacity, still isolated
/// let translucent = ScopeBlend::new(
///     ScopeBlendMode::Multiply,
///     Some(ScopeOpacity::new(0.5)?),
/// );
/// assert_eq!(translucent.mode(), ScopeBlendMode::Multiply);
/// # Ok::<(), rframe::ScopeOpacityError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeBlend {
    mode: ScopeBlendMode,
    opacity: Option<ScopeOpacity>,
}

impl ScopeBlend {
    /// Combine an admitted blend function and already-checked opacity.
    /// `None` means opacity 1; it never means absence of isolation.
    #[must_use]
    pub const fn new(mode: ScopeBlendMode, opacity: Option<ScopeOpacity>) -> Self {
        Self { mode, opacity }
    }

    /// The blend function used only when the completed group joins its backdrop.
    #[must_use]
    pub const fn mode(self) -> ScopeBlendMode {
        self.mode
    }

    /// The final group opacity, or `None` for unit opacity.
    #[must_use]
    pub const fn opacity(self) -> Option<ScopeOpacity> {
        self.opacity
    }
}

/// The compositing effect a scope applies to its group's composite.
#[derive(Clone, Debug, PartialEq)]
pub enum ScopeEffect {
    /// The group composites at this opacity through one isolated layer.
    Opacity(ScopeOpacity),
    /// Isolate the group, then blend its completed composite with the enclosing
    /// backdrop at the optional opacity in one final operation. Normal blend
    /// at unit opacity still retains this scope's isolation boundary.
    Blend(ScopeBlend),
    /// Intersect every enclosed paint with resolved geometric coverage.
    /// Unlike opacity this creates no isolated layer: the clip is paint state,
    /// and its path facts reference no source or external resource.
    Clip(ClipPath),
    /// Apply one resolved image-filter program to the isolated group, hard
    /// clipped to its resolved effect region.
    Filter(Filter),
}

/// One compositing scope: its owner and its effect.
///
/// The owner is the source construct whose compositing the scope states
/// (a group element, a shape whose fill and stroke composite together) —
/// the same opaque identity/provenance a node carries, because damage and
/// diagnostics need to name a scope exactly as they name a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub owner: VisualRef,
    pub effect: ScopeEffect,
}
