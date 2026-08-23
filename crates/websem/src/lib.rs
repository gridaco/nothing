//! `websem` — the Web-first track's Web semantic front-end.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Compiles the Web semantic family —
//! HTML-with-inline-SVG and standalone SVG — into the source-neutral
//! [`rframe::Frame`], through **one** namespace-aware document
//! (`csscascade::DemoDom`) and **one** browser-grade cascade
//! (`csscascade::CascadeDriver`, Stylo). Inline SVG is compiled in place from
//! the shared document — never serialized and reparsed — and its descendant
//! style comes from the surrounding cascade. Standalone SVG enters through
//! csscascade's conforming namespace-aware XML grammar into the same
//! semantic document shape; recorded XML recoveries are refused explicitly.
//!
//! This crate produces the contract; it does not paint. It touches no legacy
//! import path, no node model, no `.grida` codec, and no backend
//! (`tests/architecture.rs` locks these out). See the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md).
//!
//! [`SvgFrameSource`] retains source + cascade state and emits immutable Base
//! or explicit-time frames for the first admitted rect-x animation slice.
//! Base and Sample(time) resolve to one Web-owned effective-value view and
//! compile through the same SVG compiler — time changes effective values,
//! never which compiler runs, and no compiled frame is mutated afterward. The
//! exact sampler is shared; SVG syntax and target association remain here.

mod effective_values;
pub mod svg;
mod svg_animation;
mod svg_number;
mod svg_paint_server;
mod svg_path;
mod svg_path_length;
mod svg_path_length_metric;
mod svg_text;
mod svg_transform;

pub use svg::{
    CompileError, Degradation, DegradationAction, InitialViewport, SvgFrameSource,
    compile_html_inline_svg, compile_standalone_svg,
};
pub use svg_animation::AnimationError;
