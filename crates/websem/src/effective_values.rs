//! The Web-owned effective-value view.
//!
//! One frame request — Base or Sample(time) — resolves to this view before
//! compilation, and the one SVG compiler reads attribute values through it.
//! It answers the *effective* (presentation) value of an attribute on a
//! source node: Base carries no animated contributions, and a sample overlays
//! the admitted animation's value at its exact requested time. The view is
//! read-only — producing or consuming it never writes to the document — and
//! time changes only the values it carries, never which compiler runs.

use csscascade::dom::NodeId;

/// Effective attribute values for one frame request.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EffectiveValues {
    /// Animated scalar attribute overrides: (target node, attribute local
    /// name, effective value). Authored values apply wherever no entry
    /// matches. The admitted rect-x slice carries at most one entry.
    scalar_overrides: Vec<(NodeId, String, f32)>,
}

impl EffectiveValues {
    /// The Base view: animation contributes no values.
    pub(crate) const fn base() -> Self {
        Self {
            scalar_overrides: Vec::new(),
        }
    }

    /// A sampled view carrying one animated scalar attribute value.
    pub(crate) fn with_scalar(node: NodeId, attribute: impl Into<String>, value: f32) -> Self {
        Self {
            scalar_overrides: vec![(node, attribute.into(), value)],
        }
    }

    /// The animated override for `attribute` on `node`, if this request
    /// carries one.
    pub(crate) fn scalar(&self, node: NodeId, attribute: &str) -> Option<f32> {
        self.scalar_overrides
            .iter()
            .find(|(target, name, _)| *target == node && name == attribute)
            .map(|(_, _, value)| *value)
    }
}
