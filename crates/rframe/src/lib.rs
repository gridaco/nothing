//! `rframe` — the Web-first track's shared downstream kernel.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Three layers:
//!
//! - [`frame`] — the source-neutral **resolved render contract** ([`Frame`]).
//!   Skia-free. The shared boundary; carries only derived visual facts.
//! - [`drawlist`] — rframe's **private** compiled form. Skia-free.
//! - [`paint`] — the **one** Skia painter; replays a drawlist, records no
//!   `SkPicture`.
//!
//! The pipeline is `Frame → drawlist::build → paint::paint`. Two real
//! producers shape the `Frame`: the `websem` Web semantic front-end and the
//! n0 canary (`tests/n0_canary.rs`) — so a later owner evidence spike can
//! decide where each producer joins. See the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md).
//!
//! `use skia_safe` is confined to [`paint`]; `tests/architecture.rs` locks the
//! contract and the drawlist Skia-free.

pub mod drawlist;
pub mod frame;
pub mod paint;

pub use drawlist::{DrawItem, DrawList};
pub use frame::{Color, Frame, FrameNode, Geometry, NodeId, Paint, PaintStack};
pub use paint::{Raster, decode_png, raster, render, render_png};
