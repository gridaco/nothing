//! `rframe` — the provisional source-neutral resolved-frame contract.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. The permanent role being proved here is
//! the Skia-free [`Frame`] contract. The current private drawlist and painter
//! remain only as replacement evidence while n0 adopts the contract:
//!
//! - [`frame`] — the source-neutral **resolved render contract** ([`Frame`]).
//!   Skia-free. The shared boundary; carries only derived visual facts.
//! - `drawlist` and `paint` — a temporary `skia`-gated proving downstream,
//!   retained only until byte-identical replacement through n0.
//!
//! The pipeline is `Frame → drawlist::build → paint::paint`. `websem` is the
//! current producer; the n0 canary (`tests/n0_canary.rs`) exercises the same
//! contract with real n0 resolved data but is not a second production consumer
//! or API-promotion evidence. A later owner evidence spike decides where each
//! producer joins. See the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md).
//!
//! `use skia_safe` is confined to `paint`; `tests/architecture.rs` locks the
//! contract and the drawlist Skia-free.

#[cfg(feature = "skia")]
mod drawlist;
pub mod frame;
#[cfg(feature = "skia")]
mod paint;

pub use frame::{
    Frame, FrameNode, Geometry, Identity, Provenance, SolidPaintStack, SolidPaintStackError,
    VisualRef,
};
#[cfg(feature = "skia")]
pub use paint::{Raster, decode_png, render, render_png};
