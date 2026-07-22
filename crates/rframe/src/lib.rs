//! `rframe` — the Web-first track's shared downstream kernel.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Three layers:
//!
//! - [`frame`] — the source-neutral **resolved render contract** ([`Frame`]).
//!   Skia-free. The shared boundary; carries only derived visual facts.
//! - `drawlist` — rframe's **private** compiled form. Skia-free.
//! - `paint` — the **one**, `skia`-gated Skia painter; replays a drawlist, records no
//!   `SkPicture`.
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

pub use frame::{Color, Frame, FrameNode, Geometry, NodeId, Paint, PaintStack};
#[cfg(feature = "skia")]
pub use paint::{Raster, decode_png, render, render_png};
