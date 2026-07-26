//! `rframe` — the provisional source-neutral resolved render contract.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Contract-only: [`frame`] carries the
//! derived visual facts a producer emits after resolving its own source, and
//! nothing in this crate paints. The one downstream is the n0 engine kernel,
//! which compiles the contract into its private drawlist and painter — the
//! taken D-M vector join (see
//! [n0-join-point](../../../docs/wg/consolidation/n0-join-point.md)). The
//! temporary proving drawlist and painter that once lived here retired with
//! that decision. `websem` is the current producer; the source-neutrality
//! canary lives with the engine (`crates/n0/tests/glyphless_canary.rs`).
//!
//! The whole crate is backend-free (locked by `tests/architecture.rs`).

pub mod frame;
pub mod path;
pub mod stroke;

pub use frame::{
    Frame, FrameNode, Geometry, Identity, Provenance, SolidPaintStack, SolidPaintStackError,
    VisualRef,
};
pub use path::{FillRule, PathCommand, PathData, PathDataError};
pub use stroke::{Stroke, StrokeCap, StrokeError, StrokeJoin};
