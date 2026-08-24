//! The compositing scope: the group fact of the resolved contract.
//!
//! A scope states that the items it encloses composite as **one isolated
//! group** — its effect applies to the group's composite, never per item.
//! That is the only thing a scope is for: a fact a producer *could* state on
//! one paint pass is stated there instead. [`crate::PaintAlphaFactor`], for
//! example, modulates each paint entry without isolation after its own alpha
//! materializes. A scope is the byte-distinct fact that no such per-paint
//! statement can express — a translucent group whose contents overlap, or a
//! fill and its stroke composited together.
//!
//! What a scope refuses is what this crate refuses: an effect that
//! references a resource (a mask image, a filter graph, a pattern) has no
//! representation here and stays a producer refusal by name. Geometric
//! clipping is the source-neutral exception carried as resolved path coverage.
//! Image masking uses its own checked two-phase [`crate::Mask`] contract rather
//! than becoming a resource-bearing [`ScopeEffect`]; filter graphs remain
//! outside this crate.

use crate::clip::ClipPath;
use crate::frame::VisualRef;

/// Why an opacity cannot be a scope fact.
///
/// A scope opacity lives in the **open** unit interval: `1` is identity
/// (the producer omits the scope) and `0` composites nothing (the
/// producer emits nothing), so neither is a fact a scope can carry.
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

/// The compositing effect a scope applies to its group's composite.
#[derive(Clone, Debug, PartialEq)]
pub enum ScopeEffect {
    /// The group composites at this opacity through one isolated layer.
    Opacity(ScopeOpacity),
    /// Intersect every enclosed paint with resolved geometric coverage.
    /// Unlike opacity this creates no isolated layer: the clip is paint state,
    /// and its path facts reference no source or external resource.
    Clip(ClipPath),
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
