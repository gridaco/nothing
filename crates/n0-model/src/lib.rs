//! n0-model — the `anchor` model crate (formerly the model-v2 proving lab).
//!
//! Implements the model of `model-v2/models/anchor.md` (the archived spec draft),
//! proven by the experiment ledger of `model-v2/anchor/README.md`:
//! - rotation-in-flow (both semantics behind [`resolve::RotationInFlow`])
//! - the agent text IR ([`textir`])
//! - the resolver ([`resolve`])
//!
//! Consumed as a library by `n0` (the engine); serialization
//! (`Op`/`ResizeDrag`/`Axis`) is gated behind the optional `serde` feature.
//! This crate stays skia-free — backends live in its consumers.

pub mod animation;
pub mod math;
pub mod measure;
pub mod model;
pub mod n0_xml;
pub mod n0_xml_source;
pub mod ops;
pub mod path;
pub mod pick;
pub mod properties;
pub mod renderability;
pub mod resolve;
pub mod rounded_box;
pub mod svg_animation;
pub mod svgout;
pub mod text_layout;
pub mod textir;
