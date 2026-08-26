//! `rframe` — the provisional source-neutral resolved render contract.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Contract-only: [`Frame`] carries the
//! derived visual facts a producer emits after resolving its own source, and
//! nothing in this crate paints. The one downstream is the n0 engine kernel,
//! which compiles the contract into its private drawlist and painter — the
//! taken vector join (see
//! [n0-join-point](../../../docs/wg/consolidation/n0-join-point.md)). The
//! temporary proving drawlist and painter that once lived here retired with
//! that decision. `websem` is the current producer; the source-neutrality
//! canary lives with the engine (`crates/n0/tests/glyphless_canary.rs`).
//!
//! The whole crate is backend-free (locked by `tests/architecture.rs`).

mod clip;
mod filter;
mod frame;
mod mask;
mod path;
mod scope;
mod stroke;

pub use clip::{
    ClipGeometry, ClipGeometryError, ClipLayer, ClipLayerError, ClipPath, ClipPathError,
    MAX_CLIP_GEOMETRIES_PER_LAYER, MAX_CLIP_LAYERS,
};
pub use filter::{
    Filter, FilterBlend, FilterChannelTables, FilterColorSpace, FilterComposite,
    FilterConvolveEdgeMode, FilterDisplacementChannel, FilterError, FilterInput, FilterLightSource,
    FilterMorphology, FilterNode, FilterPrimitive, FilterProgram, FilterProgramError,
    FilterTurbulenceKind, MAX_FILTER_CONVOLVE_KERNEL_VALUES, MAX_FILTER_NODES,
};
pub use frame::{
    Frame, FrameItem, FrameItems, FrameItemsError, FrameNode, Geometry, Identity, MAX_SCOPE_DEPTH,
    PaintAlphaFactor, PaintAlphaFactorError, PaintStack, PaintStackError, Provenance, VisualRef,
};
pub use mask::{Mask, MaskMode};
pub use path::{FillRule, PathCommand, PathData, PathDataError};
pub use scope::{Scope, ScopeEffect, ScopeOpacity, ScopeOpacityError};
pub use stroke::{
    Stroke, StrokeCap, StrokeDash, StrokeDashError, StrokeDashIntervals, StrokeDashIntervalsError,
    StrokeError, StrokeJoin,
};
