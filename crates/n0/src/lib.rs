//! n0 — the canvas engine on the `anchor` model ([`n0_model`], consumed
//! as a library).
//!
//! This crate is the pipeline:
//! `(document + immutable effective values) -> resolve -> drawlist -> paint`
//! (the browser's staged-and-pure discipline) plus the read tier
//! (`query`), time-as-data (`journal`/`replay`), and the sockets every
//! future optimization plugs into (`damage`, `ident`, `oracle`). The
//! contracts it encodes are catalogued in `archive/model-v2/anchor/ENGINE.md`
//! (ENG-0…ENG-5, archived); each module names the contract it serves.
//!
//! Host chrome (winit/egui/GL) lives in the host (`n0_dev`), never here.
//! Raster access is confined to [`paint`]; [`text_layout`] may use Skia
//! Paragraph only as an explicit shaping oracle.

pub mod cache;
pub mod damage;
pub mod drawlist;
pub mod frame;
pub mod ident;
pub mod journal;
pub mod oracle;
pub mod paint;
pub mod playback_clock;
pub mod query;
pub mod replay;
pub mod svg_animation_frame;
mod text_layout;
pub mod trace;
